//! The requests that start, list, inspect, end and describe terminals.
//!
//! Each type holds exactly the parameters the schema lists for its method and sends only the ones
//! a caller set. Where the core refuses a combination, the types make it impossible to write: a
//! `terminal.run` takes a [`Command`] instead of an `argv` and a `line` that could both be set, and
//! a label is set with a value or removed with a [`LabelChange`], never set to an empty value.
//!
//! Responsibilities:
//!
//! - the `Params` type of `terminal.run`, `terminal.list`, `terminal.get`, `terminal.kill`,
//!   `terminal.rm`, `terminal.keep` and `terminal.label`, each with its wire name and reply type
//! - [`Command`], [`Keep`], [`LabelChange`] and [`LabelChanges`], which stand in for parameters
//!   the core accepts only in certain combinations
//!
//! Non-responsibilities:
//!
//! - the requests that read, send to or wait on a terminal, which `interaction` holds
//! - sending the request or reading the reply, which the client does
//! - deciding which terminals, names or sizes the core accepts, which the core decides

use std::collections::BTreeMap;
use std::time::Duration;

use serde::{Serialize, Serializer};

use crate::protocol::methods::seconds::Seconds;
use crate::protocol::{
    Argv, CallerId, DisplayFacts, Environment, InitialSize, LabelKey, LabelValue, Labels,
    ListResult, MachineName, Method, RemotePath, RemoveResult, RunResult, Terminal, TerminalName,
    TerminalRef,
};

/// What a `terminal.run` request starts in the new terminal.
///
/// The core refuses a request that gives both a program and a shell line, so the two cannot be
/// combined here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// The shell of the machine the terminal runs on, for a request that names no program.
    Shell,
    /// A program and its arguments, run as given with no shell, or a template name and its task.
    Argv(Argv),
    /// A line for the machine's shell. It may be empty.
    Line(String),
}

/// The parameters of `terminal.run`.
///
/// Nothing is sent that was not set. A [`Command::Shell`] sends neither `argv` nor `line`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerminalRun {
    #[serde(skip_serializing_if = "Option::is_none")]
    argv: Option<Argv>,
    #[serde(skip_serializing_if = "Option::is_none")]
    caller: Option<CallerId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cwd: Option<RemotePath>,
    #[serde(skip_serializing_if = "Option::is_none")]
    die_with_parent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    display: Option<DisplayFacts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    env: Option<Environment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    keep: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    labels: Option<Labels>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<TerminalName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    no_notify: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    no_parent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    on: Option<MachineName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    outlive_parent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent: Option<TerminalRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rm: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<InitialSize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout: Option<Seconds>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wait: Option<bool>,
}

impl TerminalRun {
    /// A `terminal.run` request that starts `command`.
    #[must_use]
    pub fn new(command: Command) -> Self {
        let (argv, line) = match command {
            Command::Shell => (None, None),
            Command::Argv(argv) => (Some(argv), None),
            Command::Line(line) => (None, Some(line)),
        };
        Self {
            argv,
            caller: None,
            cwd: None,
            die_with_parent: None,
            display: None,
            env: None,
            keep: None,
            labels: None,
            line,
            name: None,
            no_notify: None,
            no_parent: None,
            on: None,
            outlive_parent: None,
            parent: None,
            rm: None,
            size: None,
            timeout: None,
            wait: None,
        }
    }

    /// Who is calling.
    #[must_use]
    pub fn caller(mut self, caller: CallerId) -> Self {
        self.caller = Some(caller);
        self
    }

    /// The working directory, on the machine that runs it.
    #[must_use]
    pub fn cwd(mut self, cwd: RemotePath) -> Self {
        self.cwd = Some(cwd);
        self
    }

    /// Sends `die_with_parent`, which the core ignores: a terminal ends with its parent unless
    /// [`outlive_parent`](Self::outlive_parent) is set.
    #[must_use]
    pub const fn die_with_parent(mut self) -> Self {
        self.die_with_parent = Some(true);
        self
    }

    /// What the viewer's terminal looks like.
    #[must_use]
    pub fn display(mut self, display: DisplayFacts) -> Self {
        self.display = Some(display);
        self
    }

    /// Environment variables set on top of the core's own.
    #[must_use]
    pub fn env(mut self, env: Environment) -> Self {
        self.env = Some(env);
        self
    }

    /// Replaces a lost program with a shell in place instead of ending the terminal.
    #[must_use]
    pub const fn keep(mut self) -> Self {
        self.keep = Some(true);
        self
    }

    /// Labels to set on the new terminal.
    #[must_use]
    pub fn labels(mut self, labels: Labels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The terminal's name, unique among live terminals.
    #[must_use]
    pub fn name(mut self, name: TerminalName) -> Self {
        self.name = Some(name);
        self
    }

    /// Does not type notes about the terminal into its parent.
    #[must_use]
    pub const fn no_notify(mut self) -> Self {
        self.no_notify = Some(true);
        self
    }

    /// Gives the terminal no parent, even when started from a terminal.
    #[must_use]
    pub const fn no_parent(mut self) -> Self {
        self.no_parent = Some(true);
        self
    }

    /// Runs the terminal on another machine.
    #[must_use]
    pub fn on(mut self, machine: MachineName) -> Self {
        self.on = Some(machine);
        self
    }

    /// Keeps the terminal running when its parent's program ends.
    #[must_use]
    pub const fn outlive_parent(mut self) -> Self {
        self.outlive_parent = Some(true);
        self
    }

    /// The parent terminal.
    #[must_use]
    pub fn parent(mut self, parent: TerminalRef) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Removes the terminal right after it exits.
    #[must_use]
    pub const fn rm(mut self) -> Self {
        self.rm = Some(true);
        self
    }

    /// The size the terminal starts with.
    #[must_use]
    pub const fn size(mut self, size: InitialSize) -> Self {
        self.size = Some(size);
        self
    }

    /// How long a wait lasts before it checks in, leaving the terminal running.
    #[must_use]
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(Seconds(timeout));
        self
    }

    /// Waits until the terminal is not working, as `terminal.wait` does.
    #[must_use]
    pub const fn wait(mut self) -> Self {
        self.wait = Some(true);
        self
    }
}

impl Method for TerminalRun {
    const NAME: &'static str = "terminal.run";
    type Params = Self;
    type Output = RunResult;
}

/// The parameters of `terminal.list`.
///
/// Nothing is sent that was not set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct TerminalList {
    #[serde(skip_serializing_if = "Option::is_none")]
    all: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    labels: Option<Labels>,
}

impl TerminalList {
    /// A `terminal.list` request for the live terminals.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            all: None,
            labels: None,
        }
    }

    /// Includes finished terminals.
    #[must_use]
    pub const fn all(mut self) -> Self {
        self.all = Some(true);
        self
    }

    /// Only terminals that carry these labels.
    #[must_use]
    pub fn labels(mut self, labels: Labels) -> Self {
        self.labels = Some(labels);
        self
    }
}

impl Method for TerminalList {
    const NAME: &'static str = "terminal.list";
    type Params = Self;
    type Output = ListResult;
}

/// The parameters of `terminal.get`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerminalGet {
    terminal: TerminalRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    caller: Option<CallerId>,
}

impl TerminalGet {
    /// A `terminal.get` request for `terminal`.
    #[must_use]
    pub const fn new(terminal: TerminalRef) -> Self {
        Self {
            terminal,
            caller: None,
        }
    }

    /// Who is calling.
    #[must_use]
    pub fn caller(mut self, caller: CallerId) -> Self {
        self.caller = Some(caller);
        self
    }
}

impl Method for TerminalGet {
    const NAME: &'static str = "terminal.get";
    type Params = Self;
    type Output = Terminal;
}

/// The parameters of `terminal.kill`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerminalKill {
    terminal: TerminalRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    caller: Option<CallerId>,
}

impl TerminalKill {
    /// A `terminal.kill` request for `terminal`.
    #[must_use]
    pub const fn new(terminal: TerminalRef) -> Self {
        Self {
            terminal,
            caller: None,
        }
    }

    /// Who is calling.
    #[must_use]
    pub fn caller(mut self, caller: CallerId) -> Self {
        self.caller = Some(caller);
        self
    }
}

impl Method for TerminalKill {
    const NAME: &'static str = "terminal.kill";
    type Params = Self;
    type Output = Terminal;
}

/// The parameters of `terminal.rm`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerminalRm {
    terminal: TerminalRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    caller: Option<CallerId>,
}

impl TerminalRm {
    /// A `terminal.rm` request for `terminal`.
    #[must_use]
    pub const fn new(terminal: TerminalRef) -> Self {
        Self {
            terminal,
            caller: None,
        }
    }

    /// Who is calling.
    #[must_use]
    pub fn caller(mut self, caller: CallerId) -> Self {
        self.caller = Some(caller);
        self
    }
}

impl Method for TerminalRm {
    const NAME: &'static str = "terminal.rm";
    type Params = Self;
    type Output = RemoveResult;
}

/// Whether a `terminal.keep` request keeps the terminal after it is lost.
///
/// It serializes as the boolean the core requires, so a caller never passes a bare `bool`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keep {
    /// Keep the terminal.
    On,
    /// Stop keeping the terminal.
    Off,
}

impl Serialize for Keep {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bool(matches!(self, Self::On))
    }
}

/// The parameters of `terminal.keep`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerminalKeep {
    terminal: TerminalRef,
    keep: Keep,
}

impl TerminalKeep {
    /// A `terminal.keep` request that applies `keep` to `terminal`.
    #[must_use]
    pub const fn new(terminal: TerminalRef, keep: Keep) -> Self {
        Self { terminal, keep }
    }
}

impl Method for TerminalKeep {
    const NAME: &'static str = "terminal.keep";
    type Params = Self;
    type Output = Terminal;
}

/// One change to a label: set it to a value, or remove it.
///
/// The core removes a label when it is set to the empty string, and a [`LabelValue`] is never
/// empty, so removal is its own variant. `Remove` serializes as the empty string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelChange {
    /// Sets the label to this value.
    Set(LabelValue),
    /// Removes the label.
    Remove,
}

impl Serialize for LabelChange {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Set(value) => serializer.serialize_str(value.as_str()),
            Self::Remove => serializer.serialize_str(""),
        }
    }
}

/// The label changes of one `terminal.label` request, in key order.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct LabelChanges(BTreeMap<LabelKey, LabelChange>);

impl LabelChanges {
    /// A set of changes with none in it.
    #[must_use]
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Records `change` for the label `key`, replacing an earlier change for the same key.
    pub fn insert(&mut self, key: LabelKey, change: LabelChange) {
        self.0.insert(key, change);
    }

    /// How many labels are changed.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether no label is changed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl FromIterator<(LabelKey, LabelChange)> for LabelChanges {
    fn from_iter<I: IntoIterator<Item = (LabelKey, LabelChange)>>(entries: I) -> Self {
        Self(entries.into_iter().collect())
    }
}

/// The parameters of `terminal.label`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerminalLabel {
    terminal: TerminalRef,
    labels: LabelChanges,
}

impl TerminalLabel {
    /// A `terminal.label` request that applies `labels` to `terminal`.
    #[must_use]
    pub const fn new(terminal: TerminalRef, labels: LabelChanges) -> Self {
        Self { terminal, labels }
    }
}

impl Method for TerminalLabel {
    const NAME: &'static str = "terminal.label";
    type Params = Self;
    type Output = Terminal;
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::error::Error;
    use std::time::Duration;

    use serde_json::{Value, json};

    use crate::protocol::{
        Argv, CallerId, Command, DisplayFacts, Environment, InitialSize, Keep, LabelChange,
        LabelChanges, LabelKey, LabelValue, Labels, MachineName, Method, TerminalGet, TerminalKeep,
        TerminalKill, TerminalLabel, TerminalList, TerminalName, TerminalRef, TerminalRm,
        TerminalRun,
    };

    type TestResult = Result<(), Box<dyn Error>>;

    fn keys(value: &Value) -> BTreeSet<&str> {
        value
            .as_object()
            .map(|object| object.keys().map(String::as_str).collect())
            .unwrap_or_default()
    }

    fn t7() -> Result<TerminalRef, Box<dyn Error>> {
        Ok(TerminalRef::new("t7")?)
    }

    /// The schema's 19 `terminal.run` params.
    const RUN_PARAMS: [&str; 19] = [
        "argv",
        "caller",
        "cwd",
        "die_with_parent",
        "display",
        "env",
        "keep",
        "labels",
        "line",
        "name",
        "no_notify",
        "no_parent",
        "on",
        "outlive_parent",
        "parent",
        "rm",
        "size",
        "timeout",
        "wait",
    ];

    fn full_run(command: Command) -> Result<TerminalRun, Box<dyn Error>> {
        let size = InitialSize {
            cols: 80.try_into()?,
            rows: 24.try_into()?,
        };
        Ok(TerminalRun::new(command)
            .caller(CallerId::new("c"))
            .cwd("/work".to_owned().into())
            .die_with_parent()
            .display(DisplayFacts {
                bg: Some("#000000".to_owned()),
                cell: Some([8, 16]),
                cursor: None,
                fg: None,
            })
            .env(std::iter::once(("A", "1")).collect::<Environment>())
            .keep()
            .labels(
                std::iter::once((LabelKey::new("k")?, LabelValue::new("v")?)).collect::<Labels>(),
            )
            .name(TerminalName::new("job")?)
            .no_notify()
            .no_parent()
            .on(MachineName::new("m1")?)
            .outlive_parent()
            .parent(t7()?)
            .rm()
            .size(size)
            .timeout(Duration::from_millis(1500))
            .wait())
    }

    #[test]
    fn terminal_run_shell_sends_no_command() -> TestResult {
        assert_eq!(
            serde_json::to_value(TerminalRun::new(Command::Shell))?,
            json!({})
        );
        assert_eq!(TerminalRun::NAME, "terminal.run");
        Ok(())
    }

    #[test]
    fn terminal_run_argv_and_line() -> TestResult {
        let argv = Argv::new(vec!["sleep".to_owned(), "5".to_owned()])?;
        assert_eq!(
            serde_json::to_value(TerminalRun::new(Command::Argv(argv)))?,
            json!({"argv": ["sleep", "5"]})
        );
        assert_eq!(
            serde_json::to_value(TerminalRun::new(Command::Line(String::new())))?,
            json!({"line": ""})
        );
        Ok(())
    }

    #[test]
    fn terminal_run_every_setter() -> TestResult {
        let argv = Argv::new(vec!["true".to_owned()])?;
        let with_argv = serde_json::to_value(full_run(Command::Argv(argv))?)?;
        let with_line = serde_json::to_value(full_run(Command::Line("echo".to_owned()))?)?;
        let with_shell = serde_json::to_value(full_run(Command::Shell)?)?;
        let names = |skip: &[&str]| -> BTreeSet<&str> {
            RUN_PARAMS
                .iter()
                .copied()
                .filter(|name| !skip.contains(name))
                .collect()
        };
        assert_eq!(keys(&with_argv), names(&["line"]));
        assert_eq!(keys(&with_line), names(&["argv"]));
        assert_eq!(keys(&with_shell), names(&["argv", "line"]));
        assert_eq!(with_argv["timeout"], json!(1.5));
        assert_eq!(with_argv["wait"], json!(true));
        assert_eq!(with_argv["size"], json!({"cols": 80, "rows": 24}));
        assert_eq!(with_argv["on"], json!("m1"));
        Ok(())
    }

    #[test]
    fn terminal_keep_serializes_the_boolean() -> TestResult {
        assert_eq!(
            serde_json::to_value(TerminalKeep::new(t7()?, Keep::On))?,
            json!({"terminal": "t7", "keep": true})
        );
        assert_eq!(
            serde_json::to_value(TerminalKeep::new(t7()?, Keep::Off))?,
            json!({"terminal": "t7", "keep": false})
        );
        assert_eq!(TerminalKeep::NAME, "terminal.keep");
        Ok(())
    }

    #[test]
    fn label_remove_serializes_as_empty_string() -> TestResult {
        let mut changes = LabelChanges::new();
        changes.insert(LabelKey::new("a")?, LabelChange::Set(LabelValue::new("x")?));
        changes.insert(LabelKey::new("b")?, LabelChange::Remove);
        assert_eq!(
            serde_json::to_value(TerminalLabel::new(t7()?, changes))?,
            json!({"terminal": "t7", "labels": {"a": "x", "b": ""}})
        );
        assert_eq!(TerminalLabel::NAME, "terminal.label");
        Ok(())
    }

    #[test]
    fn terminal_lifecycle_required_only() -> TestResult {
        let only_terminal = json!({"terminal": "t7"});
        assert_eq!(
            serde_json::to_value(TerminalGet::new(t7()?))?,
            only_terminal
        );
        assert_eq!(
            serde_json::to_value(TerminalKill::new(t7()?))?,
            only_terminal
        );
        assert_eq!(serde_json::to_value(TerminalRm::new(t7()?))?, only_terminal);
        assert_eq!(TerminalGet::NAME, "terminal.get");
        assert_eq!(TerminalKill::NAME, "terminal.kill");
        assert_eq!(TerminalRm::NAME, "terminal.rm");
        Ok(())
    }

    #[test]
    fn terminal_lifecycle_every_setter() -> TestResult {
        let caller = || CallerId::new("c");
        let with_caller = json!({"terminal": "t7", "caller": "c"});
        assert_eq!(
            serde_json::to_value(TerminalGet::new(t7()?).caller(caller()))?,
            with_caller
        );
        assert_eq!(
            serde_json::to_value(TerminalKill::new(t7()?).caller(caller()))?,
            with_caller
        );
        assert_eq!(
            serde_json::to_value(TerminalRm::new(t7()?).caller(caller()))?,
            with_caller
        );
        let labels: Labels =
            std::iter::once((LabelKey::new("k")?, LabelValue::new("v")?)).collect();
        assert_eq!(
            serde_json::to_value(TerminalList::new().all().labels(labels))?,
            json!({"all": true, "labels": {"k": "v"}})
        );
        assert_eq!(serde_json::to_value(TerminalList::new())?, json!({}));
        assert_eq!(TerminalList::NAME, "terminal.list");
        Ok(())
    }
}
