//! The words and numbers a command takes as arguments.
//!
//! herdr-app takes a fixed set of words for a direction, a place, a click target and the like. They
//! are closed enums here, each writing the word herdr-app expects, so a misspelt word is a compile
//! error. A word herdr-app learns later goes through the raw command instead. The numbers have
//! ranges herdr-app does not check, so each is refused here when built.
//!
//! Responsibilities:
//!
//! - the closed enums of input words, each serializing as its wire word
//! - [`Position`], [`Px`], [`Weight`] and [`FiniteF64`], the numbers checked when built
//! - [`ArgumentRangeError`], the refusal of a number out of range
//! - the argument enums: [`Focus`], [`Pin`], [`Machine`], [`Rename`] and [`At`], which
//!   serialize as the value of one key, and [`SpaceRef`], [`TabRef`], [`TabMoveTo`],
//!   [`PaneTarget`], [`SwapWith`], [`Scroll`] and [`Resize`], which serialize as the one key they
//!   stand for, so a command struct flattens them
//! - [`TabContent`] and [`PaneContent`], what a new tab or pane holds, which serialize as the keys
//!   of the form they take
//!
//! Non-responsibilities:
//!
//! - deciding which command takes which argument, which each command does
//! - checking a value herdr-app refuses for a reason only it knows, such as a position past the
//!   last space

use std::fmt;
use std::num::NonZeroU32;

use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use serde_json::{Map, Value};
use thiserror::Error;

use crate::files::{ModName, TerminalAddress};
use crate::protocol::{Argv, Folder, MachineName, PaneId, SpaceId, TabId, Task};

/// A number that is out of the range a command argument takes.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::Position;
///
/// let refused = Position::new(0).unwrap_err();
/// assert_eq!(refused.argument, "index");
/// assert_eq!(refused.to_string(), "0 is out of range: positions start at 1");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{value} is out of range: {rule}")]
pub struct ArgumentRangeError {
    /// The name of the argument that was refused.
    pub argument: &'static str,
    /// The value as it was given.
    pub value: String,
    /// The rule the value broke.
    pub rule: &'static str,
}

impl ArgumentRangeError {
    fn new(argument: &'static str, value: impl fmt::Display, rule: &'static str) -> Self {
        Self {
            argument,
            value: value.to_string(),
            rule,
        }
    }
}

/// Which way a pane is split.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::SplitDirection;
///
/// assert_eq!(serde_json::to_string(&SplitDirection::Right)?, "\"right\"");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SplitDirection {
    /// The new pane goes to the right.
    Right,
    /// The new pane goes below.
    Down,
}

/// A direction to move, resize or swap a pane in.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::Direction;
///
/// assert_eq!(serde_json::to_string(&Direction::Left)?, "\"left\"");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Towards the left edge.
    Left,
    /// Towards the right edge.
    Right,
    /// Towards the top edge.
    Up,
    /// Towards the bottom edge.
    Down,
}

/// Where a terminal is shown.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::Place;
///
/// assert_eq!(serde_json::to_string(&Place::Tab)?, "\"tab\"");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Place {
    /// In a tab of its own.
    Tab,
    /// In a new pane to the right of the current one.
    Right,
    /// In a new pane below the current one.
    Down,
}

/// What a right click on a terminal opens.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::ClickTarget;
///
/// assert_eq!(serde_json::to_string(&ClickTarget::Menu)?, "\"menu\"");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClickTarget {
    /// The menu of the program running in the terminal.
    Program,
    /// The menu of herdr-app.
    Menu,
}

/// How much text a selection takes at its start.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::SelectMode;
///
/// assert_eq!(serde_json::to_string(&SelectMode::Word)?, "\"word\"");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectMode {
    /// One cell.
    Cell,
    /// One word.
    Word,
    /// One line.
    Line,
}

/// How to move the edge of a selection.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::SelectionMove;
///
/// assert_eq!(serde_json::to_string(&SelectionMove::PageUp)?, "\"page_up\"");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionMove {
    /// One cell to the left.
    Left,
    /// One cell to the right.
    Right,
    /// One line up.
    Up,
    /// One line down.
    Down,
    /// One page up.
    PageUp,
    /// One page down.
    PageDown,
    /// To the start of the scrollback.
    Home,
    /// To the end of the scrollback.
    End,
    /// To the start of the line.
    LineStart,
    /// To the end of the line.
    LineEnd,
}

/// An end of a terminal's scrollback.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::ScrollEdge;
///
/// assert_eq!(serde_json::to_string(&ScrollEdge::Bottom)?, "\"bottom\"");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScrollEdge {
    /// The oldest line.
    Top,
    /// The newest line.
    Bottom,
}

/// A section of the settings screen.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::SettingsSection;
///
/// assert_eq!(serde_json::to_string(&SettingsSection::Look)?, "\"look\"");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingsSection {
    /// Appearance.
    Look,
    /// Machines herdr-app can reach.
    Machines,
    /// Alerts and notifications.
    Alerts,
    /// Updates.
    Updates,
    /// Privacy.
    Privacy,
}

/// Who a piece of feedback comes from.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::FeedbackFrom;
///
/// assert_eq!(serde_json::to_string(&FeedbackFrom::Agent)?, "\"agent\"");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackFrom {
    /// A person.
    Person,
    /// An agent.
    Agent,
}

/// A place in a list of spaces or tabs, counted from 1.
///
/// herdr-app answers `space.select` with position 0 by refusing it as "no such space", so
/// position 0 is refused here when built, for tabs as well as spaces.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::Position;
///
/// let second = Position::new(2)?;
/// assert_eq!(serde_json::to_string(&second)?, "2");
/// assert!(Position::new(0).is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct Position(NonZeroU32);

impl Position {
    /// Wraps `index` as a position.
    ///
    /// # Errors
    ///
    /// Returns [`ArgumentRangeError`] for the argument `index` when `index` is 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::protocol::Position;
    ///
    /// assert!(Position::new(1).is_ok());
    /// assert_eq!(Position::new(0).unwrap_err().rule, "positions start at 1");
    /// ```
    pub fn new(index: u32) -> Result<Self, ArgumentRangeError> {
        NonZeroU32::new(index)
            .map(Self)
            .ok_or_else(|| ArgumentRangeError::new("index", index, "positions start at 1"))
    }
}

/// A size in pixels: finite and not negative.
///
/// herdr-app takes a negative size without complaint, so it is refused here when built. A
/// fractional size is accepted, and negative zero is written as `0.0`. It is written as a
/// floating-point number, which herdr-app accepts.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::Px;
///
/// let width = Px::new(200.0)?;
/// assert_eq!(serde_json::to_string(&width)?, "200.0");
/// assert!(Px::new(-5.0).is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize)]
pub struct Px(f64);

impl Px {
    /// Wraps `value` as a size in pixels.
    ///
    /// # Errors
    ///
    /// Returns [`ArgumentRangeError`] for the argument `px` when `value` is not finite or is
    /// negative.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::protocol::Px;
    ///
    /// assert!(Px::new(0.0).is_ok());
    /// assert_eq!(
    ///     Px::new(-5.0).unwrap_err().to_string(),
    ///     "-5 is out of range: must be finite and not negative"
    /// );
    /// ```
    pub fn new(value: f64) -> Result<Self, ArgumentRangeError> {
        if value.is_finite() && value >= 0.0 {
            // `abs` turns negative zero into `0.0` and leaves every other accepted value as it is.
            Ok(Self(value.abs()))
        } else {
            Err(ArgumentRangeError::new(
                "px",
                value,
                "must be finite and not negative",
            ))
        }
    }
}

/// A share of the space a layout splits among its children: finite and above 0.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::Weight;
///
/// assert!(Weight::new(1.5).is_ok());
/// assert!(Weight::new(0.0).is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize)]
pub struct Weight(f64);

impl Weight {
    /// Wraps `value` as a weight.
    ///
    /// # Errors
    ///
    /// Returns [`ArgumentRangeError`] for the argument `weight` when `value` is not finite or is
    /// not above 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::protocol::Weight;
    ///
    /// assert_eq!(
    ///     Weight::new(0.0).unwrap_err().to_string(),
    ///     "0 is out of range: must be finite and above 0"
    /// );
    /// ```
    pub fn new(value: f64) -> Result<Self, ArgumentRangeError> {
        if value.is_finite() && value > 0.0 {
            Ok(Self(value))
        } else {
            Err(ArgumentRangeError::new(
                "weight",
                value,
                "must be finite and above 0",
            ))
        }
    }
}

/// The number of pages a scroll moves: any finite number, which may be negative or fractional.
///
/// It is the value of the `pages` argument of a scroll, so its refusal names `pages`.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::FiniteF64;
///
/// assert!(FiniteF64::new(-0.5).is_ok());
/// assert!(FiniteF64::new(f64::NAN).is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize)]
pub struct FiniteF64(f64);

impl FiniteF64 {
    /// Wraps `value` as a finite number.
    ///
    /// # Errors
    ///
    /// Returns [`ArgumentRangeError`] for the argument `pages` when `value` is NaN or infinite.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::protocol::FiniteF64;
    ///
    /// assert_eq!(
    ///     FiniteF64::new(f64::NAN).unwrap_err().to_string(),
    ///     "NaN is out of range: must be finite"
    /// );
    /// ```
    pub fn new(value: f64) -> Result<Self, ArgumentRangeError> {
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err(ArgumentRangeError::new("pages", value, "must be finite"))
        }
    }
}

/// Whether a command moves the focus to what it makes.
///
/// Written as the boolean herdr-app takes.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::Focus;
///
/// assert_eq!(serde_json::to_string(&Focus::Background)?, "false");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Focus {
    /// Focus what the command makes.
    Show,
    /// Leave the focus where it is.
    Background,
}

impl Serialize for Focus {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bool(matches!(self, Self::Show))
    }
}

/// Whether a terminal is pinned.
///
/// Written as the boolean herdr-app takes.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::Pin;
///
/// assert_eq!(serde_json::to_string(&Pin::Pinned)?, "true");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pin {
    /// Keep the terminal pinned.
    Pinned,
    /// Unpin the terminal.
    Unpinned,
}

impl Serialize for Pin {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bool(matches!(self, Self::Pinned))
    }
}

/// The machine a space runs on.
///
/// Written as the machine's name, or as the empty string for this computer.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{Machine, MachineName};
///
/// assert_eq!(serde_json::to_string(&Machine::ThisComputer)?, "\"\"");
/// let vps = Machine::Named(MachineName::new("vps")?);
/// assert_eq!(serde_json::to_string(&vps)?, "\"vps\"");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Machine {
    /// The computer herdr-app runs on.
    ThisComputer,
    /// A machine herdr-app knows by this name.
    Named(MachineName),
}

impl Serialize for Machine {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::ThisComputer => serializer.serialize_str(""),
            Self::Named(name) => name.serialize(serializer),
        }
    }
}

/// The name types a [`Rename`] can carry, kept in a module of its own so the bound cannot be
/// named, and so cannot be implemented, outside this crate.
mod sealed {
    use crate::protocol::{PaneName, SpaceName, TabName};

    /// Implemented for `SpaceName`, `TabName` and `PaneName` only.
    pub trait RenameTarget {}

    impl RenameTarget for SpaceName {}
    impl RenameTarget for TabName {}
    impl RenameTarget for PaneName {}
}

/// The new name of a space, tab or pane.
///
/// Written as the name, or as the empty string to give the name back to herdr-app. `N` is the
/// name type of a space, tab or pane: `SpaceName`, `TabName` or `PaneName`. No other type is
/// accepted.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{Rename, TabName};
///
/// assert_eq!(serde_json::to_string(&Rename::<TabName>::Automatic)?, "\"\"");
/// let api = Rename::To(TabName::new("api")?);
/// assert_eq!(serde_json::to_string(&api)?, "\"api\"");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Only the name types of spaces, tabs and panes can be renamed to; a plain string is refused:
///
/// ```compile_fail,E0277
/// use herdrify_app::protocol::Rename;
///
/// let _ = Rename::<String>::To("api".to_owned());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Rename<N: sealed::RenameTarget> {
    /// Drop the name, so herdr-app chooses one.
    Automatic,
    /// Use this name.
    To(N),
}

impl<N: sealed::RenameTarget + Serialize> Serialize for Rename<N> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Automatic => serializer.serialize_str(""),
            Self::To(name) => name.serialize(serializer),
        }
    }
}

/// A space named by its place in the list or by its identifier.
///
/// Serializes to the one key it stands for, so a command struct flattens it.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{Position, SpaceRef};
///
/// assert_eq!(serde_json::to_string(&SpaceRef::Index(Position::new(2)?))?, r#"{"index":2}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SpaceRef {
    /// The space at this place in the list.
    Index(Position),
    /// The space with this identifier.
    Id(SpaceId),
}

/// A tab named by its place in the list or by its identifier.
///
/// Serializes to the one key it stands for, so a command struct flattens it.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{TabId, TabRef};
///
/// assert_eq!(serde_json::to_string(&TabRef::Id(TabId::new("t1")?))?, r#"{"id":"t1"}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TabRef {
    /// The tab at this place in the list.
    Index(Position),
    /// The tab with this identifier.
    Id(TabId),
}

/// Where a tab moves to: a place in the list, or a number of places from where it is.
///
/// Serializes to the one key it stands for, so a command struct flattens it.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::TabMoveTo;
///
/// assert_eq!(serde_json::to_string(&TabMoveTo::By(-1))?, r#"{"by":-1}"#);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TabMoveTo {
    /// The place in the list, counted from 1.
    Index(Position),
    /// This many places after the tab's own, or before it when negative.
    // A signed count of places in a tab list, which is never near `i32::MAX` long.
    By(i32),
}

/// A pane named directly or by the terminal it shows.
///
/// Serializes to the one key it stands for, so a command struct flattens it.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{PaneId, PaneTarget};
///
/// let target = PaneTarget::Pane(PaneId::new("p1")?);
/// assert_eq!(serde_json::to_string(&target)?, r#"{"pane":"p1"}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PaneTarget {
    /// The pane with this identifier.
    Pane(PaneId),
    /// The pane showing this terminal.
    Terminal(TerminalAddress),
}

/// What a pane swaps places with: its neighbour in a direction, or a named pane.
///
/// Serializes to the one key it stands for, so a command struct flattens it.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{Direction, SwapWith};
///
/// let with = SwapWith::Direction(Direction::Left);
/// assert_eq!(serde_json::to_string(&with)?, r#"{"direction":"left"}"#);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SwapWith {
    /// The neighbouring pane in this direction.
    Direction(Direction),
    /// The pane with this identifier.
    Target(PaneId),
}

/// How far to scroll a terminal.
///
/// Serializes to the one key it stands for, so a command struct flattens it.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{Scroll, ScrollEdge};
///
/// assert_eq!(serde_json::to_string(&Scroll::Lines(-1))?, r#"{"lines":-1}"#);
/// assert_eq!(serde_json::to_string(&Scroll::To(ScrollEdge::Top))?, r#"{"to":"top"}"#);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Scroll {
    /// This many lines, up when negative.
    // Signed 64-bit so any line count a scrollback could hold fits.
    Lines(i64),
    /// This many pages, up when negative.
    Pages(FiniteF64),
    /// To an end of the scrollback.
    To(ScrollEdge),
}

/// Where a selection starts: at the cursor or at a cell.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::At;
///
/// assert_eq!(serde_json::to_string(&At::Cursor)?, "\"cursor\"");
/// let cell = serde_json::to_value(At::Cell { row: 2, col: 0 })?;
/// assert_eq!(cell, serde_json::json!({"row": 2, "col": 0}));
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum At {
    /// At the cursor, written as the word `cursor`.
    Cursor,
    /// At a cell of the screen, counted from 0.
    Cell {
        /// The row of the cell.
        // Unsigned like `Position`: a cell is never counted from before the screen's first row.
        row: u32,
        /// The column of the cell.
        // Unsigned for the same reason as `row`.
        col: u32,
    },
}

impl Serialize for At {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Cursor => serializer.serialize_str("cursor"),
            Self::Cell { row, col } => {
                let mut cell = serializer.serialize_struct("Cell", 2)?;
                cell.serialize_field("row", row)?;
                cell.serialize_field("col", col)?;
                cell.end()
            }
        }
    }
}

/// The new size of a layout node: a pixel size or a weight.
///
/// Serializes to the one key it stands for, so a command struct flattens it.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{Px, Resize};
///
/// let resize = Resize::Px(Px::new(200.0)?);
/// assert_eq!(serde_json::to_string(&resize)?, r#"{"px":200.0}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Resize {
    /// A size in pixels.
    Px(Px),
    /// A share of the space among the node's siblings.
    Weight(Weight),
}

/// What a new tab holds: a program or a mod.
///
/// Serializes to the keys of the form it takes, so a command struct flattens it. A program left
/// without arguments is herdr-app's default shell. When both a program and a mod could apply,
/// herdr-app takes the mod, so the two forms are separate variants.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::ModName;
/// use herdrify_app::protocol::TabContent;
///
/// let content = TabContent::Mod { module: ModName::new("files")?, props: None };
/// assert_eq!(serde_json::to_string(&content)?, r#"{"mod":"files"}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum TabContent {
    /// A program, with whatever of its arguments, task and folder are set.
    Shell {
        /// The program and its arguments.
        #[serde(skip_serializing_if = "Option::is_none")]
        argv: Option<Argv>,
        /// The task to run.
        #[serde(skip_serializing_if = "Option::is_none")]
        task: Option<Task>,
        /// The folder to run in.
        #[serde(skip_serializing_if = "Option::is_none")]
        folder: Option<Folder>,
    },
    /// A mod, with the properties it is given.
    Mod {
        /// The mod to show.
        #[serde(rename = "mod")]
        module: ModName,
        /// The properties of the mod, written when set.
        #[serde(skip_serializing_if = "Option::is_none")]
        props: Option<Map<String, Value>>,
    },
}

/// What a new pane holds: a program or a mod.
///
/// Serializes to the keys of the form it takes, so a command struct flattens it. A program left
/// without arguments is herdr-app's default shell.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{Argv, PaneContent};
///
/// let content = PaneContent::Shell { argv: Some(Argv::new(["sh"])?) };
/// assert_eq!(serde_json::to_string(&content)?, r#"{"argv":["sh"]}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum PaneContent {
    /// A program, with its arguments when set.
    Shell {
        /// The program and its arguments.
        #[serde(skip_serializing_if = "Option::is_none")]
        argv: Option<Argv>,
    },
    /// A mod, with the properties it is given.
    Mod {
        /// The mod to show.
        #[serde(rename = "mod")]
        module: ModName,
        /// The properties of the mod, written when set.
        #[serde(skip_serializing_if = "Option::is_none")]
        props: Option<Map<String, Value>>,
    },
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde_json::{Map, Value, json};

    use super::{
        ArgumentRangeError, At, ClickTarget, Direction, FeedbackFrom, FiniteF64, Focus, Machine,
        PaneContent, PaneTarget, Pin, Place, Position, Px, Rename, Resize, Scroll, ScrollEdge,
        SelectMode, SelectionMove, SettingsSection, SpaceRef, SplitDirection, SwapWith, TabContent,
        TabMoveTo, TabRef, Weight,
    };
    use crate::files::{ModName, TerminalAddress};
    use crate::protocol::{Argv, Folder, MachineName, PaneId, SpaceId, TabId, TabName, Task};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// A struct that flattens `inner` into itself, as a command struct flattens an argument enum.
    #[derive(Serialize)]
    struct Flat<T> {
        #[serde(flatten)]
        inner: T,
    }

    /// `inner` written as the keys of a command struct, as parsed JSON.
    fn flat<T: Serialize>(inner: T) -> Result<Value, serde_json::Error> {
        serde_json::to_value(Flat { inner })
    }

    #[test]
    fn single_value_enums_serialize_their_value() -> TestResult {
        assert_eq!(serde_json::to_value(Focus::Show)?, json!(true));
        assert_eq!(serde_json::to_value(Focus::Background)?, json!(false));
        assert_eq!(serde_json::to_value(Pin::Pinned)?, json!(true));
        assert_eq!(serde_json::to_value(Pin::Unpinned)?, json!(false));
        assert_eq!(serde_json::to_value(Machine::ThisComputer)?, json!(""));
        assert_eq!(
            serde_json::to_value(Machine::Named(MachineName::new("vps")?))?,
            json!("vps")
        );
        assert_eq!(
            serde_json::to_value(Rename::<TabName>::Automatic)?,
            json!("")
        );
        assert_eq!(
            serde_json::to_value(Rename::To(TabName::new("api")?))?,
            json!("api")
        );
        assert_eq!(serde_json::to_value(At::Cursor)?, json!("cursor"));
        assert_eq!(
            serde_json::to_value(At::Cell { row: 2, col: 0 })?,
            json!({"col": 0, "row": 2})
        );
        Ok(())
    }

    #[test]
    fn one_of_groups_write_their_key() -> TestResult {
        assert_eq!(
            flat(SpaceRef::Index(Position::new(2)?))?,
            json!({"index": 2})
        );
        assert_eq!(
            flat(SpaceRef::Id(SpaceId::new("sp1")?))?,
            json!({"id": "sp1"})
        );
        assert_eq!(flat(TabRef::Index(Position::new(3)?))?, json!({"index": 3}));
        assert_eq!(flat(TabRef::Id(TabId::new("t1")?))?, json!({"id": "t1"}));
        assert_eq!(
            flat(TabMoveTo::Index(Position::new(1)?))?,
            json!({"index": 1})
        );
        assert_eq!(flat(TabMoveTo::By(-1))?, json!({"by": -1}));
        assert_eq!(
            flat(PaneTarget::Pane(PaneId::new("p1")?))?,
            json!({"pane": "p1"})
        );
        assert_eq!(
            flat(PaneTarget::Terminal(TerminalAddress::new("t1")?))?,
            json!({"terminal": "t1"})
        );
        assert_eq!(
            flat(SwapWith::Direction(Direction::Left))?,
            json!({"direction": "left"})
        );
        assert_eq!(
            flat(SwapWith::Target(PaneId::new("p1")?))?,
            json!({"target": "p1"})
        );
        assert_eq!(flat(Scroll::Lines(-1))?, json!({"lines": -1}));
        assert_eq!(
            flat(Scroll::Pages(FiniteF64::new(0.5)?))?,
            json!({"pages": 0.5})
        );
        assert_eq!(
            flat(Scroll::To(ScrollEdge::Bottom))?,
            json!({"to": "bottom"})
        );
        assert_eq!(flat(Resize::Px(Px::new(200.0)?))?, json!({"px": 200.0}));
        assert_eq!(
            flat(Resize::Weight(Weight::new(1.0)?))?,
            json!({"weight": 1.0})
        );
        Ok(())
    }

    #[test]
    fn content_enums_write_their_keys() -> TestResult {
        let mut props = Map::new();
        props.insert("role".to_owned(), json!("x"));
        assert_eq!(
            flat(TabContent::Mod {
                module: ModName::new("files")?,
                props: Some(props),
            })?,
            json!({"mod": "files", "props": {"role": "x"}})
        );
        assert_eq!(
            flat(TabContent::Mod {
                module: ModName::new("files")?,
                props: None,
            })?,
            json!({"mod": "files"})
        );
        assert_eq!(
            flat(TabContent::Shell {
                argv: Some(Argv::new(["sh"])?),
                task: Some(Task::new("t")?),
                folder: Some(Folder::new(if cfg!(windows) { r"C:\tmp" } else { "/tmp" })?),
            })?,
            json!({
                "argv": ["sh"],
                "folder": if cfg!(windows) { r"C:\tmp" } else { "/tmp" },
                "task": "t"
            })
        );
        assert_eq!(
            flat(TabContent::Shell {
                argv: None,
                task: None,
                folder: None,
            })?,
            json!({})
        );
        assert_eq!(
            flat(PaneContent::Mod {
                module: ModName::new("files")?,
                props: None,
            })?,
            json!({"mod": "files"})
        );
        assert_eq!(
            flat(PaneContent::Shell {
                argv: Some(Argv::new(["sh"])?),
            })?,
            json!({"argv": ["sh"]})
        );
        Ok(())
    }

    /// The wire text of each item, in order.
    fn wire<T: Serialize>(items: &[T]) -> Result<Vec<String>, serde_json::Error> {
        items
            .iter()
            .map(|item| serde_json::to_string(item))
            .collect()
    }

    /// The `Display` of the refusal in `result`, or an error when it was accepted.
    fn refusal<T>(result: Result<T, ArgumentRangeError>) -> Result<String, &'static str> {
        result
            .err()
            .map(|error| error.to_string())
            .ok_or("the value was accepted")
    }

    #[test]
    fn input_words_serialize_as_wire_words() -> TestResult {
        use ClickTarget::{Menu, Program};
        use Direction::{Down, Left, Right, Up};
        use FeedbackFrom::{Agent, Person};
        use ScrollEdge::{Bottom, Top};
        use SelectMode::{Cell, Line, Word};
        use SelectionMove::{End, Home, LineEnd, LineStart, PageDown, PageUp};
        use SettingsSection::{Alerts, Look, Machines, Privacy, Updates};

        assert_eq!(
            wire(&[SplitDirection::Right, SplitDirection::Down])?,
            [r#""right""#, r#""down""#]
        );
        assert_eq!(
            wire(&[Left, Right, Up, Down])?,
            [r#""left""#, r#""right""#, r#""up""#, r#""down""#]
        );
        assert_eq!(
            wire(&[Place::Tab, Place::Right, Place::Down])?,
            [r#""tab""#, r#""right""#, r#""down""#]
        );
        assert_eq!(wire(&[Program, Menu])?, [r#""program""#, r#""menu""#]);
        assert_eq!(
            wire(&[Cell, Word, Line])?,
            [r#""cell""#, r#""word""#, r#""line""#]
        );
        assert_eq!(
            wire(&[
                SelectionMove::Left,
                SelectionMove::Right,
                SelectionMove::Up,
                SelectionMove::Down,
                PageUp,
                PageDown,
                Home,
                End,
                LineStart,
                LineEnd,
            ])?,
            [
                r#""left""#,
                r#""right""#,
                r#""up""#,
                r#""down""#,
                r#""page_up""#,
                r#""page_down""#,
                r#""home""#,
                r#""end""#,
                r#""line_start""#,
                r#""line_end""#,
            ]
        );
        assert_eq!(wire(&[Top, Bottom])?, [r#""top""#, r#""bottom""#]);
        assert_eq!(
            wire(&[Look, Machines, Alerts, Updates, Privacy])?,
            [
                r#""look""#,
                r#""machines""#,
                r#""alerts""#,
                r#""updates""#,
                r#""privacy""#
            ]
        );
        assert_eq!(wire(&[Person, Agent])?, [r#""person""#, r#""agent""#]);
        Ok(())
    }

    #[test]
    fn position_refuses_zero() -> TestResult {
        let error = Position::new(0).err().ok_or("position 0 was accepted")?;
        assert_eq!(error.argument, "index");
        assert_eq!(error.to_string(), "0 is out of range: positions start at 1");
        assert_eq!(serde_json::to_string(&Position::new(1)?)?, "1");
        Ok(())
    }

    #[test]
    fn px_weight_pages_ranges() -> TestResult {
        assert_eq!(
            refusal(Px::new(-5.0))?,
            "-5 is out of range: must be finite and not negative"
        );
        assert_eq!(
            refusal(Px::new(f64::INFINITY))?,
            "inf is out of range: must be finite and not negative"
        );
        assert!(Px::new(0.0).is_ok());
        assert_eq!(
            refusal(Weight::new(0.0))?,
            "0 is out of range: must be finite and above 0"
        );
        assert_eq!(
            refusal(Weight::new(f64::NAN))?,
            "NaN is out of range: must be finite and above 0"
        );
        assert!(Weight::new(0.5).is_ok());
        assert_eq!(
            refusal(FiniteF64::new(f64::NAN))?,
            "NaN is out of range: must be finite"
        );
        assert!(FiniteF64::new(-2.5).is_ok());
        Ok(())
    }

    #[test]
    fn px_takes_fractional_sizes() -> TestResult {
        assert_eq!(serde_json::to_string(&Px::new(2.5)?)?, "2.5");
        Ok(())
    }

    #[test]
    fn px_writes_negative_zero_as_zero() -> TestResult {
        assert_eq!(serde_json::to_string(&Px::new(-0.0)?)?, "0.0");
        assert_eq!(serde_json::to_string(&Px::new(0.0)?)?, "0.0");
        Ok(())
    }

    #[test]
    fn range_errors_name_their_argument() -> TestResult {
        let px = Px::new(-1.0).err().ok_or("accepted")?;
        let weight = Weight::new(-1.0).err().ok_or("accepted")?;
        let pages = FiniteF64::new(f64::INFINITY).err().ok_or("accepted")?;
        assert_eq!(
            (px.argument, weight.argument, pages.argument),
            ("px", "weight", "pages")
        );
        assert_eq!(px.value, "-1");
        Ok(())
    }
}
