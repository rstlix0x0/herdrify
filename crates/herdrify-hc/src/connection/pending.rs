//! The request-id allocator and the table of calls waiting for their reply.
//!
//! Every call takes the next id and registers a one-shot channel under it. The router takes the
//! entry when the reply with the same id arrives and completes its channel. A call that is dropped
//! removes its own entry, so a reply that arrives later finds no entry and is discarded. A call that
//! opens a stream also registers an [`Opening`], so the router can build the stream's handle when
//! the reply arrives.
//!
//! Responsibilities:
//!
//! - [`Pending`], which hands out request ids from 1 upward and holds each waiting call's channel
//!   and its [`Opening`], if it has one
//! - [`Answer`], what a waiting call receives: the outcome, and the stream handle if one was opened
//! - taking and removing entries, and failing all of them when the connection ends
//!
//! Non-responsibilities:
//!
//! - reading frames or writing them, which the router and the writer do
//! - deciding what a reply means to the caller, which the client does

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde_json::Value;
use tokio::sync::oneshot;

use super::error::{ConnectionEndKind, EndRecord};
use super::stream::{Opening, StreamHandle};
use crate::protocol::{RequestId, RpcError};

/// The outcome of a call: the reply's `result`, or the refusal the core sent.
pub(super) type Outcome = Result<Value, RpcError>;

/// What a waiting call receives when its reply arrives.
pub(super) struct Answer {
    /// The reply's `result`, or the refusal the core sent.
    pub(super) outcome: Outcome,
    /// The stream the reply opened. Set only for a call registered with an [`Opening`], when the
    /// reply is a success that names a channel.
    pub(super) stream: Option<StreamHandle>,
}

/// A call waiting for its reply.
pub(super) struct Waiter {
    /// Where the answer goes.
    pub(super) reply: oneshot::Sender<Answer>,
    /// Set when the call opens a stream.
    pub(super) opening: Option<Opening>,
}

/// The calls waiting for a reply, and the next id to hand out.
///
/// The handshake used id 0, so the first call takes 1.
pub(super) struct Pending {
    state: Mutex<State>,
}

struct State {
    next: u64,
    waiting: HashMap<RequestId, Waiter>,
}

impl Pending {
    pub(super) fn new() -> Self {
        Self {
            state: Mutex::new(State {
                next: 1,
                waiting: HashMap::new(),
            }),
        }
    }

    /// Takes the next id and registers a call under it. The receiver resolves with the answer, or
    /// is dropped unresolved when the entry is removed or the connection fails.
    pub(super) fn register(&self) -> (RequestId, oneshot::Receiver<Answer>) {
        self.insert(None)
    }

    /// As [`Pending::register`], for a call whose successful reply opens a stream: the router
    /// builds the stream's handle from `opening` when that reply arrives.
    pub(super) fn register_opening(
        &self,
        opening: Opening,
    ) -> (RequestId, oneshot::Receiver<Answer>) {
        self.insert(Some(opening))
    }

    fn insert(&self, opening: Option<Opening>) -> (RequestId, oneshot::Receiver<Answer>) {
        let (reply, receiver) = oneshot::channel();
        let id = {
            let mut state = self.lock();
            let id = RequestId(state.next);
            state.next += 1;
            state.waiting.insert(id, Waiter { reply, opening });
            id
        };
        (id, receiver)
    }

    /// Removes and returns the call registered under `id`, if any.
    pub(super) fn take(&self, id: RequestId) -> Option<Waiter> {
        self.lock().waiting.remove(&id)
    }

    /// Forgets the call registered under `id`, if any.
    pub(super) fn remove(&self, id: RequestId) {
        self.lock().waiting.remove(&id);
    }

    /// Ends the connection: records `message` in `record` unless an end is already recorded, then
    /// fails every waiting call.
    ///
    /// The end is recorded first. A call that registers after the entries were dropped therefore
    /// finds the end already recorded when it looks.
    pub(super) fn end(&self, record: &EndRecord, kind: ConnectionEndKind, message: String) {
        record.record(kind, message);
        self.fail_all();
    }

    /// Drops every entry, so each waiting call sees its channel close.
    fn fail_all(&self) {
        let waiting = std::mem::take(&mut self.lock().waiting);
        drop(waiting);
    }

    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.lock().waiting.is_empty()
    }

    /// Holds the table's lock for as long as the returned value lives, so a test can stop
    /// [`Pending::end`] part-way.
    #[cfg(test)]
    pub(super) fn hold(&self) -> impl Sized + '_ {
        self.lock()
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::sync::Arc;

    use serde_json::json;

    use tokio::sync::mpsc;

    use super::{Answer, Pending};
    use crate::connection::ConnectionEndKind;
    use crate::connection::error::EndRecord;
    use crate::connection::stream::Opening;
    use crate::protocol::RequestId;

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn ids_start_at_one_and_increase() {
        let pending = Pending::new();
        let (first, _first) = pending.register();
        let (second, _second) = pending.register();
        assert_eq!((first, second), (RequestId(1), RequestId(2)));
    }

    #[test]
    fn removed_entry_is_not_taken() {
        let pending = Pending::new();
        let (id, _reply) = pending.register();
        pending.remove(id);
        assert!(pending.take(id).is_none());
        assert!(pending.is_empty());
    }

    #[test]
    fn take_hands_over_the_entry_once() -> TestResult {
        let pending = Pending::new();
        let (id, mut reply) = pending.register();
        let waiter = pending.take(id).ok_or("no entry")?;
        assert!(waiter.opening.is_none());
        assert!(pending.is_empty());
        assert!(pending.take(id).is_none());
        assert!(
            waiter
                .reply
                .send(Answer {
                    outcome: Ok(json!({"x": 1})),
                    stream: None,
                })
                .is_ok()
        );
        let answer = reply.try_recv()?;
        assert_eq!(answer.outcome.map_err(|e| e.to_string())?, json!({"x": 1}));
        assert!(answer.stream.is_none());
        Ok(())
    }

    #[test]
    fn an_opening_call_keeps_its_opening_in_the_entry() -> TestResult {
        let pending = Pending::new();
        let (commands, _queue) = mpsc::unbounded_channel();
        let (id, _reply) = pending.register_opening(Opening {
            commands,
            end: Arc::new(EndRecord::new()),
        });
        assert_eq!(id, RequestId(1));
        let waiter = pending.take(id).ok_or("no entry")?;
        assert!(waiter.opening.is_some());
        Ok(())
    }

    #[test]
    fn fail_all_drops_every_entry() {
        let pending = Pending::new();
        let (_one, mut one) = pending.register();
        let (_two, mut two) = pending.register();
        pending.fail_all();
        assert!(pending.is_empty());
        assert!(one.try_recv().is_err());
        assert!(two.try_recv().is_err());
    }

    #[test]
    fn end_records_the_reason_and_fails_every_entry() -> TestResult {
        let pending = Pending::new();
        let record = EndRecord::new();
        let (_id, mut reply) = pending.register();
        pending.end(&record, ConnectionEndKind::PeerClosed, "gone".to_owned());
        assert!(reply.try_recv().is_err());
        assert!(pending.is_empty());
        assert_eq!(record.get().ok_or("no end")?.message, "gone");
        Ok(())
    }
}
