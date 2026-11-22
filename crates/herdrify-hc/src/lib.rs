//! Async client for herdr-core: JSON-RPC over `hc connect`.
//!
//! This crate root holds no logic, so it carries no unit tests. Its doctest pins the re-exports.
//!
//! A standalone client any Rust program can use to drive herdr-core, independent of the rest of
//! herdrify.
//!
//! # Examples
//!
//! The transport traits and their error are named at the crate root:
//!
//! ```
//! use herdrify_hc::{FrameSink, FrameSource, Transport, TransportError};
//!
//! fn takes_the_traits<T: Transport, K: FrameSink, S: FrameSource>(_: Option<TransportError>) {}
//! ```
//!
//! Responsibilities:
//!
//! - the herdr-core wire protocol and its message types
//! - the transport a client talks through
//!
//! Non-responsibilities:
//!
//! - herdrify components, grants or bundles

pub mod port;
pub mod protocol;

#[doc(inline)]
pub use port::{FrameSink, FrameSource, Transport, TransportError};
