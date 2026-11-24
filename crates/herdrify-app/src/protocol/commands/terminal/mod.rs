//! The herdr-app commands that act on terminals.
//!
//! A terminal is one running program, on screen in a pane or not. The commands are split by what
//! they do to it.
//!
//! Responsibilities:
//!
//! - the lifecycle commands, which start, keep, park, pin, show and end a terminal
//! - the selection commands, which select, scroll, search, copy and paste in a terminal
//!
//! This file only lists the submodules and re-exports their items, so it carries no unit tests
//! (unit-test-mandate exemption #1, export-only).

mod lifecycle;
mod selection;

#[doc(inline)]
pub use lifecycle::{
    TerminalKeep, TerminalKill, TerminalOpen, TerminalPark, TerminalPeek, TerminalPin,
    TerminalRightClick, TerminalShow,
};
#[doc(inline)]
pub use selection::{
    TerminalAdjustSelection, TerminalClearSelection, TerminalCopy, TerminalPaste,
    TerminalPasteImage, TerminalScroll, TerminalSearch, TerminalSelect, TerminalSelectAll,
};
