//! Async client for herdr-core: JSON-RPC over `hc connect`.
//!
//! This crate root holds no logic, so it carries no unit tests. Its doctests compile the connect
//! example and pin the re-exports.
//!
//! A standalone client any Rust program can use to drive herdr-core, independent of the rest of
//! herdrify.
//!
//! # Examples
//!
//! Start a terminal on the core that `hc connect` reaches. This needs `hc` on `PATH` (the crate was
//! tested against hc 0.1.21), so the example is compiled but never run:
//!
//! ```no_run
//! use herdrify_hc::protocol::{Command, TerminalRun};
//! use herdrify_hc::{Client, ProcessTransport};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let transport = ProcessTransport::hc_connect().spawn()?;
//!     let client = Client::connect(transport).await?;
//!     let terminal = client.terminal_run(TerminalRun::new(Command::Shell)).await?;
//!     println!("started {}", terminal.address);
//!     Ok(())
//! }
//! ```
//!
//! The client, its error, the transport traits, their error and the two transports are named at
//! the crate root:
//!
//! ```
//! use herdrify_hc::{
//!     Client, Error, FrameSink, FrameSource, ProcessTransport, StreamTransport, Transport,
//!     TransportError,
//! };
//!
//! fn takes_the_traits<T: Transport, K: FrameSink, S: FrameSource>(_: Option<TransportError>) {}
//!
//! fn the_client_is_generic_over_a_transport<T: Transport>(_: Option<Client<T>>, _: Option<Error>) {}
//!
//! fn the_process_transport_is_a_transport()
//! where
//!     ProcessTransport: Transport,
//! {
//! }
//!
//! fn the_stream_transport_is_a_transport<R, W>(_: StreamTransport<R, W>)
//! where
//!     StreamTransport<R, W>: Transport,
//! {
//! }
//! ```
//!
//! Responsibilities:
//!
//! - the herdr-core wire protocol and its message types
//! - the transport a client talks through
//! - the client: a framed connection that runs concurrent calls over any transport
//!
//! Non-responsibilities:
//!
//! - herdrify components, grants or bundles

pub mod adapter;
pub mod connection;
pub mod port;
pub mod protocol;

#[doc(inline)]
pub use adapter::{ProcessTransport, StreamTransport};
#[doc(inline)]
pub use connection::{Client, Error};
#[doc(inline)]
pub use port::{FrameSink, FrameSource, Transport, TransportError};
