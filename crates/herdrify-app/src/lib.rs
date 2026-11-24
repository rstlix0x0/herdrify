//! Async client for herdr-app and its layout, theme and keys files.
//!
//! This crate root only declares the layers and re-exports their main items, so it carries no unit
//! tests (unit-test-mandate exemption #1, export-only). Its doctests compile the connect example
//! and pin the re-exports.
//!
//! A standalone client any Rust program can use to drive the herdr-app window and to read and write
//! its files, independent of the rest of herdrify.
//!
//! There are two ways in:
//!
//! - [`HerdrApp`], the entrypoint: it starts the `herdr-app` program, checks its version once and
//!   builds every command from plain values. Each operation returns a call: chain its options,
//!   then `.await` it.
//! - [`Client`] with the typed commands in [`protocol`] and a [`Transport`] of your choice, for
//!   full control. `HerdrApp` is built on it, and [`HerdrApp::client`] reaches it.
//!
//! # Examples
//!
//! Open a space and a tab in the window that `herdr-app` reaches. This needs `herdr-app` on `PATH`
//! (the crate was tested against herdr-app 0.1.17) and a running window, so the example is
//! compiled but never run:
//!
//! ```no_run
//! use herdrify_app::prelude::*;
//!
//! #[tokio::main(flavor = "current_thread")]
//! async fn main() -> Result<(), Error> {
//!     let app = HerdrApp::local().await?;
//!     let created = app
//!         .spaces()
//!         .new_space()
//!         .name("api")
//!         .argv(["claude"])
//!         .focus(Focus::Background)
//!         .await?;
//!     app.tabs().new_tab().name("build").space(created.space.as_str()).await?;
//!     app.panes().split(SplitDirection::Right).await?;
//!     Ok(())
//! }
//! ```
//!
//! The same with the client and a typed command:
//!
//! ```no_run
//! use herdrify_app::protocol::TabNext;
//! use herdrify_app::{Client, Error, ProcessTransport};
//!
//! #[tokio::main(flavor = "current_thread")]
//! async fn main() -> Result<(), Error> {
//!     let client = Client::new(ProcessTransport::herdr_app());
//!     client.check_version().await?;
//!     client.run(TabNext::new()).await?;
//!     Ok(())
//! }
//! ```
//!
//! The entrypoint, its builder, the client, the error, the transport trait, its error and the
//! process transport are named at the crate root:
//!
//! ```
//! use herdrify_app::{
//!     Client, Error, HerdrApp, HerdrAppBuilder, ProcessTransport, Transport, TransportError,
//! };
//!
//! fn the_app_and_the_client_are_generic_over_a_transport<T: Transport>(
//!     _: Option<HerdrApp<T>>,
//!     _: Option<Client<T>>,
//!     _: Option<TransportError>,
//! ) {
//! }
//!
//! fn the_builder_makes_an_app_over_the_process_transport(
//!     _: Option<HerdrAppBuilder>,
//!     _: Option<HerdrApp<ProcessTransport>>,
//!     _: Option<Error>,
//! ) {
//! }
//!
//! fn the_process_transport_is_a_transport()
//! where
//!     ProcessTransport: Transport,
//! {
//! }
//! ```
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
//! - [`adapter`], the implementation of the transport that runs the `herdr-app` program
//! - [`driver`], the typed client that runs commands and returns typed replies
//! - [`files`], the layout, theme and keys files as pure data
//! - [`herdr_app`], the entrypoint that connects, checks the version and reaches herdr-app
//! - [`port`], the seam to whatever runs the `herdr-app` program
//! - [`prelude`], the names most programs need in one import
//! - [`protocol`], herdr-app's command-line language as pure data

pub mod adapter;
pub mod driver;
pub mod files;
pub mod herdr_app;
pub mod port;
pub mod prelude;
pub mod protocol;

#[doc(inline)]
pub use adapter::ProcessTransport;

#[doc(inline)]
pub use driver::{Client, Error};

#[doc(inline)]
pub use herdr_app::{HerdrApp, HerdrAppBuilder};

#[doc(inline)]
pub use port::{Transport, TransportError};
