//! A [`Transport`] that runs each invocation as a `herdr-app` child process.
//!
//! The port layer names the capability of running one invocation, and this file supplies it for the
//! real program: it starts `herdr-app` with the invocation's arguments, waits for it to end, and
//! hands back its exit status and both output streams. It depends on [`port`](crate::port) and
//! [`protocol`](crate::protocol), and holds the crate's only use of tokio's process API.
//!
//! Responsibilities:
//!
//! - [`ProcessTransport`], which starts one child process per run
//! - [`ProcessTransportBuilder`], which chooses the program, `HERDR_APP_HOME` and environment changes
//!
//! Non-responsibilities:
//!
//! - telling a refused command from a successful one, which the protocol layer does
//! - limiting how long a run takes, which the caller does by dropping the run's future
//!
//! The unit tests start no real `herdr-app`. They run a program name that is not on `PATH`, and, on
//! Unix only, small `sh` scripts they write.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::future::Future;
use std::path::PathBuf;
use std::process::Stdio;

use tokio::process::Command;

use crate::port::{Transport, TransportError};
use crate::protocol::{Exit, Invocation, RunOutput};

/// The program a default [`ProcessTransport`] starts, looked up on `PATH`.
const DEFAULT_PROGRAM: &str = "herdr-app";

/// The variable through which `herdr-app` is told where its home is.
const HOME_VARIABLE: &str = "HERDR_APP_HOME";

/// One change to the environment of the child, applied in the order it was added.
#[derive(Clone)]
enum EnvChange {
    Set { key: OsString, value: OsString },
    Remove { key: OsString },
}

/// Shows which variable a change names and never the value it sets, since the value may be a secret.
impl fmt::Debug for EnvChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Set { key, .. } => f.debug_tuple("Set").field(key).finish(),
            Self::Remove { key } => f.debug_tuple("Remove").field(key).finish(),
        }
    }
}

/// A [`Transport`] that starts the `herdr-app` program once per run.
///
/// The child inherits the environment of the calling process, `HC_*` variables and
/// `HERDR_APP_HOME` included, unless the builder removes or replaces them. The child reads no
/// standard input. Dropping the future of a run kills the child. There is no timeout: a run lasts as
/// long as the program does, so wrap it in one when that matters.
///
/// A run fails with a [`TransportError`] when the program cannot be started, and also when the
/// operating system fails while the started program runs, such as when reading its output. A
/// program that starts and exits with an error is not a failure of the run.
///
/// The `Debug` output names the program, the home and each environment change by its variable name.
/// It never shows the value of a variable set through the builder.
///
/// # Examples
///
/// ```
/// use herdrify_app::adapter::ProcessTransport;
///
/// let transport = ProcessTransport::builder().env("API_TOKEN", "secret").build();
/// let shown = format!("{transport:?}");
/// assert!(shown.contains("API_TOKEN"));
/// assert!(!shown.contains("secret"));
/// ```
#[derive(Debug, Clone)]
pub struct ProcessTransport {
    program: OsString,
    home: Option<PathBuf>,
    env: Vec<EnvChange>,
}

impl ProcessTransport {
    /// The transport that starts `herdr-app`, found on `PATH`, with the inherited environment.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::adapter::ProcessTransport;
    ///
    /// let _transport = ProcessTransport::herdr_app();
    /// ```
    #[must_use]
    pub fn herdr_app() -> Self {
        Self {
            program: OsString::from(DEFAULT_PROGRAM),
            home: None,
            env: Vec::new(),
        }
    }

    /// Starts a builder that begins as [`herdr_app`](Self::herdr_app).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use herdrify_app::adapter::ProcessTransport;
    /// use herdrify_app::port::{Transport, TransportError};
    /// use herdrify_app::protocol::Invocation;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), TransportError> {
    /// let transport = ProcessTransport::builder()
    ///     .program("herdr-app")
    ///     .home("/tmp/herdr-app-home")
    ///     .env("NO_COLOR", "1")
    ///     .env_remove("HC_SESSION")
    ///     .build();
    /// let output = transport.run(&Invocation::version()).await?;
    /// println!("{}", String::from_utf8_lossy(&output.stdout));
    /// # Ok(())
    /// # }
    /// ```
    pub fn builder() -> ProcessTransportBuilder {
        ProcessTransportBuilder {
            transport: Self::herdr_app(),
        }
    }
}

impl Transport for ProcessTransport {
    fn run(
        &self,
        invocation: &Invocation,
    ) -> impl Future<Output = Result<RunOutput, TransportError>> + Send {
        let mut command = Command::new(&self.program);
        // `spawn` leaves all three streams inherited unless they are set here.
        command
            .args(invocation.argv())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(home) = &self.home {
            command.env(HOME_VARIABLE, home);
        }
        for change in &self.env {
            match change {
                EnvChange::Set { key, value } => command.env(key, value),
                EnvChange::Remove { key } => command.env_remove(key),
            };
        }
        command.kill_on_drop(true);
        let program = self.program.clone();
        async move {
            let child = command
                .spawn()
                .map_err(|source| TransportError::from_spawn(&program, source))?;
            let output = child
                .wait_with_output()
                .await
                .map_err(|source| TransportError::from_wait(&program, source))?;
            Ok(RunOutput {
                exit: output.status.code().map_or(Exit::Signal, Exit::Code),
                stdout: output.stdout,
                stderr: output.stderr,
            })
        }
    }
}

/// Builds a [`ProcessTransport`].
///
/// Environment changes apply in the order they were added, after `HERDR_APP_HOME` is set from
/// [`home`](Self::home), so a later change wins over an earlier one and over the home.
///
/// # Examples
///
/// ```
/// use herdrify_app::adapter::ProcessTransport;
///
/// let _transport = ProcessTransport::builder()
///     .program("/opt/herdr/herdr-app")
///     .home("/tmp/herdr-app-home")
///     .env("NO_COLOR", "1")
///     .build();
/// ```
#[derive(Debug, Clone)]
#[must_use]
pub struct ProcessTransportBuilder {
    transport: ProcessTransport,
}

impl ProcessTransportBuilder {
    /// Starts `program` instead of `herdr-app`. A bare name is looked up on `PATH`.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::adapter::ProcessTransport;
    ///
    /// let _transport = ProcessTransport::builder().program("/opt/herdr/herdr-app").build();
    /// ```
    pub fn program(mut self, program: impl AsRef<OsStr>) -> Self {
        program.as_ref().clone_into(&mut self.transport.program);
        self
    }

    /// Sets `HERDR_APP_HOME` for the child to `home`.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::adapter::ProcessTransport;
    ///
    /// let _transport = ProcessTransport::builder().home("/tmp/herdr-app-home").build();
    /// ```
    pub fn home(mut self, home: impl Into<PathBuf>) -> Self {
        self.transport.home = Some(home.into());
        self
    }

    /// Sets the variable `key` to `value` for the child.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::adapter::ProcessTransport;
    ///
    /// let _transport = ProcessTransport::builder().env("NO_COLOR", "1").build();
    /// ```
    pub fn env(mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        self.transport.env.push(EnvChange::Set {
            key: key.as_ref().to_owned(),
            value: value.as_ref().to_owned(),
        });
        self
    }

    /// Removes the variable `key` from the environment the child inherits.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::adapter::ProcessTransport;
    ///
    /// let _transport = ProcessTransport::builder().env_remove("HC_SESSION").build();
    /// ```
    pub fn env_remove(mut self, key: impl AsRef<OsStr>) -> Self {
        self.transport.env.push(EnvChange::Remove {
            key: key.as_ref().to_owned(),
        });
        self
    }

    /// Finishes the builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::adapter::ProcessTransport;
    ///
    /// let _transport = ProcessTransport::builder().build();
    /// ```
    #[must_use]
    pub fn build(self) -> ProcessTransport {
        self.transport
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs a program name that is not on `PATH` and checks the refusal.
    ///
    /// On Unix this runs inside `process_transport_on_unix`, after its scripts are written: a child
    /// forked while a script is open for writing would hold that descriptor and make the later
    /// exec of the script fail with ETXTBSY.
    async fn assert_missing_program_is_not_found() -> Result<(), &'static str> {
        let transport = ProcessTransport::builder()
            .program("herdrify-app-no-such-program")
            .build();
        let Err(refused) = transport.run(&Invocation::version()).await else {
            return Err("a missing program must not run");
        };
        assert!(matches!(
            &refused,
            TransportError::NotFound { program } if program == "herdrify-app-no-such-program"
        ));
        assert_eq!(
            refused.to_string(),
            "cannot start `herdrify-app-no-such-program`: it was not found"
        );
        Ok(())
    }

    #[cfg(not(unix))]
    #[tokio::test]
    async fn missing_program_is_not_found() -> Result<(), &'static str> {
        assert_missing_program_is_not_found().await
    }

    #[test]
    fn debug_shows_env_keys_but_never_values() {
        let transport = ProcessTransport::builder()
            .program("herdr-app")
            .home("/tmp/herdr-app-home")
            .env("API_TOKEN", "s3cret-value")
            .env_remove("HC_SESSION")
            .build();
        let shown = format!("{transport:?}");
        assert!(shown.contains("herdr-app"), "{shown}");
        assert!(shown.contains("/tmp/herdr-app-home"), "{shown}");
        assert!(shown.contains("API_TOKEN"), "{shown}");
        assert!(shown.contains("HC_SESSION"), "{shown}");
        assert!(!shown.contains("s3cret-value"), "{shown}");
    }

    #[cfg(unix)]
    mod unix {
        use std::error::Error;
        use std::fs;
        use std::io::Write as _;
        use std::os::unix::fs::OpenOptionsExt as _;
        use std::path::{Path, PathBuf};
        use std::time::Duration;

        use super::*;

        type TestResult = Result<(), Box<dyn Error>>;

        /// A scratch directory that is removed, with anything a failed run left in it, on drop.
        struct ScratchDir(PathBuf);

        impl ScratchDir {
            fn create() -> Result<Self, std::io::Error> {
                let dir = std::env::temp_dir()
                    .join(format!("herdrify-app-adapter-{}", std::process::id()));
                match fs::remove_dir_all(&dir) {
                    Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                        return Err(error);
                    }
                    _ => {}
                }
                fs::create_dir_all(&dir)?;
                Ok(Self(dir))
            }
        }

        impl Drop for ScratchDir {
            fn drop(&mut self) {
                // Cleanup must not replace the outcome of the test that is ending.
                drop(fs::remove_dir_all(&self.0));
            }
        }

        fn write_script(dir: &Path, name: &str, body: &str) -> Result<PathBuf, std::io::Error> {
            let path = dir.join(name);
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o755)
                .open(&path)?;
            file.write_all(format!("#!/bin/sh\n{body}\n").as_bytes())?;
            file.sync_all()?;
            Ok(path)
        }

        struct Scripts {
            argv: PathBuf,
            env: PathBuf,
            signal: PathBuf,
            slow: PathBuf,
            marker: PathBuf,
        }

        async fn exercise(scripts: &Scripts) -> TestResult {
            let echo = ProcessTransport::builder().program(&scripts.argv).build();
            let output = echo.run(&Invocation::ls()).await?;
            assert_eq!(output.exit, Exit::Code(0));
            assert_eq!(output.stdout, b"ls");
            assert_eq!(output.stderr, b"err");

            let env = ProcessTransport::builder()
                .program(&scripts.env)
                .home("/tmp/h")
                .env("A", "1")
                .env_remove("A")
                .build();
            let output = env.run(&Invocation::ls()).await?;
            assert_eq!(output.stdout, b"/tmp/h|");

            let later_wins = ProcessTransport::builder()
                .program(&scripts.env)
                .home("/tmp/h")
                .env("HERDR_APP_HOME", "/tmp/later")
                .env("A", "1")
                .build();
            let output = later_wins.run(&Invocation::ls()).await?;
            assert_eq!(output.stdout, b"/tmp/later|1");

            let signalled = ProcessTransport::builder().program(&scripts.signal).build();
            let output = signalled.run(&Invocation::ls()).await?;
            assert_eq!(output.exit, Exit::Signal);

            assert_missing_program_is_not_found().await?;
            Ok(())
        }

        /// Drops a run of a script that would write a marker after a second, and checks that the
        /// child died with the dropped future instead of finishing the script.
        async fn assert_drop_kills_the_child(scripts: &Scripts) {
            let slow = ProcessTransport::builder().program(&scripts.slow).build();
            let invocation = Invocation::ls();
            let run = slow.run(&invocation);
            let timed_out = tokio::time::timeout(Duration::from_millis(200), run).await;
            assert!(
                timed_out.is_err(),
                "the slow script must outlast the timeout"
            );
            tokio::time::sleep(Duration::from_millis(1500)).await;
            assert!(
                !scripts.marker.exists(),
                "the child kept running after its future was dropped"
            );
        }

        #[tokio::test]
        async fn process_transport_on_unix() -> TestResult {
            let scratch = ScratchDir::create()?;
            let dir = &scratch.0;
            let marker = dir.join("marker");
            let slow_body = format!("sleep 1\ntouch '{}'", marker.display());
            let scripts = Scripts {
                argv: write_script(dir, "argv.sh", "printf '%s' \"$1\"\nprintf err >&2")?,
                env: write_script(dir, "env.sh", "printf '%s|%s' \"$HERDR_APP_HOME\" \"$A\"")?,
                signal: write_script(dir, "signal.sh", "kill -TERM $$")?,
                slow: write_script(dir, "slow.sh", &slow_body)?,
                marker,
            };
            exercise(&scripts).await?;
            assert_drop_kills_the_child(&scripts).await;
            Ok(())
        }
    }
}
