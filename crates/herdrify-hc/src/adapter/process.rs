//! `ProcessTransport`: the transport traits over a spawned command such as `hc connect`.
//!
//! herdr-core is reached by running a command that speaks the protocol on its standard input and
//! output. This file starts that command and carries the protocol over its pipes with a
//! [`StreamTransport`]. The child handle travels with the write half, so closing the sink ends the
//! child's input and `hc connect` exits.
//!
//! Responsibilities:
//!
//! - [`ProcessTransportBuilder`], which assembles the command, its environment and its session
//! - [`ProcessTransport`], the transport over the started child
//! - [`ProcessSink`] and [`ProcessSource`], its frame halves, which keep the process types private
//!
//! Non-responsibilities:
//!
//! - moving bytes, which [`StreamTransport`] does
//! - waiting for the child to exit or reporting its exit status

use std::ffi::{OsStr, OsString};
use std::io;
use std::process::Stdio;

use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use super::stream::{StreamSink, StreamSource, StreamTransport};
use crate::port::{FrameSink, FrameSource, Transport, TransportError};
use crate::protocol::{Argv, Frame, Line, SessionName};

/// A transport over a spawned command's standard input and output.
///
/// Start one with [`hc_connect`](ProcessTransport::hc_connect) or
/// [`from_argv`](ProcessTransport::from_argv), set the environment on the returned builder, then
/// call [`spawn`](ProcessTransportBuilder::spawn). The command's standard error is inherited.
///
/// The command runs with the caller's environment unchanged. Inside a Herdr terminal that includes
/// `HC_SCOPE`, `HC_TERMINAL_ID` and `HC_TERMINAL_SOCKET`; call
/// [`env_remove`](ProcessTransportBuilder::env_remove) to opt out of any of them.
///
/// Closing the sink returned by [`into_framed`](Transport::into_framed) closes the child's input,
/// and `hc connect` then exits. Dropping the sink kills the child it started, though not any
/// process that child started in turn.
///
/// # Examples
///
/// ```
/// use herdrify_hc::ProcessTransport;
/// use herdrify_hc::protocol::SessionName;
///
/// # fn main() -> Result<(), herdrify_hc::protocol::EmptyValueError> {
/// let builder = ProcessTransport::hc_connect()
///     .session(SessionName::new("work")?)
///     .env_remove("HC_SCOPE");
/// // builder.spawn() starts `hc connect --session work`.
/// # let _ = builder;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct ProcessTransport {
    inner: StreamTransport<ChildStdout, ChildStdin>,
    child: Child,
}

impl ProcessTransport {
    /// The builder for `["hc", "connect"]`, with `hc` resolved on `PATH`.
    #[must_use]
    pub fn hc_connect() -> ProcessTransportBuilder {
        Self::builder(Argv::from_parts(
            "hc".to_owned(),
            vec!["connect".to_owned()],
        ))
    }

    /// The builder for any command, such as `["ssh", "box", "hc", "connect"]`.
    #[must_use]
    pub const fn from_argv(argv: Argv) -> ProcessTransportBuilder {
        Self::builder(argv)
    }

    /// The builder for `argv`: its first element is the program and the rest are its arguments.
    #[must_use]
    pub const fn builder(argv: Argv) -> ProcessTransportBuilder {
        ProcessTransportBuilder {
            argv,
            session: None,
            env: Vec::new(),
        }
    }
}

impl Transport for ProcessTransport {
    type Sink = ProcessSink;
    type Source = ProcessSource;

    async fn send_line(&mut self, line: Line) -> Result<(), TransportError> {
        self.inner.send_line(line).await
    }

    async fn recv_line(&mut self) -> Result<Option<Line>, TransportError> {
        self.inner.recv_line().await
    }

    fn into_framed(self) -> (Self::Sink, Self::Source) {
        let (sink, source) = self.inner.into_framed();
        (
            ProcessSink {
                stdin: Some(sink),
                _child: self.child,
            },
            ProcessSource { inner: source },
        )
    }
}

/// Assembles the command a [`ProcessTransport`] runs.
///
/// Created by [`ProcessTransport::hc_connect`], [`ProcessTransport::from_argv`] or
/// [`ProcessTransport::builder`].
#[derive(Debug)]
pub struct ProcessTransportBuilder {
    argv: Argv,
    session: Option<SessionName>,
    env: Vec<(OsString, Option<OsString>)>,
}

impl ProcessTransportBuilder {
    /// Sets an environment variable for the child.
    #[must_use]
    pub fn env(mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        self.env
            .push((key.as_ref().to_owned(), Some(value.as_ref().to_owned())));
        self
    }

    /// Removes an environment variable the child would otherwise inherit.
    #[must_use]
    pub fn env_remove(mut self, key: impl AsRef<OsStr>) -> Self {
        self.env.push((key.as_ref().to_owned(), None));
        self
    }

    /// Appends `--session <name>` to the arguments.
    ///
    /// `hc connect --session <name>` reaches the same core as the `HC_SESSION` variable.
    #[must_use]
    pub fn session(mut self, session: SessionName) -> Self {
        self.session = Some(session);
        self
    }

    /// The command with its arguments, session flag and environment edits, before any I/O setup.
    fn command(&self) -> Command {
        let mut command = Command::new(self.argv.program());
        command.args(self.argv.args());
        if let Some(session) = &self.session {
            command.arg("--session").arg(session.as_str());
        }
        for (key, value) in &self.env {
            match value {
                Some(value) => command.env(key, value),
                None => command.env_remove(key),
            };
        }
        command
    }

    /// Starts the command with piped input and output and an inherited standard error.
    ///
    /// # Errors
    ///
    /// Returns [`TransportError::Spawn`] naming the program when the operating system cannot start
    /// it, with the operating system's error as the source.
    ///
    /// # Panics
    ///
    /// Panics when called outside a Tokio runtime.
    pub fn spawn(self) -> Result<ProcessTransport, TransportError> {
        let mut child = self
            .command()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .map_err(|source| TransportError::Spawn {
                program: self.argv.program().to_owned(),
                source,
            })?;
        // Both pipes were requested above, so the handles are always present.
        let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            return Err(TransportError::Io(io::Error::other(
                "the child's pipes are missing",
            )));
        };
        Ok(ProcessTransport {
            inner: StreamTransport::new(stdout, stdin),
            child,
        })
    }
}

/// The write half of a [`ProcessTransport`] after it has been split into frames.
///
/// It owns the child process. [`close`](FrameSink::close) closes the child's input; dropping the
/// sink kills the child if it is still running.
///
/// # Cancel safety
///
/// `send` is not cancel safe: dropping it while it runs can leave part of a frame on the wire.
#[derive(Debug)]
pub struct ProcessSink {
    stdin: Option<StreamSink<ChildStdin>>,
    _child: Child,
}

impl FrameSink for ProcessSink {
    async fn send(&mut self, frame: Frame) -> Result<(), TransportError> {
        match self.stdin.as_mut() {
            Some(stdin) => stdin.send(frame).await,
            None => Err(TransportError::Io(io::Error::from(
                io::ErrorKind::BrokenPipe,
            ))),
        }
    }

    /// Shuts down the child's input and then drops it, so the child reads the end of its input.
    ///
    /// Every send has already flushed. Shutting a child's input down does not signal the end of
    /// the stream; only dropping it does.
    /// Closing twice is a no-op.
    async fn close(&mut self) -> Result<(), TransportError> {
        match self.stdin.take() {
            Some(mut stdin) => stdin.close().await,
            None => Ok(()),
        }
    }
}

/// The read half of a [`ProcessTransport`] after it has been split into frames.
///
/// `recv` is cancel safe: a dropped call loses no byte read from the child.
#[derive(Debug)]
pub struct ProcessSource {
    inner: StreamSource<ChildStdout>,
}

impl FrameSource for ProcessSource {
    async fn recv(&mut self) -> Result<Option<Frame>, TransportError> {
        self.inner.recv().await
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::ffi::OsStr;
    use std::io;
    use std::time::Duration;

    use tokio::time::timeout;

    use super::ProcessTransport;
    use crate::port::{FrameSink, FrameSource, Transport, TransportError};
    use crate::protocol::{Argv, Channel, Frame, Line, SessionName};

    type TestResult = Result<(), Box<dyn Error>>;

    /// How long a test waits for a child that should already have answered.
    const PATIENCE: Duration = Duration::from_secs(5);

    fn argv(words: &[&str]) -> Result<Argv, Box<dyn Error>> {
        Ok(Argv::new(
            words.iter().map(|word| (*word).to_owned()).collect(),
        )?)
    }

    #[test]
    fn builder_assembles_argv() -> TestResult {
        let builder = ProcessTransport::from_argv(argv(&["ssh", "box", "hc", "connect"])?);
        let command = builder.command();
        let command = command.as_std();
        assert_eq!(command.get_program(), "ssh");
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            ["box", "hc", "connect"]
        );
        Ok(())
    }

    #[test]
    fn session_appends_the_flag() -> TestResult {
        let builder = ProcessTransport::hc_connect().session(SessionName::new("s1")?);
        let command = builder.command();
        let command = command.as_std();
        assert_eq!(command.get_program(), "hc");
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            ["connect", "--session", "s1"]
        );
        Ok(())
    }

    #[test]
    fn env_and_env_remove_are_recorded() -> TestResult {
        let builder = ProcessTransport::hc_connect()
            .env("A", "1")
            .env_remove("HC_SCOPE");
        let command = builder.command();
        let envs: Vec<_> = command.as_std().get_envs().collect();
        let set = envs
            .iter()
            .find(|(key, _)| *key == "A")
            .ok_or("A was not recorded")?;
        assert_eq!(set.1, Some(OsStr::new("1")));
        let removed = envs
            .iter()
            .find(|(key, _)| *key == "HC_SCOPE")
            .ok_or("HC_SCOPE was not recorded")?;
        assert_eq!(removed.1, None);
        Ok(())
    }

    #[tokio::test]
    async fn missing_program_is_spawn_error() -> TestResult {
        let refused = ProcessTransport::from_argv(argv(&["hfy-no-such-program-zz"])?).spawn();
        let Err(TransportError::Spawn { program, source }) = refused else {
            return Err("a missing program must be a spawn error".into());
        };
        assert_eq!(program, "hfy-no-such-program-zz");
        let shown = TransportError::Spawn { program, source }.to_string();
        assert!(shown.starts_with("cannot start hfy-no-such-program-zz: "));
        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn close_gives_the_child_eof() -> TestResult {
        let transport = ProcessTransport::from_argv(argv(&["cat"])?).spawn()?;
        let (mut sink, mut source) = transport.into_framed();
        sink.close().await?;
        let end = timeout(PATIENCE, source.recv()).await??;
        assert_eq!(end, None);
        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn lines_and_frames_round_trip_through_cat() -> TestResult {
        let mut transport = ProcessTransport::from_argv(argv(&["cat"])?).spawn()?;
        let line = Line::new("{}")?;
        timeout(PATIENCE, transport.send_line(line.clone())).await??;
        let echoed = timeout(PATIENCE, transport.recv_line()).await??;
        assert_eq!(echoed, Some(line));

        let (mut sink, mut source) = transport.into_framed();
        let frame = Frame::credit(Channel::new(1), 5);
        timeout(PATIENCE, sink.send(frame.clone())).await??;
        let echoed = timeout(PATIENCE, source.recv()).await??;
        assert_eq!(echoed, Some(frame));
        timeout(PATIENCE, sink.close()).await??;
        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn dropping_the_sink_kills_a_child_that_ignores_its_input() -> TestResult {
        let transport = ProcessTransport::from_argv(argv(&["sleep", "30"])?).spawn()?;
        let (sink, mut source) = transport.into_framed();
        drop(sink);
        let end = timeout(PATIENCE, source.recv()).await??;
        assert_eq!(end, None);
        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn send_after_close_is_a_broken_pipe_and_close_twice_is_a_no_op() -> TestResult {
        let transport = ProcessTransport::from_argv(argv(&["cat"])?).spawn()?;
        let (mut sink, _source) = transport.into_framed();
        sink.close().await?;
        sink.close().await?;
        let refused = sink.send(Frame::credit(Channel::new(1), 5)).await;
        let Err(TransportError::Io(source)) = refused else {
            return Err("a send after close must be an I/O error".into());
        };
        assert_eq!(source.kind(), io::ErrorKind::BrokenPipe);
        Ok(())
    }
}
