//! The herdrify home: config, lock, installed components, cache and state.
//!
//! This crate root holds no logic, so it carries no unit tests.
//!
//! herdrify's on-disk state has one owner, so its layout is defined and tested in one place.
//!
//! Responsibilities:
//!
//! - reading the person's configuration
//! - reading and writing the lock
//! - installed components, the fetch cache and per-component state
//!
//! Non-responsibilities:
//!
//! - fetching repositories; the git crate does that
