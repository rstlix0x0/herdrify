//! The herdr-app commands that act on panes.
//!
//! A pane is one terminal or mod inside a tab. Each command here is a value that serializes to the
//! JSON object of arguments herdr-app takes, and names the type its reply decodes into, so a caller
//! that runs one knows what comes back.
//!
//! Responsibilities:
//!
//! - [`PaneSplit`], [`PaneClose`], [`PaneRename`], [`PaneResize`], [`PaneZoom`], [`PaneFocus`],
//!   [`PaneGo`] and [`PaneSwap`], one type per command, with the arguments herdr-app requires as
//!   parameters of `new` and the rest as setters
//!
//! Non-responsibilities:
//!
//! - building the command line from a command, which the invocation does
//! - running the command and decoding its reply, which a layer above does
//! - checking that a pane exists or is on screen, which herdr-app decides
//!
//! The unit tests below check each command's arguments against the JSON object herdr-app takes, and
//! its name and reply type.

use serde::Serialize;

use crate::protocol::{
    Ack, Command, Direction, PaneContent, PaneId, PaneName, PaneTarget, Px, RawReply, Rename,
    Split, SplitDirection, SwapWith,
};

/// Splits a pane with a new terminal or a mod beside it.
///
/// Without a pane it splits the focused one. The content writes only the keys of its own form, so a
/// split that shows a mod never carries a program's arguments.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{Argv, PaneContent, PaneId, PaneSplit, SplitDirection};
///
/// let content = PaneContent::Shell { argv: Some(Argv::new(["sh"])?) };
/// let command = PaneSplit::new(SplitDirection::Right, content).pane(PaneId::new("p1")?);
/// assert_eq!(
///     serde_json::to_string(&command)?,
///     r#"{"direction":"right","pane":"p1","argv":["sh"]}"#,
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct PaneSplit {
    direction: SplitDirection,
    #[serde(skip_serializing_if = "Option::is_none")]
    pane: Option<PaneId>,
    #[serde(flatten)]
    content: PaneContent,
}

impl PaneSplit {
    /// A command that splits the focused pane toward `direction` and fills the new half with
    /// `content`.
    #[must_use]
    pub const fn new(direction: SplitDirection, content: PaneContent) -> Self {
        Self {
            direction,
            pane: None,
            content,
        }
    }

    /// Splits this pane.
    #[must_use]
    pub fn pane(mut self, pane: PaneId) -> Self {
        self.pane = Some(pane);
        self
    }

    /// What the new half holds, for the entrypoint to fill in the options of its form.
    pub(crate) const fn content_mut(&mut self) -> &mut PaneContent {
        &mut self.content
    }
}

impl Command for PaneSplit {
    const NAME: &'static str = "pane.split";
    type Output = Split;
}

/// Closes a pane and ends its terminal.
///
/// Without a pane it closes the focused one.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{PaneClose, PaneId};
///
/// let command = PaneClose::new().pane(PaneId::new("p1")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"pane":"p1"}"#);
/// assert_eq!(serde_json::to_string(&PaneClose::new())?, "{}");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct PaneClose {
    #[serde(skip_serializing_if = "Option::is_none")]
    pane: Option<PaneId>,
}

impl PaneClose {
    /// A command that sets no argument.
    #[must_use]
    pub const fn new() -> Self {
        Self { pane: None }
    }

    /// Closes this pane.
    #[must_use]
    pub fn pane(mut self, pane: PaneId) -> Self {
        self.pane = Some(pane);
        self
    }
}

impl Command for PaneClose {
    const NAME: &'static str = "pane.close";
    type Output = Ack;
}

/// Names a pane, or leaves it unnamed again.
///
/// herdr-app refuses the command without a name, so the name is a parameter of `new`. Without a
/// pane it renames the focused one.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{PaneId, PaneName, PaneRename, Rename};
///
/// let command = PaneRename::new(Rename::To(PaneName::new("logs")?)).pane(PaneId::new("p1")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"pane":"p1","name":"logs"}"#);
///
/// let unnamed = PaneRename::new(Rename::Automatic);
/// assert_eq!(serde_json::to_string(&unnamed)?, r#"{"name":""}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct PaneRename {
    #[serde(skip_serializing_if = "Option::is_none")]
    pane: Option<PaneId>,
    name: Rename<PaneName>,
}

impl PaneRename {
    /// A command that renames the focused pane.
    #[must_use]
    pub const fn new(name: Rename<PaneName>) -> Self {
        Self { pane: None, name }
    }

    /// Renames this pane.
    #[must_use]
    pub fn pane(mut self, pane: PaneId) -> Self {
        self.pane = Some(pane);
        self
    }
}

impl Command for PaneRename {
    const NAME: &'static str = "pane.rename";
    type Output = Ack;
}

/// Moves the border of a pane, as dragging its divider does.
///
/// Without a pane it moves the border of the focused one, and without a size it moves it by
/// herdr-app's default. No success of this command was recorded on herdr-app 0.1.17: it answered
/// that the pane was not on screen even for a pane of a visible tab, so its reply is a
/// [`RawReply`] rather than an acknowledgement.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{Direction, PaneId, PaneResize, Px};
///
/// let command = PaneResize::new(Direction::Left).pane(PaneId::new("p1")?).px(Px::new(30.0)?);
/// assert_eq!(
///     serde_json::to_string(&command)?,
///     r#"{"direction":"left","pane":"p1","px":30.0}"#,
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct PaneResize {
    direction: Direction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pane: Option<PaneId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    px: Option<Px>,
}

impl PaneResize {
    /// A command that moves the border toward `direction`.
    #[must_use]
    pub const fn new(direction: Direction) -> Self {
        Self {
            direction,
            pane: None,
            px: None,
        }
    }

    /// Moves the border of this pane.
    #[must_use]
    pub fn pane(mut self, pane: PaneId) -> Self {
        self.pane = Some(pane);
        self
    }

    /// Moves the border by this many pixels.
    #[must_use]
    pub const fn px(mut self, px: Px) -> Self {
        self.px = Some(px);
        self
    }
}

impl Command for PaneResize {
    const NAME: &'static str = "pane.resize";
    type Output = RawReply;
}

/// Shows only one pane of its tab, or all of them again.
///
/// Without a pane it acts on the focused one.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{PaneId, PaneZoom};
///
/// let command = PaneZoom::new().pane(PaneId::new("p1")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"pane":"p1"}"#);
/// assert_eq!(serde_json::to_string(&PaneZoom::new())?, "{}");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct PaneZoom {
    #[serde(skip_serializing_if = "Option::is_none")]
    pane: Option<PaneId>,
}

impl PaneZoom {
    /// A command that sets no argument.
    #[must_use]
    pub const fn new() -> Self {
        Self { pane: None }
    }

    /// Zooms this pane.
    #[must_use]
    pub fn pane(mut self, pane: PaneId) -> Self {
        self.pane = Some(pane);
        self
    }
}

impl Command for PaneZoom {
    const NAME: &'static str = "pane.zoom";
    type Output = Ack;
}

/// Focuses a pane, or the pane that shows a terminal.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{PaneFocus, PaneId, PaneTarget};
///
/// let command = PaneFocus::new(PaneTarget::Pane(PaneId::new("p1")?));
/// assert_eq!(serde_json::to_string(&command)?, r#"{"pane":"p1"}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct PaneFocus {
    #[serde(flatten)]
    target: PaneTarget,
}

impl PaneFocus {
    /// A command that focuses `target`.
    #[must_use]
    pub const fn new(target: PaneTarget) -> Self {
        Self { target }
    }
}

impl Command for PaneFocus {
    const NAME: &'static str = "pane.focus";
    type Output = Ack;
}

/// Focuses the pane or focusable mod in a direction from the focused pane.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{Direction, PaneGo};
///
/// let command = PaneGo::new(Direction::Up);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"direction":"up"}"#);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct PaneGo {
    direction: Direction,
}

impl PaneGo {
    /// A command that focuses what lies toward `direction`.
    #[must_use]
    pub const fn new(direction: Direction) -> Self {
        Self { direction }
    }
}

impl Command for PaneGo {
    const NAME: &'static str = "pane.go";
    type Output = Ack;
}

/// Swaps a pane with its neighbor in a direction, or with another pane of its tab.
///
/// Without a pane it swaps the focused one. The other side is one of the two forms of
/// [`SwapWith`], so herdr-app, which refuses both at once, never gets both.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{Direction, PaneId, PaneSwap, SwapWith};
///
/// let command = PaneSwap::new(SwapWith::Direction(Direction::Right)).pane(PaneId::new("p1")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"pane":"p1","direction":"right"}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct PaneSwap {
    #[serde(skip_serializing_if = "Option::is_none")]
    pane: Option<PaneId>,
    #[serde(flatten)]
    with: SwapWith,
}

impl PaneSwap {
    /// A command that swaps the focused pane with `with`.
    #[must_use]
    pub const fn new(with: SwapWith) -> Self {
        Self { pane: None, with }
    }

    /// Swaps this pane.
    #[must_use]
    pub fn pane(mut self, pane: PaneId) -> Self {
        self.pane = Some(pane);
        self
    }
}

impl Command for PaneSwap {
    const NAME: &'static str = "pane.swap";
    type Output = Ack;
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde_json::{Map, Value, json};

    use super::{
        PaneClose, PaneFocus, PaneGo, PaneRename, PaneResize, PaneSplit, PaneSwap, PaneZoom,
    };
    use crate::files::{ModName, TerminalAddress};
    use crate::protocol::{
        Ack, Argv, Command, Direction, PaneContent, PaneId, PaneName, PaneTarget, Px, RawReply,
        Rename, Split, SplitDirection, SwapWith,
    };

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// `command` written as the JSON object herdr-app takes, parsed.
    fn arguments<C: Serialize>(command: &C) -> Result<Value, serde_json::Error> {
        serde_json::to_value(command)
    }

    /// Compiles only when `C` replies with an `O`.
    const fn replies_with<C: Command<Output = O>, O>() {}

    #[test]
    fn pane_commands_serialize_full_and_minimal() -> TestResult {
        let shell = PaneContent::Shell {
            argv: Some(Argv::new(["sh", "-c", "top"])?),
        };
        assert_eq!(
            arguments(&PaneSplit::new(SplitDirection::Right, shell.clone()))?,
            json!({"direction": "right", "argv": ["sh", "-c", "top"]}),
        );
        assert_eq!(
            arguments(&PaneSplit::new(SplitDirection::Down, shell).pane(PaneId::new("p1")?))?,
            json!({"direction": "down", "pane": "p1", "argv": ["sh", "-c", "top"]}),
        );
        assert_eq!(
            arguments(&PaneSplit::new(
                SplitDirection::Right,
                PaneContent::Shell { argv: None },
            ))?,
            json!({"direction": "right"}),
        );
        let mut props = Map::new();
        props.insert("path".to_owned(), json!("/srv"));
        let module = PaneContent::Mod {
            module: ModName::new("files")?,
            props: Some(props),
        };
        assert_eq!(
            arguments(&PaneSplit::new(SplitDirection::Down, module))?,
            json!({"direction": "down", "mod": "files", "props": {"path": "/srv"}}),
        );

        assert_eq!(
            arguments(&PaneClose::new().pane(PaneId::new("p1")?))?,
            json!({"pane": "p1"}),
        );
        assert_eq!(arguments(&PaneClose::new())?, json!({}));

        let rename = PaneRename::new(Rename::To(PaneName::new("logs")?)).pane(PaneId::new("p1")?);
        assert_eq!(arguments(&rename)?, json!({"pane": "p1", "name": "logs"}));
        assert_eq!(
            arguments(&PaneRename::new(Rename::Automatic))?,
            json!({"name": ""}),
        );

        let resize = PaneResize::new(Direction::Left)
            .pane(PaneId::new("p1")?)
            .px(Px::new(30.0)?);
        assert_eq!(
            arguments(&resize)?,
            json!({"direction": "left", "pane": "p1", "px": 30.0}),
        );
        assert_eq!(
            arguments(&PaneResize::new(Direction::Right))?,
            json!({"direction": "right"}),
        );

        assert_eq!(
            arguments(&PaneZoom::new().pane(PaneId::new("p1")?))?,
            json!({"pane": "p1"}),
        );
        assert_eq!(arguments(&PaneZoom::new())?, json!({}));

        assert_eq!(
            arguments(&PaneFocus::new(PaneTarget::Pane(PaneId::new("p1")?)))?,
            json!({"pane": "p1"}),
        );
        assert_eq!(
            arguments(&PaneFocus::new(PaneTarget::Terminal(TerminalAddress::new(
                "t4"
            )?)))?,
            json!({"terminal": "t4"}),
        );

        assert_eq!(
            arguments(&PaneGo::new(Direction::Up))?,
            json!({"direction": "up"}),
        );

        assert_eq!(
            arguments(&PaneSwap::new(SwapWith::Direction(Direction::Right)))?,
            json!({"direction": "right"}),
        );
        assert_eq!(
            arguments(
                &PaneSwap::new(SwapWith::Target(PaneId::new("p2")?)).pane(PaneId::new("p1")?)
            )?,
            json!({"pane": "p1", "target": "p2"}),
        );
        Ok(())
    }

    #[test]
    fn pane_commands_name_their_wire_command() {
        assert_eq!(PaneSplit::NAME, "pane.split");
        assert_eq!(PaneClose::NAME, "pane.close");
        assert_eq!(PaneRename::NAME, "pane.rename");
        assert_eq!(PaneResize::NAME, "pane.resize");
        assert_eq!(PaneZoom::NAME, "pane.zoom");
        assert_eq!(PaneFocus::NAME, "pane.focus");
        assert_eq!(PaneGo::NAME, "pane.go");
        assert_eq!(PaneSwap::NAME, "pane.swap");

        replies_with::<PaneSplit, Split>();
        replies_with::<PaneClose, Ack>();
        replies_with::<PaneRename, Ack>();
        replies_with::<PaneResize, RawReply>();
        replies_with::<PaneZoom, Ack>();
        replies_with::<PaneFocus, Ack>();
        replies_with::<PaneGo, Ack>();
        replies_with::<PaneSwap, Ack>();
    }
}
