//! Fetches herdrify component and bundle repositories.
//!
//! This crate root holds no logic, so it carries no unit tests.
//!
//! Fetching sources is isolated so the rest of herdrify never touches git directly.
//!
//! Responsibilities:
//!
//! - fetching a repository at a revision into the cache
//!
//! Non-responsibilities:
//!
//! - deciding what to install; the domain crate does that
