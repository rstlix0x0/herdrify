//! The handle a stream channel shares between the router and the caller that owns the stream.
//!
//! The core opens a stream channel when it replies to `events.subscribe` or `terminal.attach`. The
//! router answers that reply with a [`StreamHandle`] and keeps the matching [`Route`], so the
//! frames the core sends on the channel reach the handle's owner in the order they arrived.
//!
//! A stream is closed at most once by its owner. The handle sends the CLOSE, either when its owner
//! asks for it with [`StreamHandle::close`] or, if it never did, when the handle drops. It sends
//! none once the core has closed the channel: the router marks the route closed by the core before
//! it forwards the core's CLOSE, so the mark is set even while that frame waits to be read. A route
//! opened for a caller that never gets a stream is closed by the same drop.
//!
//! Responsibilities:
//!
//! - [`open`], which makes the two ends of a route for one channel
//! - [`Route`], the router's end, which forwards frames to the handle and marks the channel closed
//!   by the core
//! - [`StreamHandle`], the caller's end: the channel, the frames routed to it, the way to send
//!   credit, the checked write of a frame and the CLOSE rule
//! - [`Opening`], what a call that opens a stream hands the router
//!
//! Non-responsibilities:
//!
//! - deciding which frames belong to a stream, which the router does
//! - reading what a stream carries, which `Subscription` and `AttachStream` do

use std::future::{Future, poll_fn};
use std::pin::{Pin, pin};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::Poll;

use tokio::sync::{mpsc, oneshot};

use super::Error;
use super::error::EndRecord;
use super::writer::{self, Command, Commands};
use crate::protocol::{Channel, Frame, FrameKind, MAX_PAYLOAD_LEN};

/// What a call that opens a stream gives the router, so it can build the stream's handle.
pub(super) struct Opening {
    /// A sender to the writer, which the handle keeps for its own frames.
    pub(super) commands: Commands,
    /// The connection's end record, which the handle reads to tell a closed connection.
    pub(super) end: Arc<EndRecord>,
}

/// The router's end of a stream channel.
pub(super) struct Route {
    frames: mpsc::UnboundedSender<Frame>,
    closed_by_core: Arc<AtomicBool>,
}

impl Route {
    /// Hands `frame` to the handle. Returns `false` when the handle is gone.
    pub(super) fn forward(&self, frame: Frame) -> bool {
        self.frames.send(frame).is_ok()
    }

    /// Whether the handle is gone, so nothing can read what this route forwards.
    pub(super) fn is_closed(&self) -> bool {
        self.frames.is_closed()
    }

    /// Records that the core closed the channel. The router calls it before it forwards the
    /// core's CLOSE, so the handle sends no CLOSE of its own from then on.
    pub(super) fn mark_closed_by_core(&self) {
        self.closed_by_core.store(true, Ordering::Release);
    }
}

/// The caller's end of a stream channel.
///
/// Dropping it sends CLOSE, unless the caller closed the stream or the core did.
pub(super) struct StreamHandle {
    channel: Channel,
    frames: mpsc::UnboundedReceiver<Frame>,
    commands: Commands,
    end: Arc<EndRecord>,
    closed_by_core: Arc<AtomicBool>,
    closed_locally: bool,
}

/// Makes the two ends of the route for `channel`.
pub(super) fn open(channel: Channel, opening: Opening) -> (Route, StreamHandle) {
    let (sender, frames) = mpsc::unbounded_channel();
    let closed_by_core = Arc::new(AtomicBool::new(false));
    (
        Route {
            frames: sender,
            closed_by_core: Arc::clone(&closed_by_core),
        },
        StreamHandle {
            channel,
            frames,
            commands: opening.commands,
            end: opening.end,
            closed_by_core,
            closed_locally: false,
        },
    )
}

impl StreamHandle {
    /// The channel the stream runs on.
    pub(super) const fn channel(&self) -> Channel {
        self.channel
    }

    /// The next frame the router routed here, or `None` once the router has dropped the route: the
    /// core closed the channel, or the connection ended.
    ///
    /// Cancel-safe: a frame is taken only when this returns it.
    pub(super) async fn recv(&mut self) -> Option<Frame> {
        self.frames.recv().await
    }

    /// Queues CREDIT for `bytes` stream bytes, without waiting for it to be written.
    pub(super) fn credit(&self, bytes: u32) {
        writer::send_credit(&self.commands, self.channel, bytes);
    }

    /// Writes a frame of `kind` carrying `payload` on the stream, checked in this order before
    /// anything is sent:
    ///
    /// 1. the connection has ended: [`Error::Closed`]
    /// 2. the stream has ended, because the core closed it or the caller closed it:
    ///    [`Error::StreamClosed`]
    /// 3. `payload` is over 16777216 bytes: [`Error::PayloadTooLarge`]
    ///
    /// Then it waits for the frame's write, which ends with [`Error::Transport`] if the write
    /// fails, and with [`Error::Closed`] if the connection ends first. `payload` is copied only
    /// once the checks have passed.
    ///
    /// A call dropped once polled has already queued its frame, so the frame is still written.
    pub(super) async fn write(&self, kind: FrameKind, payload: &[u8]) -> Result<(), Error> {
        if self.end.is_ended() {
            return Err(Error::Closed);
        }
        if self.closed_locally || self.closed_by_core.load(Ordering::Acquire) {
            return Err(Error::StreamClosed);
        }
        if payload.len() > MAX_PAYLOAD_LEN {
            return Err(Error::PayloadTooLarge { len: payload.len() });
        }
        let (ack, written) = oneshot::channel();
        self.commands
            .send(Command {
                frame: Frame {
                    channel: self.channel,
                    kind,
                    payload: payload.to_vec(),
                },
                ack: Some(ack),
            })
            .map_err(|_| Error::Closed)?;
        acknowledged(written, &self.end).await
    }

    /// Closes the stream from this side, checked in this order:
    ///
    /// 1. the connection has ended: [`Error::Closed`], nothing sent
    /// 2. the stream has ended, because the core closed it or this already ran: `Ok(())`, nothing
    ///    sent
    /// 3. otherwise CLOSE is sent, and this waits for its write
    ///
    /// The wait ends with [`Error::Closed`] if the connection ends first, so a write the core has
    /// stopped reading cannot hold the caller after the connection is over.
    pub(super) async fn close(&mut self) -> Result<(), Error> {
        if self.end.is_ended() {
            return Err(Error::Closed);
        }
        if self.closed_locally || self.closed_by_core.load(Ordering::Acquire) {
            return Ok(());
        }
        let (ack, written) = oneshot::channel();
        self.commands
            .send(writer::close_command(self.channel, Some(ack)))
            .map_err(|_| Error::Closed)?;
        self.closed_locally = true;
        acknowledged(written, &self.end).await
    }
}

impl Drop for StreamHandle {
    fn drop(&mut self) {
        if !self.closed_locally && !self.closed_by_core.load(Ordering::Acquire) {
            writer::send_close(&self.commands, self.channel);
        }
    }
}

/// Waits for the writer's report on a frame, or for the connection to end, whichever comes first.
///
/// A report that is already there wins over an end recorded at the same time.
async fn acknowledged(
    mut written: oneshot::Receiver<Result<(), Error>>,
    end: &EndRecord,
) -> Result<(), Error> {
    let mut ended = pin!(end.wait_ended());
    // `None` is the end of the connection.
    let report = poll_fn(|cx| {
        if let Poll::Ready(report) = Pin::new(&mut written).poll(cx) {
            return Poll::Ready(Some(report));
        }
        ended.as_mut().poll(cx).map(|()| None)
    })
    .await;
    match report {
        Some(Ok(report)) => report,
        Some(Err(_)) | None => Err(Error::Closed),
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;
    use std::sync::Arc;
    use std::time::Duration;

    use tokio::sync::{mpsc, oneshot};

    use super::{Opening, open};
    use crate::connection::error::EndRecord;
    use crate::connection::memory_link::{MemoryPeer, link};
    use crate::connection::pending::Pending;
    use crate::connection::writer::{self, Command, Commands};
    use crate::connection::{ConnectionEndKind, Error};
    use crate::port::Transport;
    use crate::protocol::{Channel, Frame, FrameKind};

    type TestResult = Result<(), Box<dyn StdError>>;

    fn frame(kind: FrameKind, payload: &[u8]) -> Frame {
        Frame {
            channel: Channel::new(3),
            kind,
            payload: payload.to_vec(),
        }
    }

    fn close_frame() -> Frame {
        frame(FrameKind::Close, b"")
    }

    #[tokio::test]
    async fn routed_frames_arrive_in_order() {
        let (commands, _queue) = mpsc::unbounded_channel();
        let opening = Opening {
            commands,
            end: Arc::new(EndRecord::new()),
        };
        let (route, mut handle) = open(Channel::new(3), opening);
        assert_eq!(handle.channel(), Channel::new(3));
        assert!(route.forward(frame(FrameKind::Data, b"a")));
        assert!(route.forward(frame(FrameKind::Resize, b"bbbb")));
        assert_eq!(handle.recv().await, Some(frame(FrameKind::Data, b"a")));
        assert_eq!(handle.recv().await, Some(frame(FrameKind::Resize, b"bbbb")));
        drop(route);
        assert_eq!(handle.recv().await, None);
    }

    #[test]
    fn forward_reports_a_dropped_handle() {
        let (commands, _queue) = mpsc::unbounded_channel();
        let opening = Opening {
            commands,
            end: Arc::new(EndRecord::new()),
        };
        let (route, handle) = open(Channel::new(3), opening);
        drop(handle);
        assert!(!route.forward(frame(FrameKind::Data, b"a")));
    }

    /// A writer over a memory link, and the end record it shares with the handles built on it.
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

    /// Writes a marker frame and waits for it, so every command queued before it was handled.
    async fn flush(commands: &Commands) -> TestResult {
        let (ack, written) = oneshot::channel();
        commands.send(Command {
            frame: frame(FrameKind::Data, b"marker"),
            ack: Some(ack),
        })?;
        written.await??;
        Ok(())
    }

    #[tokio::test]
    async fn handle_drop_follows_the_close_rule() -> TestResult {
        let (commands, end, mut peer) = writer();
        let opening = |commands: &Commands| Opening {
            commands: commands.clone(),
            end: Arc::clone(&end),
        };

        // An open handle sends one CLOSE when it drops.
        let (_route, handle) = open(Channel::new(3), opening(&commands));
        drop(handle);
        assert_eq!(peer.frame().await?, close_frame());

        // A handle that closed already sends nothing more.
        let (_route, mut handle) = open(Channel::new(3), opening(&commands));
        handle.close().await?;
        assert_eq!(peer.frame().await?, close_frame());
        drop(handle);
        flush(&commands).await?;
        assert_eq!(peer.frame().await?, frame(FrameKind::Data, b"marker"));

        // A handle the core closed sends nothing.
        let (route, handle) = open(Channel::new(3), opening(&commands));
        route.mark_closed_by_core();
        drop(handle);
        flush(&commands).await?;
        assert_eq!(peer.frame().await?, frame(FrameKind::Data, b"marker"));
        assert!(!peer.has_unread());
        Ok(())
    }

    #[tokio::test]
    async fn write_sends_a_frame_on_the_channel() -> TestResult {
        let (commands, end, mut peer) = writer();
        let (_route, handle) = open(
            Channel::new(3),
            Opening {
                commands: commands.clone(),
                end: Arc::clone(&end),
            },
        );
        handle.write(FrameKind::Facts, b"{}").await?;
        assert_eq!(peer.frame().await?, frame(FrameKind::Facts, b"{}"));

        // A write the writer fails is reported, not lost.
        peer.fail_next_write();
        let failed = handle.write(FrameKind::Data, b"x").await;
        assert!(matches!(failed, Err(Error::Transport(_))), "{failed:?}");
        assert!(end.is_ended());
        Ok(())
    }

    #[tokio::test]
    async fn write_accepts_the_largest_payload_and_refuses_one_byte_more() -> TestResult {
        let (commands, end, mut peer) = writer();
        let (_route, handle) = open(
            Channel::new(3),
            Opening {
                commands: commands.clone(),
                end: Arc::clone(&end),
            },
        );
        let largest = vec![7_u8; 16_777_216];
        let over = vec![7_u8; 16_777_217];

        let refused = handle.write(FrameKind::Data, &over).await;
        assert!(
            matches!(refused, Err(Error::PayloadTooLarge { len: 16_777_217 })),
            "{refused:?}"
        );

        tokio::time::timeout(Duration::from_secs(10), async {
            handle.write(FrameKind::Data, &largest).await?;
            assert_eq!(peer.frame().await?, frame(FrameKind::Data, &largest));
            Ok::<(), Box<dyn StdError>>(())
        })
        .await??;
        Ok(())
    }

    #[tokio::test]
    async fn close_follows_the_rule_in_order() -> TestResult {
        let (commands, end, mut peer) = writer();
        let opening = || Opening {
            commands: commands.clone(),
            end: Arc::clone(&end),
        };

        // Once only: the second call sends nothing.
        let (_route, mut handle) = open(Channel::new(3), opening());
        handle.close().await?;
        handle.close().await?;
        assert_eq!(peer.frame().await?, close_frame());
        flush(&commands).await?;
        assert_eq!(peer.frame().await?, frame(FrameKind::Data, b"marker"));

        // Not when the core closed the channel first.
        let (route, mut handle) = open(Channel::new(3), opening());
        route.mark_closed_by_core();
        handle.close().await?;
        flush(&commands).await?;
        assert_eq!(peer.frame().await?, frame(FrameKind::Data, b"marker"));

        // A connection that ended wins over an ended stream.
        end.record(ConnectionEndKind::PeerClosed, "gone".to_owned());
        assert!(matches!(handle.close().await, Err(Error::Closed)));
        let (_route, mut fresh) = open(Channel::new(3), opening());
        assert!(matches!(fresh.close().await, Err(Error::Closed)));
        assert!(!peer.has_unread());
        Ok(())
    }

    #[tokio::test]
    async fn close_stops_waiting_when_the_connection_ends() -> TestResult {
        // A writer that never acknowledges: its queue is held by the test.
        let (commands, mut queue) = mpsc::unbounded_channel();
        let end = Arc::new(EndRecord::new());
        let (_route, mut handle) = open(
            Channel::new(3),
            Opening {
                commands,
                end: Arc::clone(&end),
            },
        );
        let ender = Arc::clone(&end);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            ender.record(ConnectionEndKind::PeerClosed, "gone".to_owned());
        });
        let closed = tokio::time::timeout(Duration::from_secs(5), handle.close()).await?;
        assert!(matches!(closed, Err(Error::Closed)));
        assert!(queue.try_recv().is_ok(), "the CLOSE was queued");
        Ok(())
    }
}
