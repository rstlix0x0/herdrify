//! The shapes of a herdr-app layout file: what its blocks show, how its panes are framed and what it
//! paints with.
//!
//! A layout file is a tree of nodes. This module holds the types that describe such a file, one
//! file per role, so a layout read from disk keeps every key and word it does not name and is
//! written back unchanged in meaning.
//!
//! Responsibilities:
//!
//! - [`Content`], what a block shows, and [`PaneFrame`] with [`PaneHeader`], [`HeaderPosition`]
//!   and [`Reveal`], the frame a tab's panes are drawn in
//! - [`Paint`], [`Fit`] and [`Line`], the fills and strokes a layout applies to its panes
//! - [`FloatingBlock`] with [`Edge`] and [`Align`], a block that floats over its node and where it
//!   sits
//! - [`Look`] with [`ThemeChoice`], the theme, colours, fonts and terminal options of a layout
//! - [`RootOptions`] with [`Titlebar`] and [`ClonedFrom`], the options only the root node takes
//! - [`Layout`], the file, and [`Node`] with [`NodeKind`], the nodes it is a tree of
//! - [`Size`], [`Shadow`] and [`FocusStyle`], the values a node takes besides its content and fills
//! - [`Padding`], a padding value of one number or two
//!
//! Non-responsibilities:
//!
//! - reading or writing a layout file, which a layer above does
//! - deciding whether a colour name refers to a colour the theme defines, which herdr-app does
//!
//! This file only lists the submodules and re-exports their items, so it carries no unit tests
//! (unit-test-mandate exemption #1, export-only).

mod content;
mod floating;
mod look;
mod node;
mod paint;

#[doc(inline)]
pub use content::{Content, HeaderPosition, PaneFrame, PaneHeader, Reveal};
#[doc(inline)]
pub use floating::{Align, Edge, FloatingBlock};
#[doc(inline)]
pub use look::{ClonedFrom, Look, RootOptions, ThemeChoice, Titlebar};
#[doc(inline)]
pub use node::{FocusStyle, Layout, Node, NodeKind, Padding, Shadow, Size};
#[doc(inline)]
pub use paint::{Fit, Line, Paint};
