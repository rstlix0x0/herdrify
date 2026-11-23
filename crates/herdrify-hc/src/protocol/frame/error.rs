//! The errors the frame codec returns.
//!
//! A frame is refused for one of two reasons: its payload is larger than the wire allows, or a
//! control frame carries a payload of the wrong size. One small type names both so the decoder, the
//! encoder and the control readers share the same refusal and the same message.
//!
//! Responsibilities:
//!
//! - [`FrameError`], which names the refusal and the sizes involved
//!
//! Non-responsibilities:
//!
//! - deciding when a frame is refused, which the frame and the decoder do

use thiserror::Error;

use super::FrameKind;

/// A frame the codec refuses to encode, decode or read.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::FrameError;
///
/// let error = FrameError::TooLarge { len: 16_777_217 };
/// assert_eq!(
///     error.to_string(),
///     "frame payload of 16777217 bytes exceeds the 16777216-byte limit"
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FrameError {
    /// A payload is longer than the 16777216 bytes a frame may carry.
    #[error("frame payload of {len} bytes exceeds the 16777216-byte limit")]
    TooLarge {
        /// The refused payload length, in bytes.
        len: usize,
    },
    /// A control frame's payload is not the size its kind requires.
    #[error("{kind} frame payload must be {expected} bytes, got {got}")]
    BadControlPayload {
        /// The control kind being read.
        kind: FrameKind,
        /// The payload size the kind requires, in bytes.
        expected: usize,
        /// The payload size the frame carried, in bytes.
        got: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::FrameError;
    use crate::protocol::FrameKind;

    #[test]
    fn too_large_names_the_length() {
        assert_eq!(
            FrameError::TooLarge { len: 20_000_000 }.to_string(),
            "frame payload of 20000000 bytes exceeds the 16777216-byte limit"
        );
    }

    #[test]
    fn bad_control_payload_names_the_kind_in_upper_case() {
        let error = FrameError::BadControlPayload {
            kind: FrameKind::Resize,
            expected: 4,
            got: 5,
        };
        assert_eq!(
            error.to_string(),
            "RESIZE frame payload must be 4 bytes, got 5"
        );
    }

    #[test]
    fn frame_error_is_send_sync_static() {
        fn assert_bounds<T: Send + Sync + 'static>() {}
        assert_bounds::<FrameError>();
    }
}
