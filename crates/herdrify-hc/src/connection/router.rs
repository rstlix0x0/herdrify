//! The task that reads frames from the core and routes them.
//!
//! The router owns the read half of the transport. It reads one frame at a time and hands each
//! channel-0 response to the call waiting for it. When that call opens a stream, the router opens
//! the stream's route and builds its handle before it reads the next frame, so a stream frame that
//! follows the reply in the same read is not lost. Data, resize and close frames on an open stream
//! channel go to that stream. Everything else the core can send, a stray reply, a notification,
//! bytes that are not JSON-RPC, a frame on a channel no call opened or a kind this client never
//! acts on, is discarded and the connection stays open, as the core's own `id: null` error replies
//! must not end it.
//!
//! The connection ends when the source returns the end of the stream or an error, whichever comes
//! first. The router records why, fails every waiting call and drops every route, which ends every
//! stream.
//!
//! Responsibilities:
//!
//! - [`spawn`], which starts the task over a [`FrameSource`]
//! - classifying a channel-0 payload and completing the call its response answers
//! - opening a stream's route when the reply to a stream-opening call arrives, unless the reply
//!   names channel 0 or a channel another stream holds, and forwarding the channel's frames to it
//! - recording the end of the connection when the source ends
//!
//! Non-responsibilities:
//!
//! - writing frames, which the writer does
//! - registering calls or reading their replies, which the client does

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use tokio::task::AbortHandle;

use super::ConnectionEndKind;
use super::error::EndRecord;
use super::pending::{Answer, Outcome, Pending, Waiter};
use super::stream::{self, Route};
use crate::port::{FrameSource, TransportError};
use crate::protocol::{Channel, Frame, FrameKind, Incoming};

/// The open stream channels and where each one's frames go.
type Routes = HashMap<Channel, Route>;

/// The `ConnectionEnd` message when the core closes the connection, which has no causing error.
const PEER_CLOSED_MESSAGE: &str = "herdr-core closed the connection";

/// Starts the router over `source` and returns a handle that stops it.
pub(super) fn spawn<S: FrameSource>(
    mut source: S,
    pending: Arc<Pending>,
    end: Arc<EndRecord>,
) -> AbortHandle {
    tokio::spawn(async move {
        let mut routes = Routes::new();
        let (kind, message) = loop {
            match source.recv().await {
                Ok(Some(frame)) => route(&pending, &mut routes, frame),
                Ok(None) => {
                    break (
                        ConnectionEndKind::PeerClosed,
                        PEER_CLOSED_MESSAGE.to_owned(),
                    );
                }
                Err(error) => break (kind_of(&error), error.to_string()),
            }
        };
        pending.end(&end, kind, message);
        // The end is recorded before the routes drop, so a stream that sees its route end finds
        // the reason already recorded.
        drop(routes);
    })
    .abort_handle()
}

/// How a failed read ends the connection.
const fn kind_of(error: &TransportError) -> ConnectionEndKind {
    match error {
        TransportError::Frame(_) => ConnectionEndKind::ProtocolViolation,
        TransportError::Io(_) | TransportError::Spawn { .. } => ConnectionEndKind::TransportFailed,
    }
}

/// Routes one frame: a channel-0 response to its pending call, a frame on a stream channel to its
/// stream, anything else nowhere.
fn route(pending: &Pending, routes: &mut Routes, frame: Frame) {
    if frame.channel != Channel::RPC {
        forward(routes, frame);
    } else if frame.kind == FrameKind::Data {
        respond(pending, routes, &frame.payload);
    }
}

/// Completes the call a channel-0 payload answers, if it is a response to one.
fn respond(pending: &Pending, routes: &mut Routes, payload: &[u8]) {
    if let Ok(Incoming::Response {
        id: Some(id),
        outcome,
    }) = Incoming::classify(payload)
        && let Some(waiter) = pending.take(id)
    {
        answer(routes, waiter, outcome);
    }
}

/// Answers a waiting call, opening the route first when the call opens a stream and the reply
/// names a channel it can open.
///
/// A reply that names no channel, one that is not a `u32`, channel 0, or a channel another stream
/// still holds opens nothing and sends nothing: the caller reads the reply and reports it. If the
/// caller is gone, the answer is dropped here with its handle.
fn answer(routes: &mut Routes, waiter: Waiter, outcome: Outcome) {
    let Waiter { reply, opening } = waiter;
    let stream = match (opening, &outcome) {
        (Some(opening), Ok(result)) => channel_of(result)
            .filter(|channel| is_free(routes, *channel))
            .map(|channel| {
                let (route, handle) = stream::open(channel, opening);
                routes.insert(channel, route);
                handle
            }),
        _ => None,
    };
    let _ = reply.send(Answer { outcome, stream });
}

/// The stream channel a reply names, when it is a `u32`.
fn channel_of(result: &Value) -> Option<Channel> {
    let number = result.get("channel")?.as_u64()?;
    Some(Channel::new(u32::try_from(number).ok()?))
}

/// Whether a stream can open on `channel`: it is not the call channel and no live stream holds it.
/// A route whose handle is gone does not hold its channel.
fn is_free(routes: &Routes, channel: Channel) -> bool {
    channel != Channel::RPC && routes.get(&channel).is_none_or(Route::is_closed)
}

/// Forwards a frame on a stream channel to its stream. Data and resize frames are forwarded. A
/// close frame marks the stream closed by the core, is forwarded and ends the route. A frame on a
/// channel with no route, and any other kind, is discarded. A route whose handle is gone is
/// removed when a frame fails to reach it.
fn forward(routes: &mut Routes, frame: Frame) {
    let channel = frame.channel;
    let Some(route) = routes.get(&channel) else {
        return;
    };
    match frame.kind {
        FrameKind::Data | FrameKind::Resize => {
            if !route.forward(frame) {
                routes.remove(&channel);
            }
        }
        FrameKind::Close => {
            // The mark comes first: the stream counts as closed by the core from this moment, even
            // while the frame waits to be read.
            route.mark_closed_by_core();
            route.forward(frame);
            routes.remove(&channel);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::sync::Arc;
    use std::time::Duration;

    use serde_json::json;
    use tokio::sync::mpsc;

    use super::{Routes, route, spawn};
    use crate::connection::ConnectionEndKind;
    use crate::connection::error::EndRecord;
    use crate::connection::memory_link::link;
    use crate::connection::pending::{Answer, Pending};
    use crate::connection::stream::Opening;
    use crate::connection::writer::Command;
    use crate::port::Transport;
    use crate::protocol::{Channel, Frame, FrameKind, RequestId};

    type TestResult = Result<(), Box<dyn Error>>;

    fn frame(channel: u32, kind: FrameKind, payload: &[u8]) -> Frame {
        Frame {
            channel: Channel::new(channel),
            kind,
            payload: payload.to_vec(),
        }
    }

    fn reply(id: RequestId, result: &serde_json::Value) -> Frame {
        frame(
            0,
            FrameKind::Data,
            json!({"id": id.get(), "jsonrpc": "2.0", "result": result})
                .to_string()
                .as_bytes(),
        )
    }

    #[test]
    fn router_routes_a_response_to_its_pending_entry() -> TestResult {
        let pending = Pending::new();
        let mut routes = Routes::new();
        let (id, mut waiting) = pending.register();
        route(
            &pending,
            &mut routes,
            frame(
                0,
                FrameKind::Data,
                format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{{"ok":true}}}}"#).as_bytes(),
            ),
        );
        let answer = waiting.try_recv()?;
        assert_eq!(
            answer.outcome.map_err(|e| e.to_string())?,
            json!({"ok": true})
        );
        assert!(answer.stream.is_none(), "a plain call opens no stream");

        let (other, mut waiting) = pending.register();
        assert_ne!(other, RequestId(99));
        route(
            &pending,
            &mut routes,
            frame(
                0,
                FrameKind::Data,
                br#"{"id":99,"jsonrpc":"2.0","result":1}"#,
            ),
        );
        assert!(waiting.try_recv().is_err());
        assert!(
            !pending.is_empty(),
            "an unknown id leaves the table unchanged"
        );
        Ok(())
    }

    #[test]
    fn everything_else_is_discarded() {
        let pending = Pending::new();
        let mut routes = Routes::new();
        let (_id, mut waiting) = pending.register();
        let response = br#"{"id":1,"jsonrpc":"2.0","result":1}"#;
        let discarded = [
            frame(0, FrameKind::Data, br#"{"id":null,"jsonrpc":"2.0","error":{"code":-32700,"message":"m","data":{"kind":"parse_error"}}}"#),
            frame(0, FrameKind::Data, br#"{"jsonrpc":"2.0","method":"heartbeat","params":{"seq":1}}"#),
            frame(0, FrameKind::Data, b"not json"),
            frame(0, FrameKind::Data, b"[1,2]"),
            frame(0, FrameKind::Credit, response),
            frame(0, FrameKind::Close, response),
            frame(0, FrameKind::Unknown(77), response),
            frame(5, FrameKind::Data, response),
            frame(5, FrameKind::Close, b""),
        ];
        for frame in discarded {
            route(&pending, &mut routes, frame);
        }
        assert!(waiting.try_recv().is_err());
        assert!(!pending.is_empty());
        assert!(routes.is_empty());
    }

    #[test]
    fn a_stream_reply_opens_its_route_and_hands_over_the_handle() -> TestResult {
        let pending = Pending::new();
        let mut routes = Routes::new();
        let (commands, _queue) = mpsc::unbounded_channel();
        let (id, mut waiting) = pending.register_opening(Opening {
            commands,
            end: Arc::new(EndRecord::new()),
        });
        route(
            &pending,
            &mut routes,
            reply(id, &json!({"channel": 7, "seq": 1})),
        );
        let answer = waiting.try_recv()?;
        assert!(answer.outcome.is_ok());
        let handle = answer.stream.ok_or("no handle")?;
        assert_eq!(handle.channel(), Channel::new(7));
        assert!(routes.contains_key(&Channel::new(7)));
        Ok(())
    }

    #[test]
    fn a_reply_that_opens_nothing_has_no_handle() -> TestResult {
        let replies = [
            json!({"seq": 1}),
            json!({"channel": "x"}),
            json!({"channel": -1}),
            json!({"channel": 4_294_967_296_u64}),
            json!({"channel": 1.5}),
        ];
        for result in replies {
            let pending = Pending::new();
            let mut routes = Routes::new();
            let (commands, _queue) = mpsc::unbounded_channel();
            let (id, mut waiting) = pending.register_opening(Opening {
                commands,
                end: Arc::new(EndRecord::new()),
            });
            route(&pending, &mut routes, reply(id, &result));
            let answer = waiting.try_recv()?;
            assert_eq!(answer.outcome.map_err(|e| e.to_string())?, result);
            assert!(answer.stream.is_none(), "{result} opens nothing");
            assert!(routes.is_empty());
        }

        let pending = Pending::new();
        let mut routes = Routes::new();
        let (commands, _queue) = mpsc::unbounded_channel();
        let (id, mut waiting) = pending.register_opening(Opening {
            commands,
            end: Arc::new(EndRecord::new()),
        });
        route(
            &pending,
            &mut routes,
            frame(
                0,
                FrameKind::Data,
                format!(
                    r#"{{"id":{id},"jsonrpc":"2.0","error":{{"code":-32000,"message":"m","data":{{"kind":"not_found"}}}}}}"#
                )
                .as_bytes(),
            ),
        );
        let answer = waiting.try_recv()?;
        assert!(answer.outcome.is_err());
        assert!(answer.stream.is_none(), "a refusal opens nothing");
        assert!(routes.is_empty());
        Ok(())
    }

    /// Registers a stream-opening call and routes `result` as its reply, returning the answer.
    fn open_with(
        pending: &Pending,
        routes: &mut Routes,
        result: &serde_json::Value,
    ) -> Result<(Answer, mpsc::UnboundedReceiver<Command>), Box<dyn Error>> {
        let (commands, queue) = mpsc::unbounded_channel();
        let (id, mut waiting) = pending.register_opening(Opening {
            commands,
            end: Arc::new(EndRecord::new()),
        });
        route(pending, routes, reply(id, result));
        Ok((waiting.try_recv()?, queue))
    }

    #[test]
    fn a_reply_naming_channel_zero_opens_nothing() -> TestResult {
        let pending = Pending::new();
        let mut routes = Routes::new();
        let (answer, mut queue) = open_with(&pending, &mut routes, &json!({"channel": 0}))?;
        assert!(
            answer.outcome.is_ok(),
            "the reply itself is still delivered"
        );
        assert!(answer.stream.is_none(), "channel 0 carries calls");
        assert!(routes.is_empty());
        assert!(queue.try_recv().is_err(), "nothing is sent for channel 0");
        Ok(())
    }

    #[tokio::test]
    async fn a_reply_naming_a_channel_in_use_leaves_the_first_route_alone() -> TestResult {
        let pending = Pending::new();
        let mut routes = Routes::new();
        let (first, mut first_queue) = open_with(&pending, &mut routes, &json!({"channel": 2}))?;
        let mut first = first.stream.ok_or("no handle")?;

        let (second, mut second_queue) = open_with(&pending, &mut routes, &json!({"channel": 2}))?;
        assert!(second.stream.is_none(), "channel 2 is taken");
        drop(second);
        assert!(
            second_queue.try_recv().is_err(),
            "the refused call sends nothing"
        );

        route(&pending, &mut routes, frame(2, FrameKind::Data, b"a"));
        assert_eq!(first.recv().await, Some(frame(2, FrameKind::Data, b"a")));
        assert!(
            first_queue.try_recv().is_err(),
            "the first stream was not closed"
        );
        Ok(())
    }

    #[test]
    fn a_channel_whose_handle_is_gone_can_be_opened_again() -> TestResult {
        let pending = Pending::new();
        let mut routes = Routes::new();
        let (first, _queue) = open_with(&pending, &mut routes, &json!({"channel": 2}))?;
        drop(first);

        let (second, _queue) = open_with(&pending, &mut routes, &json!({"channel": 2}))?;
        assert!(second.stream.is_some());
        assert_eq!(routes.len(), 1);
        Ok(())
    }

    #[test]
    fn a_route_whose_handle_is_gone_is_removed() -> TestResult {
        let pending = Pending::new();
        let mut routes = Routes::new();
        let (answer, _queue) = open_with(&pending, &mut routes, &json!({"channel": 2}))?;
        drop(answer);
        assert!(
            routes.contains_key(&Channel::new(2)),
            "nothing has noticed yet"
        );

        route(&pending, &mut routes, frame(2, FrameKind::Data, b"a"));
        assert!(
            routes.is_empty(),
            "a frame for a gone handle forgets the route"
        );
        Ok(())
    }

    #[tokio::test]
    async fn stream_frames_reach_their_stream_and_a_close_ends_the_route() -> TestResult {
        let pending = Pending::new();
        let mut routes = Routes::new();
        let (commands, _queue) = mpsc::unbounded_channel();
        let (id, mut waiting) = pending.register_opening(Opening {
            commands,
            end: Arc::new(EndRecord::new()),
        });
        route(&pending, &mut routes, reply(id, &json!({"channel": 2})));
        let mut handle = waiting.try_recv()?.stream.ok_or("no handle")?;

        route(&pending, &mut routes, frame(2, FrameKind::Data, b"a"));
        route(&pending, &mut routes, frame(2, FrameKind::Resize, b"bbbb"));
        route(&pending, &mut routes, frame(2, FrameKind::Credit, b"cccc"));
        route(&pending, &mut routes, frame(2, FrameKind::Focus, b"d"));
        route(&pending, &mut routes, frame(3, FrameKind::Data, b"other"));
        route(&pending, &mut routes, frame(2, FrameKind::Close, b""));
        route(&pending, &mut routes, frame(2, FrameKind::Data, b"late"));
        assert!(routes.is_empty(), "a close forgets the channel");

        assert_eq!(handle.recv().await, Some(frame(2, FrameKind::Data, b"a")));
        assert_eq!(
            handle.recv().await,
            Some(frame(2, FrameKind::Resize, b"bbbb"))
        );
        assert_eq!(handle.recv().await, Some(frame(2, FrameKind::Close, b"")));
        assert_eq!(handle.recv().await, None);
        Ok(())
    }

    #[tokio::test]
    async fn a_core_close_suppresses_the_handles_own_close() -> TestResult {
        let pending = Pending::new();
        let mut routes = Routes::new();
        let mut opened = Vec::new();
        for channel in [2, 3] {
            let (commands, queue) = mpsc::unbounded_channel();
            let (id, mut waiting) = pending.register_opening(Opening {
                commands,
                end: Arc::new(EndRecord::new()),
            });
            route(
                &pending,
                &mut routes,
                reply(id, &json!({"channel": channel})),
            );
            opened.push((waiting.try_recv()?.stream.ok_or("no handle")?, queue));
        }

        // The core closes channel 2 only. Its handle has not read the frame yet.
        route(&pending, &mut routes, frame(2, FrameKind::Close, b""));
        let (closed, mut closed_queue) = opened.remove(0);
        let (open, mut open_queue) = opened.remove(0);
        drop(closed);
        drop(open);
        assert!(
            closed_queue.try_recv().is_err(),
            "no CLOSE after the core's"
        );
        let sent = open_queue
            .try_recv()
            .map_err(|_| "no CLOSE from the open handle")?;
        assert_eq!(sent.frame.kind, FrameKind::Close);
        assert_eq!(sent.frame.channel, Channel::new(3));
        Ok(())
    }

    #[tokio::test]
    async fn route_opens_before_the_next_frame() -> TestResult {
        let (transport, peer) = link();
        let (_sink, source) = transport.into_framed();
        let pending = Arc::new(Pending::new());
        let (commands, _queue) = mpsc::unbounded_channel();
        let (id, waiting) = pending.register_opening(Opening {
            commands,
            end: Arc::new(EndRecord::new()),
        });
        let end = Arc::new(EndRecord::new());

        peer.push(reply(id, &json!({"channel": 1, "seq": 4})));
        peer.push(frame(1, FrameKind::Data, b"first"));
        spawn(source, Arc::clone(&pending), Arc::clone(&end));

        let answer = tokio::time::timeout(Duration::from_secs(5), waiting).await??;
        let mut handle = answer.stream.ok_or("no handle")?;
        let received = tokio::time::timeout(Duration::from_secs(5), handle.recv()).await?;
        assert_eq!(received, Some(frame(1, FrameKind::Data, b"first")));
        Ok(())
    }

    #[tokio::test]
    async fn the_end_of_the_connection_ends_every_route() -> TestResult {
        let (transport, mut peer) = link();
        let (_sink, source) = transport.into_framed();
        let pending = Arc::new(Pending::new());
        let (commands, _queue) = mpsc::unbounded_channel();
        let (id, waiting) = pending.register_opening(Opening {
            commands,
            end: Arc::new(EndRecord::new()),
        });
        let end = Arc::new(EndRecord::new());
        peer.push(reply(id, &json!({"channel": 1})));
        spawn(source, Arc::clone(&pending), Arc::clone(&end));
        let mut handle = tokio::time::timeout(Duration::from_secs(5), waiting)
            .await??
            .stream
            .ok_or("no handle")?;

        peer.end();
        let ended = tokio::time::timeout(Duration::from_secs(5), handle.recv()).await?;
        assert_eq!(ended, None);
        assert!(end.is_ended());
        Ok(())
    }

    /// The router records the end, fails the waiting calls and only then drops its routes. The
    /// test holds the pending table's lock, which stops the router after it recorded the end and
    /// before it fails the calls, and checks that the stream's route is still open there.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_end_is_recorded_before_the_routes_drop() -> TestResult {
        let (transport, mut peer) = link();
        let (_sink, source) = transport.into_framed();
        let pending = Arc::new(Pending::new());
        let (commands, _queue) = mpsc::unbounded_channel();
        let (id, waiting) = pending.register_opening(Opening {
            commands,
            end: Arc::new(EndRecord::new()),
        });
        let end = Arc::new(EndRecord::new());
        peer.push(reply(id, &json!({"channel": 1})));
        spawn(source, Arc::clone(&pending), Arc::clone(&end));
        let mut handle = tokio::time::timeout(Duration::from_secs(5), waiting)
            .await??
            .stream
            .ok_or("no handle")?;

        let (locked, is_locked) = std::sync::mpsc::channel();
        let (release, released) = std::sync::mpsc::channel::<()>();
        let holder = std::thread::spawn({
            let pending = Arc::clone(&pending);
            move || {
                let table = pending.hold();
                let _ = locked.send(());
                let _ = released.recv_timeout(Duration::from_secs(10));
                drop(table);
            }
        });
        is_locked.recv_timeout(Duration::from_secs(5))?;

        peer.end();
        tokio::time::timeout(Duration::from_secs(5), async {
            while !end.is_ended() {
                tokio::task::yield_now().await;
            }
        })
        .await?;
        let early = tokio::time::timeout(Duration::from_millis(200), handle.recv()).await;
        let _ = release.send(());
        holder.join().map_err(|_| "the lock holder panicked")?;
        assert!(
            early.is_err(),
            "the route ended before the end was fully handled"
        );

        let ended = tokio::time::timeout(Duration::from_secs(5), handle.recv()).await?;
        assert_eq!(ended, None);
        Ok(())
    }

    #[tokio::test]
    async fn eof_ends_the_connection_as_peer_closed() -> TestResult {
        let (transport, mut peer) = link();
        let (_sink, source) = transport.into_framed();
        let pending = Arc::new(Pending::new());
        let (_id, mut waiting) = pending.register();
        let end = Arc::new(EndRecord::new());
        spawn(source, Arc::clone(&pending), Arc::clone(&end));

        peer.end();
        tokio::time::timeout(Duration::from_secs(5), async {
            while !end.is_ended() {
                tokio::task::yield_now().await;
            }
        })
        .await?;
        let recorded = end.get().ok_or("no end recorded")?;
        assert_eq!(recorded.kind, ConnectionEndKind::PeerClosed);
        assert_eq!(recorded.message, "herdr-core closed the connection");
        assert!(waiting.try_recv().is_err());
        assert!(pending.is_empty());
        Ok(())
    }
}
