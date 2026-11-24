//! The trait every herdr-app command implements.
//!
//! This file holds one trait and no executable logic, so it carries no unit tests (unit-test-mandate
//! exemption #3, pure trait definition).
//!
//! A command is a value that serializes to the JSON object herdr-app takes as its arguments. The
//! trait ties that value to the command's name and to the type its reply decodes into, so a caller
//! that runs a command knows what comes back without being told.
//!
//! Responsibilities:
//!
//! - [`Command`], the name of a command, its arguments as a serializable value and its output type
//!
//! Non-responsibilities:
//!
//! - building the command line from a command, which the invocation does
//! - running the command and decoding its reply, which a layer above does

use serde::Serialize;
use serde::de::DeserializeOwned;

/// A herdr-app command: its name, its arguments and the type of its reply.
///
/// The implementing value serializes to the JSON object of arguments. An argument that is not set
/// is left out of the object rather than written as `null`.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{Ack, Command};
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct SpaceNext {}
///
/// impl Command for SpaceNext {
///     const NAME: &'static str = "space.next";
///     type Output = Ack;
/// }
///
/// assert_eq!(SpaceNext::NAME, "space.next");
/// ```
pub trait Command: Serialize {
    /// The name herdr-app knows the command by, such as `space.next`.
    const NAME: &'static str;

    /// What the command's successful reply decodes into.
    type Output: DeserializeOwned;
}
