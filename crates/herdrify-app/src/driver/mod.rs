//! The typed client over a [`Transport`](crate::port::Transport).
//!
//! The protocol layer describes herdr-app's commands and replies as data, and the port layer runs
//! one invocation. This layer joins them: it turns a command into an invocation, runs it, and turns
//! what came back into a typed reply or an [`Error`]. The layer depends on [`port`](crate::port),
//! [`protocol`](crate::protocol) and [`files`](crate::files), never on an adapter.
//!
//! Responsibilities:
//!
//! - [`Client`], the calls for the version, the command list, the window listing and status, every
//!   typed command, and any other invocation
//! - [`Error`], every way a call can end without a reply
//!
//! Non-responsibilities:
//!
//! - running a process, which a [`Transport`](crate::port::Transport) does
//! - telling a failed run apart from a successful one, which the protocol layer does
//!
//! This file only lists the submodules and re-exports their items, so it carries no unit tests
//! (unit-test-mandate exemption #1, export-only).

mod client;
mod error;

#[doc(inline)]
pub use client::Client;

#[doc(inline)]
pub use error::Error;
