//! `HerdrAppBuilder`: the `herdr-app` program to reach, from plain values.
//!
//! The builder holds what a [`ProcessTransportBuilder`] needs: the program, the home and the
//! environment edits. [`connect`](HerdrAppBuilder::connect) builds the transport and checks the
//! version of the program it starts.
//!
//! Responsibilities:
//!
//! - [`HerdrAppBuilder`]
//!
//! Non-responsibilities:
//!
//! - starting the program, which [`ProcessTransport`](crate::adapter::ProcessTransport) does
//! - checking the version, which [`HerdrApp::from_client`] does

use std::ffi::OsStr;
use std::path::PathBuf;

use super::HerdrApp;
use crate::adapter::{ProcessTransport, ProcessTransportBuilder};
use crate::driver::{Client, Error};

/// Assembles the `herdr-app` program to run, then connects to it.
///
/// Created by [`HerdrApp::builder`]. Without a setting it runs `herdr-app` found on `PATH`, with
/// the environment of the calling process.
///
/// # Examples
///
/// ```no_run
/// use herdrify_app::HerdrApp;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), herdrify_app::Error> {
/// let app = HerdrApp::builder()
///     .home("/tmp/herdr-app-home")
///     .env("NO_COLOR", "1")
///     .connect()
///     .await?;
/// # let _ = app;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
#[must_use = "a builder connects nothing until `connect` is awaited"]
pub struct HerdrAppBuilder {
    transport: ProcessTransportBuilder,
}

impl HerdrAppBuilder {
    /// The builder for `herdr-app` on `PATH`.
    pub(super) fn new() -> Self {
        Self {
            transport: ProcessTransport::builder(),
        }
    }

    /// Starts `program` instead of `herdr-app`. A bare name is looked up on `PATH`.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::HerdrApp;
    ///
    /// let _builder = HerdrApp::builder().program("/opt/herdr/herdr-app");
    /// ```
    pub fn program(mut self, program: impl AsRef<OsStr>) -> Self {
        self.transport = self.transport.program(program);
        self
    }

    /// Sets `HERDR_APP_HOME` for the program to `home`.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::HerdrApp;
    ///
    /// let _builder = HerdrApp::builder().home("/tmp/herdr-app-home");
    /// ```
    pub fn home(mut self, home: impl Into<PathBuf>) -> Self {
        self.transport = self.transport.home(home);
        self
    }

    /// Sets the variable `key` to `value` for the program.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::HerdrApp;
    ///
    /// let _builder = HerdrApp::builder().env("NO_COLOR", "1");
    /// ```
    pub fn env(mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        self.transport = self.transport.env(key, value);
        self
    }

    /// Removes the variable `key` from the environment the program inherits, such as `HC_SESSION`.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::HerdrApp;
    ///
    /// let _builder = HerdrApp::builder().env_remove("HC_SESSION");
    /// ```
    pub fn env_remove(mut self, key: impl AsRef<OsStr>) -> Self {
        self.transport = self.transport.env_remove(key);
        self
    }

    /// Builds the transport for the program these values describe and checks its version, once.
    ///
    /// # Errors
    ///
    /// - [`Error::Transport`] when the program cannot be started or run.
    /// - [`Error::UnsupportedVersion`] when it is older than
    ///   [`AppVersion::MINIMUM`](crate::protocol::AppVersion::MINIMUM).
    /// - The errors of [`Client::version`](crate::driver::Client::version) otherwise.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use herdrify_app::HerdrApp;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), herdrify_app::Error> {
    /// let app = HerdrApp::builder().connect().await?;
    /// println!("herdr-app {}", app.version());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn connect(self) -> Result<HerdrApp, Error> {
        HerdrApp::from_client(Client::new(self.transport.build())).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::TransportError;

    #[tokio::test]
    async fn a_program_that_cannot_start_is_a_transport_error() -> Result<(), &'static str> {
        let Err(error) = HerdrApp::builder()
            .program("herdrify-app-no-such-program")
            .connect()
            .await
        else {
            return Err("a missing program should be refused");
        };
        assert!(matches!(
            error,
            Error::Transport(TransportError::NotFound { program })
                if program == "herdrify-app-no-such-program"
        ));
        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn the_settings_reach_the_program_and_its_version_is_checked() -> Result<(), String> {
        use std::io::Write as _;
        use std::os::unix::fs::OpenOptionsExt as _;

        // Prints a supported version only when it was started with the home and variable the
        // builder was given and without the variable it removed.
        let body = "#!/bin/sh\n\
            if [ \"$HERDR_APP_HOME\" = /tmp/h ] && [ \"$MARK\" = 1 ] && [ -z \"$GONE\" ]; then\n\
              printf 'herdr-app 0.1.17\\n'\n\
            else\n\
              printf 'herdr-app 0.0.1\\n'\n\
            fi\n";
        let dir = std::env::temp_dir().join(format!("herdrify-app-builder-{}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
        let program = dir.join("herdr-app.sh");
        let written = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o755)
            .open(&program)
            .and_then(|mut file| {
                file.write_all(body.as_bytes())
                    .and_then(|()| file.sync_all())
            });
        let connected = match written {
            Ok(()) => {
                let builder = HerdrApp::builder()
                    .program(&program)
                    .home("/tmp/h")
                    .env("MARK", "1")
                    .env("GONE", "1")
                    .env_remove("GONE");
                // Another test's fork can hold the script's write descriptor for a moment, which
                // makes the first exec fail with ETXTBSY; start it again until it is released.
                let mut attempt = 0;
                loop {
                    let result = builder.clone().connect().await;
                    attempt += 1;
                    let busy = matches!(
                        &result,
                        Err(Error::Transport(TransportError::Io { source, .. }))
                            if source.kind() == std::io::ErrorKind::ExecutableFileBusy
                    );
                    if !busy || attempt == 20 {
                        break result;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
            }
            Err(error) => Err(Error::Cli {
                message: error.to_string(),
            }),
        };
        std::fs::remove_dir_all(&dir).map_err(|error| error.to_string())?;
        let app = connected.map_err(|error| error.to_string())?;
        assert_eq!(app.version().to_string(), "0.1.17");
        Ok(())
    }
}
