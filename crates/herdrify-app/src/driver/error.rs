//! Every way a call through the client can end without a reply.
//!
//! A call can fail before herdr-app runs, while it runs, or when its output is read. [`Error`] names
//! each case and carries the text herdr-app gave, so a caller can show it as it is or match on the
//! case.
//!
//! Responsibilities:
//!
//! - [`Error`], the failure of a run, of reading its output, of a version check and of a value a
//!   command refused
//! - turning the [`Failure`] of a classified run into the matching [`Error`]
//!
//! Non-responsibilities:
//!
//! - telling a failed run apart from a successful one, which
//!   [`classify`](crate::protocol::classify) does
//! - running the program, which a [`Transport`](crate::port::Transport) does

use thiserror::Error as ThisError;

use crate::port::TransportError;
use crate::protocol::{AppVersion, Exit, Failure, InvalidAppVersion};

/// Why a call through the client returned no reply.
///
/// `invocation` is the label of the invocation, such as `do tab.new`. `message` is herdr-app's
/// text. For [`NoWindow`](Self::NoWindow) and [`Cli`](Self::Cli) the leading `herdr-app: ` and
/// the trailing whitespace are removed. [`Refused`](Self::Refused) keeps the `error` text of the
/// window's reply unchanged. Only [`Unexpected`](Self::Unexpected) writes `(no output)` for an
/// empty `stderr`. The other variants show their `message` as it is, even when it is empty.
///
/// # Examples
///
/// ```
/// use herdrify_app::driver::Error;
///
/// let refused = Error::Refused {
///     invocation: "do tab.new".to_owned(),
///     message: "tab.new focus is true or false".to_owned(),
/// };
/// assert_eq!(
///     refused.to_string(),
///     "herdr-app do tab.new refused: tab.new focus is true or false",
/// );
/// ```
#[derive(Debug, ThisError)]
pub enum Error {
    /// The program could not be started.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// The window answered with an error.
    #[error("herdr-app {invocation} refused: {message}")]
    Refused {
        /// The label of the invocation that was refused.
        invocation: String,
        /// The text of the window's error.
        message: String,
    },
    /// There is no window to answer, so the command could not be delivered.
    #[error("herdr-app has no window to answer: {message}")]
    NoWindow {
        /// herdr-app's description of the missing window.
        message: String,
    },
    /// herdr-app rejected the command line itself.
    #[error("herdr-app did not accept the invocation: {message}")]
    Cli {
        /// herdr-app's description of what it rejected.
        message: String,
    },
    /// The run ended in a way no other variant describes.
    #[error("{}", unexpected(*.exit, .stderr))]
    Unexpected {
        /// How the process ended.
        exit: Exit,
        /// Standard error with surrounding whitespace removed, possibly empty.
        stderr: String,
    },
    /// The output of a successful run could not be read.
    #[error("cannot read the output of herdr-app {invocation}: {detail}")]
    Decode {
        /// The label of the invocation whose output was read.
        invocation: String,
        /// Why the output could not be read.
        detail: String,
    },
    /// The text herdr-app printed for its version is not a version.
    #[error(transparent)]
    InvalidVersion(#[from] InvalidAppVersion),
    /// herdr-app is older than the oldest version this crate supports.
    #[error("herdr-app {found} is older than {minimum}, the oldest version this crate supports")]
    UnsupportedVersion {
        /// The version herdr-app reported.
        found: AppVersion,
        /// The oldest version this crate supports.
        minimum: AppVersion,
    },
    /// A value was refused before anything ran.
    #[error("invalid {argument}: {detail}")]
    InvalidArgument {
        /// The name of the argument the value was for.
        argument: String,
        /// Why the value was refused.
        detail: String,
    },
}

impl Error {
    /// The error for a run that failed: the [`Failure`] with the label of its invocation.
    pub(crate) fn from_failure(invocation: &str, failure: Failure) -> Self {
        match failure {
            Failure::Refused { message } => Self::Refused {
                invocation: invocation.to_owned(),
                message,
            },
            Failure::NoWindow { message } => Self::NoWindow { message },
            Failure::Cli { message } => Self::Cli { message },
            Failure::Unexpected { exit, stderr } => Self::Unexpected { exit, stderr },
        }
    }
}

/// The text of an unexpected ending, with `(no output)` for an empty `stderr`.
fn unexpected(exit: Exit, stderr: &str) -> String {
    let stderr = if stderr.is_empty() {
        "(no output)"
    } else {
        stderr
    };
    match exit {
        Exit::Code(code) => format!("herdr-app exited with code {code}: {stderr}"),
        Exit::Signal => format!("herdr-app was ended by a signal: {stderr}"),
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;

    fn assert_send_sync_static<T: Send + Sync + 'static>() {}

    #[test]
    fn error_display_per_variant() -> Result<(), InvalidAppVersion> {
        let transport = Error::Transport(TransportError::NotFound {
            program: "herdr-app".to_owned(),
        });
        assert_eq!(
            transport.to_string(),
            "cannot start `herdr-app`: it was not found"
        );

        let transport = Error::Transport(TransportError::Io {
            program: "herdr-app".to_owned(),
            source: io::Error::new(io::ErrorKind::PermissionDenied, "permission denied"),
        });
        assert_eq!(
            transport.to_string(),
            "cannot start `herdr-app`: permission denied"
        );

        let refused = Error::Refused {
            invocation: "do tab.new".to_owned(),
            message: "tab.new focus is true or false".to_owned(),
        };
        assert_eq!(
            refused.to_string(),
            "herdr-app do tab.new refused: tab.new focus is true or false"
        );

        let no_window = Error::NoWindow {
            message: "no Herdr window at /tmp/hzp/herdr-app.sock".to_owned(),
        };
        assert_eq!(
            no_window.to_string(),
            "herdr-app has no window to answer: no Herdr window at /tmp/hzp/herdr-app.sock"
        );

        let cli = Error::Cli {
            message: "args are not JSON".to_owned(),
        };
        assert_eq!(
            cli.to_string(),
            "herdr-app did not accept the invocation: args are not JSON"
        );

        let code = Error::Unexpected {
            exit: Exit::Code(3),
            stderr: "boom".to_owned(),
        };
        assert_eq!(code.to_string(), "herdr-app exited with code 3: boom");

        let signal = Error::Unexpected {
            exit: Exit::Signal,
            stderr: "boom".to_owned(),
        };
        assert_eq!(signal.to_string(), "herdr-app was ended by a signal: boom");

        let decode = Error::Decode {
            invocation: "ls".to_owned(),
            detail: "missing field `spaces` at line 1 column 13".to_owned(),
        };
        assert_eq!(
            decode.to_string(),
            "cannot read the output of herdr-app ls: missing field `spaces` at line 1 column 13"
        );

        let invalid = Error::InvalidVersion(InvalidAppVersion {
            text: "herdr-app 0.1".to_owned(),
        });
        assert_eq!(
            invalid.to_string(),
            r#"cannot read a herdr-app version from "herdr-app 0.1""#
        );

        let unsupported = Error::UnsupportedVersion {
            found: AppVersion::parse("herdr-app 0.1.15")?,
            minimum: AppVersion::MINIMUM,
        };
        assert_eq!(
            unsupported.to_string(),
            "herdr-app 0.1.15 is older than 0.1.16, the oldest version this crate supports"
        );

        let argument = Error::InvalidArgument {
            argument: "name".to_owned(),
            detail: "a space name cannot be empty".to_owned(),
        };
        assert_eq!(
            argument.to_string(),
            "invalid name: a space name cannot be empty"
        );
        Ok(())
    }

    #[test]
    fn unexpected_shows_no_output_for_empty_stderr() {
        let code = Error::Unexpected {
            exit: Exit::Code(3),
            stderr: String::new(),
        };
        assert_eq!(
            code.to_string(),
            "herdr-app exited with code 3: (no output)"
        );

        let signal = Error::Unexpected {
            exit: Exit::Signal,
            stderr: String::new(),
        };
        assert_eq!(
            signal.to_string(),
            "herdr-app was ended by a signal: (no output)"
        );
    }

    #[test]
    fn cli_shows_an_empty_message_as_is() {
        let cli = Error::Cli {
            message: String::new(),
        };
        assert_eq!(cli.to_string(), "herdr-app did not accept the invocation: ");
    }

    #[test]
    fn from_failure_keeps_the_label_and_the_text() {
        let refused = Error::from_failure(
            "do tab.new",
            Failure::Refused {
                message: "x".to_owned(),
            },
        );
        assert_eq!(refused.to_string(), "herdr-app do tab.new refused: x");

        let no_window = Error::from_failure(
            "ls",
            Failure::NoWindow {
                message: "no Herdr window at /tmp/s".to_owned(),
            },
        );
        assert_eq!(
            no_window.to_string(),
            "herdr-app has no window to answer: no Herdr window at /tmp/s"
        );

        let cli = Error::from_failure(
            "ls",
            Failure::Cli {
                message: "bad".to_owned(),
            },
        );
        assert_eq!(
            cli.to_string(),
            "herdr-app did not accept the invocation: bad"
        );

        let unexpected = Error::from_failure(
            "ls",
            Failure::Unexpected {
                exit: Exit::Code(9),
                stderr: String::new(),
            },
        );
        assert_eq!(
            unexpected.to_string(),
            "herdr-app exited with code 9: (no output)"
        );
    }

    #[test]
    fn error_is_send_sync_static() {
        assert_send_sync_static::<Error>();
    }
}
