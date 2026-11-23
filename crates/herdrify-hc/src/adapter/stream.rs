//! `StreamTransport`: the transport traits over any async reader and writer.
//!
//! A herdr-core connection is two byte streams, one to read and one to write. This file carries the
//! protocol over any such pair, such as a child process's output and input or the two ends of an
//! in-memory pipe. Before the handshake it moves lines; [`Transport::into_framed`] then hands the
//! reader's undecoded bytes to a frame decoder and splits the pair into a sink and a source.
//!
//! Responsibilities:
//!
//! - [`StreamTransport`], which sends and receives lines and splits into frame halves
//! - [`StreamSink`] and [`StreamSource`], which write and read frames
//! - flushing after every send, so a request never waits in a buffer
//! - treating the end of the stream inside a line or a frame as the peer closing, discarding the
//!   partial bytes
//!
//! Non-responsibilities:
//!
//! - starting a process, which the caller does before it hands the streams over
//! - deciding what a line or a frame means, which the client does

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::port::{FrameSink, FrameSource, Transport, TransportError};
use crate::protocol::{Frame, FrameDecoder, Line, LineDecoder};

/// How many bytes one read from the connection asks for.
const READ_CHUNK_LEN: usize = 8192;

/// A transport over any async reader and writer.
///
/// `send_line` writes the line, a `\n` and flushes. `recv_line` returns each line as it completes.
/// Lines have no length cap: the SDK trusts the peer the caller chose, and line mode is not
/// hardened against a hostile peer. If the reader ends inside a line or, after
/// [`into_framed`](Transport::into_framed), inside a frame, the partial bytes are discarded and
/// the receive returns `Ok(None)`, the same as a clean close.
///
/// # Cancel safety
///
/// `send_line` is not cancel safe: dropping it while it runs can leave part of a line on the wire,
/// and the stream is then unusable for writing. `recv_line` is cancel safe.
///
/// # Examples
///
/// ```
/// use herdrify_hc::{StreamTransport, Transport};
///
/// let transport = StreamTransport::new(tokio::io::empty(), tokio::io::sink());
/// let (_sink, _source) = transport.into_framed();
/// ```
#[derive(Debug)]
pub struct StreamTransport<R, W> {
    reader: R,
    writer: W,
    chunk: Vec<u8>,
    decoder: LineDecoder,
}

impl<R, W> StreamTransport<R, W>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    /// A transport that reads from `reader` and writes to `writer`.
    #[must_use]
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader,
            writer,
            chunk: Vec::with_capacity(READ_CHUNK_LEN),
            decoder: LineDecoder::new(),
        }
    }
}

impl<R, W> Transport for StreamTransport<R, W>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    type Sink = StreamSink<W>;
    type Source = StreamSource<R>;

    async fn send_line(&mut self, line: Line) -> Result<(), TransportError> {
        let mut bytes = line.into_string().into_bytes();
        bytes.push(b'\n');
        write_and_flush(&mut self.writer, &bytes).await
    }

    async fn recv_line(&mut self) -> Result<Option<Line>, TransportError> {
        loop {
            if let Some(line) = self.decoder.next_line() {
                return Ok(Some(line));
            }
            if !read_into(&mut self.reader, &mut self.chunk, |bytes| {
                self.decoder.push(bytes);
            })
            .await?
            {
                self.decoder = LineDecoder::new();
                return Ok(None);
            }
        }
    }

    fn into_framed(self) -> (Self::Sink, Self::Source) {
        (
            StreamSink {
                writer: self.writer,
            },
            StreamSource {
                reader: self.reader,
                chunk: self.chunk,
                decoder: self.decoder.into_frame_decoder(),
            },
        )
    }
}

/// The write half of a [`StreamTransport`] after it has been split into frames.
///
/// # Cancel safety
///
/// `send` is not cancel safe: dropping it while it runs can leave part of a frame on the wire,
/// and the peer then reads a corrupt stream. Do not time out or select against it.
#[derive(Debug)]
pub struct StreamSink<W> {
    writer: W,
}

impl<W> FrameSink for StreamSink<W>
where
    W: AsyncWrite + Unpin + Send + 'static,
{
    async fn send(&mut self, frame: Frame) -> Result<(), TransportError> {
        let bytes = frame.encode()?;
        write_and_flush(&mut self.writer, &bytes).await
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        self.writer.shutdown().await.map_err(TransportError::Io)
    }
}

/// The read half of a [`StreamTransport`] after it has been split into frames.
///
/// `recv` is cancel safe: a dropped call loses no byte read from the connection.
#[derive(Debug)]
pub struct StreamSource<R> {
    reader: R,
    chunk: Vec<u8>,
    decoder: FrameDecoder,
}

impl<R> FrameSource for StreamSource<R>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    async fn recv(&mut self) -> Result<Option<Frame>, TransportError> {
        loop {
            if let Some(frame) = self.decoder.next_frame()? {
                return Ok(Some(frame));
            }
            if !read_into(&mut self.reader, &mut self.chunk, |bytes| {
                self.decoder.push(bytes);
            })
            .await?
            {
                self.decoder = FrameDecoder::new();
                return Ok(None);
            }
        }
    }
}

/// Writes all of `bytes`, then flushes.
async fn write_and_flush<W>(writer: &mut W, bytes: &[u8]) -> Result<(), TransportError>
where
    W: AsyncWrite + Unpin,
{
    writer.write_all(bytes).await.map_err(TransportError::Io)?;
    writer.flush().await.map_err(TransportError::Io)
}

/// Reads one chunk and hands it to `push`; `false` means the stream has ended.
///
/// `chunk` is the buffer the read fills; it is emptied first and keeps its capacity across reads.
async fn read_into<R>(
    reader: &mut R,
    chunk: &mut Vec<u8>,
    push: impl FnOnce(&[u8]),
) -> Result<bool, TransportError>
where
    R: AsyncRead + Unpin,
{
    chunk.clear();
    let read = reader.read_buf(chunk).await.map_err(TransportError::Io)?;
    if read == 0 {
        return Ok(false);
    }
    push(chunk);
    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use std::time::Duration;

    use tokio::io::{AsyncRead, AsyncReadExt, BufWriter, ReadBuf, duplex};
    use tokio::time::timeout;

    use super::StreamTransport;
    use crate::port::{FrameSink, FrameSource, Transport, TransportError};
    use crate::protocol::{Channel, Frame, FrameError, Line};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// How long a test waits for bytes that should already have arrived.
    const PATIENCE: Duration = Duration::from_secs(5);

    /// A reader that returns its chunks one per read; an empty chunk is a read of zero bytes, the
    /// end of the stream, after which the next read goes on with the following chunk.
    struct Scripted(VecDeque<Vec<u8>>);

    impl Scripted {
        fn new(chunks: &[&[u8]]) -> Self {
            Self(chunks.iter().map(|chunk| chunk.to_vec()).collect())
        }
    }

    impl AsyncRead for Scripted {
        fn poll_read(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            if let Some(chunk) = self.0.pop_front() {
                buf.put_slice(&chunk);
            }
            Poll::Ready(Ok(()))
        }
    }

    #[tokio::test]
    async fn send_line_appends_newline() -> TestResult {
        let (writer, mut peer) = duplex(64);
        let mut transport = StreamTransport::new(tokio::io::empty(), writer);
        transport.send_line(Line::new("{}")?).await?;
        drop(transport);
        let mut written = Vec::new();
        peer.read_to_end(&mut written).await?;
        assert_eq!(written, b"{}\n");
        Ok(())
    }

    #[tokio::test]
    async fn recv_line_returns_none_on_clean_eof() -> TestResult {
        let mut transport = StreamTransport::new(&b""[..], tokio::io::sink());
        assert_eq!(transport.recv_line().await?, None);
        Ok(())
    }

    #[tokio::test]
    async fn recv_line_discards_a_partial_line_at_eof() -> TestResult {
        let mut transport = StreamTransport::new(&b"{\"id\":0"[..], tokio::io::sink());
        assert_eq!(transport.recv_line().await?, None);
        Ok(())
    }

    #[tokio::test]
    async fn recv_discards_a_partial_frame_at_eof() -> TestResult {
        let transport = StreamTransport::new(&[0, 0, 0, 0, 0][..], tokio::io::sink());
        let (_sink, mut source) = transport.into_framed();
        assert_eq!(source.recv().await?, None);
        Ok(())
    }

    #[tokio::test]
    async fn recv_reports_an_oversized_header() -> TestResult {
        let mut header = vec![0, 0, 0, 0, 0];
        header.extend_from_slice(&16_777_217_u32.to_le_bytes());
        let transport = StreamTransport::new(std::io::Cursor::new(header), tokio::io::sink());
        let (_sink, mut source) = transport.into_framed();
        let refused = source.recv().await;
        assert!(matches!(
            refused,
            Err(TransportError::Frame(FrameError::TooLarge {
                len: 16_777_217
            }))
        ));
        Ok(())
    }

    #[tokio::test]
    async fn sink_encodes_frames() -> TestResult {
        let (writer, mut peer) = duplex(64);
        let transport = StreamTransport::new(tokio::io::empty(), writer);
        let (mut sink, source) = transport.into_framed();
        sink.send(Frame::credit(Channel::new(1), 5)).await?;
        drop((sink, source));
        let mut written = Vec::new();
        peer.read_to_end(&mut written).await?;
        assert_eq!(written, [1, 0, 0, 0, 1, 4, 0, 0, 0, 5, 0, 0, 0]);
        Ok(())
    }

    #[tokio::test]
    async fn recv_line_drops_a_partial_line_for_good() -> TestResult {
        let reader = Scripted::new(&[b"{\"id\":0", b"", b"}\n"]);
        let mut transport = StreamTransport::new(reader, tokio::io::sink());
        assert_eq!(transport.recv_line().await?, None);
        assert_eq!(
            transport.recv_line().await?,
            Some(Line::new("}")?),
            "the dropped bytes must not join the next line"
        );
        Ok(())
    }

    #[tokio::test]
    async fn recv_drops_a_partial_frame_for_good() -> TestResult {
        let frame = Frame::credit(Channel::new(1), 5);
        let bytes = frame.encode()?;
        let reader = Scripted::new(&[&bytes[..6], b"", &bytes]);
        let (_sink, mut source) = StreamTransport::new(reader, tokio::io::sink()).into_framed();
        assert_eq!(source.recv().await?, None);
        assert_eq!(
            source.recv().await?,
            Some(frame),
            "the dropped bytes must not prefix the next frame"
        );
        Ok(())
    }

    #[tokio::test]
    async fn send_line_flushes_a_buffering_writer() -> TestResult {
        let (writer, mut peer) = duplex(64);
        let mut transport =
            StreamTransport::new(tokio::io::empty(), BufWriter::with_capacity(32, writer));
        transport.send_line(Line::new("{}")?).await?;
        let mut written = [0_u8; 3];
        timeout(PATIENCE, peer.read_exact(&mut written)).await??;
        assert_eq!(&written, b"{}\n");
        Ok(())
    }

    #[tokio::test]
    async fn send_flushes_a_buffering_writer() -> TestResult {
        let (writer, mut peer) = duplex(64);
        let transport =
            StreamTransport::new(tokio::io::empty(), BufWriter::with_capacity(32, writer));
        let (mut sink, _source) = transport.into_framed();
        sink.send(Frame::credit(Channel::new(1), 5)).await?;
        let mut written = [0_u8; 13];
        timeout(PATIENCE, peer.read_exact(&mut written)).await??;
        assert_eq!(written, [1, 0, 0, 0, 1, 4, 0, 0, 0, 5, 0, 0, 0]);
        Ok(())
    }
}
