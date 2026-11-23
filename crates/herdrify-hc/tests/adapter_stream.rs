//! `StreamTransport` over an in-memory pipe, driven only through the public API.
//!
//! The test plays herdr-core's side of a `tokio::io::duplex` pair and checks what the client side
//! reads and writes: the hello reply line followed by frames in one write, the end of the stream
//! inside a line and inside a frame, and closing the sink.
//!
//! Responsibilities:
//!
//! - the line-to-frame hand-off, the partial-message ends and the sink close of `StreamTransport`
//!
//! Non-responsibilities:
//!
//! - the codecs themselves, which their own unit tests cover

use std::time::Duration;

use herdrify_hc::protocol::{Channel, Frame, FrameKind, Line};
use herdrify_hc::{FrameSink, FrameSource, StreamTransport, Transport};
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream, ReadHalf, WriteHalf, duplex, split};
use tokio::time::timeout;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// How long a test waits for something that should already have arrived, so a regression fails
/// the test instead of hanging it.
const PATIENCE: Duration = Duration::from_secs(5);

/// A transport over one end of a pipe, and the other end for the test to play the core with.
fn connected() -> (
    StreamTransport<ReadHalf<DuplexStream>, WriteHalf<DuplexStream>>,
    DuplexStream,
) {
    let (client, core) = duplex(1024);
    let (reader, writer) = split(client);
    (StreamTransport::new(reader, writer), core)
}

fn data(payload: &[u8]) -> Frame {
    Frame {
        channel: Channel::RPC,
        kind: FrameKind::Data,
        payload: payload.to_vec(),
    }
}

#[tokio::test]
async fn hello_line_then_frames_in_one_write() -> TestResult {
    let (mut transport, mut core) = connected();
    let first = data(b"{\"id\":1}");
    let second = data(b"{\"id\":2}");
    let mut bytes = b"{\"id\":0,\"result\":true}\n".to_vec();
    bytes.extend(first.encode()?);
    bytes.extend(second.encode()?);
    core.write_all(&bytes).await?;

    let line = timeout(PATIENCE, transport.recv_line()).await??;
    assert_eq!(
        line.as_ref().map(Line::as_str),
        Some("{\"id\":0,\"result\":true}")
    );
    let (_sink, mut source) = transport.into_framed();
    assert_eq!(timeout(PATIENCE, source.recv()).await??, Some(first));
    assert_eq!(timeout(PATIENCE, source.recv()).await??, Some(second));
    Ok(())
}

#[tokio::test]
async fn eof_inside_a_line_is_peer_closed() -> TestResult {
    let (mut transport, mut core) = connected();
    core.write_all(b"{\"id\":0").await?;
    drop(core);
    assert_eq!(transport.recv_line().await?, None);
    Ok(())
}

#[tokio::test]
async fn eof_inside_a_frame_is_peer_closed() -> TestResult {
    let (mut transport, mut core) = connected();
    core.write_all(b"{}\n").await?;
    assert!(transport.recv_line().await?.is_some());
    let (_sink, mut source) = transport.into_framed();
    let bytes = data(b"{\"id\":1}").encode()?;
    core.write_all(&bytes[..bytes.len() / 2]).await?;
    drop(core);
    assert_eq!(source.recv().await?, None);
    Ok(())
}

#[tokio::test]
async fn sink_close_reaches_the_peer_as_eof() -> TestResult {
    let (transport, mut core) = connected();
    let (mut sink, _source) = transport.into_framed();
    sink.close().await?;
    assert_eq!(timeout(PATIENCE, core.read(&mut [0; 8])).await??, 0);
    Ok(())
}
