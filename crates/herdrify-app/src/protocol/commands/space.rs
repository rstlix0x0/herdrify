//! The herdr-app commands that act on spaces.
//!
//! A space is a top-level group of tabs in the herdr-app window. Each command here is a value that
//! serializes to the JSON object of arguments herdr-app takes, and names the type its reply decodes
//! into, so a caller that runs one knows what comes back.
//!
//! Responsibilities:
//!
//! - [`SpaceNew`], [`SpaceClose`], [`SpaceRename`], [`SpaceMove`], [`SpaceSelect`], [`SpaceNext`]
//!   and [`SpacePrev`], one type per command, with the arguments herdr-app requires as parameters
//!   of `new` and the rest as setters
//!
//! Non-responsibilities:
//!
//! - building the command line from a command, which the invocation does
//! - running the command and decoding its reply, which a layer above does
//! - checking that a space exists, which herdr-app decides
//!
//! The unit tests below check each command's arguments against the JSON object herdr-app takes, and
//! its name and reply type.

use serde::Serialize;

use crate::protocol::{
    Ack, Argv, Command, Created, Focus, Folder, Machine, Position, Rename, SpaceId, SpaceName,
    SpaceRef, TabName, Task,
};

/// Makes a space.
///
/// Every argument is optional. Without a name the space is called after where its first terminal
/// is, and without a folder its terminals start where the focused terminal is. The space is shown
/// at once unless the focus is [`Focus::Background`].
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{Argv, Focus, Folder, Machine, SpaceName, SpaceNew, TabName, Task};
///
/// let command = SpaceNew::new()
///     .name(SpaceName::new("work")?)
///     .folder(Folder::new(std::env::temp_dir())?)
///     .tab(TabName::new("build")?)
///     .argv(Argv::new(["htop"])?)
///     .task(Task::new("fix the build")?)
///     .machine(Machine::ThisComputer)
///     .focus(Focus::Background);
/// let arguments = serde_json::to_value(&command)?;
/// assert_eq!(arguments["name"], "work");
/// assert_eq!(arguments["machine"], "");
/// assert_eq!(arguments["focus"], false);
/// assert_eq!(serde_json::to_string(&SpaceNew::new())?, "{}");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct SpaceNew {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<SpaceName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    folder: Option<Folder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tab: Option<TabName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    argv: Option<Argv>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task: Option<Task>,
    #[serde(skip_serializing_if = "Option::is_none")]
    machine: Option<Machine>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focus: Option<Focus>,
}

impl SpaceNew {
    /// A command that sets no argument.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            name: None,
            folder: None,
            tab: None,
            argv: None,
            task: None,
            machine: None,
            focus: None,
        }
    }

    /// Names the space.
    #[must_use]
    pub fn name(mut self, name: SpaceName) -> Self {
        self.name = Some(name);
        self
    }

    /// Sets the folder the space's terminals start in.
    #[must_use]
    pub fn folder(mut self, folder: Folder) -> Self {
        self.folder = Some(folder);
        self
    }

    /// Names the space's first tab.
    #[must_use]
    pub fn tab(mut self, tab: TabName) -> Self {
        self.tab = Some(tab);
        self
    }

    /// Runs this program, an agent, in the first tab.
    #[must_use]
    pub fn argv(mut self, argv: Argv) -> Self {
        self.argv = Some(argv);
        self
    }

    /// Gives the agent its first message.
    #[must_use]
    pub fn task(mut self, task: Task) -> Self {
        self.task = Some(task);
        self
    }

    /// Puts the space on this machine.
    #[must_use]
    pub fn machine(mut self, machine: Machine) -> Self {
        self.machine = Some(machine);
        self
    }

    /// Sets whether the new space is shown.
    #[must_use]
    pub const fn focus(mut self, focus: Focus) -> Self {
        self.focus = Some(focus);
        self
    }
}

impl Command for SpaceNew {
    const NAME: &'static str = "space.new";
    type Output = Created;
}

/// Closes a space and ends its terminals.
///
/// Without an id it closes the shown space. herdr-app refuses a command that is not
/// [`confirmed`](Self::confirmed), saying `space.close ends its terminals: pass confirmed: true`.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{SpaceClose, SpaceId};
///
/// let command = SpaceClose::new().id(SpaceId::new("sp1")?).confirmed();
/// assert_eq!(serde_json::to_string(&command)?, r#"{"id":"sp1","confirmed":true}"#);
/// assert_eq!(serde_json::to_string(&SpaceClose::new())?, "{}");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct SpaceClose {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<SpaceId>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    confirmed: bool,
}

impl SpaceClose {
    /// A command that sets no argument.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            id: None,
            confirmed: false,
        }
    }

    /// Closes this space.
    #[must_use]
    pub fn id(mut self, id: SpaceId) -> Self {
        self.id = Some(id);
        self
    }

    /// Confirms the close, without which herdr-app refuses it.
    #[must_use]
    pub const fn confirmed(mut self) -> Self {
        self.confirmed = true;
        self
    }
}

impl Command for SpaceClose {
    const NAME: &'static str = "space.close";
    type Output = Ack;
}

/// Renames a space, or gives it back the name herdr-app derives.
///
/// herdr-app refuses the command without a name, so the name is a parameter of `new`. Without an
/// id it renames the shown space.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{Rename, SpaceId, SpaceName, SpaceRename};
///
/// let command = SpaceRename::new(Rename::To(SpaceName::new("work")?)).id(SpaceId::new("sp1")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"id":"sp1","name":"work"}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SpaceRename {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<SpaceId>,
    name: Rename<SpaceName>,
}

impl SpaceRename {
    /// A command that renames the shown space.
    #[must_use]
    pub const fn new(name: Rename<SpaceName>) -> Self {
        Self { id: None, name }
    }

    /// Renames this space.
    #[must_use]
    pub fn id(mut self, id: SpaceId) -> Self {
        self.id = Some(id);
        self
    }
}

impl Command for SpaceRename {
    const NAME: &'static str = "space.rename";
    type Output = Ack;
}

/// Moves a space to a place in the list.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{Position, SpaceId, SpaceMove};
///
/// let command = SpaceMove::new(SpaceId::new("sp1")?, Position::new(1)?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"id":"sp1","index":1}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SpaceMove {
    id: SpaceId,
    index: Position,
}

impl SpaceMove {
    /// A command that moves the space `id` to the place `index`, counted from 1.
    #[must_use]
    pub const fn new(id: SpaceId, index: Position) -> Self {
        Self { id, index }
    }
}

impl Command for SpaceMove {
    const NAME: &'static str = "space.move";
    type Output = Ack;
}

/// Shows a space, named by its place in the list or by its identifier.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{Position, SpaceRef, SpaceSelect};
///
/// let command = SpaceSelect::new(SpaceRef::Index(Position::new(2)?));
/// assert_eq!(serde_json::to_string(&command)?, r#"{"index":2}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SpaceSelect {
    #[serde(flatten)]
    space: SpaceRef,
}

impl SpaceSelect {
    /// A command that shows `space`.
    #[must_use]
    pub const fn new(space: SpaceRef) -> Self {
        Self { space }
    }
}

impl Command for SpaceSelect {
    const NAME: &'static str = "space.select";
    type Output = Ack;
}

/// Shows the next space.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::SpaceNext;
///
/// assert_eq!(serde_json::to_string(&SpaceNext::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::SpaceNext {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct SpaceNext {}

impl SpaceNext {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for SpaceNext {
    const NAME: &'static str = "space.next";
    type Output = Ack;
}

/// Shows the previous space.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::SpacePrev;
///
/// assert_eq!(serde_json::to_string(&SpacePrev::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::SpacePrev {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct SpacePrev {}

impl SpacePrev {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for SpacePrev {
    const NAME: &'static str = "space.prev";
    type Output = Ack;
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde_json::{Value, json};

    use super::{SpaceClose, SpaceMove, SpaceNew, SpaceNext, SpacePrev, SpaceRename, SpaceSelect};
    use crate::protocol::{
        Ack, Argv, Command, Created, Focus, Folder, Machine, MachineName, Position, Rename,
        SpaceId, SpaceName, SpaceRef, TabName, Task,
    };

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// `command` written as the JSON object herdr-app takes, parsed.
    fn arguments<C: Serialize>(command: &C) -> Result<Value, serde_json::Error> {
        serde_json::to_value(command)
    }

    /// Compiles only when `C` replies with an `O`.
    const fn replies_with<C: Command<Output = O>, O>() {}

    #[test]
    fn space_commands_serialize_full_and_minimal() -> TestResult {
        let folder = Folder::new(std::env::temp_dir())?;
        let full = SpaceNew::new()
            .name(SpaceName::new("work")?)
            .folder(folder.clone())
            .tab(TabName::new("build")?)
            .argv(Argv::new(["htop", "--tree"])?)
            .task(Task::new("fix the build")?)
            .machine(Machine::Named(MachineName::new("vps")?))
            .focus(Focus::Background);
        assert_eq!(
            arguments(&full)?,
            json!({
                "name": "work",
                "folder": folder.as_str(),
                "tab": "build",
                "argv": ["htop", "--tree"],
                "task": "fix the build",
                "machine": "vps",
                "focus": false,
            }),
        );
        assert_eq!(arguments(&SpaceNew::new())?, json!({}));

        assert_eq!(
            arguments(&SpaceClose::new().id(SpaceId::new("sp1")?).confirmed())?,
            json!({"id": "sp1", "confirmed": true}),
        );
        assert_eq!(arguments(&SpaceClose::new())?, json!({}));

        let rename = SpaceRename::new(Rename::To(SpaceName::new("work")?)).id(SpaceId::new("sp1")?);
        assert_eq!(arguments(&rename)?, json!({"id": "sp1", "name": "work"}));
        assert_eq!(
            arguments(&SpaceRename::new(Rename::Automatic))?,
            json!({"name": ""}),
        );

        assert_eq!(
            arguments(&SpaceMove::new(SpaceId::new("sp1")?, Position::new(3)?))?,
            json!({"id": "sp1", "index": 3}),
        );

        assert_eq!(
            arguments(&SpaceSelect::new(SpaceRef::Index(Position::new(2)?)))?,
            json!({"index": 2}),
        );
        assert_eq!(
            arguments(&SpaceSelect::new(SpaceRef::Id(SpaceId::new("sp1")?)))?,
            json!({"id": "sp1"}),
        );

        assert_eq!(arguments(&SpaceNext::new())?, json!({}));
        assert_eq!(arguments(&SpacePrev::new())?, json!({}));
        Ok(())
    }

    #[test]
    fn space_commands_name_their_wire_command() {
        assert_eq!(SpaceNew::NAME, "space.new");
        assert_eq!(SpaceClose::NAME, "space.close");
        assert_eq!(SpaceRename::NAME, "space.rename");
        assert_eq!(SpaceMove::NAME, "space.move");
        assert_eq!(SpaceSelect::NAME, "space.select");
        assert_eq!(SpaceNext::NAME, "space.next");
        assert_eq!(SpacePrev::NAME, "space.prev");

        replies_with::<SpaceNew, Created>();
        replies_with::<SpaceClose, Ack>();
        replies_with::<SpaceRename, Ack>();
        replies_with::<SpaceMove, Ack>();
        replies_with::<SpaceSelect, Ack>();
        replies_with::<SpaceNext, Ack>();
        replies_with::<SpacePrev, Ack>();
    }
}
