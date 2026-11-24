//! `HerdrApp`: one entrypoint that connects to herdr-app and reaches it from plain values.
//!
//! [`HerdrApp`] wraps a [`Client`](crate::driver::Client) and checks the version of herdr-app once,
//! when it is built. It builds the transport for the caller, so a program names a path or a
//! string instead of a [`ProcessTransport`](crate::adapter::ProcessTransport). The
//! [`Client`](crate::driver::Client) underneath stays reachable through [`HerdrApp::client`] for
//! anything the entrypoint does not cover.
//!
//! This is the one layer that depends on every other, so most programs start here. Values the
//! operations take that are not plain strings or numbers, such as
//! [`Focus`](crate::protocol::Focus), are in [`protocol`](crate::protocol) and the
//! [`prelude`](crate::prelude).
//!
//! The commands are grouped by what they act on. Each group method returns operations whose
//! methods return a [`Call`]: chain its options, then `.await` it.
//!
//! | Reach | Commands |
//! |---|---|
//! | [`HerdrApp::spaces`] → [`SpaceOps`] | `space.*` |
//! | [`HerdrApp::tabs`] → [`TabOps`] | `tab.*` |
//! | [`HerdrApp::panes`] → [`PaneOps`] | `pane.*` |
//! | [`HerdrApp::terminals`] → [`TerminalOps`] | `terminal.*` |
//! | [`HerdrApp::agents`] → [`AgentOps`] | `agent.*` |
//! | [`HerdrApp::layouts`] → [`LayoutOps`] | `layout.*` |
//! | [`HerdrApp::mods`] → [`ModOps`] | `mod.*` |
//! | [`HerdrApp::overlays`] → [`OverlayOps`] | `overlay.*` |
//! | [`HerdrApp::window`] → [`WindowOps`] | `app.*`, `window.*` |
//! | [`HerdrApp::ls`], [`HerdrApp::status`], [`HerdrApp::commands`] | `ls`, `status`, `commands` |
//!
//! Responsibilities:
//!
//! - [`HerdrApp`] and [`HerdrAppBuilder`], which connect and check the version
//! - [`Call`], a command with its options, run when awaited
//! - the operation groups, one for each row of the table above
//!
//! Non-responsibilities:
//!
//! - running a command and decoding its reply, which [`Client`](crate::driver::Client) does
//! - starting the program, which the adapter does
//!
//! This file only lists the submodules and re-exports their items, so it carries no unit tests
//! (unit-test-mandate exemption #1, export-only).

mod builder;
mod call;
mod entry;
mod layouts;
mod panes;
mod spaces;
mod tabs;
mod terminals;
mod window;

#[cfg(test)]
mod fake;

#[doc(inline)]
pub use builder::HerdrAppBuilder;

#[doc(inline)]
pub use call::Call;

#[doc(inline)]
pub use entry::HerdrApp;

#[doc(inline)]
pub use layouts::LayoutOps;

#[doc(inline)]
pub use panes::PaneOps;

#[doc(inline)]
pub use spaces::SpaceOps;

#[doc(inline)]
pub use tabs::TabOps;

#[doc(inline)]
pub use terminals::{AgentOps, TerminalOps};

#[doc(inline)]
pub use window::{ModOps, OverlayOps, WindowOps};
