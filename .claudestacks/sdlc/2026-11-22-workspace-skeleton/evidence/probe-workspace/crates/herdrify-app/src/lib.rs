//! Async client for herdr-app and its layout, theme and keys files.
//!
//! This crate root holds no logic, so it carries no unit tests.
//!
//! A standalone client any Rust program can use to drive the herdr-app window and to read and write
//! its files, independent of the rest of herdrify.
//!
//! Responsibilities:
//!
//! - herdr-app commands
//! - models of the layout, theme and keys files
//!
//! Non-responsibilities:
//!
//! - herdrify components, grants or bundles
