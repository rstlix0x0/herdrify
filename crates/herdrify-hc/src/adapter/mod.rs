//! The implementations of the transport traits.
//!
//! An adapter carries the bytes the transport traits promise to move. It depends on `port` and
//! `protocol`, and it is the only layer that names an async runtime's I/O types.
//!
//! Responsibilities:
//!
//! - [`StreamTransport`], and its halves [`StreamSink`] and [`StreamSource`], which carry the
//!   protocol over any async reader and writer
//! - [`ProcessTransport`], its builder [`ProcessTransportBuilder`], and its halves [`ProcessSink`]
//!   and [`ProcessSource`], which carry the protocol over a spawned command such as `hc connect`
//!
//! Non-responsibilities:
//!
//! - the traits themselves, which `port` defines
//! - the handshake or any message's meaning, which the client does
//!
//! This file only lists the submodules and re-exports their items, so it carries no unit tests
//! (the unit-test mandate's first structural exemption).

mod process;
mod stream;

#[doc(inline)]
pub use process::{ProcessSink, ProcessSource, ProcessTransport, ProcessTransportBuilder};
#[doc(inline)]
pub use stream::{StreamSink, StreamSource, StreamTransport};
