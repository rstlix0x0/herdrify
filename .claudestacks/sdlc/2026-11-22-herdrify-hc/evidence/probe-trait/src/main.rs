//! Does the chosen Transport shape compile on 1.94, and can a generic Client spawn its reader?
use std::future::Future;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug)]
struct Frame(Vec<u8>);
#[derive(Debug)]
struct Line(String);

trait FrameSink: Send + 'static {
    fn send(&mut self, f: Frame) -> impl Future<Output = std::io::Result<()>> + Send;
}
trait FrameSource: Send + 'static {
    fn recv(&mut self) -> impl Future<Output = std::io::Result<Option<Frame>>> + Send;
}
trait Transport: Send + 'static {
    type Sink: FrameSink;
    type Source: FrameSource;
    fn send_line(&mut self, l: Line) -> impl Future<Output = std::io::Result<()>> + Send;
    fn recv_line(&mut self) -> impl Future<Output = std::io::Result<Option<Line>>> + Send;
    fn into_framed(self) -> (Self::Sink, Self::Source);
}

struct StreamTransport<R, W> { r: R, w: W }
struct Sink<W>(W);
struct Source<R>(R);
impl<W: AsyncWrite + Unpin + Send + 'static> FrameSink for Sink<W> {
    async fn send(&mut self, f: Frame) -> std::io::Result<()> { self.0.write_all(&f.0).await }
}
impl<R: AsyncRead + Unpin + Send + 'static> FrameSource for Source<R> {
    async fn recv(&mut self) -> std::io::Result<Option<Frame>> {
        let mut b = [0u8; 4];
        match self.0.read_exact(&mut b).await { Ok(_) => Ok(Some(Frame(b.to_vec()))), Err(_) => Ok(None) }
    }
}
impl<R, W> Transport for StreamTransport<R, W>
where R: AsyncRead + Unpin + Send + 'static, W: AsyncWrite + Unpin + Send + 'static {
    type Sink = Sink<W>;
    type Source = Source<R>;
    async fn send_line(&mut self, l: Line) -> std::io::Result<()> {
        self.w.write_all(l.0.as_bytes()).await?; self.w.write_all(b"\n").await
    }
    async fn recv_line(&mut self) -> std::io::Result<Option<Line>> {
        let mut out = Vec::new(); let mut c = [0u8; 1];
        loop { if self.r.read(&mut c).await? == 0 { return Ok(None); } if c[0] == b'\n' { break; } out.push(c[0]); }
        Ok(Some(Line(String::from_utf8_lossy(&out).into_owned())))
    }
    fn into_framed(self) -> (Sink<W>, Source<R>) { (Sink(self.w), Source(self.r)) }
}

/// Generic over any Transport: spawning the reader requires the RPITIT futures to be Send.
async fn connect<T: Transport>(mut t: T) -> std::io::Result<Vec<Frame>> {
    t.send_line(Line("hello".into())).await?;
    let reply = t.recv_line().await?;
    println!("hello reply line: {reply:?}");
    let (mut sink, mut source) = t.into_framed();
    let reader = tokio::spawn(async move {
        let mut got = Vec::new();
        while let Ok(Some(f)) = source.recv().await { got.push(f); }
        got
    });
    sink.send(Frame(b"ping".to_vec())).await?;
    drop(sink);
    Ok(reader.await.expect("reader task"))
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let (client, mut core) = tokio::io::duplex(64);
    let (cr, cw) = tokio::io::split(client);
    let core_task = tokio::spawn(async move {
        let mut buf = [0u8; 6];
        core.read_exact(&mut buf).await.expect("hello line");
        core.write_all(b"ok\nABCDEFGH").await.expect("reply + frames");
        let mut f = [0u8; 4];
        core.read_exact(&mut f).await.expect("ping frame");
        println!("core got frame bytes: {:?}", std::str::from_utf8(&f));
    });
    let frames = connect(StreamTransport { r: cr, w: cw }).await?;
    core_task.await.expect("core task");
    println!("client got frames: {frames:?}");
    Ok(())
}
