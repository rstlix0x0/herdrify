//! The task that writes frames to the core.
//!
//! The writer owns the write half of the transport. Everything that sends a frame hands it a
//! [`Command`] over an unbounded channel, so a write never waits on another caller and frames go
//! out in the order they were queued. A command may carry a one-shot channel that receives the
//! result of its write.
//!
//! A failed write ends the connection: the writer records why, fails every waiting call and stops
//! the router. Once the connection has ended, a command is acknowledged with `Closed` and nothing
//! is written.
//!
//! Responsibilities:
//!
//! - [`spawn`], which starts the task over a [`FrameSink`] and returns the channel that feeds it
//! - [`send_credit`] and [`send_close`], which queue a CREDIT or CLOSE frame without waiting for
//!   its write
//! - [`close_command`], the CLOSE command for a caller that does wait for its write
//! - writing each command's frame and acknowledging it
//! - ending the connection when a write fails
//! - closing the sink when the last sender of the channel has gone
//!
//! Non-responsibilities:
//!
//! - building frames or deciding when to send them, which the client does
//! - reading frames, which the router does

use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};
use tokio::task::AbortHandle;

use super::error::EndRecord;
use super::pending::Pending;
use super::{ConnectionEndKind, Error};
use crate::port::FrameSink;
use crate::protocol::{Channel, Frame, FrameKind};

/// Where a command's write result goes.
pub(super) type Ack = oneshot::Sender<Result<(), Error>>;

/// A frame to write, and where to report the result.
pub(super) struct Command {
    pub(super) frame: Frame,
    pub(super) ack: Option<Ack>,
}

/// The channel that feeds the writer.
pub(super) type Commands = mpsc::UnboundedSender<Command>;

/// Starts the writer over `sink` and returns the channel that feeds it.
///
/// A failed write ends the connection through `pending` and `end`, and aborts `router`. The task
/// holds no sender of its own, so it ends, closing the sink, when the last clone of the returned
/// sender drops.
pub(super) fn spawn<S: FrameSink>(
    mut sink: S,
    pending: Arc<Pending>,
    end: Arc<EndRecord>,
    router: AbortHandle,
) -> Commands {
    let (commands, mut queue) = mpsc::unbounded_channel::<Command>();
    tokio::spawn(async move {
        while let Some(Command { frame, ack }) = queue.recv().await {
            let result = if end.is_ended() {
                Err(Error::Closed)
            } else {
                match sink.send(frame).await {
                    Ok(()) => Ok(()),
                    Err(error) => {
                        pending.end(&end, ConnectionEndKind::TransportFailed, error.to_string());
                        router.abort();
                        Err(Error::Transport(error))
                    }
                }
            };
            if let Some(ack) = ack {
                let _ = ack.send(result);
            }
        }
        let _ = sink.close().await;
    });
    commands
}

/// Queues a CREDIT frame granting `bytes` more bytes on `channel`, and does not wait for its write.
///
/// The writer runs for as long as any sender exists, so the frame is queued even after the
/// connection has ended. The writer then drops it without writing.
pub(super) fn send_credit(commands: &Commands, channel: Channel, bytes: u32) {
    let _ = commands.send(Command {
        frame: Frame::credit(channel, bytes),
        ack: None,
    });
}

/// A command that writes an empty CLOSE frame on `channel`, acknowledged on `ack` if one is given.
pub(super) const fn close_command(channel: Channel, ack: Option<Ack>) -> Command {
    Command {
        frame: Frame {
            channel,
            kind: FrameKind::Close,
            payload: Vec::new(),
        },
        ack,
    }
}

/// Queues a CLOSE frame for `channel` without waiting for its write, as a `Drop` must.
///
/// Nothing is written when the connection has ended: the writer drops the command, and there is no
/// channel left to close.
pub(super) fn send_close(commands: &Commands, channel: Channel) {
    let _ = commands.send(close_command(channel, None));
}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;
    use std::sync::Arc;

    use tokio::sync::{mpsc, oneshot};
    use tokio::task::JoinHandle;

    use super::{Command, send_close, send_credit, spawn};
    use crate::connection::error::EndRecord;
    use crate::connection::memory_link::link;
    use crate::connection::pending::Pending;
    use crate::connection::{ConnectionEndKind, Error};
    use crate::port::{Transport, TransportError};
    use crate::protocol::{Channel, Frame, FrameKind};

    type TestResult = Result<(), Box<dyn StdError>>;

    fn data(payload: &[u8]) -> Frame {
        Frame {
            channel: Channel::RPC,
            kind: FrameKind::Data,
            payload: payload.to_vec(),
        }
    }

    /// A task standing in for the router, which never ends on its own.
    fn idle_router() -> JoinHandle<()> {
        tokio::spawn(std::future::pending())
    }

    #[tokio::test]
    async fn writer_acknowledges_a_write() -> TestResult {
        let (transport, mut peer) = link();
        let (sink, _source) = transport.into_framed();
        let router = idle_router();
        let commands = spawn(
            sink,
            Arc::new(Pending::new()),
            Arc::new(EndRecord::new()),
            router.abort_handle(),
        );
        let (ack, acknowledged) = oneshot::channel();
        commands.send(Command {
            frame: data(b"{}"),
            ack: Some(ack),
        })?;
        acknowledged.await??;
        assert_eq!(peer.frame().await?, data(b"{}"));
        assert!(!router.is_finished());
        Ok(())
    }

    #[tokio::test]
    async fn writer_failure_ends_the_connection() -> TestResult {
        let (transport, peer) = link();
        let (sink, _source) = transport.into_framed();
        let pending = Arc::new(Pending::new());
        let (_id, mut waiting) = pending.register();
        let end = Arc::new(EndRecord::new());
        let router = idle_router();
        let commands = spawn(
            sink,
            Arc::clone(&pending),
            Arc::clone(&end),
            router.abort_handle(),
        );

        peer.fail_next_write();
        let (ack, acknowledged) = oneshot::channel();
        commands.send(Command {
            frame: data(b"{}"),
            ack: Some(ack),
        })?;
        assert!(matches!(
            acknowledged.await?,
            Err(Error::Transport(TransportError::Io(_)))
        ));
        let recorded = end.get().ok_or("no end recorded")?;
        assert_eq!(recorded.kind, ConnectionEndKind::TransportFailed);
        assert!(recorded.message.starts_with("I/O error: "));
        assert!(waiting.try_recv().is_err(), "a waiting call is failed");
        assert!(pending.is_empty());
        assert!(router.await.is_err_and(|error| error.is_cancelled()));

        let (ack, acknowledged) = oneshot::channel();
        commands.send(Command {
            frame: data(b"{}"),
            ack: Some(ack),
        })?;
        assert!(matches!(acknowledged.await?, Err(Error::Closed)));
        Ok(())
    }

    #[tokio::test]
    async fn credit_is_queued_without_an_acknowledgement() -> TestResult {
        let (transport, mut peer) = link();
        let (sink, _source) = transport.into_framed();
        let router = idle_router();
        let commands = spawn(
            sink,
            Arc::new(Pending::new()),
            Arc::new(EndRecord::new()),
            router.abort_handle(),
        );
        send_credit(&commands, Channel::new(4), 339);
        assert_eq!(peer.frame().await?, Frame::credit(Channel::new(4), 339));
        Ok(())
    }

    #[tokio::test]
    async fn drop_time_close_on_open_and_ended_connections() -> TestResult {
        let close = |channel| Frame {
            channel: Channel::new(channel),
            kind: FrameKind::Close,
            payload: Vec::new(),
        };
        let (transport, mut peer) = link();
        let (sink, _source) = transport.into_framed();
        let end = Arc::new(EndRecord::new());
        let router = idle_router();
        let commands = spawn(
            sink,
            Arc::new(Pending::new()),
            Arc::clone(&end),
            router.abort_handle(),
        );

        send_close(&commands, Channel::new(5));
        assert_eq!(peer.frame().await?, close(5));

        end.record(ConnectionEndKind::PeerClosed, "gone".to_owned());
        send_close(&commands, Channel::new(5));
        let (ack, acknowledged) = oneshot::channel();
        commands.send(Command {
            frame: data(b"{}"),
            ack: Some(ack),
        })?;
        assert!(matches!(acknowledged.await?, Err(Error::Closed)));
        assert!(
            !peer.has_unread(),
            "nothing is written once the connection ended"
        );

        let (stopped, queue) = mpsc::unbounded_channel();
        drop(queue);
        send_close(&stopped, Channel::new(5));
        Ok(())
    }
}
