//! A scripted `Transport` for driving a `Client` from outside the crate.
//!
//! The transport implements the public traits only, which also shows that a caller can write one.
//! The test holds its other end, a [`FakePeer`], and plays herdr-core with it: it scripts the hello
//! reply line, reads each line and frame the client sent, pushes frames back and ends the stream.
//! Every wait on the client is bounded, so a regression fails the test instead of hanging it.

use std::error::Error;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use herdrify_hc::protocol::{Channel, Frame, FrameKind, Line};
use herdrify_hc::{FrameSink, FrameSource, Transport, TransportError};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio::time::timeout;

/// How long a test waits for something the client should already have done.
const PATIENCE: Duration = Duration::from_secs(5);

/// How long a test waits to be sure the client is not sending.
const QUIET: Duration = Duration::from_millis(100);

/// Anything a test can fail with.
pub type TestError = Box<dyn Error + Send + Sync>;

/// What the client sent.
enum Sent {
    Line(Line),
    Frame(Frame),
    Closed,
}

/// What the peer sends to the client.
enum Inbound {
    Line(Line),
    Frame(Frame),
    Fail(TransportError),
}

/// A transport and the peer on its other end.
#[must_use]
pub fn fake_pair() -> (FakeTransport, FakePeer) {
    let (sent_tx, sent_rx) = mpsc::unbounded_channel();
    let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
    let fail_next_write = Arc::new(AtomicBool::new(false));
    let stall_next_write = Arc::new(AtomicBool::new(false));
    (
        FakeTransport {
            sent: sent_tx,
            inbound: inbound_rx,
            fail_next_write: Arc::clone(&fail_next_write),
            stall_next_write: Arc::clone(&stall_next_write),
        },
        FakePeer {
            sent: sent_rx,
            inbound: Some(inbound_tx),
            fail_next_write,
            stall_next_write,
        },
    )
}

/// The test's side: herdr-core as the test scripts it.
pub struct FakePeer {
    sent: mpsc::UnboundedReceiver<Sent>,
    inbound: Option<mpsc::UnboundedSender<Inbound>>,
    fail_next_write: Arc<AtomicBool>,
    stall_next_write: Arc<AtomicBool>,
}

impl FakePeer {
    /// Sends the client one line, such as the hello reply.
    ///
    /// # Errors
    ///
    /// Fails if the stream has ended, the client is gone, or `text` holds a newline.
    pub fn reply_line(&self, text: &str) -> Result<(), TestError> {
        let inbound = self.inbound.as_ref().ok_or("the stream has ended")?;
        inbound
            .send(Inbound::Line(Line::new(text)?))
            .map_err(|_| "the client is gone")?;
        Ok(())
    }

    /// Sends the client one frame.
    ///
    /// # Errors
    ///
    /// Fails if the stream has ended or the client is gone.
    pub fn push_frame(&self, frame: Frame) -> Result<(), TestError> {
        let inbound = self.inbound.as_ref().ok_or("the stream has ended")?;
        inbound
            .send(Inbound::Frame(frame))
            .map_err(|_| "the client is gone")?;
        Ok(())
    }

    /// Sends the client one channel-0 DATA frame holding `payload`.
    ///
    /// # Errors
    ///
    /// Fails if the stream has ended or the client is gone.
    pub fn push_data(&self, payload: &[u8]) -> Result<(), TestError> {
        self.push_frame(Frame {
            channel: Channel::RPC,
            kind: FrameKind::Data,
            payload: payload.to_vec(),
        })
    }

    /// Answers request `id` with `result`.
    ///
    /// # Errors
    ///
    /// Fails if the stream has ended or the client is gone.
    pub fn reply(&self, id: u64, result: &Value) -> Result<(), TestError> {
        let reply = serde_json::json!({"id": id, "jsonrpc": "2.0", "result": result});
        self.push_data(reply.to_string().as_bytes())
    }

    /// Makes the client's source return `error` the next time it reads.
    ///
    /// # Errors
    ///
    /// Fails if the stream has ended or the client is gone.
    pub fn fail_source(&self, error: TransportError) -> Result<(), TestError> {
        let inbound = self.inbound.as_ref().ok_or("the stream has ended")?;
        inbound
            .send(Inbound::Fail(error))
            .map_err(|_| "the client is gone")?;
        Ok(())
    }

    /// Makes the client's next frame write fail.
    pub fn fail_next_write(&self) {
        self.fail_next_write.store(true, Ordering::SeqCst);
    }

    /// Makes the client's next frame write never complete, as when the core stops reading.
    pub fn stall_next_write(&self) {
        self.stall_next_write.store(true, Ordering::SeqCst);
    }

    /// Ends the stream the client reads, so it sees the peer close.
    pub fn end(&mut self) {
        self.inbound = None;
    }

    /// The next line the client sent.
    ///
    /// # Errors
    ///
    /// Fails if the client sends a frame next, sends nothing in time, or is gone.
    pub async fn line(&mut self) -> Result<Line, TestError> {
        match timeout(PATIENCE, self.sent.recv()).await? {
            Some(Sent::Line(line)) => Ok(line),
            Some(Sent::Frame(_)) => Err("expected a line, the client sent a frame".into()),
            Some(Sent::Closed) => Err("expected a line, the client closed the sink".into()),
            None => Err("the client is gone".into()),
        }
    }

    /// The next frame the client sent.
    ///
    /// # Errors
    ///
    /// Fails if the client sends a line next, sends nothing in time, or is gone.
    pub async fn frame(&mut self) -> Result<Frame, TestError> {
        match timeout(PATIENCE, self.sent.recv()).await? {
            Some(Sent::Frame(frame)) => Ok(frame),
            Some(Sent::Line(_)) => Err("expected a frame, the client sent a line".into()),
            Some(Sent::Closed) => Err("expected a frame, the client closed the sink".into()),
            None => Err("the client is gone".into()),
        }
    }

    /// Succeeds when the client sends nothing more for a short while.
    ///
    /// # Errors
    ///
    /// Fails if the client sends something.
    pub async fn nothing_sent(&mut self) -> Result<(), TestError> {
        match timeout(QUIET, self.sent.recv()).await {
            Err(_) | Ok(None) => Ok(()),
            Ok(Some(_)) => Err("the client sent something".into()),
        }
    }

    /// Waits until the client closes its write side.
    ///
    /// # Errors
    ///
    /// Fails if the client does not close its sink in time, or drops it without closing.
    pub async fn sink_closed(&mut self) -> Result<(), TestError> {
        loop {
            match timeout(PATIENCE, self.sent.recv()).await? {
                Some(Sent::Closed) => return Ok(()),
                Some(_) => {}
                None => return Err("the client dropped its sink without closing it".into()),
            }
        }
    }

    /// The next frame the client sent, read as a JSON-RPC request.
    ///
    /// # Errors
    ///
    /// Fails if the next frame is not a channel-0 DATA frame holding JSON, or as for `frame`.
    pub async fn request(&mut self) -> Result<Value, TestError> {
        let frame = self.frame().await?;
        if frame.channel != Channel::RPC || frame.kind != FrameKind::Data {
            return Err("expected a channel-0 DATA frame".into());
        }
        Ok(serde_json::from_slice(&frame.payload)?)
    }
}

/// The client's side of the fake, before the handshake.
pub struct FakeTransport {
    sent: mpsc::UnboundedSender<Sent>,
    inbound: mpsc::UnboundedReceiver<Inbound>,
    fail_next_write: Arc<AtomicBool>,
    stall_next_write: Arc<AtomicBool>,
}

impl Transport for FakeTransport {
    type Sink = FakeSink;
    type Source = FakeSource;

    async fn send_line(&mut self, line: Line) -> Result<(), TransportError> {
        self.sent.send(Sent::Line(line)).map_err(|_| broken_pipe())
    }

    async fn recv_line(&mut self) -> Result<Option<Line>, TransportError> {
        match self.inbound.recv().await {
            Some(Inbound::Line(line)) => Ok(Some(line)),
            Some(Inbound::Frame(_)) => Err(wrong_mode()),
            Some(Inbound::Fail(error)) => Err(error),
            None => Ok(None),
        }
    }

    fn into_framed(self) -> (FakeSink, FakeSource) {
        (
            FakeSink {
                sent: self.sent,
                fail_next_write: self.fail_next_write,
                stall_next_write: self.stall_next_write,
            },
            FakeSource {
                inbound: self.inbound,
            },
        )
    }
}

/// The write half of the fake.
pub struct FakeSink {
    sent: mpsc::UnboundedSender<Sent>,
    fail_next_write: Arc<AtomicBool>,
    stall_next_write: Arc<AtomicBool>,
}

impl FrameSink for FakeSink {
    async fn send(&mut self, frame: Frame) -> Result<(), TransportError> {
        if self.fail_next_write.swap(false, Ordering::SeqCst) {
            return Err(broken_pipe());
        }
        if self.stall_next_write.swap(false, Ordering::SeqCst) {
            std::future::pending::<()>().await;
        }
        self.sent
            .send(Sent::Frame(frame))
            .map_err(|_| broken_pipe())
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        self.sent.send(Sent::Closed).map_err(|_| broken_pipe())
    }
}

/// The read half of the fake.
pub struct FakeSource {
    inbound: mpsc::UnboundedReceiver<Inbound>,
}

impl FrameSource for FakeSource {
    async fn recv(&mut self) -> Result<Option<Frame>, TransportError> {
        match self.inbound.recv().await {
            Some(Inbound::Frame(frame)) => Ok(Some(frame)),
            Some(Inbound::Line(_)) => Err(wrong_mode()),
            Some(Inbound::Fail(error)) => Err(error),
            None => Ok(None),
        }
    }
}

fn broken_pipe() -> TransportError {
    TransportError::Io(io::Error::from(io::ErrorKind::BrokenPipe))
}

fn wrong_mode() -> TransportError {
    TransportError::Io(io::Error::from(io::ErrorKind::InvalidData))
}
