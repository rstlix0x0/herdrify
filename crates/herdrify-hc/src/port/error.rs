//! The error a transport returns.
//!
//! A transport can fail in three ways: the process it drives cannot be started, the byte stream
//! fails, or the bytes on it do not form a legal frame. One type names all three so the client can
//! end a connection with a single reason and a chained cause.
//!
//! Responsibilities:
//!
//! - [`TransportError`], with one variant per way a transport fails
//!
//! Non-responsibilities:
//!
//! - deciding when a transport fails, which each adapter does
//! - the frame refusals themselves, which `protocol` defines

use std::io;

use thiserror::Error;

use crate::protocol::FrameError;

/// A failure of the transport that carries messages to and from herdr-core.
///
/// # Examples
///
/// ```
/// use std::io;
///
/// use herdrify_hc::TransportError;
///
/// let error = TransportError::Io(io::Error::from(io::ErrorKind::BrokenPipe));
/// assert!(error.to_string().starts_with("I/O error: "));
/// ```
#[derive(Debug, Error)]
pub enum TransportError {
    /// The program a transport drives could not be started.
    #[error("cannot start {program}: {source}")]
    Spawn {
        /// The program that was to be started.
        program: String,
        /// Why it could not be started.
        source: io::Error,
    },
    /// Reading from or writing to the connection failed.
    #[error("I/O error: {0}")]
    Io(#[source] io::Error),
    /// The bytes on the connection do not form a legal frame.
    #[error(transparent)]
    Frame(#[from] FrameError),
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::TransportError;
    use crate::protocol::FrameError;

    #[test]
    fn spawn_display() {
        let error = TransportError::Spawn {
            program: "hc".to_owned(),
            source: io::Error::from(io::ErrorKind::NotFound),
        };
        assert!(error.to_string().starts_with("cannot start hc: "));
    }

    #[test]
    fn io_display() {
        let error = TransportError::Io(io::Error::from(io::ErrorKind::BrokenPipe));
        assert!(error.to_string().starts_with("I/O error: "));
    }

    #[test]
    fn frame_is_transparent() {
        let error = TransportError::Frame(FrameError::TooLarge { len: 16_777_217 });
        assert_eq!(
            error.to_string(),
            "frame payload of 16777217 bytes exceeds the 16777216-byte limit"
        );
    }

    #[test]
    fn a_frame_error_converts_into_the_frame_variant() {
        let error = TransportError::from(FrameError::TooLarge { len: 16_777_217 });
        assert!(matches!(
            error,
            TransportError::Frame(FrameError::TooLarge { len: 16_777_217 })
        ));
    }

    #[test]
    fn transport_error_is_send_sync_static() {
        fn assert_bounds<T: Send + Sync + 'static>() {}
        assert_bounds::<TransportError>();
    }
}
