//! The terminal methods: run and list terminals, and act on one through its handle.
//!
//! [`TerminalOps`] holds the methods that are not about one terminal: `terminal.run`,
//! `terminal.list` and a wait on the caller's own children. [`TerminalHandle`] holds every method
//! about one terminal, so its reference is named once. Running a terminal returns [`Started`]: the
//! core's answer and the handle on the new terminal.
//!
//! Responsibilities:
//!
//! - [`TerminalOps`], [`TerminalHandle`] and [`Started`]
//! - the options of every terminal request
//!
//! Non-responsibilities:
//!
//! - `events.subscribe`'s options, which `entry` defines next to [`HerdrCore::events`]
//!
//! [`HerdrCore::events`]: super::HerdrCore::events

use std::fmt;
use std::num::NonZeroU16;
use std::time::Duration;

use super::HerdrCoreError;
use super::call::{Call, Invoke, argv, environment, invoke_by, labels};
use crate::adapter::ProcessTransport;
use crate::connection::{AttachStream, Client};
use crate::port::Transport;
use crate::protocol::{
    CallerId, Children, Command, DisplayFacts, EventsSubscribe, FileName, IdleOrWorking,
    InitialSize, InterruptResult, Keep, KeyName, KeysResult, LabelChange, LabelChanges, LabelKey,
    LabelValue, ListResult, MachineName, PutResult, ReadResult, RemotePath, RemoveResult,
    RunResult, SendResult, Terminal, TerminalAttach, TerminalGet, TerminalInterrupt, TerminalKeep,
    TerminalKeys, TerminalKill, TerminalLabel, TerminalList, TerminalName, TerminalPut,
    TerminalRead, TerminalRef, TerminalRm, TerminalRun, TerminalSend, TerminalWait, WaitResult,
    WaitTarget,
};

/// The terminal methods that are not about one terminal.
///
/// Reached through [`HerdrCore::terminals`](super::HerdrCore::terminals).
///
/// # Examples
///
/// ```no_run
/// use herdrify_hc::HerdrCore;
///
/// # async fn run() -> Result<(), herdrify_hc::HerdrCoreError> {
/// let core = HerdrCore::local().await?;
/// for terminal in core.terminals().list().await?.terminals {
///     println!("{} runs {:?}", terminal.address, terminal.argv);
/// }
/// # Ok(())
/// # }
/// ```
pub struct TerminalOps<T: Transport = ProcessTransport> {
    client: Client<T>,
}

impl<T: Transport> TerminalOps<T> {
    /// The terminal methods over `client`.
    pub(super) const fn new(client: Client<T>) -> Self {
        Self { client }
    }

    /// `terminal.run`: starts `words` (the program, then its arguments) in a new terminal.
    ///
    /// No words is reported by the `.await` as [`HerdrCoreError::Empty`].
    pub fn run<S: Into<String>>(&self, words: impl IntoIterator<Item = S>) -> Call<T, TerminalRun> {
        let params = argv(words).map(|argv| TerminalRun::new(Command::Argv(argv)));
        Call::new(self.client.clone(), params)
    }

    /// `terminal.run`: starts a command line with the machine's shell in a new terminal.
    pub fn run_line(&self, line: impl Into<String>) -> Call<T, TerminalRun> {
        let params = TerminalRun::new(Command::Line(line.into()));
        Call::new(self.client.clone(), Ok(params))
    }

    /// `terminal.run`: starts the machine's shell in a new terminal.
    pub fn run_shell(&self) -> Call<T, TerminalRun> {
        Call::new(self.client.clone(), Ok(TerminalRun::new(Command::Shell)))
    }

    /// `terminal.list`: the live terminals, and with [`all`](Call::all) the finished ones too.
    pub fn list(&self) -> Call<T, TerminalList> {
        Call::new(self.client.clone(), Ok(TerminalList::new()))
    }

    /// `terminal.wait`: waits for `children` of the terminal this program runs in to end.
    pub fn wait_children(&self, children: Children) -> Call<T, TerminalWait> {
        let params = TerminalWait::new(WaitTarget::Children(children));
        Call::new(self.client.clone(), Ok(params))
    }

    /// A handle on the terminal `reference` names, such as `t3` or a terminal's name.
    ///
    /// # Errors
    ///
    /// [`HerdrCoreError::Empty`] when `reference` is empty.
    pub fn handle(
        &self,
        reference: impl Into<String>,
    ) -> Result<TerminalHandle<T>, HerdrCoreError> {
        Ok(TerminalHandle {
            client: self.client.clone(),
            reference: TerminalRef::new(reference)?,
        })
    }
}

impl<T: Transport> fmt::Debug for TerminalOps<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TerminalOps").finish_non_exhaustive()
    }
}

/// Every method about one terminal, with its reference named once.
///
/// Returned by [`TerminalOps::handle`] and, for a terminal just started, in [`Started`]. Cloning
/// it is cheap.
///
/// # Examples
///
/// ```no_run
/// use std::time::Duration;
///
/// use herdrify_hc::HerdrCore;
///
/// # async fn run() -> Result<(), herdrify_hc::HerdrCoreError> {
/// let core = HerdrCore::local().await?;
/// let shell = core.terminals().run(["sh"]).await?.terminal;
/// shell.send("echo $((6 * 7))").await?;
/// shell.interrupt().timeout(Duration::from_secs(2)).await?;
/// shell.kill().await?;
/// shell.rm().await?;
/// # Ok(())
/// # }
/// ```
pub struct TerminalHandle<T: Transport = ProcessTransport> {
    client: Client<T>,
    reference: TerminalRef,
}

impl<T: Transport> TerminalHandle<T> {
    /// The reference every request of this handle names.
    #[must_use]
    pub const fn reference(&self) -> &TerminalRef {
        &self.reference
    }

    /// A call about this terminal.
    fn call<P>(&self, params: P) -> Call<T, P> {
        Call::new(self.client.clone(), Ok(params))
    }

    /// `terminal.get`: the terminal's record.
    pub fn get(&self) -> Call<T, TerminalGet> {
        self.call(TerminalGet::new(self.reference.clone()))
    }

    /// `terminal.read`: the text on the terminal's screen.
    pub fn read(&self) -> Call<T, TerminalRead> {
        self.call(TerminalRead::new(self.reference.clone()))
    }

    /// `terminal.send`: pastes `text` into the terminal and submits it with Enter.
    pub fn send(&self, text: impl Into<String>) -> Call<T, TerminalSend> {
        self.call(TerminalSend::new(self.reference.clone(), text.into()))
    }

    /// `terminal.keys`: presses `keys` in order, such as `["ctrl+c", "enter"]`. The list may be
    /// empty.
    ///
    /// An empty key name is reported by the `.await` as [`HerdrCoreError::Empty`].
    pub fn keys<S: Into<String>>(
        &self,
        keys: impl IntoIterator<Item = S>,
    ) -> Call<T, TerminalKeys> {
        let keys: Result<Vec<KeyName>, _> = keys.into_iter().map(KeyName::new).collect();
        let reference = self.reference.clone();
        let params = keys
            .map(|keys| TerminalKeys::new(reference, keys))
            .map_err(HerdrCoreError::from);
        Call::new(self.client.clone(), params)
    }

    /// `terminal.interrupt`: presses Ctrl-C and reports the effect the core saw.
    pub fn interrupt(&self) -> Call<T, TerminalInterrupt> {
        self.call(TerminalInterrupt::new(self.reference.clone()))
    }

    /// `terminal.wait`: waits for the terminal to end, or with [`output`](Call::output) for a
    /// text in its output.
    pub fn wait(&self) -> Call<T, TerminalWait> {
        self.call(TerminalWait::new(WaitTarget::Terminal(
            self.reference.clone(),
        )))
    }

    /// `terminal.wait`: waits for `children` of this terminal to end.
    pub fn wait_children(&self, children: Children) -> Call<T, TerminalWait> {
        self.call(TerminalWait::new(WaitTarget::ChildrenOf(
            self.reference.clone(),
            children,
        )))
    }

    /// `terminal.put`: stores `data` as a file called `name` for the terminal.
    pub fn put(&self, name: impl Into<String>, data: impl Into<Vec<u8>>) -> Call<T, TerminalPut> {
        self.call(TerminalPut::new(
            self.reference.clone(),
            FileName::new(name),
            data.into(),
        ))
    }

    /// `terminal.keep`: with [`Keep::On`], a lost program is replaced with a shell in place
    /// instead of ending the terminal; [`Keep::Off`] turns that off.
    pub fn keep(&self, keep: Keep) -> Call<T, TerminalKeep> {
        self.call(TerminalKeep::new(self.reference.clone(), keep))
    }

    /// `terminal.label`: sets each label in `pairs`.
    ///
    /// An empty key or value is reported by the `.await` as [`HerdrCoreError::Empty`].
    pub fn set_labels<K: Into<String>, V: Into<String>>(
        &self,
        pairs: impl IntoIterator<Item = (K, V)>,
    ) -> Call<T, TerminalLabel> {
        let changes = pairs
            .into_iter()
            .map(|(key, value)| {
                Ok((
                    LabelKey::new(key)?,
                    LabelChange::Set(LabelValue::new(value)?),
                ))
            })
            .collect::<Result<LabelChanges, HerdrCoreError>>();
        self.label(changes)
    }

    /// `terminal.label`: removes each label in `keys`.
    ///
    /// An empty key is reported by the `.await` as [`HerdrCoreError::Empty`].
    pub fn remove_labels<S: Into<String>>(
        &self,
        keys: impl IntoIterator<Item = S>,
    ) -> Call<T, TerminalLabel> {
        let changes = keys
            .into_iter()
            .map(|key| Ok((LabelKey::new(key)?, LabelChange::Remove)))
            .collect::<Result<LabelChanges, HerdrCoreError>>();
        self.label(changes)
    }

    /// A `terminal.label` call with `changes`.
    fn label(&self, changes: Result<LabelChanges, HerdrCoreError>) -> Call<T, TerminalLabel> {
        let reference = self.reference.clone();
        let params = changes.map(|changes| TerminalLabel::new(reference, changes));
        Call::new(self.client.clone(), params)
    }

    /// `terminal.kill`: kills the terminal's program.
    pub fn kill(&self) -> Call<T, TerminalKill> {
        self.call(TerminalKill::new(self.reference.clone()))
    }

    /// `terminal.rm`: removes the terminal.
    pub fn rm(&self) -> Call<T, TerminalRm> {
        self.call(TerminalRm::new(self.reference.clone()))
    }

    /// `terminal.attach`: an [`AttachStream`] of the terminal's output, for a viewer `cols` by
    /// `rows` in size.
    pub fn attach(&self, cols: u16, rows: u16) -> Call<T, TerminalAttach> {
        self.call(TerminalAttach::new(self.reference.clone(), cols, rows))
    }

    /// `events.subscribe`: a subscription to this terminal's events.
    pub fn events(&self) -> Call<T, EventsSubscribe> {
        self.call(EventsSubscribe::new().terminal(self.reference.clone()))
    }
}

impl<T: Transport> Clone for TerminalHandle<T> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            reference: self.reference.clone(),
        }
    }
}

impl<T: Transport> fmt::Debug for TerminalHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TerminalHandle")
            .field("reference", &self.reference)
            .finish_non_exhaustive()
    }
}

/// A terminal `terminal.run` started: the core's answer and a handle on the terminal.
pub struct Started<T: Transport = ProcessTransport> {
    /// The handle on the new terminal.
    pub terminal: TerminalHandle<T>,
    /// What the core answered: the terminal's address, state and screen when it started.
    pub result: RunResult,
}

impl<T: Transport> fmt::Debug for Started<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Started")
            .field("terminal", &self.terminal)
            .field("result", &self.result)
            .finish()
    }
}

impl<T: Transport> Invoke<T> for TerminalRun {
    type Output = Started<T>;

    async fn invoke(self, client: &Client<T>) -> Result<Started<T>, HerdrCoreError> {
        let result = client.terminal_run(self).await?;
        let reference = TerminalRef::new(result.address.as_str())?;
        Ok(Started {
            terminal: TerminalHandle {
                client: client.clone(),
                reference,
            },
            result,
        })
    }
}

invoke_by! {
    TerminalList => ListResult, terminal_list;
    TerminalGet => Terminal, terminal_get;
    TerminalKill => Terminal, terminal_kill;
    TerminalRm => RemoveResult, terminal_rm;
    TerminalKeep => Terminal, terminal_keep;
    TerminalLabel => Terminal, terminal_label;
    TerminalRead => ReadResult, terminal_read;
    TerminalSend => SendResult, terminal_send;
    TerminalWait => WaitResult, terminal_wait;
    TerminalKeys => KeysResult, terminal_keys;
    TerminalInterrupt => InterruptResult, terminal_interrupt;
    TerminalPut => PutResult, terminal_put;
    TerminalAttach => AttachStream, terminal_attach;
}

/// A terminal size from plain numbers, refusing zero.
fn initial_size(cols: u16, rows: u16) -> Result<InitialSize, HerdrCoreError> {
    let cols = NonZeroU16::new(cols).ok_or(HerdrCoreError::Zero {
        what: "terminal columns",
    })?;
    let rows = NonZeroU16::new(rows).ok_or(HerdrCoreError::Zero {
        what: "terminal rows",
    })?;
    Ok(InitialSize { cols, rows })
}

impl<T: Transport> Call<T, TerminalRun> {
    /// Who is calling.
    pub fn caller(self, caller: impl Into<String>) -> Self {
        self.with(|run| run.caller(CallerId::new(caller)))
    }

    /// The working directory, on the machine that runs it.
    pub fn cwd(self, cwd: impl Into<String>) -> Self {
        self.with(|run| run.cwd(RemotePath::new(cwd)))
    }

    /// Sends `die_with_parent`, which the core ignores: a terminal ends with its parent unless
    /// [`outlive_parent`](Self::outlive_parent) is set.
    pub fn die_with_parent(self) -> Self {
        self.with(TerminalRun::die_with_parent)
    }

    /// What the viewer's terminal looks like.
    pub fn display(self, display: DisplayFacts) -> Self {
        self.with(|run| run.display(display))
    }

    /// Environment variables set on top of the core's own.
    pub fn env<K: Into<String>, V: Into<String>>(
        self,
        variables: impl IntoIterator<Item = (K, V)>,
    ) -> Self {
        let variables = environment(variables);
        self.with(|run| run.env(variables))
    }

    /// Replaces a lost program with a shell in place instead of ending the terminal.
    pub fn keep(self) -> Self {
        self.with(TerminalRun::keep)
    }

    /// Labels to set on the new terminal.
    pub fn labels<K: Into<String>, V: Into<String>>(
        self,
        pairs: impl IntoIterator<Item = (K, V)>,
    ) -> Self {
        let labels = labels(pairs);
        self.try_with(|run| Ok(run.labels(labels?)))
    }

    /// The terminal's name, unique among live terminals.
    pub fn name(self, name: impl Into<String>) -> Self {
        let name = TerminalName::new(name);
        self.try_with(|run| Ok(run.name(name?)))
    }

    /// Does not type notes about the terminal into its parent.
    pub fn no_notify(self) -> Self {
        self.with(TerminalRun::no_notify)
    }

    /// Gives the terminal no parent, even when started from a terminal.
    pub fn no_parent(self) -> Self {
        self.with(TerminalRun::no_parent)
    }

    /// Runs the terminal on the machine called `machine`.
    pub fn on(self, machine: impl Into<String>) -> Self {
        let machine = MachineName::new(machine);
        self.try_with(|run| Ok(run.on(machine?)))
    }

    /// Keeps the terminal running when its parent's program ends.
    pub fn outlive_parent(self) -> Self {
        self.with(TerminalRun::outlive_parent)
    }

    /// The parent terminal.
    pub fn parent(self, parent: impl Into<String>) -> Self {
        let parent = TerminalRef::new(parent);
        self.try_with(|run| Ok(run.parent(parent?)))
    }

    /// Removes the terminal right after it exits.
    pub fn rm(self) -> Self {
        self.with(TerminalRun::rm)
    }

    /// The size the terminal starts with: `cols` columns by `rows` rows.
    ///
    /// A zero is reported by the `.await` as [`HerdrCoreError::Zero`].
    pub fn size(self, cols: u16, rows: u16) -> Self {
        let size = initial_size(cols, rows);
        self.try_with(|run| Ok(run.size(size?)))
    }

    /// How long a wait lasts before it checks in, leaving the terminal running.
    pub fn timeout(self, timeout: Duration) -> Self {
        self.with(|run| run.timeout(timeout))
    }

    /// Waits until the terminal is not working, as `terminal.wait` does.
    pub fn wait(self) -> Self {
        self.with(TerminalRun::wait)
    }
}

impl<T: Transport> Call<T, TerminalList> {
    /// Includes finished terminals.
    pub fn all(self) -> Self {
        self.with(TerminalList::all)
    }

    /// Only terminals that carry these labels.
    pub fn labels<K: Into<String>, V: Into<String>>(
        self,
        pairs: impl IntoIterator<Item = (K, V)>,
    ) -> Self {
        let labels = labels(pairs);
        self.try_with(|list| Ok(list.labels(labels?)))
    }
}

impl<T: Transport> Call<T, TerminalRead> {
    /// Who is calling.
    pub fn caller(self, caller: impl Into<String>) -> Self {
        self.with(|read| read.caller(CallerId::new(caller)))
    }

    /// How many lines of history to include above the screen.
    pub fn lines(self, lines: u64) -> Self {
        self.with(|read| read.lines(lines))
    }
}

impl<T: Transport> Call<T, TerminalSend> {
    /// Who is sending.
    pub fn caller(self, caller: impl Into<String>) -> Self {
        self.with(|send| send.caller(CallerId::new(caller)))
    }

    /// Types even if the program is blocked.
    pub fn force(self) -> Self {
        self.with(TerminalSend::force)
    }

    /// Types only if the terminal's status is exactly `status`.
    pub fn only_if(self, status: IdleOrWorking) -> Self {
        self.with(|send| send.only_if(status))
    }

    /// With [`wait`](Self::wait), how long to wait before checking in.
    pub fn timeout(self, timeout: Duration) -> Self {
        self.with(|send| send.timeout(timeout))
    }

    /// Waits for the turn this send started.
    pub fn wait(self) -> Self {
        self.with(TerminalSend::wait)
    }
}

impl<T: Transport> Call<T, TerminalWait> {
    /// Who is waiting.
    pub fn caller(self, caller: impl Into<String>) -> Self {
        self.with(|wait| wait.caller(CallerId::new(caller)))
    }

    /// Ends the wait when output printed after it started contains `text`.
    pub fn output(self, text: impl Into<String>) -> Self {
        self.with(|wait| wait.output(text.into()))
    }

    /// How long to wait before checking in.
    pub fn timeout(self, timeout: Duration) -> Self {
        self.with(|wait| wait.timeout(timeout))
    }
}

impl<T: Transport> Call<T, TerminalInterrupt> {
    /// How long to watch for an effect.
    pub fn timeout(self, timeout: Duration) -> Self {
        self.with(|interrupt| interrupt.timeout(timeout))
    }
}

impl<T: Transport> Call<T, TerminalGet> {
    /// Who is calling.
    pub fn caller(self, caller: impl Into<String>) -> Self {
        self.with(|get| get.caller(CallerId::new(caller)))
    }
}

impl<T: Transport> Call<T, TerminalKill> {
    /// Who is calling.
    pub fn caller(self, caller: impl Into<String>) -> Self {
        self.with(|kill| kill.caller(CallerId::new(caller)))
    }
}

impl<T: Transport> Call<T, TerminalRm> {
    /// Who is calling.
    pub fn caller(self, caller: impl Into<String>) -> Self {
        self.with(|rm| rm.caller(CallerId::new(caller)))
    }
}

impl<T: Transport> Call<T, TerminalKeys> {
    /// Who is calling.
    pub fn caller(self, caller: impl Into<String>) -> Self {
        self.with(|keys| keys.caller(CallerId::new(caller)))
    }
}

impl<T: Transport> Call<T, TerminalAttach> {
    /// Who is calling.
    pub fn caller(self, caller: impl Into<String>) -> Self {
        self.with(|attach| attach.caller(CallerId::new(caller)))
    }

    /// What the viewer's terminal looks like.
    pub fn display(self, display: DisplayFacts) -> Self {
        self.with(|attach| attach.display(display))
    }

    /// How many lines of scrollback to show first, on the main screen only.
    pub fn history(self, lines: u64) -> Self {
        self.with(|attach| attach.history(lines))
    }

    /// Watches only: the viewer never types and never resizes.
    pub fn observe(self) -> Self {
        self.with(TerminalAttach::observe)
    }

    /// Tells the core the observer understands a core-to-viewer resize before replacement
    /// paints.
    pub fn resize_notifications(self) -> Self {
        self.with(TerminalAttach::resize_notifications)
    }

    /// Marks the attach as a return after a dropped line, so the size is set only once the
    /// viewer is active.
    pub fn resumed(self) -> Self {
        self.with(TerminalAttach::resumed)
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU16;
    use std::time::Duration;

    use serde_json::{Value, json};

    use crate::herdr_core::fake::{TestError, connected, request_of, serve};
    use crate::herdr_core::{HerdrCore, HerdrCoreError};
    use crate::protocol::{
        Argv, CallerId, Children, Command, DisplayFacts, Environment, EventsSubscribe, FileName,
        IdleOrWorking, InitialSize, Keep, KeyName, LabelChange, LabelChanges, LabelKey, LabelValue,
        Labels, MachineName, RemotePath, TerminalAttach, TerminalGet, TerminalInterrupt,
        TerminalKeep, TerminalKeys, TerminalKill, TerminalLabel, TerminalList, TerminalName,
        TerminalPut, TerminalRead, TerminalRef, TerminalRm, TerminalRun, TerminalSend,
        TerminalWait, WaitTarget,
    };

    type TestResult = Result<(), TestError>;

    /// The smallest record the core sends for a terminal.
    fn record(address: &str) -> Value {
        json!({"address": address, "argv": ["sh"], "cols": 80, "created_ms": 0, "cwd": "/",
            "labels": {}, "rows": 24, "screen": "exact", "state": "live"})
    }

    const fn no_facts() -> DisplayFacts {
        DisplayFacts {
            bg: None,
            cell: None,
            cursor: None,
            fg: None,
        }
    }

    fn t(reference: &str) -> Result<TerminalRef, TestError> {
        Ok(TerminalRef::new(reference)?)
    }

    fn pairs(entries: &[(&str, &str)]) -> Result<Labels, TestError> {
        let mut labels = Labels::new();
        for (key, value) in entries {
            labels.insert(LabelKey::new(*key)?, LabelValue::new(*value)?);
        }
        Ok(labels)
    }

    #[tokio::test]
    async fn run_builds_each_command_with_every_option() -> TestResult {
        let (client, _peer) = connected().await?;
        let terminals = HerdrCore::from_client(client).terminals();
        let display = no_facts();
        let call = terminals
            .run(["sh", "-c", "true"])
            .caller("me")
            .cwd("/work")
            .die_with_parent()
            .display(display.clone())
            .env([("A", "1")])
            .keep()
            .labels([("role", "build")])
            .name("build")
            .no_notify()
            .no_parent()
            .on("box")
            .outlive_parent()
            .parent("t1")
            .rm()
            .size(100, 30)
            .timeout(Duration::from_secs(3))
            .wait();
        let mut env = Environment::new();
        env.insert("A", "1");
        let argv = Argv::new(vec!["sh".to_owned(), "-c".to_owned(), "true".to_owned()])?;
        let size = InitialSize {
            cols: NonZeroU16::new(100).ok_or("zero")?,
            rows: NonZeroU16::new(30).ok_or("zero")?,
        };
        let expected = TerminalRun::new(Command::Argv(argv))
            .caller(CallerId::new("me"))
            .cwd(RemotePath::new("/work"))
            .die_with_parent()
            .display(display)
            .env(env)
            .keep()
            .labels(pairs(&[("role", "build")])?)
            .name(TerminalName::new("build")?)
            .no_notify()
            .no_parent()
            .on(MachineName::new("box")?)
            .outlive_parent()
            .parent(t("t1")?)
            .rm()
            .size(size)
            .timeout(Duration::from_secs(3))
            .wait();
        assert_eq!(request_of(&call)?, &expected);

        let line = terminals.run_line("echo hi");
        assert_eq!(
            request_of(&line)?,
            &TerminalRun::new(Command::Line("echo hi".to_owned()))
        );
        let shell = terminals.run_shell();
        assert_eq!(request_of(&shell)?, &TerminalRun::new(Command::Shell));
        Ok(())
    }

    #[tokio::test]
    async fn run_refuses_bad_values_by_what_they_are() -> TestResult {
        let (client, _peer) = connected().await?;
        let terminals = HerdrCore::from_client(client).terminals();
        let none: [&str; 0] = [];
        let refusals = [
            (terminals.run(none), "argv"),
            (terminals.run_shell().name(""), "terminal name"),
            (terminals.run_shell().parent(""), "terminal reference"),
            (terminals.run_shell().on(""), "machine name"),
            (terminals.run_shell().labels([("", "v")]), "label key"),
        ];
        for (call, what) in refusals {
            assert!(
                matches!(call.request(), Err(HerdrCoreError::Empty(error)) if error.what == what),
                "{what}"
            );
        }
        for (cols, rows, what) in [(0, 24, "terminal columns"), (80, 0, "terminal rows")] {
            let call = terminals.run_shell().size(cols, rows);
            assert!(
                matches!(call.request(), Err(HerdrCoreError::Zero { what: zero }) if *zero == what)
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn run_returns_a_handle_on_the_started_terminal() -> TestResult {
        let (client, mut peer) = connected().await?;
        let call = HerdrCore::from_client(client).terminals().run(["sh"]);
        let (sent, started) = serve(&mut peer, call, record("t4")).await?;
        assert_eq!(sent["method"], "terminal.run");
        let started = started?;
        assert_eq!(started.result.address.as_str(), "t4");
        assert_eq!(started.terminal.reference().as_str(), "t4");
        Ok(())
    }

    #[tokio::test]
    async fn list_and_wait_children_build_their_requests() -> TestResult {
        let (client, _peer) = connected().await?;
        let terminals = HerdrCore::from_client(client).terminals();
        let list = terminals.list().all().labels([("a", "1")]);
        let expected = TerminalList::new().all().labels(pairs(&[("a", "1")])?);
        assert_eq!(request_of(&list)?, &expected);
        let wait = terminals.wait_children(Children::All);
        let expected = TerminalWait::new(WaitTarget::Children(Children::All));
        assert_eq!(request_of(&wait)?, &expected);
        Ok(())
    }

    #[tokio::test]
    async fn handle_refuses_an_empty_reference() -> TestResult {
        let (client, _peer) = connected().await?;
        let refused = HerdrCore::from_client(client).terminals().handle("");
        assert!(matches!(
            refused,
            Err(HerdrCoreError::Empty(error)) if error.what == "terminal reference"
        ));
        Ok(())
    }

    #[tokio::test]
    async fn handle_builds_every_request_about_its_terminal() -> TestResult {
        let (client, _peer) = connected().await?;
        let shell = HerdrCore::from_client(client).terminals().handle("t2")?;
        let r = t("t2")?;
        assert_eq!(shell.reference(), &r);
        assert_eq!(
            request_of(&shell.get().caller("me"))?,
            &TerminalGet::new(r.clone()).caller(CallerId::new("me"))
        );
        assert_eq!(
            request_of(&shell.read().caller("me").lines(50))?,
            &TerminalRead::new(r.clone())
                .caller(CallerId::new("me"))
                .lines(50)
        );
        let send = shell
            .send("ls")
            .caller("me")
            .force()
            .only_if(IdleOrWorking::Idle)
            .timeout(Duration::from_secs(1))
            .wait();
        let expected = TerminalSend::new(r.clone(), "ls".to_owned())
            .caller(CallerId::new("me"))
            .force()
            .only_if(IdleOrWorking::Idle)
            .timeout(Duration::from_secs(1))
            .wait();
        assert_eq!(request_of(&send)?, &expected);
        assert_eq!(
            request_of(&shell.keys(["ctrl+c", "enter"]).caller("me"))?,
            &TerminalKeys::new(
                r.clone(),
                vec![KeyName::new("ctrl+c")?, KeyName::new("enter")?]
            )
            .caller(CallerId::new("me"))
        );
        assert_eq!(
            request_of(&shell.interrupt().timeout(Duration::from_secs(2)))?,
            &TerminalInterrupt::new(r.clone()).timeout(Duration::from_secs(2))
        );
        let wait = shell
            .wait()
            .caller("me")
            .output("done")
            .timeout(Duration::from_secs(4));
        let expected = TerminalWait::new(WaitTarget::Terminal(r.clone()))
            .caller(CallerId::new("me"))
            .output("done".to_owned())
            .timeout(Duration::from_secs(4));
        assert_eq!(request_of(&wait)?, &expected);
        assert_eq!(
            request_of(&shell.wait_children(Children::Any))?,
            &TerminalWait::new(WaitTarget::ChildrenOf(r.clone(), Children::Any))
        );
        assert_eq!(
            request_of(&shell.put("notes.txt", "hi"))?,
            &TerminalPut::new(r.clone(), FileName::new("notes.txt"), b"hi".to_vec())
        );
        assert_eq!(
            request_of(&shell.keep(Keep::Off))?,
            &TerminalKeep::new(r.clone(), Keep::Off)
        );
        assert_eq!(
            request_of(&shell.kill().caller("me"))?,
            &TerminalKill::new(r.clone()).caller(CallerId::new("me"))
        );
        assert_eq!(
            request_of(&shell.rm().caller("me"))?,
            &TerminalRm::new(r.clone()).caller(CallerId::new("me"))
        );
        let attach = shell
            .attach(80, 24)
            .caller("me")
            .display(no_facts())
            .history(10)
            .observe()
            .resize_notifications()
            .resumed();
        let expected = TerminalAttach::new(r.clone(), 80, 24)
            .caller(CallerId::new("me"))
            .display(no_facts())
            .history(10)
            .observe()
            .resize_notifications()
            .resumed();
        assert_eq!(request_of(&attach)?, &expected);
        assert_eq!(
            request_of(&shell.events())?,
            &EventsSubscribe::new().terminal(r)
        );
        Ok(())
    }

    #[tokio::test]
    async fn labels_are_set_or_removed_and_empty_values_are_refused() -> TestResult {
        let (client, _peer) = connected().await?;
        let shell = HerdrCore::from_client(client).terminals().handle("t2")?;
        let mut set = LabelChanges::new();
        set.insert(LabelKey::new("a")?, LabelChange::Set(LabelValue::new("1")?));
        assert_eq!(
            request_of(&shell.set_labels([("a", "1")]))?,
            &TerminalLabel::new(t("t2")?, set)
        );
        let mut removed = LabelChanges::new();
        removed.insert(LabelKey::new("a")?, LabelChange::Remove);
        assert_eq!(
            request_of(&shell.remove_labels(["a"]))?,
            &TerminalLabel::new(t("t2")?, removed)
        );
        assert!(matches!(
            shell.set_labels([("a", "")]).request(),
            Err(HerdrCoreError::Empty(error)) if error.what == "label value"
        ));
        assert!(matches!(
            shell.remove_labels([""]).request(),
            Err(HerdrCoreError::Empty(error)) if error.what == "label key"
        ));
        assert!(matches!(
            shell.keys([""]).request(),
            Err(HerdrCoreError::Empty(error)) if error.what == "key name"
        ));
        Ok(())
    }

    #[tokio::test]
    async fn each_request_is_sent_as_its_method() -> TestResult {
        let (client, mut peer) = connected().await?;
        let core = HerdrCore::from_client(client);
        let shell = core.terminals().handle("t2")?;
        let (sent, list) = serve(
            &mut peer,
            core.terminals().list(),
            json!({"terminals": [], "hidden_finished": 0, "machines": []}),
        )
        .await?;
        assert_eq!(sent["method"], "terminal.list");
        assert!(list?.terminals.is_empty());
        for (sent, method) in [
            (
                serve(&mut peer, shell.get(), record("t2")).await?.0,
                "terminal.get",
            ),
            (
                serve(&mut peer, shell.kill(), record("t2")).await?.0,
                "terminal.kill",
            ),
            (
                serve(&mut peer, shell.keep(Keep::On), record("t2"))
                    .await?
                    .0,
                "terminal.keep",
            ),
            (
                serve(&mut peer, shell.set_labels([("a", "1")]), record("t2"))
                    .await?
                    .0,
                "terminal.label",
            ),
        ] {
            assert_eq!(sent["method"], method);
        }
        let (sent, removed) = serve(&mut peer, shell.rm(), json!({"removed": "t2"})).await?;
        assert_eq!(sent["method"], "terminal.rm");
        assert_eq!(removed?.removed, "t2");
        let screen = json!({"text": "hi", "complete": true, "how": "screen"});
        let (sent, read) = serve(&mut peer, shell.read(), screen).await?;
        assert_eq!(sent["method"], "terminal.read");
        assert_eq!(read?.text, "hi");
        let (sent, _) = serve(&mut peer, shell.send("ls"), json!({"turn": 1})).await?;
        assert_eq!(sent["method"], "terminal.send");
        let (sent, _) = serve(&mut peer, shell.keys(["enter"]), json!({"turn": 1})).await?;
        assert_eq!(sent["method"], "terminal.keys");
        let interrupted = json!({"outcome": "stopped", "key": "ctrl+c"});
        let (sent, _) = serve(&mut peer, shell.interrupt(), interrupted).await?;
        assert_eq!(sent["method"], "terminal.interrupt");
        let put = json!({"path": "/tmp/x", "bytes": 2});
        let (sent, _) = serve(&mut peer, shell.put("x", "hi"), put).await?;
        assert_eq!(sent["method"], "terminal.put");
        let waited =
            json!({"terminal": "t2", "ended": true, "reason": "exit", "turn": 0, "children": []});
        let (sent, _) = serve(&mut peer, shell.wait(), waited).await?;
        assert_eq!(sent["method"], "terminal.wait");
        Ok(())
    }
}
