//! The frames of a framed herdr-core connection, in three roles.
//!
//! `wire` holds the frame values and their byte encoding, `decoder` cuts frames out of a byte
//! stream, and `error` holds the refusal both return. This file only lists them.

mod decoder;
mod error;
mod wire;

#[doc(inline)]
pub use decoder::FrameDecoder;
#[doc(inline)]
pub use error::FrameError;
#[doc(inline)]
pub use wire::{Channel, Focus, Frame, FrameKind, MAX_PAYLOAD_LEN};
