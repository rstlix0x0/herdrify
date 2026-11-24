//! The refusal of a run that produced no output.
//!
//! A [`Transport`](super::Transport) that cannot start the `herdr-app` program reports it as a
//! [`TransportError`], and so does one whose started program could not be waited for or read from. A
//! program that started and then failed is not an error here: its exit status and output are a
//! [`RunOutput`](crate::protocol::RunOutput).
//!
//! Responsibilities:
//!
//! - [`TransportError`], whether the program was missing, could not be started for another reason, or
//!   hit an operating system failure while it ran
//!
//! Non-responsibilities:
//!
//! - what a started program's exit status means, which the protocol layer decides
//! - starting the program, which an implementor of [`Transport`](super::Transport) does and then
//!   reports through this error

use std::ffi::OsStr;
use std::io;

use thiserror::Error;

/// Why a run produced no output.
///
/// Both variants read "cannot start" in their message, but [`Io`](Self::Io) also covers an operating
/// system failure after the program started, so the message alone does not say which happened.
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
    /// The program could not be started for a reason other than being missing, or the operating
    /// system failed while waiting for the started program or reading its output.
    #[error("cannot start `{program}`: {source}")]
    Io {
        /// The program that was run.
        program: String,
        /// The error the operating system reported.
        #[source]
        source: io::Error,
    },
}

impl TransportError {
    /// Maps the error of starting `program` to a [`TransportError`]: a missing program is
    /// [`NotFound`](Self::NotFound), any other error is [`Io`](Self::Io) and keeps its source.
    ///
    /// A program name that is not valid Unicode is shown with its invalid parts replaced.
    pub(crate) fn from_spawn(program: &OsStr, source: io::Error) -> Self {
        let program = program.to_string_lossy().into_owned();
        match source.kind() {
            io::ErrorKind::NotFound => Self::NotFound { program },
            _ => Self::Io { program, source },
        }
    }

    /// Maps the error of waiting for `program`, after it started, to [`Io`](Self::Io) whatever its
    /// kind, because the program was found and a `NotFound` here would not be about the program.
    pub(crate) fn from_wait(program: &OsStr, source: io::Error) -> Self {
        Self::Io {
            program: program.to_string_lossy().into_owned(),
            source,
        }
    }
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
    fn from_spawn_maps_not_found() -> Result<(), &'static str> {
        let missing = TransportError::from_spawn(
            OsStr::new("herdr-app"),
            io::Error::from(io::ErrorKind::NotFound),
        );
        assert!(matches!(
            &missing,
            TransportError::NotFound { program } if program == "herdr-app"
        ));

        let denied = TransportError::from_spawn(
            OsStr::new("herdr-app"),
            io::Error::new(io::ErrorKind::PermissionDenied, "permission denied"),
        );
        let TransportError::Io { program, .. } = &denied else {
            return Err("PermissionDenied maps to Io");
        };
        assert_eq!(program, "herdr-app");
        let source = denied.source().ok_or("Io has a source")?;
        let source = source
            .downcast_ref::<io::Error>()
            .ok_or("the source is the io::Error")?;
        assert_eq!(source.kind(), io::ErrorKind::PermissionDenied);
        Ok(())
    }

    #[test]
    fn from_wait_is_io_even_for_not_found() -> Result<(), &'static str> {
        let waited = TransportError::from_wait(
            OsStr::new("herdr-app"),
            io::Error::from(io::ErrorKind::NotFound),
        );
        let TransportError::Io { program, source } = &waited else {
            return Err("a wait failure maps to Io");
        };
        assert_eq!(program, "herdr-app");
        assert_eq!(source.kind(), io::ErrorKind::NotFound);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn non_unicode_program_is_shown_lossily() {
        use std::os::unix::ffi::OsStrExt as _;

        let program = OsStr::from_bytes(b"herdr\xffapp");
        let missing = TransportError::from_spawn(program, io::Error::from(io::ErrorKind::NotFound));
        assert_eq!(
            missing.to_string(),
            "cannot start `herdr\u{FFFD}app`: it was not found"
        );
        let waited = TransportError::from_wait(program, io::Error::other("broken pipe"));
        assert_eq!(
            waited.to_string(),
            "cannot start `herdr\u{FFFD}app`: broken pipe"
        );
    }

    #[test]
    fn transport_error_is_send_sync_static() {
        assert_send_sync_static::<TransportError>();
    }
}
