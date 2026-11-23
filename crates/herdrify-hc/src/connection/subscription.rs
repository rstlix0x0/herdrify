//! `Subscription`: the events herdr-core delivers on a stream channel.
//!
//! The core's event channel carries a newline-delimited JSON byte stream that is not aligned to
//! frames: a long line is split over several frames, and short lines share one. A subscription
//! takes the channel's DATA frames from its stream handle, grants the core credit for every byte it
//! takes, joins the bytes into lines and reads each line as a notification.
//!
//! Responsibilities:
//!
//! - [`Subscription`], with the fields of the subscribe reply and [`Subscription::next`]
//! - granting credit per frame taken, partial lines included, so a line of any length completes
//! - reporting each bad line as an error that leaves the stream open
//! - [`Subscription::close`], which sends CLOSE at most once
//! - dropping the events a `missed` notification makes stale
//!
//! Non-responsibilities:
//!
//! - opening the channel and building the handle, which the client and the router do
//! - decoding lines and notifications, which `protocol` does

use std::fmt;

use super::Error;
use super::stream::StreamHandle;
use crate::protocol::{
    Channel, FrameKind, Incoming, NdjsonDecoder, Notification, ProtocolError, Seq, SubscribeResult,
    Terminal,
};

/// A stream of herdr-core's notifications.
///
/// It is opened by [`Client::events_subscribe`](super::Client::events_subscribe). The connection
/// stays open while a subscription exists.
pub struct Subscription {
    handle: StreamHandle,
    seq: Seq,
    terminals: Vec<Terminal>,
    decoder: NdjsonDecoder,
    stale_through: Option<Seq>,
    phase: Phase,
}

/// How far a subscription has got.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Frames are still being read.
    Reading,
    /// A line passed the size limit. The lines already buffered are delivered, then the error.
    Overflowed,
    /// The subscription has ended: `next` returns `None`.
    Finished,
}

impl fmt::Debug for Subscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Subscription")
            .field("channel", &self.handle.channel())
            .field("seq", &self.seq)
            .finish_non_exhaustive()
    }
}

impl Subscription {
    /// A subscription reading from `handle`, for the reply `result`.
    pub(super) fn new(handle: StreamHandle, result: SubscribeResult) -> Self {
        Self {
            handle,
            seq: Seq(result.seq),
            terminals: result.terminals,
            decoder: NdjsonDecoder::new(),
            stale_through: None,
            phase: Phase::Reading,
        }
    }

    /// The sequence number events follow: every later event has a higher one.
    #[must_use]
    pub const fn seq(&self) -> Seq {
        self.seq
    }

    /// The terminals that matched when the subscription was opened.
    #[must_use]
    pub fn terminals(&self) -> &[Terminal] {
        &self.terminals
    }

    /// The channel the events arrive on.
    #[must_use]
    pub const fn channel(&self) -> Channel {
        self.handle.channel()
    }

    /// The next notification, or `None` once the subscription has ended.
    ///
    /// A line that cannot be read yields one `Err` and the subscription goes on with the next
    /// line:
    ///
    /// - [`ProtocolError::InvalidJson`] when the line is not JSON.
    /// - [`ProtocolError::NotJsonRpc`] when it is JSON that is neither a response nor a
    ///   notification.
    /// - [`ProtocolError::ResponseOnStream`] when it is a JSON-RPC response.
    /// - [`ProtocolError::InvalidNotification`] when a `heartbeat` or `missed` has parameters that
    ///   do not decode, or an `event` has an envelope that does not.
    /// - [`ProtocolError::InvalidEvent`] when the data of a known event type does not match it.
    ///
    /// After a `missed` notification, events at or below its sequence number are dropped. The
    /// subscription ends with `None` when the core closes the channel, whether or not the close
    /// carried a payload. It ends with [`ProtocolError::EventLineTooLong`], then `None`, when a
    /// line passes 16777216 bytes without a newline, and with [`Error::Closed`], then `None`, when
    /// the connection ends.
    ///
    /// The core's window counts bytes, so credit is granted for each frame taken, not for each
    /// line.
    ///
    /// # Cancel safety
    ///
    /// Cancel-safe. The only await is the wait for a frame. Taking a frame, granting its credit and
    /// buffering its bytes happen together, so a cancelled call loses no notification and no
    /// credit.
    pub async fn next(&mut self) -> Option<Result<Notification, Error>> {
        loop {
            if let Some(line) = self.decoder.next_line() {
                if let Some(item) = self.read(line.as_str()) {
                    return Some(item);
                }
                continue;
            }
            match self.phase {
                Phase::Overflowed => {
                    self.phase = Phase::Finished;
                    return Some(Err(Error::Protocol(ProtocolError::EventLineTooLong)));
                }
                Phase::Finished => return None,
                Phase::Reading => {}
            }
            let Some(frame) = self.handle.recv().await else {
                self.phase = Phase::Finished;
                return Some(Err(Error::Closed));
            };
            match frame.kind {
                FrameKind::Data => self.take(&frame.payload),
                FrameKind::Close => {
                    self.phase = Phase::Finished;
                    return None;
                }
                _ => {}
            }
        }
    }

    /// Closes the subscription from this side.
    ///
    /// CLOSE is sent once. The subscription keeps yielding what arrives until the core closes the
    /// channel in answer, and that ends it with `None`. Dropping a subscription that was not
    /// closed sends the same CLOSE.
    ///
    /// # Errors
    ///
    /// [`Error::Closed`] when the connection has ended, including while the CLOSE waits to be
    /// written. That is checked first: on an open connection, a subscription that is already
    /// closed, by this call or by the core, returns `Ok(())` and sends nothing.
    pub async fn close(&mut self) -> Result<(), Error> {
        self.handle.close().await
    }

    /// Grants credit for a DATA frame's bytes and buffers them. A zero-length frame is skipped.
    fn take(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        // A frame's payload is at most `MAX_PAYLOAD_LEN`, which fits a `u32`, so the saturation
        // never applies.
        self.handle
            .credit(u32::try_from(bytes.len()).unwrap_or(u32::MAX));
        if self.decoder.push(bytes).is_err() {
            self.phase = Phase::Overflowed;
        }
    }

    /// Reads one line as a notification, or `None` when the line is a stale event to drop.
    fn read(&mut self, line: &str) -> Option<Result<Notification, Error>> {
        let (method, params) = match Incoming::classify(line.as_bytes()) {
            Ok(Incoming::Notification { method, params }) => (method, params),
            Ok(Incoming::Response { .. }) => {
                return Some(Err(Error::Protocol(ProtocolError::ResponseOnStream)));
            }
            Err(error) => return Some(Err(Error::Protocol(error))),
        };
        let notification = match Notification::from_parts(&method, params) {
            Ok(notification) => notification,
            Err(error) => return Some(Err(Error::Protocol(error))),
        };
        match &notification {
            Notification::Missed { seq, .. } => {
                self.stale_through = self.stale_through.max(Some(*seq));
            }
            Notification::Event(event)
                if self.stale_through.is_some_and(|seq| event.seq <= seq) =>
            {
                return None;
            }
            _ => {}
        }
        Some(Ok(notification))
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;
    use std::future::Future;
    use std::pin::pin;
    use std::sync::Arc;
    use std::task::{Context, Waker};
    use std::time::Duration;

    use tokio::sync::mpsc;

    use super::Subscription;
    use crate::connection::error::EndRecord;
    use crate::connection::stream::{self, Opening, Route};
    use crate::connection::writer::Command;
    use crate::protocol::{Channel, Frame, FrameKind, Notification, Seq, SubscribeResult};

    type TestResult = Result<(), Box<dyn StdError>>;

    const CHANNEL: u32 = 1;

    fn open() -> (Subscription, Route, mpsc::UnboundedReceiver<Command>) {
        let (commands, queue) = mpsc::unbounded_channel();
        let (route, handle) = stream::open(
            Channel::new(CHANNEL),
            Opening {
                commands,
                end: Arc::new(EndRecord::new()),
            },
        );
        let result = SubscribeResult {
            seq: 12,
            terminals: Vec::new(),
            channel: Some(CHANNEL),
        };
        (Subscription::new(handle, result), route, queue)
    }

    fn data(payload: &[u8]) -> Frame {
        Frame {
            channel: Channel::new(CHANNEL),
            kind: FrameKind::Data,
            payload: payload.to_vec(),
        }
    }

    /// The CREDIT amounts queued so far.
    fn credits(
        queue: &mut mpsc::UnboundedReceiver<Command>,
    ) -> Result<Vec<u32>, Box<dyn StdError>> {
        let mut amounts = Vec::new();
        while let Ok(command) = queue.try_recv() {
            assert_eq!(command.frame.kind, FrameKind::Credit);
            assert_eq!(command.frame.channel, Channel::new(CHANNEL));
            assert!(command.ack.is_none(), "a credit is not acknowledged");
            amounts.push(command.frame.read_credit()?);
        }
        Ok(amounts)
    }

    const STALE: &str = r#"{"method":"event","params":{"seq":9,"type":"removed"}}"#;
    const FRESH: &str =
        r#"{"method":"event","params":{"seq":11,"type":"title","data":{"title":"abc"}}}"#;

    #[tokio::test]
    async fn credit_follows_frames() -> TestResult {
        let (mut subscription, route, mut queue) = open();
        subscription.stale_through = Some(Seq(10));
        let mut bytes = Vec::new();
        bytes.extend_from_slice(STALE.as_bytes());
        bytes.push(b'\n');
        bytes.extend_from_slice(FRESH.as_bytes());
        bytes.push(b'\n');
        bytes.push(b'{');
        assert!(route.forward(data(&bytes)));

        let item = tokio::time::timeout(Duration::from_secs(5), subscription.next()).await?;
        let Some(Ok(Notification::Event(event))) = item else {
            return Err(format!("expected an event, got {item:?}").into());
        };
        assert_eq!(event.seq, Seq(11));
        assert_eq!(credits(&mut queue)?, vec![u32::try_from(bytes.len())?]);
        Ok(())
    }

    #[tokio::test]
    async fn a_line_longer_than_the_window_completes() -> TestResult {
        let (mut subscription, route, mut queue) = open();
        let title = "x".repeat(300_000);
        let line = format!(
            r#"{{"method":"event","params":{{"seq":1,"type":"title","data":{{"title":"{title}"}}}}}}"#
        );
        let mut bytes = line.into_bytes();
        bytes.push(b'\n');
        let frames: Vec<&[u8]> = bytes.chunks(16_384).collect();
        assert_eq!(frames.len(), 19);
        for frame in &frames {
            assert!(route.forward(data(frame)));
        }

        let item = tokio::time::timeout(Duration::from_secs(5), subscription.next()).await?;
        assert!(matches!(item, Some(Ok(Notification::Event(_)))));
        let expected: Vec<u32> = frames
            .iter()
            .map(|frame| u32::try_from(frame.len()))
            .collect::<Result<_, _>>()?;
        assert_eq!(credits(&mut queue)?, expected, "one credit per frame");
        Ok(())
    }

    #[tokio::test]
    async fn zero_length_frame_sends_no_credit() -> TestResult {
        let (mut subscription, route, mut queue) = open();
        assert!(route.forward(data(b"")));
        assert!(route.forward(data(format!("{FRESH}\n").as_bytes())));

        let item = tokio::time::timeout(Duration::from_secs(5), subscription.next()).await?;
        assert!(matches!(item, Some(Ok(Notification::Event(_)))));
        assert_eq!(credits(&mut queue)?, vec![u32::try_from(FRESH.len() + 1)?]);
        Ok(())
    }

    /// A `next()` dropped after it took a frame loses neither the frame's credit nor its bytes.
    #[tokio::test]
    async fn cancelling_next_with_a_frame_queued_loses_nothing() -> TestResult {
        let (mut subscription, route, mut queue) = open();
        let line = format!("{FRESH}\n");
        let (head, tail) = line.as_bytes().split_at(10);
        assert!(route.forward(data(head)));

        // One poll takes the queued frame, which holds no complete line, and then waits for more.
        let mut context = Context::from_waker(Waker::noop());
        {
            let mut next = pin!(subscription.next());
            assert!(next.as_mut().poll(&mut context).is_pending());
        }
        assert_eq!(credits(&mut queue)?, vec![u32::try_from(head.len())?]);

        assert!(route.forward(data(tail)));
        let item = tokio::time::timeout(Duration::from_secs(5), subscription.next()).await?;
        assert!(matches!(item, Some(Ok(Notification::Event(_)))), "{item:?}");
        assert_eq!(credits(&mut queue)?, vec![u32::try_from(tail.len())?]);
        Ok(())
    }

    #[tokio::test]
    async fn cancelled_next_loses_nothing() -> TestResult {
        let (mut subscription, route, mut queue) = open();
        let cancelled = tokio::time::timeout(Duration::from_millis(50), subscription.next()).await;
        assert!(cancelled.is_err(), "no frame has arrived");
        assert!(credits(&mut queue)?.is_empty());

        assert!(route.forward(data(format!("{FRESH}\n").as_bytes())));
        let item = tokio::time::timeout(Duration::from_secs(5), subscription.next()).await?;
        assert!(matches!(item, Some(Ok(Notification::Event(_)))));
        assert_eq!(credits(&mut queue)?, vec![u32::try_from(FRESH.len() + 1)?]);
        Ok(())
    }
}
