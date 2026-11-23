//! `AttachStream`: a terminal's raw byte stream, driven from the viewer's side.
//!
//! `terminal.attach` makes this connection a viewer of a terminal. The core sends the terminal's
//! output as DATA frames on a stream channel, and the viewer answers with input, a size, a focus
//! and display facts. An attach stream takes the channel's frames from its stream handle, grants
//! the core credit for every byte of output it takes and turns each frame into an
//! [`AttachEvent`]. Its writes go out through the same handle, so they obey the handle's checks
//! and its CLOSE rule.
//!
//! Responsibilities:
//!
//! - [`AttachStream`], with the fields of the attach reply, [`AttachStream::next`] and the six
//!   writes
//! - [`AttachEvent`], what the core sends on the channel, and [`CloseReason`], why it closed it
//! - granting credit per DATA frame taken, and skipping a DATA frame with no bytes
//! - choosing between DATA and `AUTOMATIC_INPUT` for [`AttachStream::automatic_input`]
//!
//! Non-responsibilities:
//!
//! - opening the channel and building the handle, which the client and the router do
//! - the checks every write passes and the CLOSE rule, which the stream handle applies
//! - the frame encodings, which `protocol` defines

use std::fmt;

use serde::Deserialize;
use serde_json::Value;

use super::Error;
use super::error::unencodable;
use super::stream::StreamHandle;
use crate::protocol::{
    AttachResult, Channel, DisplayFacts, EndReason, Focus, Frame, FrameKind, ProtocolError,
    Terminal,
};

/// The feature a core lists in its handshake reply when it accepts `AUTOMATIC_INPUT` frames.
const AUTOMATIC_INPUT_FEATURE: &str = "automatic_input";

/// A terminal's byte stream, opened by
/// [`Client::terminal_attach`](super::Client::terminal_attach).
///
/// The connection stays open while an attach stream exists. Dropping a stream that is still open
/// sends CLOSE, so the core stops the attach.
///
/// The public fields are the attach reply as it was received. They are a record of that reply:
/// writing to them changes nothing the stream does.
pub struct AttachStream {
    /// The terminal as the reply gave it, as it was when the attach began.
    pub terminal: Terminal,
    /// The stream channel the reply named, which the terminal's bytes arrive on.
    pub channel: Channel,
    /// Whether the reply says the whole attach path accepts automatic input frames. Which kind of
    /// frame [`AttachStream::automatic_input`] sends does not read this field.
    pub automatic_input: bool,
    /// Whether the reply says the core sends a resize before replacement paints, on the whole
    /// attach path.
    pub resize_notifications: bool,
    handle: StreamHandle,
    automatic_input_negotiated: bool,
    phase: Phase,
}

/// How far an attach stream has got.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Frames are still being read.
    Reading,
    /// The core closed the channel or the connection ended: `next` returns `None`.
    Finished,
}

impl fmt::Debug for AttachStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AttachStream")
            .field("channel", &self.handle.channel())
            .field("terminal", &self.terminal.address)
            .finish_non_exhaustive()
    }
}

/// What the core sends on an attach channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachEvent {
    /// Bytes the terminal produced. Never empty.
    Output(Vec<u8>),
    /// The terminal's size changed.
    Resize {
        /// The width in columns.
        cols: u16,
        /// The height in rows.
        rows: u16,
    },
    /// The core closed the channel. The stream yields nothing after it.
    Closed(CloseReason),
}

/// Why the core closed an attach channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloseReason {
    /// The core answered this side's CLOSE, or detached the viewer, and said nothing more.
    Detached,
    /// The terminal's program ended.
    Ended {
        /// Why it ended.
        reason: EndReason,
        /// The exit code, when it exited with one.
        exit_code: Option<i32>,
        /// The signal that ended it, when one did.
        signal: Option<i32>,
        /// When it finished, in milliseconds since the epoch.
        finished_ms: Option<u64>,
    },
    /// A payload that is neither empty nor the shape of [`CloseReason::Ended`], as it was sent.
    Unrecognized {
        /// The CLOSE frame's payload.
        payload: Vec<u8>,
    },
}

/// The payload of a CLOSE that reports the end of the terminal's program.
#[derive(Deserialize)]
struct EndedPayload {
    reason: EndReason,
    exit_code: Option<i32>,
    signal: Option<i32>,
    finished_ms: Option<u64>,
}

impl CloseReason {
    /// Reads the payload of the core's CLOSE frame.
    ///
    /// An empty payload is [`CloseReason::Detached`]. A JSON object with a string `reason`, and
    /// integer `exit_code`, `signal` and `finished_ms` that may be `null` or absent, is
    /// [`CloseReason::Ended`]. Every other payload is [`CloseReason::Unrecognized`].
    fn read(payload: Vec<u8>) -> Self {
        if payload.is_empty() {
            return Self::Detached;
        }
        Self::read_ended(&payload).unwrap_or(Self::Unrecognized { payload })
    }

    fn read_ended(payload: &[u8]) -> Option<Self> {
        let value: Value = serde_json::from_slice(payload).ok()?;
        // A struct also reads from a JSON array, so the object is checked first.
        if !value.is_object() {
            return None;
        }
        let ended: EndedPayload = serde_json::from_value(value).ok()?;
        Some(Self::Ended {
            reason: ended.reason,
            exit_code: ended.exit_code,
            signal: ended.signal,
            finished_ms: ended.finished_ms,
        })
    }
}

impl AttachStream {
    /// A stream reading from `handle`, for the reply `result`.
    ///
    /// `core_accepts_automatic_input` is whether the handshake reply listed the
    /// `automatic_input` feature.
    pub(super) fn new(
        handle: StreamHandle,
        result: AttachResult,
        core_accepts_automatic_input: bool,
    ) -> Self {
        Self {
            terminal: result.terminal,
            channel: handle.channel(),
            automatic_input: result.automatic_input,
            resize_notifications: result.resize_notifications,
            automatic_input_negotiated: core_accepts_automatic_input && result.automatic_input,
            handle,
            phase: Phase::Reading,
        }
    }

    /// Whether `features`, from the handshake reply, include automatic input.
    pub(super) fn lists_automatic_input(features: &[String]) -> bool {
        features
            .iter()
            .any(|feature| feature == AUTOMATIC_INPUT_FEATURE)
    }

    /// The next event, or `None` once the stream has ended.
    ///
    /// - [`AttachEvent::Output`] for a DATA frame. Taking its bytes grants the core credit for all
    ///   of them at once, before they are returned. A DATA frame with no bytes is skipped.
    /// - [`AttachEvent::Resize`] for a RESIZE frame, in the order the frames arrived.
    /// - [`AttachEvent::Closed`] when the core closes the channel. `None` follows it.
    /// - An [`Error::Protocol`] holding [`FrameError::BadControlPayload`] for a RESIZE frame whose
    ///   payload is not 4 bytes. The stream goes on.
    /// - [`Error::Closed`] when the connection ends, then `None`.
    ///
    /// Closing the stream from this side does not end it: it keeps yielding until the core's CLOSE
    /// arrives as [`CloseReason::Detached`].
    ///
    /// # Cancel safety
    ///
    /// Cancel-safe. The only await is the wait for a frame. Taking a frame, granting its credit and
    /// returning it happen together, so a cancelled call loses no output and no credit.
    ///
    /// [`FrameError::BadControlPayload`]: crate::protocol::FrameError::BadControlPayload
    pub async fn next(&mut self) -> Option<Result<AttachEvent, Error>> {
        loop {
            if self.phase == Phase::Finished {
                return None;
            }
            let Some(frame) = self.handle.recv().await else {
                self.phase = Phase::Finished;
                return Some(Err(Error::Closed));
            };
            match frame.kind {
                FrameKind::Data if frame.payload.is_empty() => {}
                FrameKind::Data => {
                    // A frame's payload is at most `MAX_PAYLOAD_LEN`, which fits a `u32`, so the
                    // saturation never applies.
                    self.handle
                        .credit(u32::try_from(frame.payload.len()).unwrap_or(u32::MAX));
                    return Some(Ok(AttachEvent::Output(frame.payload)));
                }
                FrameKind::Resize => {
                    return Some(match frame.read_resize() {
                        Ok((cols, rows)) => Ok(AttachEvent::Resize { cols, rows }),
                        Err(error) => Err(Error::Protocol(ProtocolError::Frame(error))),
                    });
                }
                FrameKind::Close => {
                    self.phase = Phase::Finished;
                    return Some(Ok(AttachEvent::Closed(CloseReason::read(frame.payload))));
                }
                _ => {}
            }
        }
    }

    /// Types `bytes` into the terminal.
    ///
    /// # Errors
    ///
    /// Every write is refused in this order, before anything is sent:
    ///
    /// - [`Error::Closed`] when the connection has ended.
    /// - [`Error::StreamClosed`] when the stream has ended, because the core closed it or
    ///   [`AttachStream::close`] ran.
    /// - [`Error::PayloadTooLarge`] when `bytes` is longer than a frame may carry, 16777216 bytes.
    ///
    /// Then [`Error::Transport`] when the write fails, which also ends the connection.
    pub async fn input(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.handle.write(FrameKind::Data, bytes).await
    }

    /// Types `bytes` into the terminal as the terminal's own reply, not as human activity.
    ///
    /// The bytes go out as an `AUTOMATIC_INPUT` frame when the handshake reply listed the
    /// `automatic_input` feature and the attach reply says `automatic_input: true`. Otherwise
    /// they go out as [`AttachStream::input`] would send them.
    ///
    /// # Errors
    ///
    /// As [`AttachStream::input`].
    pub async fn automatic_input(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let kind = if self.automatic_input_negotiated {
            FrameKind::AutomaticInput
        } else {
            FrameKind::Data
        };
        self.handle.write(kind, bytes).await
    }

    /// Tells the core the viewer is `cols` columns by `rows` rows.
    ///
    /// # Errors
    ///
    /// As [`AttachStream::input`].
    pub async fn resize(&mut self, cols: u16, rows: u16) -> Result<(), Error> {
        let frame = Frame::resize(self.handle.channel(), cols, rows);
        self.handle.write(frame.kind, &frame.payload).await
    }

    /// Tells the core whether the viewer has focus.
    ///
    /// # Errors
    ///
    /// As [`AttachStream::input`].
    pub async fn focus(&mut self, focus: Focus) -> Result<(), Error> {
        let frame = Frame::focus(self.handle.channel(), focus);
        self.handle.write(frame.kind, &frame.payload).await
    }

    /// Tells the core what the viewer's terminal looks like.
    ///
    /// # Errors
    ///
    /// As [`AttachStream::input`].
    pub async fn facts(&mut self, facts: DisplayFacts) -> Result<(), Error> {
        // Display facts are strings and numbers, which always serialize, so this does not fail.
        let payload = serde_json::to_vec(&facts).map_err(unencodable)?;
        self.handle.write(FrameKind::Facts, &payload).await
    }

    /// Closes the stream from this side.
    ///
    /// CLOSE is sent once. The stream keeps yielding what arrives until the core closes the channel
    /// in answer, which ends it with [`CloseReason::Detached`]. Dropping a stream that was not
    /// closed sends the same CLOSE.
    ///
    /// # Errors
    ///
    /// [`Error::Closed`] when the connection has ended, including while the CLOSE waits to be
    /// written. That is checked first: on an open connection, a stream that is already closed, by
    /// this call or by the core, returns `Ok(())` and sends nothing.
    pub async fn close(&mut self) -> Result<(), Error> {
        self.handle.close().await
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;
    use std::future::Future;
    use std::pin::pin;
    use std::sync::Arc;
    use std::task::{Context, Poll, Waker};
    use std::time::Duration;

    use tokio::sync::mpsc;

    use super::{AttachEvent, AttachStream, CloseReason};
    use crate::connection::error::EndRecord;
    use crate::connection::memory_link::{MemoryPeer, link};
    use crate::connection::pending::Pending;
    use crate::connection::stream::{self, Opening, Route};
    use crate::connection::writer::{self, Command, Commands};
    use crate::connection::{ConnectionEndKind, Error};
    use crate::port::Transport;
    use crate::protocol::{AttachResult, Channel, EndReason, Frame, FrameKind};

    type TestResult = Result<(), Box<dyn StdError>>;

    const CHANNEL: u32 = 1;

    /// The attach reply of a terminal at a neutral working directory.
    const RESULT: &str = r#"{"automatic_input":true,"channel":1,"resize_notifications":false,"terminal":{"address":"t1d","argv":["sh"],"cols":80,"created_ms":1,"cwd":"/work","die_with_parent":false,"exit_code":null,"finished_ms":null,"identity":"ghostty","keep":false,"labels":{},"name":null,"notify":true,"parent":null,"pid":2,"reason":null,"reported":false,"rm":false,"rows":24,"screen":"exact","signal":null,"state":"live","status":null,"status_ms":null,"template":null,"title":"","turn":0}}"#;

    fn result() -> Result<AttachResult, Box<dyn StdError>> {
        Ok(serde_json::from_str(RESULT)?)
    }

    fn data(payload: &[u8]) -> Frame {
        Frame {
            channel: Channel::new(CHANNEL),
            kind: FrameKind::Data,
            payload: payload.to_vec(),
        }
    }

    fn close() -> Frame {
        Frame {
            channel: Channel::new(CHANNEL),
            kind: FrameKind::Close,
            payload: Vec::new(),
        }
    }

    /// A stream whose commands go to a queue the test reads.
    fn open() -> Result<(AttachStream, Route, mpsc::UnboundedReceiver<Command>), Box<dyn StdError>>
    {
        let (commands, queue) = mpsc::unbounded_channel();
        let (route, handle) = stream::open(
            Channel::new(CHANNEL),
            Opening {
                commands,
                end: Arc::new(EndRecord::new()),
            },
        );
        Ok((AttachStream::new(handle, result()?, true), route, queue))
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

    async fn output(stream: &mut AttachStream) -> Result<Vec<u8>, Box<dyn StdError>> {
        match tokio::time::timeout(Duration::from_secs(5), stream.next()).await? {
            Some(Ok(AttachEvent::Output(bytes))) => Ok(bytes),
            other => Err(format!("expected output, got {other:?}").into()),
        }
    }

    #[tokio::test]
    async fn cancelled_attach_next_loses_nothing() -> TestResult {
        let (mut stream, route, mut queue) = open()?;
        let cancelled = tokio::time::timeout(Duration::from_millis(50), stream.next()).await;
        assert!(cancelled.is_err(), "no frame has arrived");
        assert!(credits(&mut queue)?.is_empty());

        assert!(route.forward(data(b"hello")));
        assert_eq!(output(&mut stream).await?, b"hello");
        assert_eq!(credits(&mut queue)?, vec![5], "exactly one credit");
        Ok(())
    }

    /// A `next()` dropped right after one poll, with a frame already queued, loses neither the
    /// frame nor its credit: the frame is taken only by the poll that returns it.
    #[tokio::test]
    async fn dropping_next_after_one_poll_loses_nothing() -> TestResult {
        let (mut stream, route, mut queue) = open()?;
        assert!(route.forward(data(b"world")));

        let mut context = Context::from_waker(Waker::noop());
        let first = {
            let mut next = pin!(stream.next());
            next.as_mut().poll(&mut context)
        };
        let bytes = match first {
            Poll::Ready(Some(Ok(AttachEvent::Output(bytes)))) => bytes,
            Poll::Ready(other) => return Err(format!("unexpected {other:?}").into()),
            // The poll gave up the frame without returning it, so it must still be there.
            Poll::Pending => output(&mut stream).await?,
        };
        assert_eq!(bytes, b"world");
        assert_eq!(credits(&mut queue)?, vec![5], "exactly one credit");
        Ok(())
    }

    #[test]
    fn ended_accepts_the_readme_shape() {
        let ended = |payload: &str| CloseReason::read(payload.as_bytes().to_vec());
        assert_eq!(
            ended(r#"{"reason":"exited","exit_code":0,"signal":null}"#),
            CloseReason::Ended {
                reason: EndReason::Exited,
                exit_code: Some(0),
                signal: None,
                finished_ms: None,
            }
        );
        assert_eq!(
            ended(r#"{"reason":"killed","exit_code":null,"signal":1}"#),
            CloseReason::Ended {
                reason: EndReason::Killed,
                exit_code: None,
                signal: Some(1),
                finished_ms: None,
            }
        );
        assert_eq!(
            ended(r#"{"reason":"paused"}"#),
            CloseReason::Ended {
                reason: EndReason::Other("paused".to_owned()),
                exit_code: None,
                signal: None,
                finished_ms: None,
            }
        );

        for payload in [
            r#"{"exit_code":0}"#,
            r#"{"reason":"exited","signal":"TERM"}"#,
            r#"{"reason":null}"#,
            r#"{"reason":"exited","exit_code":2147483648}"#,
            r#"["exited",0,null,null]"#,
            r#""exited""#,
            "null",
            "garbage",
        ] {
            assert_eq!(
                ended(payload),
                CloseReason::Unrecognized {
                    payload: payload.as_bytes().to_vec()
                },
                "{payload}"
            );
        }
        assert_eq!(CloseReason::read(Vec::new()), CloseReason::Detached);
    }

    /// A writer over a memory link, and the end record it shares with the streams built on it.
    fn writer() -> (Commands, Arc<EndRecord>, MemoryPeer) {
        let (transport, peer) = link();
        let (sink, _source) = transport.into_framed();
        let end = Arc::new(EndRecord::new());
        let router = tokio::spawn(std::future::pending::<()>());
        let commands = writer::spawn(
            sink,
            Arc::new(Pending::new()),
            Arc::clone(&end),
            router.abort_handle(),
        );
        (commands, end, peer)
    }

    fn open_on(
        commands: &Commands,
        end: &Arc<EndRecord>,
    ) -> Result<(AttachStream, Route), Box<dyn StdError>> {
        let (route, handle) = stream::open(
            Channel::new(CHANNEL),
            Opening {
                commands: commands.clone(),
                end: Arc::clone(end),
            },
        );
        Ok((AttachStream::new(handle, result()?, true), route))
    }

    /// Writes a marker on a fresh stream and checks it is the first frame the peer sees, so no
    /// refused write before it put a frame on the wire.
    async fn nothing_before_a_marker(
        commands: &Commands,
        end: &Arc<EndRecord>,
        peer: &mut MemoryPeer,
    ) -> TestResult {
        let (mut marker, _route) = open_on(commands, end)?;
        marker.input(b"marker").await?;
        assert_eq!(peer.frame().await?, data(b"marker"));
        marker.close().await?;
        assert_eq!(peer.frame().await?, close());
        Ok(())
    }

    #[tokio::test]
    async fn write_refusal_order() -> TestResult {
        let oversized = vec![0_u8; 16_777_217];

        // Neither ended, over the limit: the size is refused, and a small write still works.
        let (commands, end, mut peer) = writer();
        let (mut stream, route) = open_on(&commands, &end)?;
        assert!(matches!(
            stream.input(&oversized).await,
            Err(Error::PayloadTooLarge { len: 16_777_217 })
        ));
        assert!(matches!(
            stream.automatic_input(&oversized).await,
            Err(Error::PayloadTooLarge { len: 16_777_217 })
        ));
        stream.input(b"small").await?;
        assert_eq!(peer.frame().await?, data(b"small"));

        // The stream ended, the connection did not: the stream's refusal wins over the size.
        route.mark_closed_by_core();
        assert!(matches!(
            stream.input(&oversized).await,
            Err(Error::StreamClosed)
        ));
        assert!(matches!(
            stream.automatic_input(b"x").await,
            Err(Error::StreamClosed)
        ));
        assert!(matches!(
            stream.resize(1, 1).await,
            Err(Error::StreamClosed)
        ));
        assert!(matches!(
            stream.focus(crate::protocol::Focus::Lost).await,
            Err(Error::StreamClosed)
        ));
        nothing_before_a_marker(&commands, &end, &mut peer).await?;

        // A stream that closed itself has ended too.
        let (mut closed, _route) = open_on(&commands, &end)?;
        closed.close().await?;
        assert_eq!(peer.frame().await?, close());
        assert!(matches!(closed.input(b"x").await, Err(Error::StreamClosed)));
        nothing_before_a_marker(&commands, &end, &mut peer).await?;

        // Both ended: the connection's refusal wins.
        end.record(ConnectionEndKind::PeerClosed, "gone".to_owned());
        assert!(matches!(stream.input(&oversized).await, Err(Error::Closed)));
        assert!(matches!(stream.input(b"x").await, Err(Error::Closed)));
        assert!(matches!(closed.input(b"x").await, Err(Error::Closed)));
        assert!(!peer.has_unread());
        Ok(())
    }
}
