//! The implementations of the transport trait.
//!
//! An adapter does what the port layer only names. It depends on [`port`](crate::port) and
//! [`protocol`](crate::protocol), and it is the only layer that names an async runtime's process
//! types.
//!
//! Responsibilities:
//!
//! - [`ProcessTransport`] and its builder [`ProcessTransportBuilder`], which run each invocation as
//!   a `herdr-app` child process
//!
//! Non-responsibilities:
//!
//! - the trait itself, which [`port`](crate::port) defines
//! - what a run's output means, which the protocol layer decides
//!
//! This file only lists the submodules and re-exports their items, so it carries no unit tests
//! (unit-test-mandate exemption #1, export-only).

mod process;

#[doc(inline)]
pub use process::{ProcessTransport, ProcessTransportBuilder};
