//! `HerdrCore`: one entrypoint that connects to herdr-core and reaches every method from plain
//! values.
//!
//! [`HerdrCore`] wraps a [`Client`](crate::Client). It builds the transport, the request types and
//! their identifiers for the caller, so a program names strings, numbers and durations instead of
//! [`Argv`](crate::protocol::Argv), [`TerminalRef`](crate::protocol::TerminalRef) or
//! [`ProcessTransport`](crate::ProcessTransport). Every request is a [`Call`]: chain its options,
//! then `.await` it. The [`Client`](crate::Client) underneath stays reachable through
//! [`HerdrCore::client`] for anything the wrapper does not cover.
//!
//! The methods are grouped by what they act on:
//!
//! | Reach | Methods |
//! |---|---|
//! | [`HerdrCore::terminals`] → [`TerminalOps`] | `terminal.run`, `terminal.list`, `terminal.wait` on the caller's children, `terminal.put_chunk`, `terminal.put_finish`, `terminal.put_abort` |
//! | [`TerminalOps::handle`] → [`TerminalHandle`] | every method about one terminal, `terminal.attach` and `events.subscribe` on it included |
//! | [`HerdrCore::machines`] → [`MachineOps`] | `machine.add`, `machine.list`, `machine.connect`, `machine.rename`, `machine.rm` |
//! | [`HerdrCore::exec`], [`HerdrCore::exec_line`] | `machine.exec` |
//! | [`HerdrCore::files`] → [`FileOps`] | `file.read`, `file.write`, `file.edit` |
//! | [`HerdrCore::docs`] → [`DocOps`] | `doc.get`, `doc.set` |
//! | [`HerdrCore::server`] → [`ServerOps`] | `server.status`, `server.stop`, `server.restart` |
//! | [`HerdrCore::events`], [`HerdrCore::templates`], [`HerdrCore::discover`] | `events.subscribe`, `templates.list`, `rpc.discover` |
//!
//! Responsibilities:
//!
//! - [`HerdrCore`] and [`HerdrCoreBuilder`], which connect
//! - [`Call`], a request with its options, sent when awaited, and [`Invoke`], how it is sent
//! - the operation groups, and [`TerminalHandle`] and [`Started`] for one terminal
//! - [`HerdrCoreError`], the error every one of them returns
//!
//! Non-responsibilities:
//!
//! - the connection, the calls and the streams, which [`Client`](crate::Client) owns
//! - the request and result types, which `protocol` defines
//!
//! This file only lists the submodules and re-exports their items, so it carries no unit tests
//! (the unit-test mandate's first structural exemption).

mod builder;
mod call;
mod docs;
mod entry;
mod error;
mod files;
mod machines;
mod put;
mod server;
mod terminals;

#[cfg(test)]
mod fake;

#[doc(inline)]
pub use builder::HerdrCoreBuilder;
#[doc(inline)]
pub use call::{Call, Invoke};
#[doc(inline)]
pub use docs::DocOps;
#[doc(inline)]
pub use entry::HerdrCore;
#[doc(inline)]
pub use error::HerdrCoreError;
#[doc(inline)]
pub use files::FileOps;
#[doc(inline)]
pub use machines::MachineOps;
#[doc(inline)]
pub use server::ServerOps;
#[doc(inline)]
pub use terminals::{Started, TerminalHandle, TerminalOps};
