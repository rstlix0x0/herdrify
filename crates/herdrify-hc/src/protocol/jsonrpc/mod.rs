//! The JSON-RPC 2.0 layer of herdr-core, in two roles.
//!
//! `envelope` holds the request a client sends and the response or notification a peer sends back,
//! and `error` holds the error object a reply carries. This file only lists them.

mod envelope;
mod error;

#[doc(inline)]
pub use envelope::{Incoming, Request, RequestId};
#[doc(inline)]
pub use error::{ErrorKind, RpcError};
