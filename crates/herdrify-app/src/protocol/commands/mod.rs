//! The herdr-app commands: the trait they share, the shapes of their replies and the commands
//! themselves.
//!
//! A command here is a plain value that knows its herdr-app name and writes its own arguments, so
//! a caller builds one without spelling a command name or an argument key.
//!
//! Responsibilities:
//!
//! - [`Command`], the trait that ties a command's name and arguments to the type of its reply
//! - the reply shapes a command's `Output` names
//! - the words, numbers and argument enums a command takes, and the refusal of a number out of
//!   range
//! - the commands, one file per group: those on spaces, tabs, panes, terminals, agents and
//!   layouts, then those on mods, overlays, the app and the window
//!
//! Non-responsibilities:
//!
//! - running a command, which a layer above does
//!
//! This file only lists the submodules and re-exports their items, so it carries no unit tests
//! (unit-test-mandate exemption #1, export-only).

mod agent;
mod app;
mod command;
mod layout;
mod outputs;
mod pane;
mod space;
mod tab;
mod terminal;
mod words;

#[doc(inline)]
pub use agent::{AgentFocus, AgentNext, AgentPrev};
#[doc(inline)]
pub use app::{
    AppCheckUpdates, AppFeedback, AppHelp, AppHide, AppHideOthers, AppPalette, AppQuit, AppReset,
    AppSettings, AppShowAll, AppTheme, AppUpdate, AppUpdateCore, AppWelcome, AppWhatsNew,
    AppZoomIn, AppZoomOut, AppZoomReset, ModClone, ModOpen, OverlayClose, WindowClose,
    WindowMaximize, WindowMinimize,
};
#[doc(inline)]
pub use command::Command;
#[doc(inline)]
pub use layout::{
    LayoutClone, LayoutHide, LayoutReset, LayoutResize, LayoutSelect, LayoutSet, LayoutShow,
    LayoutSwitch, LayoutToggle,
};
#[doc(inline)]
pub use outputs::{Ack, Created, Kept, Opened, Parked, RawReply, Split, TabMade};
#[doc(inline)]
pub use pane::{
    PaneClose, PaneFocus, PaneGo, PaneRename, PaneResize, PaneSplit, PaneSwap, PaneZoom,
};
#[doc(inline)]
pub use space::{SpaceClose, SpaceMove, SpaceNew, SpaceNext, SpacePrev, SpaceRename, SpaceSelect};
#[doc(inline)]
pub use tab::{
    TabClose, TabMove, TabNew, TabNext, TabPrev, TabRename, TabSelect, TabSettle, TabUnsettle,
};
#[doc(inline)]
pub use terminal::{
    TerminalAdjustSelection, TerminalClearSelection, TerminalCopy, TerminalKeep, TerminalKill,
    TerminalOpen, TerminalPark, TerminalPaste, TerminalPasteImage, TerminalPeek, TerminalPin,
    TerminalRightClick, TerminalScroll, TerminalSearch, TerminalSelect, TerminalSelectAll,
    TerminalShow,
};
#[doc(inline)]
pub use words::{
    ArgumentRangeError, At, ClickTarget, Direction, FeedbackFrom, FiniteF64, Focus, Machine,
    PaneContent, PaneTarget, Pin, Place, Position, Px, Rename, Resize, Scroll, ScrollEdge,
    SelectMode, SelectionMove, SettingsSection, SpaceRef, SplitDirection, SwapWith, TabContent,
    TabMoveTo, TabRef, Weight,
};
