//! The seam between the herdr-app protocol and whatever runs the program.
//!
//! The protocol layer describes commands and reads results as pure data. This layer names the one
//! thing it needs from the outside, running an [`Invocation`](crate::protocol::Invocation) and
//! handing back what it produced, so the client above can be built over any way of running the
//! program. The layer depends on [`protocol`](crate::protocol) only.
//!
//! Responsibilities:
//!
//! - [`Transport`], the capability of running one invocation to completion
//! - [`TransportError`], the refusal of a run that could not start
//!
//! Non-responsibilities:
//!
//! - running a process, which an implementor of [`Transport`] does
//! - interpreting the output of a run, which the protocol layer does
//!
//! This file only lists the submodules and re-exports their items, so it carries no unit tests
//! (unit-test-mandate exemption #1, export-only).

mod error;
mod transport;

#[doc(inline)]
pub use error::TransportError;

#[doc(inline)]
pub use transport::Transport;
