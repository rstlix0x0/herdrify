//! Domain types, ports and use cases of herdrify, with no I/O.
//!
//! This crate root holds no logic, so it carries no unit tests.
//!
//! herdrify's rules live here so they can be tested without Herdr, a filesystem, a network or a Lua
//! engine. The crate performs no I/O and depends on no async runtime, Lua engine or Herdr client.
//!
//! Responsibilities:
//!
//! - domain types: components, bundles, grants, the lock
//! - the traits through which herdrify reaches outside systems
//! - the use cases that combine them
//!
//! Non-responsibilities:
//!
//! - talking to Herdr, the filesystem, git or Lua; adapter crates do that
