//! The application layer: a client that runs concurrent calls over any transport.
//!
//! `connection` is written against the traits in `port` and the types in `protocol`, and never
//! names an adapter, so any transport, a fake included, can carry it. It is the only layer that
//! owns concurrency state: request ids, the pending calls and the two spawned tasks.
//!
//! Responsibilities:
//!
//! - [`Client`], which connects over a transport, runs calls and opens event subscriptions and
//!   terminal attaches
//! - [`Subscription`], the notifications herdr-core delivers on an event channel
//! - [`AttachStream`], a terminal's byte stream, with [`AttachEvent`] and [`CloseReason`]
//! - [`Error`], the error every client operation returns
//! - [`ConnectionEnd`] and [`ConnectionEndKind`], which say why a connection ended
//!
//! Non-responsibilities:
//!
//! - moving bytes, which an adapter does
//! - the message types and their codecs, which `protocol` defines
//!
//! This file only lists the submodules and re-exports their items, so it carries no unit tests
//! (the unit-test mandate's first structural exemption).

mod attach;
mod client;
mod error;
#[cfg(test)]
mod memory_link;
mod pending;
mod router;
mod stream;
mod subscription;
mod writer;

#[doc(inline)]
pub use attach::{AttachEvent, AttachStream, CloseReason};
#[doc(inline)]
pub use client::Client;
#[doc(inline)]
pub use error::{ConnectionEnd, ConnectionEndKind, Error};
#[doc(inline)]
pub use subscription::Subscription;
