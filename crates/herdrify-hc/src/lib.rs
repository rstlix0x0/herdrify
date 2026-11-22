//! Async client for herdr-core: JSON-RPC over `hc connect`.
//!
//! This crate root holds no logic, so it carries no unit tests.
//!
//! A standalone client any Rust program can use to drive herdr-core, independent of the rest of
//! herdrify.
//!
//! Responsibilities:
//!
//! - the herdr-core wire protocol and its message types
//! - the transport a client talks through
//!
//! Non-responsibilities:
//!
//! - herdrify components, grants or bundles
