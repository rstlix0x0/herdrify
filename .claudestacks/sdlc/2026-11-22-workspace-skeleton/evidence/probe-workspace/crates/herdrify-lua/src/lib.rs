//! Runs herdrify components under the airsl Lua sandbox.
//!
//! This crate root holds no logic, so it carries no unit tests.
//!
//! Lua execution is isolated in one crate so the sandbox, and the bridge between synchronous Lua
//! and asynchronous Rust, have a single home.
//!
//! Responsibilities:
//!
//! - running a component's Lua entry file under its approved grants
//! - the host modules components call
//!
//! Non-responsibilities:
//!
//! - deciding which grants a component gets; the domain crate does that
