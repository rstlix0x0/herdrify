//! The herdr-app commands that act on tabs.
//!
//! A tab is a group of panes inside a space. Each command here is a value that serializes to the
//! JSON object of arguments herdr-app takes, and names the type its reply decodes into, so a caller
//! that runs one knows what comes back.
//!
//! Responsibilities:
//!
//! - [`TabNew`], [`TabClose`], [`TabRename`], [`TabMove`], [`TabSelect`], [`TabSettle`],
//!   [`TabUnsettle`], [`TabNext`] and [`TabPrev`], one type per command, with the arguments
//!   herdr-app requires as parameters of `new` and the rest as setters
//!
//! Non-responsibilities:
//!
//! - building the command line from a command, which the invocation does
//! - running the command and decoding its reply, which a layer above does
//! - checking that a tab exists, which herdr-app decides
//!
//! The unit tests below check each command's arguments against the JSON object herdr-app takes, and
//! its name and reply type.

use serde::Serialize;

use crate::protocol::{
    Ack, Command, Focus, Rename, SpaceId, TabContent, TabId, TabMade, TabMoveTo, TabName, TabRef,
};

/// Makes a tab, running a program or showing a mod.
///
/// The content is a parameter of `new`, and it writes only the keys of its own form, so a tab that
/// shows a mod never carries a program's arguments. Without a space the tab goes in the shown one,
/// and it is shown at once unless the focus is [`Focus::Background`].
///
/// # Examples
///
/// ```
/// use herdrify_app::files::ModName;
/// use herdrify_app::protocol::{Focus, SpaceId, TabContent, TabName, TabNew};
///
/// let command = TabNew::new(TabContent::Mod { module: ModName::new("files")?, props: None })
///     .name(TabName::new("browse")?)
///     .space(SpaceId::new("sp1")?)
///     .focus(Focus::Background);
/// assert_eq!(
///     serde_json::to_string(&command)?,
///     r#"{"name":"browse","space":"sp1","mod":"files","focus":false}"#,
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct TabNew {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<TabName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    space: Option<SpaceId>,
    #[serde(flatten)]
    content: TabContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    focus: Option<Focus>,
}

impl TabNew {
    /// A command that makes a tab holding `content`.
    #[must_use]
    pub const fn new(content: TabContent) -> Self {
        Self {
            name: None,
            space: None,
            content,
            focus: None,
        }
    }

    /// Names the tab.
    #[must_use]
    pub fn name(mut self, name: TabName) -> Self {
        self.name = Some(name);
        self
    }

    /// Makes the tab in this space.
    #[must_use]
    pub fn space(mut self, space: SpaceId) -> Self {
        self.space = Some(space);
        self
    }

    /// Sets whether the new tab is shown.
    #[must_use]
    pub const fn focus(mut self, focus: Focus) -> Self {
        self.focus = Some(focus);
        self
    }
}

impl Command for TabNew {
    const NAME: &'static str = "tab.new";
    type Output = TabMade;
}

/// Closes a tab and ends its terminals.
///
/// Without an id it closes the shown tab. Closing the last tab of a space closes the space too, and
/// herdr-app asks first about that unless the command is [`confirmed`](Self::confirmed).
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{TabClose, TabId};
///
/// let command = TabClose::new().id(TabId::new("t1")?).confirmed();
/// assert_eq!(serde_json::to_string(&command)?, r#"{"id":"t1","confirmed":true}"#);
/// assert_eq!(serde_json::to_string(&TabClose::new())?, "{}");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct TabClose {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<TabId>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    confirmed: bool,
}

impl TabClose {
    /// A command that sets no argument.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            id: None,
            confirmed: false,
        }
    }

    /// Closes this tab.
    #[must_use]
    pub fn id(mut self, id: TabId) -> Self {
        self.id = Some(id);
        self
    }

    /// Confirms the close, so that closing the last tab of a space is not asked about first.
    #[must_use]
    pub const fn confirmed(mut self) -> Self {
        self.confirmed = true;
        self
    }
}

impl Command for TabClose {
    const NAME: &'static str = "tab.close";
    type Output = Ack;
}

/// Renames a tab, or has it called by its number again.
///
/// herdr-app refuses the command without a name, so the name is a parameter of `new`. Without an id
/// it renames the shown tab.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{Rename, TabId, TabName, TabRename};
///
/// let command = TabRename::new(Rename::To(TabName::new("build")?)).id(TabId::new("t1")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"id":"t1","name":"build"}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct TabRename {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<TabId>,
    name: Rename<TabName>,
}

impl TabRename {
    /// A command that renames the shown tab.
    #[must_use]
    pub const fn new(name: Rename<TabName>) -> Self {
        Self { id: None, name }
    }

    /// Renames this tab.
    #[must_use]
    pub fn id(mut self, id: TabId) -> Self {
        self.id = Some(id);
        self
    }
}

impl Command for TabRename {
    const NAME: &'static str = "tab.rename";
    type Output = Ack;
}

/// Moves a tab to a place in its space, or by a number of places.
///
/// Without an id it moves the shown tab. The destination is one of the two forms of [`TabMoveTo`],
/// so herdr-app, which refuses both at once, never gets both.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{TabId, TabMove, TabMoveTo};
///
/// let command = TabMove::new(TabMoveTo::By(-1)).id(TabId::new("t1")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"id":"t1","by":-1}"#);
/// assert_eq!(serde_json::to_string(&TabMove::new(TabMoveTo::By(-1)))?, r#"{"by":-1}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct TabMove {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<TabId>,
    #[serde(flatten)]
    to: TabMoveTo,
}

impl TabMove {
    /// A command that moves the shown tab to `to`.
    #[must_use]
    pub const fn new(to: TabMoveTo) -> Self {
        Self { id: None, to }
    }

    /// Moves this tab.
    #[must_use]
    pub fn id(mut self, id: TabId) -> Self {
        self.id = Some(id);
        self
    }
}

impl Command for TabMove {
    const NAME: &'static str = "tab.move";
    type Output = Ack;
}

/// Shows a tab, named by its place in the list or by its identifier.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{Position, TabRef, TabSelect};
///
/// let command = TabSelect::new(TabRef::Index(Position::new(2)?));
/// assert_eq!(serde_json::to_string(&command)?, r#"{"index":2}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct TabSelect {
    #[serde(flatten)]
    tab: TabRef,
}

impl TabSelect {
    /// A command that shows `tab`.
    #[must_use]
    pub const fn new(tab: TabRef) -> Self {
        Self { tab }
    }
}

impl Command for TabSelect {
    const NAME: &'static str = "tab.select";
    type Output = Ack;
}

/// Puts a tab away: out of the lists, still running.
///
/// The tab comes back when it is shown or when an agent in it needs the person. Without an id it
/// puts away the shown tab.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{TabId, TabSettle};
///
/// let command = TabSettle::new().id(TabId::new("t1")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"id":"t1"}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct TabSettle {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<TabId>,
}

impl TabSettle {
    /// A command that sets no argument.
    #[must_use]
    pub const fn new() -> Self {
        Self { id: None }
    }

    /// Puts this tab away.
    #[must_use]
    pub fn id(mut self, id: TabId) -> Self {
        self.id = Some(id);
        self
    }
}

impl Command for TabSettle {
    const NAME: &'static str = "tab.settle";
    type Output = Ack;
}

/// Brings a tab put away by [`TabSettle`] back to the lists.
///
/// Without an id it acts on the shown tab.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{TabId, TabUnsettle};
///
/// let command = TabUnsettle::new().id(TabId::new("t1")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"id":"t1"}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct TabUnsettle {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<TabId>,
}

impl TabUnsettle {
    /// A command that sets no argument.
    #[must_use]
    pub const fn new() -> Self {
        Self { id: None }
    }

    /// Brings this tab back.
    #[must_use]
    pub fn id(mut self, id: TabId) -> Self {
        self.id = Some(id);
        self
    }
}

impl Command for TabUnsettle {
    const NAME: &'static str = "tab.unsettle";
    type Output = Ack;
}

/// Shows the next tab.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::TabNext;
///
/// assert_eq!(serde_json::to_string(&TabNext::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::TabNext {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct TabNext {}

impl TabNext {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for TabNext {
    const NAME: &'static str = "tab.next";
    type Output = Ack;
}

/// Shows the previous tab.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::TabPrev;
///
/// assert_eq!(serde_json::to_string(&TabPrev::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::TabPrev {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct TabPrev {}

impl TabPrev {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for TabPrev {
    const NAME: &'static str = "tab.prev";
    type Output = Ack;
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde_json::{Map, Value, json};

    use super::{
        TabClose, TabMove, TabNew, TabNext, TabPrev, TabRename, TabSelect, TabSettle, TabUnsettle,
    };
    use crate::files::ModName;
    use crate::protocol::{
        Ack, Argv, Command, Focus, Folder, Position, Rename, SpaceId, TabContent, TabId, TabMade,
        TabMoveTo, TabName, TabRef, Task,
    };

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// `command` written as the JSON object herdr-app takes, parsed.
    fn arguments<C: Serialize>(command: &C) -> Result<Value, serde_json::Error> {
        serde_json::to_value(command)
    }

    /// Compiles only when `C` replies with an `O`.
    const fn replies_with<C: Command<Output = O>, O>() {}

    #[test]
    fn tab_commands_serialize_full_and_minimal() -> TestResult {
        let folder = Folder::new(std::env::temp_dir())?;
        let shell = TabContent::Shell {
            argv: Some(Argv::new(["htop", "--tree"])?),
            task: Some(Task::new("fix the build")?),
            folder: Some(folder.clone()),
        };
        let full = TabNew::new(shell)
            .name(TabName::new("build")?)
            .space(SpaceId::new("sp1")?)
            .focus(Focus::Background);
        assert_eq!(
            arguments(&full)?,
            json!({
                "name": "build",
                "space": "sp1",
                "argv": ["htop", "--tree"],
                "task": "fix the build",
                "folder": folder.as_str(),
                "focus": false,
            }),
        );
        assert_eq!(
            arguments(&TabNew::new(TabContent::Shell {
                argv: None,
                task: None,
                folder: None,
            }))?,
            json!({}),
        );

        let mut props = Map::new();
        props.insert("path".to_owned(), json!("/srv"));
        let module = TabContent::Mod {
            module: ModName::new("files")?,
            props: Some(props),
        };
        assert_eq!(
            arguments(&TabNew::new(module))?,
            json!({"mod": "files", "props": {"path": "/srv"}}),
        );
        let bare_module = TabContent::Mod {
            module: ModName::new("files")?,
            props: None,
        };
        assert_eq!(
            arguments(&TabNew::new(bare_module))?,
            json!({"mod": "files"}),
        );

        assert_eq!(
            arguments(&TabClose::new().id(TabId::new("t1")?).confirmed())?,
            json!({"id": "t1", "confirmed": true}),
        );
        assert_eq!(arguments(&TabClose::new())?, json!({}));

        let rename = TabRename::new(Rename::To(TabName::new("build")?)).id(TabId::new("t1")?);
        assert_eq!(arguments(&rename)?, json!({"id": "t1", "name": "build"}));
        assert_eq!(
            arguments(&TabRename::new(Rename::Automatic))?,
            json!({"name": ""}),
        );

        assert_eq!(
            arguments(&TabMove::new(TabMoveTo::By(-1)))?,
            json!({"by": -1}),
        );
        assert_eq!(
            arguments(&TabMove::new(TabMoveTo::Index(Position::new(1)?)).id(TabId::new("t1")?))?,
            json!({"id": "t1", "index": 1}),
        );

        assert_eq!(
            arguments(&TabSelect::new(TabRef::Index(Position::new(2)?)))?,
            json!({"index": 2}),
        );
        assert_eq!(
            arguments(&TabSelect::new(TabRef::Id(TabId::new("t1")?)))?,
            json!({"id": "t1"}),
        );

        assert_eq!(
            arguments(&TabSettle::new().id(TabId::new("t1")?))?,
            json!({"id": "t1"}),
        );
        assert_eq!(arguments(&TabSettle::new())?, json!({}));
        assert_eq!(
            arguments(&TabUnsettle::new().id(TabId::new("t1")?))?,
            json!({"id": "t1"}),
        );
        assert_eq!(arguments(&TabUnsettle::new())?, json!({}));

        assert_eq!(arguments(&TabNext::new())?, json!({}));
        assert_eq!(arguments(&TabPrev::new())?, json!({}));
        Ok(())
    }

    #[test]
    fn tab_commands_name_their_wire_command() {
        assert_eq!(TabNew::NAME, "tab.new");
        assert_eq!(TabClose::NAME, "tab.close");
        assert_eq!(TabRename::NAME, "tab.rename");
        assert_eq!(TabMove::NAME, "tab.move");
        assert_eq!(TabSelect::NAME, "tab.select");
        assert_eq!(TabSettle::NAME, "tab.settle");
        assert_eq!(TabUnsettle::NAME, "tab.unsettle");
        assert_eq!(TabNext::NAME, "tab.next");
        assert_eq!(TabPrev::NAME, "tab.prev");

        replies_with::<TabNew, TabMade>();
        replies_with::<TabClose, Ack>();
        replies_with::<TabRename, Ack>();
        replies_with::<TabMove, Ack>();
        replies_with::<TabSelect, Ack>();
        replies_with::<TabSettle, Ack>();
        replies_with::<TabUnsettle, Ack>();
        replies_with::<TabNext, Ack>();
        replies_with::<TabPrev, Ack>();
    }
}
