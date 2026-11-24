//! The herdr-app commands that start, keep, park, pin, show and end a terminal.
//!
//! Each command here is a value that serializes to the JSON object of arguments herdr-app takes,
//! and names the type its reply decodes into, so a caller that runs one knows what comes back.
//!
//! Responsibilities:
//!
//! - [`TerminalOpen`], [`TerminalKeep`], [`TerminalPark`], [`TerminalKill`], [`TerminalPeek`],
//!   [`TerminalShow`], [`TerminalPin`] and [`TerminalRightClick`], one type per command, with the
//!   arguments herdr-app requires as parameters of `new` and the rest as setters
//!
//! Non-responsibilities:
//!
//! - building the command line from a command, which the invocation does
//! - running the command and decoding its reply, which a layer above does
//! - checking that a terminal exists or has a pane, which herdr-app decides
//!
//! The unit tests below check each command's arguments against the JSON object herdr-app takes, and
//! its name and reply type.

use serde::Serialize;

use crate::files::TerminalAddress;
use crate::protocol::{
    Ack, Argv, ClickTarget, Command, Folder, Kept, Opened, PaneId, Parked, Pin, Place, TerminalName,
};

/// Starts a terminal, as a new tab or beside the focused pane.
///
/// Without a place it opens a new tab, and without a program it runs the shell.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{Argv, Folder, Place, TerminalName, TerminalOpen};
///
/// let command = TerminalOpen::new()
///     .argv(Argv::new(["top"])?)
///     .folder(Folder::new(std::env::temp_dir())?)
///     .place(Place::Right)
///     .name(TerminalName::new("monitor")?);
/// let arguments = serde_json::to_value(&command)?;
/// assert_eq!(arguments["argv"], serde_json::json!(["top"]));
/// assert_eq!(arguments["place"], "right");
/// assert_eq!(arguments["name"], "monitor");
/// assert_eq!(serde_json::to_string(&TerminalOpen::new())?, "{}");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct TerminalOpen {
    #[serde(skip_serializing_if = "Option::is_none")]
    argv: Option<Argv>,
    #[serde(skip_serializing_if = "Option::is_none")]
    folder: Option<Folder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    place: Option<Place>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<TerminalName>,
}

impl TerminalOpen {
    /// A command that sets no argument.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            argv: None,
            folder: None,
            place: None,
            name: None,
        }
    }

    /// Runs this program.
    #[must_use]
    pub fn argv(mut self, argv: Argv) -> Self {
        self.argv = Some(argv);
        self
    }

    /// Starts the terminal in this folder.
    #[must_use]
    pub fn folder(mut self, folder: Folder) -> Self {
        self.folder = Some(folder);
        self
    }

    /// Puts the terminal here: in a new tab, or right of or below the focused pane.
    #[must_use]
    pub const fn place(mut self, place: Place) -> Self {
        self.place = Some(place);
        self
    }

    /// Names the terminal.
    #[must_use]
    pub fn name(mut self, name: TerminalName) -> Self {
        self.name = Some(name);
        self
    }
}

impl Command for TerminalOpen {
    const NAME: &'static str = "terminal.open";
    type Output = Opened;
}

/// Gives a running terminal that has no pane, such as a sub-agent, a pane of its own.
///
/// Without a place the pane is a new tab.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::TerminalAddress;
/// use herdrify_app::protocol::{Place, TerminalKeep};
///
/// let command = TerminalKeep::new(TerminalAddress::new("t4")?).place(Place::Down);
/// assert_eq!(
///     serde_json::to_string(&command)?,
///     r#"{"terminal":"t4","place":"down"}"#,
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct TerminalKeep {
    terminal: TerminalAddress,
    #[serde(skip_serializing_if = "Option::is_none")]
    place: Option<Place>,
}

impl TerminalKeep {
    /// A command that gives `terminal` a pane in a new tab.
    #[must_use]
    pub const fn new(terminal: TerminalAddress) -> Self {
        Self {
            terminal,
            place: None,
        }
    }

    /// Puts the pane here: in a new tab, or right of or below the focused pane.
    #[must_use]
    pub const fn place(mut self, place: Place) -> Self {
        self.place = Some(place);
        self
    }
}

impl Command for TerminalKeep {
    const NAME: &'static str = "terminal.keep";
    type Output = Kept;
}

/// Takes a terminal's pane away and stops herdr-app from keeping one for it.
///
/// The terminal keeps running; a sub-agent shows under its parent again.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::TerminalAddress;
/// use herdrify_app::protocol::TerminalPark;
///
/// let command = TerminalPark::new(TerminalAddress::new("t4")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"terminal":"t4"}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct TerminalPark {
    terminal: TerminalAddress,
}

impl TerminalPark {
    /// A command that parks `terminal`.
    #[must_use]
    pub const fn new(terminal: TerminalAddress) -> Self {
        Self { terminal }
    }
}

impl Command for TerminalPark {
    const NAME: &'static str = "terminal.park";
    type Output = Parked;
}

/// Ends a terminal and its sub-agents.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::TerminalAddress;
/// use herdrify_app::protocol::TerminalKill;
///
/// let command = TerminalKill::new(TerminalAddress::new("t4")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"terminal":"t4"}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct TerminalKill {
    terminal: TerminalAddress,
}

impl TerminalKill {
    /// A command that ends `terminal`.
    #[must_use]
    pub const fn new(terminal: TerminalAddress) -> Self {
        Self { terminal }
    }
}

impl Command for TerminalKill {
    const NAME: &'static str = "terminal.kill";
    type Output = Ack;
}

/// Shows a running terminal over the tab, with controls to keep it as a tab or close it.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::TerminalAddress;
/// use herdrify_app::protocol::TerminalPeek;
///
/// let command = TerminalPeek::new(TerminalAddress::new("t4")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"terminal":"t4"}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct TerminalPeek {
    terminal: TerminalAddress,
}

impl TerminalPeek {
    /// A command that peeks at `terminal`.
    #[must_use]
    pub const fn new(terminal: TerminalAddress) -> Self {
        Self { terminal }
    }
}

impl Command for TerminalPeek {
    const NAME: &'static str = "terminal.peek";
    type Output = Ack;
}

/// Shows a terminal in a pane for now, in place of the pane's own terminal.
///
/// Without a terminal it puts the pane's own terminal back. Without a pane it acts on the focused
/// one.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::TerminalAddress;
/// use herdrify_app::protocol::{PaneId, TerminalShow};
///
/// let command = TerminalShow::new()
///     .terminal(TerminalAddress::new("t4")?)
///     .pane(PaneId::new("p1")?);
/// assert_eq!(
///     serde_json::to_string(&command)?,
///     r#"{"terminal":"t4","pane":"p1"}"#,
/// );
/// assert_eq!(serde_json::to_string(&TerminalShow::new())?, "{}");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct TerminalShow {
    #[serde(skip_serializing_if = "Option::is_none")]
    terminal: Option<TerminalAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pane: Option<PaneId>,
}

impl TerminalShow {
    /// A command that sets no argument.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            terminal: None,
            pane: None,
        }
    }

    /// Shows this terminal.
    #[must_use]
    pub fn terminal(mut self, terminal: TerminalAddress) -> Self {
        self.terminal = Some(terminal);
        self
    }

    /// Shows it in this pane.
    #[must_use]
    pub fn pane(mut self, pane: PaneId) -> Self {
        self.pane = Some(pane);
        self
    }
}

impl Command for TerminalShow {
    const NAME: &'static str = "terminal.show";
    type Output = Ack;
}

/// Pins a terminal to the top of the agent lists, or unpins it.
///
/// herdr-app keeps the pin with the terminal, so every client shows it, and it ends with the
/// terminal.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::TerminalAddress;
/// use herdrify_app::protocol::{Pin, TerminalPin};
///
/// let command = TerminalPin::new(TerminalAddress::new("t4")?, Pin::Pinned);
/// assert_eq!(
///     serde_json::to_string(&command)?,
///     r#"{"terminal":"t4","pinned":true}"#,
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct TerminalPin {
    terminal: TerminalAddress,
    pinned: Pin,
}

impl TerminalPin {
    /// A command that sets whether `terminal` is pinned.
    #[must_use]
    pub const fn new(terminal: TerminalAddress, pinned: Pin) -> Self {
        Self { terminal, pinned }
    }
}

impl Command for TerminalPin {
    const NAME: &'static str = "terminal.pin";
    type Output = Ack;
}

/// Sets where right-clicks in a terminal go when its program takes the mouse.
///
/// They go to the program or to herdr-app's menu, which is the default. Shift and right-click is
/// always the menu. Without a terminal it acts on the focused one.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::TerminalAddress;
/// use herdrify_app::protocol::{ClickTarget, TerminalRightClick};
///
/// let command =
///     TerminalRightClick::new(ClickTarget::Program).terminal(TerminalAddress::new("t4")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"to":"program","terminal":"t4"}"#);
/// assert_eq!(
///     serde_json::to_string(&TerminalRightClick::new(ClickTarget::Menu))?,
///     r#"{"to":"menu"}"#,
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct TerminalRightClick {
    to: ClickTarget,
    #[serde(skip_serializing_if = "Option::is_none")]
    terminal: Option<TerminalAddress>,
}

impl TerminalRightClick {
    /// A command that sends right-clicks of the focused terminal to `to`.
    #[must_use]
    pub const fn new(to: ClickTarget) -> Self {
        Self { to, terminal: None }
    }

    /// Acts on this terminal.
    #[must_use]
    pub fn terminal(mut self, terminal: TerminalAddress) -> Self {
        self.terminal = Some(terminal);
        self
    }
}

impl Command for TerminalRightClick {
    const NAME: &'static str = "terminal.right_click";
    type Output = Ack;
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde_json::{Value, json};

    use super::{
        TerminalKeep, TerminalKill, TerminalOpen, TerminalPark, TerminalPeek, TerminalPin,
        TerminalRightClick, TerminalShow,
    };
    use crate::files::TerminalAddress;
    use crate::protocol::{
        Ack, Argv, ClickTarget, Command, Folder, Kept, Opened, PaneId, Parked, Pin, Place,
        TerminalName,
    };

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// `command` written as the JSON object herdr-app takes, parsed.
    fn arguments<C: Serialize>(command: &C) -> Result<Value, serde_json::Error> {
        serde_json::to_value(command)
    }

    /// Compiles only when `C` replies with an `O`.
    const fn replies_with<C: Command<Output = O>, O>() {}

    fn terminal() -> Result<TerminalAddress, Box<dyn std::error::Error>> {
        Ok(TerminalAddress::new("t4")?)
    }

    #[test]
    fn terminal_lifecycle_commands_serialize_full_and_minimal() -> TestResult {
        let folder = Folder::new(std::env::temp_dir())?;
        let full = TerminalOpen::new()
            .argv(Argv::new(["sh", "-c", "top"])?)
            .folder(folder.clone())
            .place(Place::Right)
            .name(TerminalName::new("logs")?);
        assert_eq!(
            arguments(&full)?,
            json!({
                "argv": ["sh", "-c", "top"],
                "folder": folder.as_str(),
                "place": "right",
                "name": "logs",
            }),
        );
        assert_eq!(arguments(&TerminalOpen::new())?, json!({}));

        assert_eq!(
            arguments(&TerminalKeep::new(terminal()?).place(Place::Down))?,
            json!({"terminal": "t4", "place": "down"}),
        );
        assert_eq!(
            arguments(&TerminalKeep::new(terminal()?))?,
            json!({"terminal": "t4"}),
        );

        assert_eq!(
            arguments(&TerminalPark::new(terminal()?))?,
            json!({"terminal": "t4"}),
        );

        assert_eq!(
            arguments(&TerminalRightClick::new(ClickTarget::Menu).terminal(terminal()?))?,
            json!({"to": "menu", "terminal": "t4"}),
        );
        assert_eq!(
            arguments(&TerminalRightClick::new(ClickTarget::Program))?,
            json!({"to": "program"}),
        );

        assert_eq!(
            arguments(&TerminalPin::new(terminal()?, Pin::Pinned))?,
            json!({"terminal": "t4", "pinned": true}),
        );
        assert_eq!(
            arguments(&TerminalPin::new(terminal()?, Pin::Unpinned))?,
            json!({"terminal": "t4", "pinned": false}),
        );

        assert_eq!(
            arguments(&TerminalKill::new(terminal()?))?,
            json!({"terminal": "t4"}),
        );
        assert_eq!(
            arguments(&TerminalPeek::new(terminal()?))?,
            json!({"terminal": "t4"}),
        );

        assert_eq!(
            arguments(
                &TerminalShow::new()
                    .terminal(terminal()?)
                    .pane(PaneId::new("p1")?)
            )?,
            json!({"terminal": "t4", "pane": "p1"}),
        );
        assert_eq!(arguments(&TerminalShow::new())?, json!({}));
        Ok(())
    }

    #[test]
    fn terminal_lifecycle_commands_name_their_wire_command() {
        assert_eq!(TerminalOpen::NAME, "terminal.open");
        assert_eq!(TerminalKeep::NAME, "terminal.keep");
        assert_eq!(TerminalPark::NAME, "terminal.park");
        assert_eq!(TerminalKill::NAME, "terminal.kill");
        assert_eq!(TerminalPeek::NAME, "terminal.peek");
        assert_eq!(TerminalShow::NAME, "terminal.show");
        assert_eq!(TerminalPin::NAME, "terminal.pin");
        assert_eq!(TerminalRightClick::NAME, "terminal.right_click");

        replies_with::<TerminalOpen, Opened>();
        replies_with::<TerminalKeep, Kept>();
        replies_with::<TerminalPark, Parked>();
        replies_with::<TerminalKill, Ack>();
        replies_with::<TerminalPeek, Ack>();
        replies_with::<TerminalShow, Ack>();
        replies_with::<TerminalPin, Ack>();
        replies_with::<TerminalRightClick, Ack>();
    }
}
