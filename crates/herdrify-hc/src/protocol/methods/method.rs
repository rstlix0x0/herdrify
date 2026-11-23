//! The `Method` trait: what a request type says about its wire method and its reply.
//!
//! This file holds one trait and no logic, so it carries no colocated unit tests (a pure trait
//! file, exempt under the unit-test mandate's third structural exemption). Each implementation is
//! tested where it is defined.
//!
//! Responsibilities:
//!
//! - [`Method`], which ties a request's parameters to its wire name and the type its reply is
//!   read into
//!
//! Non-responsibilities:
//!
//! - building or sending a request, which the client does
//! - the parameter types themselves, which `methods` and `hello` define

use serde::Serialize;
use serde::de::DeserializeOwned;

/// A herdr-core method: its wire name, its parameters and the type its reply is read into.
///
/// A client sends `Params` as the request's `params` object and reads the reply's `result` into
/// `Output`. Parameters are only ever sent, so they need `Serialize` and nothing more.
pub trait Method {
    /// The method name on the wire, such as `terminal.get`.
    const NAME: &'static str;

    /// The parameters sent with the request.
    type Params: Serialize;

    /// The type the reply's `result` is read into.
    type Output: DeserializeOwned;
}
