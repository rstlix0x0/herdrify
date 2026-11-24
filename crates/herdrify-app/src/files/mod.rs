//! The herdr-app files as pure data, with no I/O.
//!
//! Nothing here touches an async runtime, a file or a process, so every type can be built, read
//! and tested without one. Whatever reads or writes the files sits above this layer and depends on
//! it.
//!
//! Responsibilities:
//!
//! - the name newtypes the files use for nodes, mods, layouts, themes, terminals, colours,
//!   commands and key chords, and the error an empty one returns
//! - the colours a layout or theme holds, as a hex value or a theme colour name
//! - the exact number a file holds, which is written back in the form it was read
//! - the layout file, its nodes and the block content, pane frames, fills, strokes, floating
//!   blocks, look, root options, sizes, shadows and padding they hold, which keep every key and
//!   word they do not name
//! - the theme file, with its colours, which keeps every key it does not name
//! - the keys file, its sections, which a group of key bindings is filed under, and the action a
//!   chord is bound to
//! - the error that reports a layout, theme or keys file that cannot be read
//!
//! Non-responsibilities:
//!
//! - reading, writing or locating a file, which a layer above does
//! - deciding which names herdr-app accepts beyond the empty string, which herdr-app decides
//!
//! This file only lists the submodules and re-exports their items, so it carries no unit tests
//! (unit-test-mandate exemption #1, export-only).

mod color;
mod error;
mod identifiers;
mod keys;
mod layout;
mod number;
mod theme;

#[doc(inline)]
pub use color::{Color, ColorError, HexColor};
#[doc(inline)]
pub use error::{FileError, FileKind};
#[doc(inline)]
pub use identifiers::{
    Chord, ColorName, CommandName, EmptyValueError, LayoutName, ModName, NodeId, TerminalAddress,
    ThemeName,
};
#[doc(inline)]
pub use keys::{Action, KeyActionError, Keys, Section};
#[doc(inline)]
pub use layout::{
    Align, ClonedFrom, Content, Edge, Fit, FloatingBlock, FocusStyle, HeaderPosition, Layout, Line,
    Look, Node, NodeKind, Padding, Paint, PaneFrame, PaneHeader, Reveal, RootOptions, Shadow, Size,
    ThemeChoice, Titlebar,
};
#[doc(inline)]
pub use number::{Number, NumberError};
#[doc(inline)]
pub use theme::Theme;

// The macro that defines a name newtype, for the protocol names to reuse.
pub(crate) use identifiers::name;
