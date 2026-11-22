//! The seam between the client and the code that moves bytes.
//!
//! The client is written against the traits here, and an adapter or a test double implements them.
//! This layer depends on `protocol` only and names no async runtime type, so a transport can be
//! built on any runtime or on none.
//!
//! Responsibilities:
//!
//! - the traits a transport implements: [`Transport`], [`FrameSink`] and [`FrameSource`]
//! - the error a transport returns
//!
//! Non-responsibilities:
//!
//! - the protocol types themselves, which `protocol` defines
//! - any implementation of a transport, which an adapter provides
//!
//! This file only lists the submodules and re-exports their items, so it carries no unit tests.

mod error;
mod transport;

#[doc(inline)]
pub use error::TransportError;
#[doc(inline)]
pub use transport::{FrameSink, FrameSource, Transport};
