//! The herdr-app commands that act on mods, overlays, the app and its window.
//!
//! A mod is a small program that herdr-app shows as an overlay or in a tab, and an overlay is a
//! dialog or popup shown over the window. Each command here is a value that serializes to the JSON
//! object of arguments herdr-app takes, and names the type its reply decodes into, so a caller that
//! runs one knows what comes back.
//!
//! Responsibilities:
//!
//! - [`ModClone`], [`ModOpen`] and [`OverlayClose`], the commands on mods and overlays
//! - the `app.*` commands, such as [`AppHelp`], [`AppSettings`], [`AppTheme`] and [`AppQuit`], and
//!   the `window.*` commands [`WindowClose`], [`WindowMinimize`] and [`WindowMaximize`], one type
//!   per command, with the arguments herdr-app requires as parameters of `new` and the rest as
//!   setters
//! - the reply of a command whose success was never recorded, which is a [`RawReply`] and is
//!   explained on the command
//!
//! Non-responsibilities:
//!
//! - building the command line from a command, which the invocation does
//! - running the command and decoding its reply, which a layer above does
//! - checking that a mod, an overlay or a theme exists, which herdr-app decides
//!
//! The unit tests below check each command's arguments against the JSON object herdr-app takes, and
//! its name and reply type. One of them also covers every command across the crate: each wire name
//! is used once and is a name in [`KNOWN_COMMANDS`](crate::protocol::KNOWN_COMMANDS), and the
//! commands are split into those that reply with an [`Ack`], with a shape of their own and with a
//! [`RawReply`].

use serde::Serialize;

use crate::files::{ModName, ThemeName};
use crate::protocol::{Ack, Command, FeedbackFrom, OverlayId, RawReply, SettingsSection};

/// Duplicates a mod.
///
/// The copy is called after the mod with `-copy` added, unless [`as_name`](Self::as_name) names it.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::ModName;
/// use herdrify_app::protocol::ModClone;
///
/// let command = ModClone::new(ModName::new("files")?).as_name(ModName::new("mine")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"name":"files","as":"mine"}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct ModClone {
    name: ModName,
    #[serde(rename = "as", skip_serializing_if = "Option::is_none")]
    as_name: Option<ModName>,
}

impl ModClone {
    /// A command that duplicates the mod `name`.
    #[must_use]
    pub const fn new(name: ModName) -> Self {
        Self {
            name,
            as_name: None,
        }
    }

    /// Calls the copy this, sent as `as`, which is a keyword in Rust.
    #[must_use]
    pub fn as_name(mut self, name: ModName) -> Self {
        self.as_name = Some(name);
        self
    }
}

impl Command for ModClone {
    const NAME: &'static str = "mod.clone";
    type Output = Ack;
}

/// Opens a mod as the overlay its `mod.json` describes.
///
/// Sending the command again closes the overlay.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::ModName;
/// use herdrify_app::protocol::ModOpen;
///
/// let command = ModOpen::new(ModName::new("files")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"name":"files"}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct ModOpen {
    name: ModName,
}

impl ModOpen {
    /// A command that opens the mod `name`.
    #[must_use]
    pub const fn new(name: ModName) -> Self {
        Self { name }
    }
}

impl Command for ModOpen {
    const NAME: &'static str = "mod.open";
    type Output = Ack;
}

/// Closes an overlay.
///
/// Without an id it closes the top dialog or popup.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{OverlayClose, OverlayId};
///
/// let command = OverlayClose::new().id(OverlayId::new("frame-welcome")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"id":"frame-welcome"}"#);
/// assert_eq!(serde_json::to_string(&OverlayClose::new())?, "{}");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct OverlayClose {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<OverlayId>,
}

impl OverlayClose {
    /// A command that sets no argument.
    #[must_use]
    pub const fn new() -> Self {
        Self { id: None }
    }

    /// Closes this overlay.
    #[must_use]
    pub fn id(mut self, id: OverlayId) -> Self {
        self.id = Some(id);
        self
    }
}

impl Command for OverlayClose {
    const NAME: &'static str = "overlay.close";
    type Output = Ack;
}

/// Toggles the keys dialog.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::AppHelp;
///
/// assert_eq!(serde_json::to_string(&AppHelp::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::AppHelp {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct AppHelp {}

impl AppHelp {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for AppHelp {
    const NAME: &'static str = "app.help";
    type Output = Ack;
}

/// Shows the welcome again.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::AppWelcome;
///
/// assert_eq!(serde_json::to_string(&AppWelcome::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::AppWelcome {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct AppWelcome {}

impl AppWelcome {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for AppWelcome {
    const NAME: &'static str = "app.welcome";
    type Output = Ack;
}

/// Shows what this release brought.
///
/// herdr-app also opens it once by itself after an update.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::AppWhatsNew;
///
/// assert_eq!(serde_json::to_string(&AppWhatsNew::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::AppWhatsNew {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct AppWhatsNew {}

impl AppWhatsNew {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for AppWhatsNew {
    const NAME: &'static str = "app.whats_new";
    type Output = Ack;
}

/// Toggles the command palette.
///
/// The palette lists every command by name with its key. Sending the command again closes it.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::AppPalette;
///
/// assert_eq!(serde_json::to_string(&AppPalette::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::AppPalette {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct AppPalette {}

impl AppPalette {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for AppPalette {
    const NAME: &'static str = "app.palette";
    type Output = Ack;
}

/// Opens the settings.
///
/// A section opens the settings at that section.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{AppSettings, SettingsSection};
///
/// let command = AppSettings::new().section(SettingsSection::Alerts);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"section":"alerts"}"#);
/// assert_eq!(serde_json::to_string(&AppSettings::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct AppSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    section: Option<SettingsSection>,
}

impl AppSettings {
    /// A command that sets no argument.
    #[must_use]
    pub const fn new() -> Self {
        Self { section: None }
    }

    /// Opens the settings at this section.
    #[must_use]
    pub const fn section(mut self, section: SettingsSection) -> Self {
        self.section = Some(section);
        self
    }
}

impl Command for AppSettings {
    const NAME: &'static str = "app.settings";
    type Output = Ack;
}

/// Sets the shown layout and keys aside and goes back to the default screen.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::AppReset;
///
/// assert_eq!(serde_json::to_string(&AppReset::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::AppReset {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct AppReset {}

impl AppReset {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for AppReset {
    const NAME: &'static str = "app.reset";
    type Output = Ack;
}

/// Uses a theme, a preset or one of the person's own.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::ThemeName;
/// use herdrify_app::protocol::AppTheme;
///
/// let command = AppTheme::new(ThemeName::new("win98")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"name":"win98"}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct AppTheme {
    name: ThemeName,
}

impl AppTheme {
    /// A command that uses the theme `name`.
    #[must_use]
    pub const fn new(name: ThemeName) -> Self {
        Self { name }
    }
}

impl Command for AppTheme {
    const NAME: &'static str = "app.theme";
    type Output = Ack;
}

/// Makes everything bigger.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::AppZoomIn;
///
/// assert_eq!(serde_json::to_string(&AppZoomIn::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::AppZoomIn {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct AppZoomIn {}

impl AppZoomIn {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for AppZoomIn {
    const NAME: &'static str = "app.zoom_in";
    type Output = Ack;
}

/// Makes everything smaller.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::AppZoomOut;
///
/// assert_eq!(serde_json::to_string(&AppZoomOut::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::AppZoomOut {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct AppZoomOut {}

impl AppZoomOut {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for AppZoomOut {
    const NAME: &'static str = "app.zoom_out";
    type Output = Ack;
}

/// Goes back to the normal size.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::AppZoomReset;
///
/// assert_eq!(serde_json::to_string(&AppZoomReset::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::AppZoomReset {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct AppZoomReset {}

impl AppZoomReset {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for AppZoomReset {
    const NAME: &'static str = "app.zoom_reset";
    type Output = Ack;
}

/// Quits the app, on macOS.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::AppQuit;
///
/// assert_eq!(serde_json::to_string(&AppQuit::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::AppQuit {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct AppQuit {}

impl AppQuit {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for AppQuit {
    const NAME: &'static str = "app.quit";
    type Output = Ack;
}

/// Drafts feedback for the person to review and send.
///
/// The message and the sender, a person or an agent, are both optional. The message is free text
/// with no rule on it, so it is a plain string and any text, the empty one included, is accepted by
/// this type. The command opens Herdr's feedback flow, so its reply has no known shape and is a
/// [`RawReply`] rather than an acknowledgement.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{AppFeedback, FeedbackFrom};
///
/// let command = AppFeedback::new()
///     .message("the tab bar flickers")
///     .from(FeedbackFrom::Agent);
/// assert_eq!(
///     serde_json::to_string(&command)?,
///     r#"{"message":"the tab bar flickers","from":"agent"}"#,
/// );
/// assert_eq!(serde_json::to_string(&AppFeedback::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct AppFeedback {
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    from: Option<FeedbackFrom>,
}

impl AppFeedback {
    /// A command that sets no argument.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            message: None,
            from: None,
        }
    }

    /// Starts the draft with this text.
    #[must_use]
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Says who the feedback comes from.
    #[must_use]
    pub const fn from(mut self, from: FeedbackFrom) -> Self {
        self.from = Some(from);
        self
    }
}

impl Command for AppFeedback {
    const NAME: &'static str = "app.feedback";
    type Output = RawReply;
}

/// Looks for a newer Herdr and herdr-core now.
///
/// Its reply has no known shape, because the command reaches the network and the shared install, so
/// it is a [`RawReply`] rather than an acknowledgement.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::AppCheckUpdates;
///
/// assert_eq!(serde_json::to_string(&AppCheckUpdates::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::AppCheckUpdates {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct AppCheckUpdates {}

impl AppCheckUpdates {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for AppCheckUpdates {
    const NAME: &'static str = "app.check_updates";
    type Output = RawReply;
}

/// Updates herdr-core to its latest release of this generation.
///
/// The terminals keep running.
///
/// Its reply has no known shape, because the command updates the shared install, so it is a
/// [`RawReply`] rather than an acknowledgement.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::AppUpdateCore;
///
/// assert_eq!(serde_json::to_string(&AppUpdateCore::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::AppUpdateCore {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct AppUpdateCore {}

impl AppUpdateCore {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for AppUpdateCore {
    const NAME: &'static str = "app.update_core";
    type Output = RawReply;
}

/// Restarts into the update Herdr downloaded.
///
/// The terminals keep running.
///
/// Its reply has no known shape, because the command restarts Herdr into the update, so it is a
/// [`RawReply`] rather than an acknowledgement.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::AppUpdate;
///
/// assert_eq!(serde_json::to_string(&AppUpdate::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::AppUpdate {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct AppUpdate {}

impl AppUpdate {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for AppUpdate {
    const NAME: &'static str = "app.update";
    type Output = RawReply;
}

/// Hides the app, on macOS.
///
/// Its reply has no known shape, because the command hides Herdr itself, so it is a [`RawReply`]
/// rather than an acknowledgement.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::AppHide;
///
/// assert_eq!(serde_json::to_string(&AppHide::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::AppHide {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct AppHide {}

impl AppHide {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for AppHide {
    const NAME: &'static str = "app.hide";
    type Output = RawReply;
}

/// Hides the other apps, on macOS.
///
/// Its reply has no known shape, because the command acts on the person's other apps, so it is a
/// [`RawReply`] rather than an acknowledgement.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::AppHideOthers;
///
/// assert_eq!(serde_json::to_string(&AppHideOthers::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::AppHideOthers {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct AppHideOthers {}

impl AppHideOthers {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for AppHideOthers {
    const NAME: &'static str = "app.hide_others";
    type Output = RawReply;
}

/// Shows all apps, on macOS.
///
/// Its reply has no known shape, because the command acts on the person's other apps, so it is a
/// [`RawReply`] rather than an acknowledgement.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::AppShowAll;
///
/// assert_eq!(serde_json::to_string(&AppShowAll::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::AppShowAll {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct AppShowAll {}

impl AppShowAll {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for AppShowAll {
    const NAME: &'static str = "app.show_all";
    type Output = RawReply;
}

/// Closes the window, which on Linux quits the app.
///
/// The window stops answering afterwards.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::WindowClose;
///
/// assert_eq!(serde_json::to_string(&WindowClose::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::WindowClose {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct WindowClose {}

impl WindowClose {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for WindowClose {
    const NAME: &'static str = "window.close";
    type Output = Ack;
}

/// Minimizes the window.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::WindowMinimize;
///
/// assert_eq!(serde_json::to_string(&WindowMinimize::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::WindowMinimize {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct WindowMinimize {}

impl WindowMinimize {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for WindowMinimize {
    const NAME: &'static str = "window.minimize";
    type Output = Ack;
}

/// Maximizes the window, or puts it back.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::WindowMaximize;
///
/// assert_eq!(serde_json::to_string(&WindowMaximize::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::WindowMaximize {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct WindowMaximize {}

impl WindowMaximize {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for WindowMaximize {
    const NAME: &'static str = "window.maximize";
    type Output = Ack;
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;
    use std::collections::HashSet;

    use serde::Serialize;
    use serde_json::{Value, json};

    use super::{
        AppCheckUpdates, AppFeedback, AppHelp, AppHide, AppHideOthers, AppPalette, AppQuit,
        AppReset, AppSettings, AppShowAll, AppTheme, AppUpdate, AppUpdateCore, AppWelcome,
        AppWhatsNew, AppZoomIn, AppZoomOut, AppZoomReset, ModClone, ModOpen, OverlayClose,
        WindowClose, WindowMaximize, WindowMinimize,
    };
    use crate::files::{ModName, ThemeName};
    use crate::protocol::{
        Ack, AgentFocus, AgentNext, AgentPrev, Command, FeedbackFrom, KNOWN_COMMANDS, LayoutClone,
        LayoutHide, LayoutReset, LayoutResize, LayoutSelect, LayoutSet, LayoutShow, LayoutSwitch,
        LayoutToggle, OverlayId, PaneClose, PaneFocus, PaneGo, PaneRename, PaneResize, PaneSplit,
        PaneSwap, PaneZoom, RawReply, SettingsSection, SpaceClose, SpaceMove, SpaceNew, SpaceNext,
        SpacePrev, SpaceRename, SpaceSelect, TabClose, TabMove, TabNew, TabNext, TabPrev,
        TabRename, TabSelect, TabSettle, TabUnsettle, TerminalAdjustSelection,
        TerminalClearSelection, TerminalCopy, TerminalKeep, TerminalKill, TerminalOpen,
        TerminalPark, TerminalPaste, TerminalPasteImage, TerminalPeek, TerminalPin,
        TerminalRightClick, TerminalScroll, TerminalSearch, TerminalSelect, TerminalSelectAll,
        TerminalShow,
    };

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn arguments<C: Serialize>(command: &C) -> Result<Value, serde_json::Error> {
        serde_json::to_value(command)
    }

    const fn replies_with<C: Command<Output = O>, O>() {}

    /// The name of `C` and the type its reply decodes into.
    fn entry<C: Command>() -> (&'static str, TypeId)
    where
        C::Output: 'static,
    {
        (C::NAME, TypeId::of::<C::Output>())
    }

    #[test]
    fn app_commands_serialize_full_and_minimal() -> TestResult {
        assert_eq!(
            arguments(&ModClone::new(ModName::new("files")?).as_name(ModName::new("mine")?))?,
            json!({"name": "files", "as": "mine"}),
        );
        assert_eq!(
            arguments(&ModClone::new(ModName::new("files")?))?,
            json!({"name": "files"}),
        );
        assert_eq!(
            arguments(&ModOpen::new(ModName::new("files")?))?,
            json!({"name": "files"}),
        );
        assert_eq!(
            arguments(&OverlayClose::new().id(OverlayId::new("frame-welcome")?))?,
            json!({"id": "frame-welcome"}),
        );
        assert_eq!(arguments(&OverlayClose::new())?, json!({}));

        assert_eq!(arguments(&AppHelp::new())?, json!({}));
        assert_eq!(arguments(&AppWelcome::new())?, json!({}));
        assert_eq!(arguments(&AppWhatsNew::new())?, json!({}));
        assert_eq!(arguments(&AppPalette::new())?, json!({}));
        assert_eq!(
            arguments(&AppSettings::new().section(SettingsSection::Alerts))?,
            json!({"section": "alerts"}),
        );
        assert_eq!(arguments(&AppSettings::new())?, json!({}));
        assert_eq!(arguments(&AppReset::new())?, json!({}));
        assert_eq!(
            arguments(&AppTheme::new(ThemeName::new("win98")?))?,
            json!({"name": "win98"}),
        );
        assert_eq!(arguments(&AppZoomIn::new())?, json!({}));
        assert_eq!(arguments(&AppZoomOut::new())?, json!({}));
        assert_eq!(arguments(&AppZoomReset::new())?, json!({}));
        assert_eq!(arguments(&AppQuit::new())?, json!({}));

        let feedback = AppFeedback::new()
            .message("the tab bar flickers")
            .from(FeedbackFrom::Agent);
        assert_eq!(
            arguments(&feedback)?,
            json!({"message": "the tab bar flickers", "from": "agent"}),
        );
        assert_eq!(arguments(&AppFeedback::new())?, json!({}));
        assert_eq!(arguments(&AppCheckUpdates::new())?, json!({}));
        assert_eq!(arguments(&AppUpdateCore::new())?, json!({}));
        assert_eq!(arguments(&AppUpdate::new())?, json!({}));
        assert_eq!(arguments(&AppHide::new())?, json!({}));
        assert_eq!(arguments(&AppHideOthers::new())?, json!({}));
        assert_eq!(arguments(&AppShowAll::new())?, json!({}));

        assert_eq!(arguments(&WindowClose::new())?, json!({}));
        assert_eq!(arguments(&WindowMinimize::new())?, json!({}));
        assert_eq!(arguments(&WindowMaximize::new())?, json!({}));
        Ok(())
    }

    #[test]
    fn app_commands_name_their_wire_command() {
        assert_eq!(ModClone::NAME, "mod.clone");
        assert_eq!(ModOpen::NAME, "mod.open");
        assert_eq!(OverlayClose::NAME, "overlay.close");
        assert_eq!(AppHelp::NAME, "app.help");
        assert_eq!(AppWelcome::NAME, "app.welcome");
        assert_eq!(AppWhatsNew::NAME, "app.whats_new");
        assert_eq!(AppPalette::NAME, "app.palette");
        assert_eq!(AppSettings::NAME, "app.settings");
        assert_eq!(AppReset::NAME, "app.reset");
        assert_eq!(AppTheme::NAME, "app.theme");
        assert_eq!(AppZoomIn::NAME, "app.zoom_in");
        assert_eq!(AppZoomOut::NAME, "app.zoom_out");
        assert_eq!(AppZoomReset::NAME, "app.zoom_reset");
        assert_eq!(AppQuit::NAME, "app.quit");
        assert_eq!(AppFeedback::NAME, "app.feedback");
        assert_eq!(AppCheckUpdates::NAME, "app.check_updates");
        assert_eq!(AppUpdateCore::NAME, "app.update_core");
        assert_eq!(AppUpdate::NAME, "app.update");
        assert_eq!(AppHide::NAME, "app.hide");
        assert_eq!(AppHideOthers::NAME, "app.hide_others");
        assert_eq!(AppShowAll::NAME, "app.show_all");
        assert_eq!(WindowClose::NAME, "window.close");
        assert_eq!(WindowMinimize::NAME, "window.minimize");
        assert_eq!(WindowMaximize::NAME, "window.maximize");

        replies_with::<ModClone, Ack>();
        replies_with::<ModOpen, Ack>();
        replies_with::<OverlayClose, Ack>();
        replies_with::<AppHelp, Ack>();
        replies_with::<AppWelcome, Ack>();
        replies_with::<AppWhatsNew, Ack>();
        replies_with::<AppPalette, Ack>();
        replies_with::<AppSettings, Ack>();
        replies_with::<AppReset, Ack>();
        replies_with::<AppTheme, Ack>();
        replies_with::<AppZoomIn, Ack>();
        replies_with::<AppZoomOut, Ack>();
        replies_with::<AppZoomReset, Ack>();
        replies_with::<AppQuit, Ack>();
        replies_with::<AppFeedback, RawReply>();
        replies_with::<AppCheckUpdates, RawReply>();
        replies_with::<AppUpdateCore, RawReply>();
        replies_with::<AppUpdate, RawReply>();
        replies_with::<AppHide, RawReply>();
        replies_with::<AppHideOthers, RawReply>();
        replies_with::<AppShowAll, RawReply>();
        replies_with::<WindowClose, Ack>();
        replies_with::<WindowMinimize, Ack>();
        replies_with::<WindowMaximize, Ack>();
    }

    #[test]
    fn command_totals_match_the_recorded_list() {
        let commands = [
            entry::<SpaceNew>(),
            entry::<SpaceClose>(),
            entry::<SpaceRename>(),
            entry::<SpaceMove>(),
            entry::<SpaceSelect>(),
            entry::<SpaceNext>(),
            entry::<SpacePrev>(),
            entry::<TabNew>(),
            entry::<TabClose>(),
            entry::<TabRename>(),
            entry::<TabMove>(),
            entry::<TabSelect>(),
            entry::<TabSettle>(),
            entry::<TabUnsettle>(),
            entry::<TabNext>(),
            entry::<TabPrev>(),
            entry::<PaneSplit>(),
            entry::<PaneClose>(),
            entry::<PaneRename>(),
            entry::<PaneResize>(),
            entry::<PaneZoom>(),
            entry::<PaneFocus>(),
            entry::<PaneGo>(),
            entry::<PaneSwap>(),
            entry::<TerminalOpen>(),
            entry::<TerminalKeep>(),
            entry::<TerminalPark>(),
            entry::<TerminalKill>(),
            entry::<TerminalPeek>(),
            entry::<TerminalShow>(),
            entry::<TerminalPin>(),
            entry::<TerminalRightClick>(),
            entry::<TerminalSelectAll>(),
            entry::<TerminalSelect>(),
            entry::<TerminalAdjustSelection>(),
            entry::<TerminalClearSelection>(),
            entry::<TerminalCopy>(),
            entry::<TerminalPaste>(),
            entry::<TerminalPasteImage>(),
            entry::<TerminalScroll>(),
            entry::<TerminalSearch>(),
            entry::<AgentFocus>(),
            entry::<AgentNext>(),
            entry::<AgentPrev>(),
            entry::<LayoutSwitch>(),
            entry::<LayoutClone>(),
            entry::<LayoutSet>(),
            entry::<LayoutToggle>(),
            entry::<LayoutShow>(),
            entry::<LayoutHide>(),
            entry::<LayoutResize>(),
            entry::<LayoutSelect>(),
            entry::<LayoutReset>(),
            entry::<ModClone>(),
            entry::<ModOpen>(),
            entry::<OverlayClose>(),
            entry::<AppHelp>(),
            entry::<AppWelcome>(),
            entry::<AppWhatsNew>(),
            entry::<AppPalette>(),
            entry::<AppSettings>(),
            entry::<AppReset>(),
            entry::<AppTheme>(),
            entry::<AppZoomIn>(),
            entry::<AppZoomOut>(),
            entry::<AppZoomReset>(),
            entry::<AppQuit>(),
            entry::<AppFeedback>(),
            entry::<AppCheckUpdates>(),
            entry::<AppUpdateCore>(),
            entry::<AppUpdate>(),
            entry::<AppHide>(),
            entry::<AppHideOthers>(),
            entry::<AppShowAll>(),
            entry::<WindowClose>(),
            entry::<WindowMinimize>(),
            entry::<WindowMaximize>(),
        ];

        let names: HashSet<&str> = commands.iter().map(|(name, _)| *name).collect();
        let recorded: HashSet<&str> = KNOWN_COMMANDS.iter().map(|command| command.name).collect();
        let acks = commands
            .iter()
            .filter(|(_, output)| *output == TypeId::of::<Ack>())
            .count();
        let raw = commands
            .iter()
            .filter(|(_, output)| *output == TypeId::of::<RawReply>())
            .count();

        assert_eq!(commands.len(), 77);
        assert_eq!(names.len(), 77, "two types share a wire name");
        assert_eq!(
            names, recorded,
            "a wire name differs from the recorded list"
        );
        assert_eq!(acks, 60);
        assert_eq!(raw, 11);
        assert_eq!(commands.len() - acks - raw, 6);
    }
}
