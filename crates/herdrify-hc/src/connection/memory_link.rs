//! An in-memory transport for the connection layer's own unit tests.
//!
//! The link is a [`Transport`] whose other end is a [`MemoryPeer`] the test holds. The test reads
//! what the client sent, answers with lines and frames, makes the next write fail and ends the
//! stream. It names no adapter and no async I/O type, only channels, so the connection layer's
//! logic is tested against the traits alone.
//!
//! Responsibilities:
//!
//! - [`link`], which returns the transport and its peer
//! - [`MemoryPeer`], the test's side of the link
//!
//! Non-responsibilities:
//!
//! - testing the traits' contract, which each real transport's own tests do

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::timeout;

use crate::port::{FrameSink, FrameSource, Transport, TransportError};
use crate::protocol::{Frame, Line};

/// How long a test waits for something the client should already have sent.
const PATIENCE: Duration = Duration::from_secs(5);

/// What the client sent.
enum Sent {
    Line(Line),
    Frame(Frame),
}

/// What the peer sends to the client.
enum Inbound {
    Line(Line),
    Frame(Frame),
}

/// The test's side of the link.
pub(super) struct MemoryPeer {
    sent: mpsc::UnboundedReceiver<Sent>,
    inbound: Option<mpsc::UnboundedSender<Inbound>>,
    fail_next_write: Arc<AtomicBool>,
}

/// A transport and the peer on its other end.
pub(super) fn link() -> (MemoryTransport, MemoryPeer) {
    let (sent_tx, sent_rx) = mpsc::unbounded_channel();
    let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
    let fail_next_write = Arc::new(AtomicBool::new(false));
    (
        MemoryTransport {
            sent: sent_tx,
            inbound: inbound_rx,
            fail_next_write: Arc::clone(&fail_next_write),
        },
        MemoryPeer {
            sent: sent_rx,
            inbound: Some(inbound_tx),
            fail_next_write,
        },
    )
}

impl MemoryPeer {
    /// The next line the client sent.
    ///
    /// # Errors
    ///
    /// Fails if the client sent a frame next, sent nothing within the wait, or is gone.
    pub(super) async fn line(&mut self) -> Result<Line, String> {
        match timeout(PATIENCE, self.sent.recv()).await {
            Ok(Some(Sent::Line(line))) => Ok(line),
            Ok(Some(Sent::Frame(_))) => Err("expected a line, the client sent a frame".to_owned()),
            Ok(None) => Err("the client is gone".to_owned()),
            Err(_) => Err("the client sent nothing".to_owned()),
        }
    }

    /// The next frame the client sent.
    ///
    /// # Errors
    ///
    /// Fails if the client sent a line next, sent nothing within the wait, or is gone.
    pub(super) async fn frame(&mut self) -> Result<Frame, String> {
        match timeout(PATIENCE, self.sent.recv()).await {
            Ok(Some(Sent::Frame(frame))) => Ok(frame),
            Ok(Some(Sent::Line(_))) => Err("expected a frame, the client sent a line".to_owned()),
            Ok(None) => Err("the client is gone".to_owned()),
            Err(_) => Err("the client sent nothing".to_owned()),
        }
    }

    /// Whether the client has sent something the test has not read.
    pub(super) fn has_unread(&mut self) -> bool {
        self.sent.try_recv().is_ok()
    }

    /// Sends the client one line.
    pub(super) fn reply_line(&self, text: &str) {
        if let (Some(inbound), Ok(line)) = (&self.inbound, Line::new(text)) {
            let _ = inbound.send(Inbound::Line(line));
        }
    }

    /// Sends the client one frame.
    pub(super) fn push(&self, frame: Frame) {
        if let Some(inbound) = &self.inbound {
            let _ = inbound.send(Inbound::Frame(frame));
        }
    }

    /// Ends the stream the client reads: it sees the peer close.
    pub(super) fn end(&mut self) {
        self.inbound = None;
    }

    /// Makes the client's next frame write fail.
    pub(super) fn fail_next_write(&self) {
        self.fail_next_write.store(true, Ordering::SeqCst);
    }
}

/// The client's side of the link, before the handshake.
pub(super) struct MemoryTransport {
    sent: mpsc::UnboundedSender<Sent>,
    inbound: mpsc::UnboundedReceiver<Inbound>,
    fail_next_write: Arc<AtomicBool>,
}

impl Transport for MemoryTransport {
    type Sink = MemorySink;
    type Source = MemorySource;

    async fn send_line(&mut self, line: Line) -> Result<(), TransportError> {
        self.sent.send(Sent::Line(line)).map_err(|_| broken_pipe())
    }

    async fn recv_line(&mut self) -> Result<Option<Line>, TransportError> {
        match self.inbound.recv().await {
            Some(Inbound::Line(line)) => Ok(Some(line)),
            Some(Inbound::Frame(_)) => Err(wrong_mode("a frame where a line was expected")),
            None => Ok(None),
        }
    }

    fn into_framed(self) -> (MemorySink, MemorySource) {
        (
            MemorySink {
                sent: self.sent,
                fail_next_write: self.fail_next_write,
            },
            MemorySource {
                inbound: self.inbound,
            },
        )
    }
}

/// The write half of the link.
pub(super) struct MemorySink {
    sent: mpsc::UnboundedSender<Sent>,
    fail_next_write: Arc<AtomicBool>,
}

impl FrameSink for MemorySink {
    async fn send(&mut self, frame: Frame) -> Result<(), TransportError> {
        if self.fail_next_write.swap(false, Ordering::SeqCst) {
            return Err(broken_pipe());
        }
        self.sent
            .send(Sent::Frame(frame))
            .map_err(|_| broken_pipe())
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        Ok(())
    }
}

/// The read half of the link.
pub(super) struct MemorySource {
    inbound: mpsc::UnboundedReceiver<Inbound>,
}

impl FrameSource for MemorySource {
    async fn recv(&mut self) -> Result<Option<Frame>, TransportError> {
        match self.inbound.recv().await {
            Some(Inbound::Frame(frame)) => Ok(Some(frame)),
            Some(Inbound::Line(_)) => Err(wrong_mode("a line where a frame was expected")),
            None => Ok(None),
        }
    }
}

fn broken_pipe() -> TransportError {
    TransportError::Io(io::Error::from(io::ErrorKind::BrokenPipe))
}

fn wrong_mode(what: &str) -> TransportError {
    TransportError::Io(io::Error::new(io::ErrorKind::InvalidData, what.to_owned()))
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::link;
    use crate::port::{FrameSink, FrameSource, Transport, TransportError};
    use crate::protocol::{Channel, Frame, FrameKind, Line};

    type TestResult = Result<(), Box<dyn Error>>;

    fn data(payload: &[u8]) -> Frame {
        Frame {
            channel: Channel::RPC,
            kind: FrameKind::Data,
            payload: payload.to_vec(),
        }
    }

    #[tokio::test]
    async fn lines_then_frames_pass_both_ways() -> TestResult {
        let (mut transport, mut peer) = link();
        transport.send_line(Line::new("out")?).await?;
        assert_eq!(peer.line().await?.as_str(), "out");
        peer.reply_line("in");
        assert_eq!(
            transport.recv_line().await?.map(Line::into_string),
            Some("in".to_owned())
        );

        let (mut sink, mut source) = transport.into_framed();
        sink.send(data(b"a")).await?;
        assert_eq!(peer.frame().await?, data(b"a"));
        peer.push(data(b"b"));
        assert_eq!(source.recv().await?, Some(data(b"b")));
        Ok(())
    }

    #[tokio::test]
    async fn ending_the_peer_is_end_of_stream() -> TestResult {
        let (transport, mut peer) = link();
        let (_sink, mut source) = transport.into_framed();
        peer.end();
        assert_eq!(source.recv().await?, None);
        Ok(())
    }

    #[tokio::test]
    async fn only_the_next_write_fails() -> TestResult {
        let (transport, mut peer) = link();
        let (mut sink, _source) = transport.into_framed();
        peer.fail_next_write();
        assert!(matches!(
            sink.send(data(b"a")).await,
            Err(TransportError::Io(_))
        ));
        sink.send(data(b"b")).await?;
        assert_eq!(peer.frame().await?, data(b"b"));
        assert!(!peer.has_unread());
        Ok(())
    }
}
