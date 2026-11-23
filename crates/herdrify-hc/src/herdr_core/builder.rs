//! `HerdrCoreBuilder`: the command that reaches herdr-core, from plain values.
//!
//! The builder holds what a [`ProcessTransportBuilder`] needs, as strings: the command, the
//! session and the environment edits. [`connect`](HerdrCoreBuilder::connect) checks them, starts
//! the command and sends the handshake.
//!
//! Responsibilities:
//!
//! - [`HerdrCoreBuilder`] and the command line it assembles
//!
//! Non-responsibilities:
//!
//! - starting the command and carrying the protocol, which [`ProcessTransport`] does
//! - the handshake, which [`Client::connect`] does

use std::ffi::{OsStr, OsString};

use super::call::argv;
use super::{HerdrCore, HerdrCoreError};
use crate::adapter::{ProcessTransport, ProcessTransportBuilder};
use crate::connection::Client;
use crate::protocol::SessionName;

/// The words of `hc connect`, the command used when no other is named.
const HC_CONNECT: [&str; 2] = ["hc", "connect"];

/// Assembles the command that reaches herdr-core, then connects through it.
///
/// Created by [`HerdrCore::builder`]. Without [`ssh`](Self::ssh) or [`command`](Self::command) it
/// runs `hc connect` with `hc` resolved on `PATH`. The values are checked by
/// [`connect`](Self::connect).
///
/// # Examples
///
/// ```no_run
/// use herdrify_hc::HerdrCore;
///
/// # async fn run() -> Result<(), herdrify_hc::HerdrCoreError> {
/// // `ssh box hc connect --session work`
/// let core = HerdrCore::builder().ssh("box").session("work").connect().await?;
/// # let _ = core;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "a builder connects nothing until `connect` is awaited"]
pub struct HerdrCoreBuilder {
    command: Vec<String>,
    session: Option<String>,
    env: Vec<(OsString, Option<OsString>)>,
}

impl HerdrCoreBuilder {
    /// The builder for `hc connect`.
    pub(super) fn new() -> Self {
        Self {
            command: HC_CONNECT.map(str::to_owned).to_vec(),
            session: None,
            env: Vec::new(),
        }
    }

    /// Reaches the core of the session called `name`: `hc connect --session <name>`.
    pub fn session(mut self, name: impl Into<String>) -> Self {
        self.session = Some(name.into());
        self
    }

    /// Reaches the core on another machine: `ssh <host> hc connect`.
    ///
    /// `host` is anything ssh accepts, such as `box` or `user@box`. The environment edits apply
    /// to the local `ssh` process, not to `hc` on the remote machine.
    pub fn ssh(mut self, host: impl Into<String>) -> Self {
        let mut command = vec!["ssh".to_owned(), host.into()];
        command.extend(HC_CONNECT.map(str::to_owned));
        self.command = command;
        self
    }

    /// Runs `words` instead of `hc connect`: any command that speaks the protocol on its standard
    /// input and output. [`session`](Self::session) appends `--session <name>` to it.
    ///
    /// No words is reported by [`connect`](Self::connect) as [`HerdrCoreError::Empty`].
    pub fn command<S: Into<String>>(mut self, words: impl IntoIterator<Item = S>) -> Self {
        self.command = words.into_iter().map(Into::into).collect();
        self
    }

    /// Sets an environment variable for the command.
    pub fn env(mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        self.env
            .push((key.as_ref().to_owned(), Some(value.as_ref().to_owned())));
        self
    }

    /// Removes an environment variable the command would otherwise inherit, such as `HC_SCOPE`
    /// inside a Herdr terminal.
    pub fn env_remove(mut self, key: impl AsRef<OsStr>) -> Self {
        self.env.push((key.as_ref().to_owned(), None));
        self
    }

    /// The transport builder these values describe.
    fn transport(self) -> Result<ProcessTransportBuilder, HerdrCoreError> {
        let mut transport = ProcessTransport::builder(argv(self.command)?);
        if let Some(session) = self.session {
            transport = transport.session(SessionName::new(session)?);
        }
        for (key, value) in self.env {
            transport = match value {
                Some(value) => transport.env(key, value),
                None => transport.env_remove(key),
            };
        }
        Ok(transport)
    }

    /// Starts the command and connects to the core it reaches.
    ///
    /// # Errors
    ///
    /// - [`HerdrCoreError::Empty`] for an empty command or session name. Nothing is started.
    /// - [`HerdrCoreError::Transport`] when the command cannot be started.
    /// - [`HerdrCoreError::Call`] when the handshake fails.
    ///
    /// # Panics
    ///
    /// Panics when called outside a Tokio runtime.
    pub async fn connect(self) -> Result<HerdrCore, HerdrCoreError> {
        let transport = self.transport()?.spawn()?;
        Ok(HerdrCore::from_client(Client::connect(transport).await?))
    }
}

#[cfg(test)]
mod tests {
    use super::HerdrCoreBuilder;
    use crate::herdr_core::HerdrCoreError;
    use crate::herdr_core::fake::TestError;
    use crate::port::TransportError;

    type TestResult = Result<(), TestError>;

    fn words(builder: &HerdrCoreBuilder) -> Vec<&str> {
        builder.command.iter().map(String::as_str).collect()
    }

    #[test]
    fn the_default_is_hc_connect_with_no_session_or_edits() {
        let builder = HerdrCoreBuilder::new();
        assert_eq!(words(&builder), ["hc", "connect"]);
        assert_eq!(builder.session, None);
        assert!(builder.env.is_empty());
    }

    #[test]
    fn ssh_runs_hc_connect_on_the_host_and_keeps_the_session() {
        let builder = HerdrCoreBuilder::new().session("work").ssh("user@box");
        assert_eq!(words(&builder), ["ssh", "user@box", "hc", "connect"]);
        assert_eq!(builder.session.as_deref(), Some("work"));
    }

    #[test]
    fn command_replaces_the_command_and_env_edits_keep_their_order() {
        let builder = HerdrCoreBuilder::new()
            .command(["my-proxy", "--stdio"])
            .env("A", "1")
            .env_remove("HC_SCOPE");
        assert_eq!(words(&builder), ["my-proxy", "--stdio"]);
        assert_eq!(
            builder.env,
            [("A".into(), Some("1".into())), ("HC_SCOPE".into(), None)]
        );
    }

    #[tokio::test]
    async fn an_empty_command_or_session_is_refused_before_anything_starts() -> TestResult {
        let none: [&str; 0] = [];
        let refused = HerdrCoreBuilder::new().command(none).connect().await;
        assert!(matches!(refused, Err(HerdrCoreError::Empty(error)) if error.what == "argv"));
        let refused = HerdrCoreBuilder::new().session("").connect().await;
        assert!(
            matches!(refused, Err(HerdrCoreError::Empty(error)) if error.what == "session name")
        );
        Ok(())
    }

    #[tokio::test]
    async fn a_command_that_cannot_start_is_a_transport_error() -> TestResult {
        let refused = HerdrCoreBuilder::new()
            .command(["herdrify-hc-no-such-program"])
            .connect()
            .await;
        assert!(matches!(
            refused,
            Err(HerdrCoreError::Transport(TransportError::Spawn { program, .. }))
                if program == "herdrify-hc-no-such-program"
        ));
        Ok(())
    }
}
