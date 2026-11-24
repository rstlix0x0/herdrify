//! The herdr-app commands that select, scroll, search, copy and paste in a terminal.
//!
//! Each command here is a value that serializes to the JSON object of arguments herdr-app takes,
//! and names the type its reply decodes into, so a caller that runs one knows what comes back.
//!
//! Responsibilities:
//!
//! - [`TerminalSelectAll`], [`TerminalSelect`], [`TerminalAdjustSelection`],
//!   [`TerminalClearSelection`], [`TerminalScroll`] and [`TerminalSearch`], which act on a
//!   terminal's selection, view and find bar
//! - [`TerminalCopy`], [`TerminalPaste`] and [`TerminalPasteImage`], which go through the system
//!   clipboard
//!
//! Non-responsibilities:
//!
//! - building the command line from a command, which the invocation does
//! - running the command and decoding its reply, which a layer above does
//! - checking that a terminal exists or has a selection, which herdr-app decides
//!
//! The unit tests below check each command's arguments against the JSON object herdr-app takes, and
//! its name and reply type.

use serde::Serialize;

use crate::files::TerminalAddress;
use crate::protocol::{Ack, At, Command, RawReply, Scroll, SelectMode, SelectionMove};

/// Selects a terminal's whole screen and scrollback, ready to copy.
///
/// On the alternate screen it selects only that screen. Without a terminal it acts on the focused
/// one.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::TerminalAddress;
/// use herdrify_app::protocol::TerminalSelectAll;
///
/// let command = TerminalSelectAll::new().terminal(TerminalAddress::new("t4")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"terminal":"t4"}"#);
/// assert_eq!(serde_json::to_string(&TerminalSelectAll::new())?, "{}");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct TerminalSelectAll {
    #[serde(skip_serializing_if = "Option::is_none")]
    terminal: Option<TerminalAddress>,
}

impl TerminalSelectAll {
    /// A command that sets no argument.
    #[must_use]
    pub const fn new() -> Self {
        Self { terminal: None }
    }

    /// Selects in this terminal.
    #[must_use]
    pub fn terminal(mut self, terminal: TerminalAddress) -> Self {
        self.terminal = Some(terminal);
        self
    }
}

impl Command for TerminalSelectAll {
    const NAME: &'static str = "terminal.select_all";
    type Output = Ack;
}

/// Starts a selection in a terminal, replacing the old one.
///
/// Without a terminal it acts on the focused one, without a start it starts at the cursor, and
/// without a mode it selects cells.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::TerminalAddress;
/// use herdrify_app::protocol::{At, SelectMode, TerminalSelect};
///
/// let command = TerminalSelect::new()
///     .terminal(TerminalAddress::new("t4")?)
///     .at(At::Cell { row: 2, col: 0 })
///     .mode(SelectMode::Line);
/// assert_eq!(
///     serde_json::to_string(&command)?,
///     r#"{"terminal":"t4","at":{"row":2,"col":0},"mode":"line"}"#,
/// );
/// assert_eq!(serde_json::to_string(&TerminalSelect::new())?, "{}");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct TerminalSelect {
    #[serde(skip_serializing_if = "Option::is_none")]
    terminal: Option<TerminalAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    at: Option<At>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<SelectMode>,
}

impl TerminalSelect {
    /// A command that sets no argument.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            terminal: None,
            at: None,
            mode: None,
        }
    }

    /// Selects in this terminal.
    #[must_use]
    pub fn terminal(mut self, terminal: TerminalAddress) -> Self {
        self.terminal = Some(terminal);
        self
    }

    /// Starts the selection here.
    #[must_use]
    pub const fn at(mut self, at: At) -> Self {
        self.at = Some(at);
        self
    }

    /// Selects by cell, word or line, as a single, double or triple click does.
    #[must_use]
    pub const fn mode(mut self, mode: SelectMode) -> Self {
        self.mode = Some(mode);
        self
    }
}

impl Command for TerminalSelect {
    const NAME: &'static str = "terminal.select";
    type Output = Ack;
}

/// Moves the moving end of a terminal's selection.
///
/// The view follows the moving end. herdr-app refuses the command when nothing is selected, and
/// without a terminal it acts on the focused one.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::TerminalAddress;
/// use herdrify_app::protocol::{SelectionMove, TerminalAdjustSelection};
///
/// let command = TerminalAdjustSelection::new(SelectionMove::PageDown)
///     .terminal(TerminalAddress::new("t4")?);
/// assert_eq!(
///     serde_json::to_string(&command)?,
///     r#"{"terminal":"t4","direction":"page_down"}"#,
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct TerminalAdjustSelection {
    #[serde(skip_serializing_if = "Option::is_none")]
    terminal: Option<TerminalAddress>,
    direction: SelectionMove,
}

impl TerminalAdjustSelection {
    /// A command that moves the selection of the focused terminal by `direction`.
    #[must_use]
    pub const fn new(direction: SelectionMove) -> Self {
        Self {
            terminal: None,
            direction,
        }
    }

    /// Adjusts the selection of this terminal.
    #[must_use]
    pub fn terminal(mut self, terminal: TerminalAddress) -> Self {
        self.terminal = Some(terminal);
        self
    }
}

impl Command for TerminalAdjustSelection {
    const NAME: &'static str = "terminal.adjust_selection";
    type Output = Ack;
}

/// Clears a terminal's selection.
///
/// Without a terminal it acts on the focused one.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::TerminalAddress;
/// use herdrify_app::protocol::TerminalClearSelection;
///
/// let command = TerminalClearSelection::new().terminal(TerminalAddress::new("t4")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"terminal":"t4"}"#);
/// assert_eq!(serde_json::to_string(&TerminalClearSelection::new())?, "{}");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct TerminalClearSelection {
    #[serde(skip_serializing_if = "Option::is_none")]
    terminal: Option<TerminalAddress>,
}

impl TerminalClearSelection {
    /// A command that sets no argument.
    #[must_use]
    pub const fn new() -> Self {
        Self { terminal: None }
    }

    /// Clears the selection of this terminal.
    #[must_use]
    pub fn terminal(mut self, terminal: TerminalAddress) -> Self {
        self.terminal = Some(terminal);
        self
    }
}

impl Command for TerminalClearSelection {
    const NAME: &'static str = "terminal.clear_selection";
    type Output = Ack;
}

/// Scrolls a terminal's view of its history.
///
/// This scrolls herdr-app's view, not the program. Without a terminal it acts on the focused one.
/// The distance is one of the three forms of [`Scroll`], so herdr-app, which refuses more than one
/// at once, never gets two.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::TerminalAddress;
/// use herdrify_app::protocol::{FiniteF64, Scroll, TerminalScroll};
///
/// let command = TerminalScroll::new(Scroll::Pages(FiniteF64::new(0.5)?))
///     .terminal(TerminalAddress::new("t4")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"terminal":"t4","pages":0.5}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct TerminalScroll {
    #[serde(skip_serializing_if = "Option::is_none")]
    terminal: Option<TerminalAddress>,
    #[serde(flatten)]
    scroll: Scroll,
}

impl TerminalScroll {
    /// A command that scrolls the focused terminal by `scroll`.
    #[must_use]
    pub const fn new(scroll: Scroll) -> Self {
        Self {
            terminal: None,
            scroll,
        }
    }

    /// Scrolls this terminal.
    #[must_use]
    pub fn terminal(mut self, terminal: TerminalAddress) -> Self {
        self.terminal = Some(terminal);
        self
    }
}

impl Command for TerminalScroll {
    const NAME: &'static str = "terminal.scroll";
    type Output = Ack;
}

/// Opens the find bar of the focused terminal, which searches its screen and scrollback.
///
/// The bar and its query stay with that terminal.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::TerminalSearch;
///
/// assert_eq!(serde_json::to_string(&TerminalSearch::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::TerminalSearch {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct TerminalSearch {}

impl TerminalSearch {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for TerminalSearch {
    const NAME: &'static str = "terminal.search";
    type Output = Ack;
}

/// Copies the focused terminal's selection to the system clipboard.
///
/// This touches the system clipboard. With nothing selected the key goes to the program. No
/// success of this command was recorded on herdr-app 0.1.17, so its reply is a [`RawReply`].
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::TerminalCopy;
///
/// assert_eq!(serde_json::to_string(&TerminalCopy::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::TerminalCopy {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct TerminalCopy {}

impl TerminalCopy {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for TerminalCopy {
    const NAME: &'static str = "terminal.copy";
    type Output = RawReply;
}

/// Pastes the system clipboard into the focused terminal.
///
/// This touches the system clipboard. Copied files paste as if dropped, and an image alone pastes
/// with [`TerminalPasteImage`]. No success of this command was recorded on herdr-app 0.1.17, so its
/// reply is a [`RawReply`].
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::TerminalPaste;
///
/// assert_eq!(serde_json::to_string(&TerminalPaste::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::TerminalPaste {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct TerminalPaste {}

impl TerminalPaste {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for TerminalPaste {
    const NAME: &'static str = "terminal.paste";
    type Output = RawReply;
}

/// Pastes the system clipboard's image into the focused terminal, as Ctrl+V does.
///
/// This touches the system clipboard. When the terminal is on another machine the image goes there
/// as a file; on Windows the clipboard's text pastes; otherwise Ctrl+V goes to the terminal. No
/// success of this command was recorded on herdr-app 0.1.17, so its reply is a [`RawReply`].
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::TerminalPasteImage;
///
/// assert_eq!(serde_json::to_string(&TerminalPasteImage::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::TerminalPasteImage {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct TerminalPasteImage {}

impl TerminalPasteImage {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for TerminalPasteImage {
    const NAME: &'static str = "terminal.paste_image";
    type Output = RawReply;
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde_json::{Value, json};

    use super::{
        TerminalAdjustSelection, TerminalClearSelection, TerminalCopy, TerminalPaste,
        TerminalPasteImage, TerminalScroll, TerminalSearch, TerminalSelect, TerminalSelectAll,
    };
    use crate::files::TerminalAddress;
    use crate::protocol::{
        Ack, At, Command, FiniteF64, RawReply, Scroll, ScrollEdge, SelectMode, SelectionMove,
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
    fn terminal_selection_commands_serialize_full_and_minimal() -> TestResult {
        assert_eq!(
            arguments(&TerminalSelectAll::new().terminal(terminal()?))?,
            json!({"terminal": "t4"}),
        );
        assert_eq!(arguments(&TerminalSelectAll::new())?, json!({}));

        let full = TerminalSelect::new()
            .terminal(terminal()?)
            .at(At::Cell { row: 2, col: 0 })
            .mode(SelectMode::Word);
        assert_eq!(
            arguments(&full)?,
            json!({"terminal": "t4", "at": {"row": 2, "col": 0}, "mode": "word"}),
        );
        assert_eq!(
            arguments(&TerminalSelect::new().at(At::Cursor))?,
            json!({"at": "cursor"}),
        );
        assert_eq!(arguments(&TerminalSelect::new())?, json!({}));

        assert_eq!(
            arguments(&TerminalAdjustSelection::new(SelectionMove::PageUp).terminal(terminal()?))?,
            json!({"terminal": "t4", "direction": "page_up"}),
        );
        assert_eq!(
            arguments(&TerminalAdjustSelection::new(SelectionMove::LineEnd))?,
            json!({"direction": "line_end"}),
        );

        assert_eq!(
            arguments(&TerminalClearSelection::new().terminal(terminal()?))?,
            json!({"terminal": "t4"}),
        );
        assert_eq!(arguments(&TerminalClearSelection::new())?, json!({}));

        assert_eq!(
            arguments(&TerminalScroll::new(Scroll::Lines(-3)).terminal(terminal()?))?,
            json!({"terminal": "t4", "lines": -3}),
        );
        assert_eq!(
            arguments(&TerminalScroll::new(Scroll::Pages(FiniteF64::new(0.5)?)))?,
            json!({"pages": 0.5}),
        );
        assert_eq!(
            arguments(&TerminalScroll::new(Scroll::To(ScrollEdge::Bottom)))?,
            json!({"to": "bottom"}),
        );

        assert_eq!(arguments(&TerminalSearch::new())?, json!({}));
        assert_eq!(arguments(&TerminalCopy::new())?, json!({}));
        assert_eq!(arguments(&TerminalPaste::new())?, json!({}));
        assert_eq!(arguments(&TerminalPasteImage::new())?, json!({}));
        Ok(())
    }

    #[test]
    fn terminal_selection_commands_name_their_wire_command() {
        assert_eq!(TerminalSelectAll::NAME, "terminal.select_all");
        assert_eq!(TerminalSelect::NAME, "terminal.select");
        assert_eq!(TerminalAdjustSelection::NAME, "terminal.adjust_selection");
        assert_eq!(TerminalClearSelection::NAME, "terminal.clear_selection");
        assert_eq!(TerminalScroll::NAME, "terminal.scroll");
        assert_eq!(TerminalSearch::NAME, "terminal.search");
        assert_eq!(TerminalCopy::NAME, "terminal.copy");
        assert_eq!(TerminalPaste::NAME, "terminal.paste");
        assert_eq!(TerminalPasteImage::NAME, "terminal.paste_image");

        replies_with::<TerminalSelectAll, Ack>();
        replies_with::<TerminalSelect, Ack>();
        replies_with::<TerminalAdjustSelection, Ack>();
        replies_with::<TerminalClearSelection, Ack>();
        replies_with::<TerminalScroll, Ack>();
        replies_with::<TerminalSearch, Ack>();
        replies_with::<TerminalCopy, RawReply>();
        replies_with::<TerminalPaste, RawReply>();
        replies_with::<TerminalPasteImage, RawReply>();
    }
}
