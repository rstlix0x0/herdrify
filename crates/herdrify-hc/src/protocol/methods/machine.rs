//! The requests that act on machines: running a command on one, and managing the list of them.
//!
//! Where the core refuses a combination, the types make it impossible to write: a `machine.exec`
//! takes an [`ExecCommand`] instead of an `argv` and a `line` that could both be set, and a
//! `machine.add` takes a [`MachineTarget`] because a `via` command needs a name while an ssh
//! target does not.
//!
//! Responsibilities:
//!
//! - the `Params` type of every `machine.*` method, each with its wire name and reply type
//! - [`ExecCommand`] and [`MachineTarget`], which stand in for parameters the core accepts only in
//!   certain combinations
//!
//! Non-responsibilities:
//!
//! - sending the request or reading the reply, which the client does
//! - deciding which machines exist or which commands run, which the core decides

use std::time::Duration;

use serde::{Serialize, Serializer};

use super::argv::Argv;
use super::seconds::Seconds;
use crate::protocol::{
    Environment, ExecResult, Machine, MachineListResult, MachineName, MachineView, Method,
    RemotePath, RemoveResult, SessionName,
};

/// What a `machine.exec` request runs.
///
/// The core refuses a request with neither a program nor a line, or with both, so exactly one is
/// given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecCommand {
    /// A program and its arguments, run as given with no shell.
    Argv(Argv),
    /// A line for the machine's shell.
    Line(String),
}

/// Standard input bytes, sent as base64 text.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Input(Vec<u8>);

impl Serialize for Input {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        crate::protocol::base64::serialize(&self.0, serializer)
    }
}

/// The parameters of `machine.exec`.
///
/// Nothing is sent that was not set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MachineExec {
    #[serde(skip_serializing_if = "Option::is_none")]
    argv: Option<Argv>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cwd: Option<RemotePath>,
    #[serde(skip_serializing_if = "Option::is_none")]
    env: Option<Environment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    input: Option<Input>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    on: Option<MachineName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout: Option<Seconds>,
}

impl MachineExec {
    /// A `machine.exec` request that runs `command`.
    #[must_use]
    pub fn new(command: ExecCommand) -> Self {
        let (argv, line) = match command {
            ExecCommand::Argv(argv) => (Some(argv), None),
            ExecCommand::Line(line) => (None, Some(line)),
        };
        Self {
            argv,
            cwd: None,
            env: None,
            input: None,
            line,
            on: None,
            timeout: None,
        }
    }

    /// The working directory, on the machine that runs it.
    #[must_use]
    pub fn cwd(mut self, cwd: RemotePath) -> Self {
        self.cwd = Some(cwd);
        self
    }

    /// Environment variables set on top of the core's own.
    #[must_use]
    pub fn env(mut self, env: Environment) -> Self {
        self.env = Some(env);
        self
    }

    /// Bytes for the command's standard input. Without them, standard input is closed.
    #[must_use]
    pub fn input(mut self, input: Vec<u8>) -> Self {
        self.input = Some(Input(input));
        self
    }

    /// Runs the command on another machine.
    #[must_use]
    pub fn on(mut self, machine: MachineName) -> Self {
        self.on = Some(machine);
        self
    }

    /// How long the command, and everything it started, may run before it is ended.
    #[must_use]
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(Seconds(timeout));
        self
    }
}

impl Method for MachineExec {
    const NAME: &'static str = "machine.exec";
    type Params = Self;
    type Output = ExecResult;
}

/// How a `machine.add` request reaches the new machine.
///
/// A `via` command needs a name to call the machine by, while an ssh target defaults to its
/// host, so each variant carries what its way of reaching the machine needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MachineTarget {
    /// Reached over ssh.
    Ssh {
        /// How ssh reaches it: a host, `user@host`, `user@host:port` or a `Host` from the ssh
        /// config.
        target: String,
        /// What to call it. Without a name, the core uses the target's host.
        name: Option<MachineName>,
    },
    /// Reached by a command instead of ssh.
    Via {
        /// What to call the machine.
        name: MachineName,
        /// The command, given a shell line to run on the machine as its last argument.
        command: String,
    },
}

/// The parameters of `machine.add`.
///
/// Nothing is sent that was not set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MachineAdd {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<MachineName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    session: Option<SessionName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temporary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    via: Option<String>,
}

impl MachineAdd {
    /// A `machine.add` request that reaches the machine as `target` says.
    #[must_use]
    pub fn new(target: MachineTarget) -> Self {
        let (name, target, via) = match target {
            MachineTarget::Ssh { target, name } => (name, Some(target), None),
            MachineTarget::Via { name, command } => (Some(name), None, Some(command)),
        };
        Self {
            name,
            session: None,
            target,
            temporary: None,
            via,
        }
    }

    /// The `hc` session on that machine. Without one, the machine's default.
    #[must_use]
    pub fn session(mut self, session: SessionName) -> Self {
        self.session = Some(session);
        self
    }

    /// Marks the machine as short-lived, so clients can show it apart from machines that stay.
    #[must_use]
    pub const fn temporary(mut self) -> Self {
        self.temporary = Some(true);
        self
    }
}

impl Method for MachineAdd {
    const NAME: &'static str = "machine.add";
    type Params = Self;
    type Output = Machine;
}

/// The parameters of `machine.list`: none.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct MachineList {}

impl MachineList {
    /// A `machine.list` request.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Method for MachineList {
    const NAME: &'static str = "machine.list";
    type Params = Self;
    type Output = MachineListResult;
}

/// The parameters of `machine.connect`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MachineConnect {
    machine: MachineName,
}

impl MachineConnect {
    /// A `machine.connect` request for `machine`.
    #[must_use]
    pub const fn new(machine: MachineName) -> Self {
        Self { machine }
    }
}

impl Method for MachineConnect {
    const NAME: &'static str = "machine.connect";
    type Params = Self;
    type Output = MachineView;
}

/// The parameters of `machine.rename`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MachineRename {
    machine: MachineName,
    new_name: MachineName,
}

impl MachineRename {
    /// A `machine.rename` request that renames `machine` to `new_name`.
    #[must_use]
    pub const fn new(machine: MachineName, new_name: MachineName) -> Self {
        Self { machine, new_name }
    }
}

impl Method for MachineRename {
    const NAME: &'static str = "machine.rename";
    type Params = Self;
    type Output = Machine;
}

/// The parameters of `machine.rm`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MachineRm {
    machine: MachineName,
}

impl MachineRm {
    /// A `machine.rm` request for `machine`.
    #[must_use]
    pub const fn new(machine: MachineName) -> Self {
        Self { machine }
    }
}

impl Method for MachineRm {
    const NAME: &'static str = "machine.rm";
    type Params = Self;
    type Output = RemoveResult;
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::error::Error;
    use std::time::Duration;

    use serde_json::{Value, json};

    use crate::protocol::{
        Argv, DocGet, DocName, DocSet, DocVersion, Edit, Environment, ExecCommand, FileEdit,
        FileRead, FileWrite, MachineAdd, MachineConnect, MachineExec, MachineList, MachineName,
        MachineRename, MachineRm, MachineTarget, Method, RemotePath, SessionName,
    };

    type TestResult = Result<(), Box<dyn Error>>;

    fn keys(value: &Value) -> BTreeSet<&str> {
        value
            .as_object()
            .map(|object| object.keys().map(String::as_str).collect())
            .unwrap_or_default()
    }

    fn name(text: &str) -> Result<MachineName, Box<dyn Error>> {
        Ok(MachineName::new(text)?)
    }

    fn full_exec(command: ExecCommand) -> Result<Value, Box<dyn Error>> {
        let params = MachineExec::new(command)
            .cwd(RemotePath::new("/work"))
            .env(std::iter::once(("A", "1")).collect::<Environment>())
            .input(b"x\n".to_vec())
            .on(name("m1")?)
            .timeout(Duration::from_secs(60));
        Ok(serde_json::to_value(params)?)
    }

    #[test]
    fn machine_exec_commands() -> TestResult {
        let argv = Argv::new(vec!["true".to_owned()])?;
        assert_eq!(
            serde_json::to_value(MachineExec::new(ExecCommand::Argv(argv)))?,
            json!({"argv": ["true"]})
        );
        assert_eq!(
            serde_json::to_value(MachineExec::new(ExecCommand::Line("echo hi".to_owned())))?,
            json!({"line": "echo hi"})
        );
        assert_eq!(MachineExec::NAME, "machine.exec");
        Ok(())
    }

    #[test]
    fn machine_add_targets() -> TestResult {
        let ssh = |name: Option<MachineName>| MachineTarget::Ssh {
            target: "box".to_owned(),
            name,
        };
        assert_eq!(
            serde_json::to_value(MachineAdd::new(ssh(None)))?,
            json!({"target": "box"})
        );
        assert_eq!(
            serde_json::to_value(MachineAdd::new(ssh(Some(name("m2")?))))?,
            json!({"target": "box", "name": "m2"})
        );
        let via = MachineTarget::Via {
            name: name("m1")?,
            command: "false".to_owned(),
        };
        assert_eq!(
            serde_json::to_value(MachineAdd::new(via))?,
            json!({"name": "m1", "via": "false"})
        );
        assert_eq!(MachineAdd::NAME, "machine.add");
        Ok(())
    }

    #[test]
    fn machine_list_connect_rename_rm() -> TestResult {
        assert_eq!(serde_json::to_value(MachineList::new())?, json!({}));
        assert_eq!(
            serde_json::to_value(MachineConnect::new(name("m1")?))?,
            json!({"machine": "m1"})
        );
        assert_eq!(
            serde_json::to_value(MachineRename::new(name("m1")?, name("m2")?))?,
            json!({"machine": "m1", "new_name": "m2"})
        );
        assert_eq!(
            serde_json::to_value(MachineRm::new(name("m1")?))?,
            json!({"machine": "m1"})
        );
        assert_eq!(MachineList::NAME, "machine.list");
        assert_eq!(MachineConnect::NAME, "machine.connect");
        assert_eq!(MachineRename::NAME, "machine.rename");
        assert_eq!(MachineRm::NAME, "machine.rm");
        Ok(())
    }

    #[test]
    fn machine_file_doc_every_setter() -> TestResult {
        let all = |names: &[&'static str]| names.iter().copied().collect::<BTreeSet<_>>();

        let argv = Argv::new(vec!["true".to_owned()])?;
        let exec = ["argv", "cwd", "env", "input", "line", "on", "timeout"];
        let with_argv = full_exec(ExecCommand::Argv(argv))?;
        let with_line = full_exec(ExecCommand::Line("echo".to_owned()))?;
        let without = |skip: &str| exec.iter().copied().filter(|n| *n != skip).collect();
        assert_eq!(keys(&with_argv), without("line"));
        assert_eq!(keys(&with_line), without("argv"));
        assert_eq!(with_argv["input"], json!("eAo="));
        assert_eq!(with_argv["timeout"], json!(60.0));

        let ssh = MachineAdd::new(MachineTarget::Ssh {
            target: "box".to_owned(),
            name: Some(name("m2")?),
        })
        .session(SessionName::new("s")?)
        .temporary();
        assert_eq!(
            keys(&serde_json::to_value(ssh)?),
            all(&["name", "session", "target", "temporary"])
        );
        let via = MachineAdd::new(MachineTarget::Via {
            name: name("m1")?,
            command: "false".to_owned(),
        })
        .session(SessionName::new("s")?)
        .temporary();
        assert_eq!(
            keys(&serde_json::to_value(via)?),
            all(&["name", "session", "temporary", "via"])
        );

        let path = || RemotePath::new("f.txt");
        let read = FileRead::new(path())
            .cwd(RemotePath::new("/w"))
            .limit(10)
            .offset(2)
            .on(name("m1")?);
        assert_eq!(
            keys(&serde_json::to_value(read)?),
            all(&["cwd", "limit", "offset", "on", "path"])
        );
        let write = FileWrite::new(path(), b"x".to_vec())
            .cwd(RemotePath::new("/w"))
            .on(name("m1")?);
        assert_eq!(
            keys(&serde_json::to_value(write)?),
            all(&["cwd", "data", "on", "path"])
        );
        let edit = FileEdit::new(
            path(),
            vec![Edit {
                old: "a".to_owned(),
                new: "b".to_owned(),
            }],
        )
        .cwd(RemotePath::new("/w"))
        .on(name("m1")?);
        assert_eq!(
            keys(&serde_json::to_value(edit)?),
            all(&["cwd", "edits", "on", "path"])
        );
        let get = DocGet::new(DocName::new("n")?).on(name("m1")?);
        assert_eq!(keys(&serde_json::to_value(get)?), all(&["name", "on"]));
        let set = DocSet::new(DocName::new("n")?, json!(1), DocVersion(0)).on(name("m1")?);
        assert_eq!(
            keys(&serde_json::to_value(set)?),
            all(&["name", "on", "value", "version"])
        );
        Ok(())
    }
}
