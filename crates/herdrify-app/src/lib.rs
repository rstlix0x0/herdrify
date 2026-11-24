//! Async client for herdr-app and its layout, theme and keys files.
//!
//! This crate root only declares the layers, so it carries no unit tests (unit-test-mandate
//! exemption #1, export-only).
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
//!
//! Layers:
//!
//! - [`driver`], the typed client that runs commands and returns typed replies
//! - [`files`], the layout, theme and keys files as pure data
//! - [`port`], the seam to whatever runs the `herdr-app` program
//! - [`protocol`], herdr-app's command-line language as pure data

pub mod driver;
pub mod files;
pub mod port;
pub mod protocol;
