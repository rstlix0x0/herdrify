//! `HerdrCore`: the entrypoint, and the methods that belong to no narrower group.
//!
//! A `HerdrCore` is a [`Client`] and the ways to reach each group of methods from it. The methods
//! about the whole core and not one group of things, `machine.exec`, `events.subscribe`,
//! `templates.list` and `rpc.discover`, are its own, with their options here.
//!
//! Responsibilities:
//!
//! - [`HerdrCore`], how it is connected, and how its groups are reached
//! - the options of `machine.exec` and `events.subscribe`
//!
//! Non-responsibilities:
//!
//! - the transport it is connected over, which [`HerdrCoreBuilder`] assembles
//! - the methods of each group, which the group's own file defines

use std::time::Duration;

use serde_json::Value;

use super::call::{Call, argv, environment, invoke_by, labels};
use super::{
    DocOps, FileOps, HerdrCoreBuilder, HerdrCoreError, MachineOps, ServerOps, TerminalOps,
};
use crate::adapter::ProcessTransport;
use crate::connection::{Client, ConnectionEnd, Subscription};
use crate::port::Transport;
use crate::protocol::{
    EventsSubscribe, ExecCommand, ExecResult, HelloResult, MachineExec, MachineName, RemotePath,
    RpcDiscover, TemplatesList, TemplatesResult, TerminalRef,
};

/// One connection to herdr-core, with every method reachable from plain values.
///
/// Connect with [`local`](HerdrCore::local) or [`builder`](HerdrCore::builder) to run
/// `hc connect`, or with [`over`](HerdrCore::over) for a transport of your own. Each method
/// returns a [`Call`]: chain its options, then `.await` it. Cloning a `HerdrCore` is cheap, and
/// every clone talks over the same connection.
///
/// # Examples
///
/// ```no_run
/// use herdrify_hc::HerdrCore;
///
/// # async fn run() -> Result<(), herdrify_hc::HerdrCoreError> {
/// let core = HerdrCore::local().await?;
/// let started = core.terminals().run(["sh", "-c", "echo hello"]).await?;
/// started.terminal.wait().await?;
/// println!("{}", started.terminal.read().await?.text);
/// started.terminal.rm().await?;
/// # Ok(())
/// # }
/// ```
pub struct HerdrCore<T: Transport = ProcessTransport> {
    client: Client<T>,
}

impl HerdrCore {
    /// Connects to the core that `hc connect` reaches, with the environment unchanged.
    ///
    /// # Errors
    ///
    /// - [`HerdrCoreError::Transport`] when `hc` cannot be started.
    /// - [`HerdrCoreError::Call`] when the handshake fails.
    ///
    /// # Panics
    ///
    /// Panics when called outside a Tokio runtime.
    pub async fn local() -> Result<Self, HerdrCoreError> {
        Self::builder().connect().await
    }

    /// The builder for a connection through `hc connect`: a named session, a remote machine over
    /// ssh, or another command, with environment edits.
    pub fn builder() -> HerdrCoreBuilder {
        HerdrCoreBuilder::new()
    }
}

impl<T: Transport> HerdrCore<T> {
    /// Connects over `transport`, such as a
    /// [`StreamTransport`](crate::StreamTransport) or a fake in a test.
    ///
    /// # Errors
    ///
    /// [`HerdrCoreError::Call`] when the handshake fails, as [`Client::connect`] describes.
    pub async fn over(transport: T) -> Result<Self, HerdrCoreError> {
        Ok(Self::from_client(Client::connect(transport).await?))
    }

    /// Wraps a client that is already connected.
    #[must_use]
    pub const fn from_client(client: Client<T>) -> Self {
        Self { client }
    }

    /// The client underneath, for the typed `protocol` requests themselves.
    #[must_use]
    pub const fn client(&self) -> &Client<T> {
        &self.client
    }

    /// What the core answered to `hello`: its version and its methods.
    #[must_use]
    pub fn hello(&self) -> &HelloResult {
        self.client.hello()
    }

    /// Why the connection ended, or `None` while it is open.
    #[must_use]
    pub fn close_reason(&self) -> Option<ConnectionEnd> {
        self.client.close_reason()
    }

    /// The terminal methods: run, list, and a handle on one terminal.
    #[must_use]
    pub fn terminals(&self) -> TerminalOps<T> {
        TerminalOps::new(self.client.clone())
    }

    /// The methods that manage the machines the core reaches.
    #[must_use]
    pub fn machines(&self) -> MachineOps<T> {
        MachineOps::new(self.client.clone())
    }

    /// The methods that read, write and edit files.
    #[must_use]
    pub fn files(&self) -> FileOps<T> {
        FileOps::new(self.client.clone())
    }

    /// The methods that read and write the core's docs.
    #[must_use]
    pub fn docs(&self) -> DocOps<T> {
        DocOps::new(self.client.clone())
    }

    /// The methods about the core's own process: status, stop and restart.
    #[must_use]
    pub fn server(&self) -> ServerOps<T> {
        ServerOps::new(self.client.clone())
    }

    /// `machine.exec`: runs `words` (the program, then its arguments) without a terminal and
    /// returns its output and exit code.
    ///
    /// No words is reported by the `.await` as [`HerdrCoreError::Empty`].
    pub fn exec<S: Into<String>>(
        &self,
        words: impl IntoIterator<Item = S>,
    ) -> Call<T, MachineExec> {
        let params = argv(words).map(|argv| MachineExec::new(ExecCommand::Argv(argv)));
        Call::new(self.client.clone(), params)
    }

    /// `machine.exec`: runs a command line with the machine's shell.
    pub fn exec_line(&self, line: impl Into<String>) -> Call<T, MachineExec> {
        let params = MachineExec::new(ExecCommand::Line(line.into()));
        Call::new(self.client.clone(), Ok(params))
    }

    /// `events.subscribe`: a [`Subscription`] to the core's events, of every terminal unless an
    /// option narrows it.
    pub fn events(&self) -> Call<T, EventsSubscribe> {
        Call::new(self.client.clone(), Ok(EventsSubscribe::new()))
    }

    /// `templates.list`: the terminal templates the core knows.
    pub fn templates(&self) -> Call<T, TemplatesList> {
        Call::new(self.client.clone(), Ok(TemplatesList::new()))
    }

    /// `rpc.discover`: the core's `OpenRPC` document, as raw JSON.
    pub fn discover(&self) -> Call<T, RpcDiscover> {
        Call::new(self.client.clone(), Ok(RpcDiscover::new()))
    }
}

impl<T: Transport> Clone for HerdrCore<T> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
        }
    }
}

impl<T: Transport> std::fmt::Debug for HerdrCore<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HerdrCore").finish_non_exhaustive()
    }
}

impl<T: Transport> Call<T, MachineExec> {
    /// The working directory, on the machine that runs it.
    pub fn cwd(self, cwd: impl Into<String>) -> Self {
        self.with(|exec| exec.cwd(RemotePath::new(cwd)))
    }

    /// Environment variables set on top of the core's own.
    pub fn env<K: Into<String>, V: Into<String>>(
        self,
        variables: impl IntoIterator<Item = (K, V)>,
    ) -> Self {
        let variables = environment(variables);
        self.with(|exec| exec.env(variables))
    }

    /// Bytes for the command's standard input. Without them, standard input is closed.
    pub fn input(self, input: impl Into<Vec<u8>>) -> Self {
        let input = input.into();
        self.with(|exec| exec.input(input))
    }

    /// Runs the command on the machine called `machine`.
    pub fn on(self, machine: impl Into<String>) -> Self {
        let machine = MachineName::new(machine);
        self.try_with(|exec| Ok(exec.on(machine?)))
    }

    /// How long the command, and everything it started, may run before it is ended.
    pub fn timeout(self, timeout: Duration) -> Self {
        self.with(|exec| exec.timeout(timeout))
    }
}

impl<T: Transport> Call<T, EventsSubscribe> {
    /// Only the events of the terminals started under `parent`.
    pub fn children_of(self, parent: impl Into<String>) -> Self {
        let parent = TerminalRef::new(parent);
        self.try_with(|subscribe| Ok(subscribe.children_of(parent?)))
    }

    /// Only the events of terminals that carry every one of these labels.
    pub fn labels<K: Into<String>, V: Into<String>>(
        self,
        pairs: impl IntoIterator<Item = (K, V)>,
    ) -> Self {
        let labels = labels(pairs);
        self.try_with(|subscribe| Ok(subscribe.labels(labels?)))
    }

    /// Only the events of `terminal`.
    pub fn terminal(self, terminal: impl Into<String>) -> Self {
        let terminal = TerminalRef::new(terminal);
        self.try_with(|subscribe| Ok(subscribe.terminal(terminal?)))
    }
}

invoke_by! {
    MachineExec => ExecResult, machine_exec;
    EventsSubscribe => Subscription, events_subscribe;
    TemplatesList => TemplatesResult, templates_list;
    RpcDiscover => Value, rpc_discover;
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;

    use super::HerdrCore;
    use crate::herdr_core::HerdrCoreError;
    use crate::herdr_core::fake::{TestError, connected, request_of, serve};
    use crate::protocol::{
        Argv, Environment, EventsSubscribe, ExecCommand, LabelKey, LabelValue, Labels, MachineExec,
        MachineName, RemotePath, TerminalRef,
    };

    type TestResult = Result<(), TestError>;

    #[tokio::test]
    async fn exec_builds_the_argv_request_with_every_option() -> TestResult {
        let (client, _peer) = connected().await?;
        let call = HerdrCore::from_client(client)
            .exec(["wc", "-c"])
            .cwd("/tmp")
            .env([("A", "1")])
            .input("hi")
            .on("box")
            .timeout(Duration::from_secs(2));
        let mut env = Environment::new();
        env.insert("A", "1");
        let argv = Argv::new(vec!["wc".to_owned(), "-c".to_owned()])?;
        let expected = MachineExec::new(ExecCommand::Argv(argv))
            .cwd(RemotePath::new("/tmp"))
            .env(env)
            .input(b"hi".to_vec())
            .on(MachineName::new("box")?)
            .timeout(Duration::from_secs(2));
        assert_eq!(request_of(&call)?, &expected);
        Ok(())
    }

    #[tokio::test]
    async fn exec_line_builds_the_line_request() -> TestResult {
        let (client, _peer) = connected().await?;
        let call = HerdrCore::from_client(client).exec_line("echo hi");
        let expected = MachineExec::new(ExecCommand::Line("echo hi".to_owned()));
        assert_eq!(request_of(&call)?, &expected);
        Ok(())
    }

    #[tokio::test]
    async fn exec_sends_machine_exec_and_returns_its_result() -> TestResult {
        let (client, mut peer) = connected().await?;
        let call = HerdrCore::from_client(client).exec(["true"]);
        let result = json!({"stdout": "", "stderr": "", "truncated": false, "timed_out": false, "ms": 1, "code": 0});
        let (sent, output) = serve(&mut peer, call, result).await?;
        assert_eq!(sent["method"], "machine.exec");
        assert_eq!(output?.code, Some(0));
        Ok(())
    }

    #[tokio::test]
    async fn exec_refuses_no_words_and_an_empty_machine_and_sends_nothing() -> TestResult {
        let (client, mut peer) = connected().await?;
        let core = HerdrCore::from_client(client);
        let none: [&str; 0] = [];
        assert!(matches!(
            core.exec(none).await,
            Err(HerdrCoreError::Empty(error)) if error.what == "argv"
        ));
        assert!(matches!(
            core.exec(["true"]).on("").await,
            Err(HerdrCoreError::Empty(error)) if error.what == "machine name"
        ));
        assert!(!peer.has_unread());
        Ok(())
    }

    #[tokio::test]
    async fn events_narrow_by_terminal_children_and_labels() -> TestResult {
        let (client, _peer) = connected().await?;
        let call = HerdrCore::from_client(client)
            .events()
            .terminal("t1")
            .children_of("t2")
            .labels([("role", "build")]);
        let mut labels = Labels::new();
        labels.insert(LabelKey::new("role")?, LabelValue::new("build")?);
        let expected = EventsSubscribe::new()
            .terminal(TerminalRef::new("t1")?)
            .children_of(TerminalRef::new("t2")?)
            .labels(labels);
        assert_eq!(request_of(&call)?, &expected);
        Ok(())
    }

    #[tokio::test]
    async fn templates_and_discover_send_their_method() -> TestResult {
        let (client, mut peer) = connected().await?;
        let core = HerdrCore::from_client(client);
        let (sent, templates) =
            serve(&mut peer, core.templates(), json!({"templates": []})).await?;
        assert_eq!(sent["method"], "templates.list");
        assert!(templates?.templates.is_empty());
        let (sent, document) = serve(&mut peer, core.discover(), json!({"openrpc": "1"})).await?;
        assert_eq!(sent["method"], "rpc.discover");
        assert_eq!(document?["openrpc"], "1");
        Ok(())
    }

    #[tokio::test]
    async fn clones_share_the_connection_and_expose_the_client() -> TestResult {
        let (client, _peer) = connected().await?;
        let core = HerdrCore::from_client(client);
        let clone = core.clone();
        assert_eq!(clone.hello().version, "0.1.21");
        assert_eq!(core.client().hello().version, "0.1.21");
        assert!(core.close_reason().is_none());
        assert_eq!(format!("{core:?}"), "HerdrCore { .. }");
        Ok(())
    }
}
