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
//! - [`files`], the layout, theme and keys files as pure data

pub mod files;
