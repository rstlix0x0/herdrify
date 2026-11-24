//! The refusal of a run that never started.
//!
//! A [`Transport`](super::Transport) that cannot start the `herdr-app` program reports it as a
//! [`TransportError`]. A program that started and then failed is not an error here: its exit status
//! and output are a [`RunOutput`](crate::protocol::RunOutput).
//!
//! Responsibilities:
//!
//! - [`TransportError`], whether the program was missing or could not be started for another reason
//!
//! Non-responsibilities:
//!
//! - what a started program's exit status means, which the protocol layer decides
//! - turning the spawn failure of a particular runtime into this error, which the implementor does

use std::io;

use thiserror::Error;

/// Why the program could not be started.
///
/// # Examples
///
/// ```
/// use herdrify_app::port::TransportError;
///
/// let refused = TransportError::NotFound {
///     program: "herdr-app".to_owned(),
/// };
/// assert_eq!(refused.to_string(), "cannot start `herdr-app`: it was not found");
/// ```
#[derive(Debug, Error)]
pub enum TransportError {
    /// The program was not found.
    #[error("cannot start `{program}`: it was not found")]
    NotFound {
        /// The program that was looked for.
        program: String,
    },
    /// The program could not be started for a reason other than being missing.
    #[error("cannot start `{program}`: {source}")]
    Io {
        /// The program that could not be started.
        program: String,
        /// The error the operating system reported.
        #[source]
        source: io::Error,
    },
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;

    use super::*;

    fn assert_send_sync_static<T: Send + Sync + 'static>() {}

    #[test]
    fn transport_error_display() {
        let missing = TransportError::NotFound {
            program: "herdr-app".to_owned(),
        };
        assert_eq!(
            missing.to_string(),
            "cannot start `herdr-app`: it was not found"
        );

        let failed = TransportError::Io {
            program: "herdr-app".to_owned(),
            source: io::Error::new(io::ErrorKind::PermissionDenied, "permission denied"),
        };
        assert_eq!(
            failed.to_string(),
            "cannot start `herdr-app`: permission denied"
        );
    }

    #[test]
    fn transport_error_keeps_its_source() -> Result<(), &'static str> {
        let failed = TransportError::Io {
            program: "herdr-app".to_owned(),
            source: io::Error::new(io::ErrorKind::PermissionDenied, "permission denied"),
        };
        let source = failed.source().ok_or("Io has a source")?;
        let source = source
            .downcast_ref::<io::Error>()
            .ok_or("the source is the io::Error")?;
        assert_eq!(source.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(source.to_string(), "permission denied");

        let missing = TransportError::NotFound {
            program: "herdr-app".to_owned(),
        };
        assert!(missing.source().is_none());
        Ok(())
    }

    #[test]
    fn transport_error_is_send_sync_static() {
        assert_send_sync_static::<TransportError>();
    }
}
