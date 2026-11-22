//! Fakes of the herdrify ports for tests.
//!
//! This crate root holds no logic, so it carries no unit tests.
//!
//! Tests of herdrify's rules, and tests authors write for their components, run against these fakes
//! instead of real systems.
//!
//! Responsibilities:
//!
//! - in-memory stand-ins for the traits the domain crate defines
//!
//! Non-responsibilities:
//!
//! - running Lua; tests use the real engine for that
