//! The error every `HerdrCore` operation returns.
//!
//! `HerdrCore` takes plain values and checks them only when a call is awaited, so one error type
//! covers a value the request cannot carry, a transport that cannot start, and every failure of
//! the [`Client`](crate::Client) call underneath.
//!
//! Responsibilities:
//!
//! - [`HerdrCoreError`], with one variant per way a `HerdrCore` operation fails
//!
//! Non-responsibilities:
//!
//! - the failures of a call itself, which [`Error`] names
//! - the failures of a transport, which [`TransportError`] names

use thiserror::Error;

use crate::connection::Error;
use crate::port::TransportError;
use crate::protocol::{EmptyValueError, RpcError};

/// A failure of a [`HerdrCore`](super::HerdrCore) operation.
///
/// # Examples
///
/// ```
/// use herdrify_hc::HerdrCoreError;
/// use herdrify_hc::protocol::EmptyValueError;
///
/// let error = HerdrCoreError::from(EmptyValueError { what: "argv" });
/// assert_eq!(error.to_string(), "argv must not be empty");
/// ```
#[derive(Debug, Error)]
pub enum HerdrCoreError {
    /// A value given as a string was empty where the request needs one.
    #[error(transparent)]
    Empty(#[from] EmptyValueError),
    /// A terminal size was zero.
    #[error("{what} must not be zero")]
    Zero {
        /// What was zero, such as `terminal columns`.
        what: &'static str,
    },
    /// The command that reaches the core could not be started.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// The call failed: the core refused it, the reply did not decode, or the connection ended.
    ///
    /// The client's error is boxed so that every `Result` of this crate's entrypoint stays small.
    #[error(transparent)]
    Call(Box<Error>),
}

impl HerdrCoreError {
    /// The core's refusal, when the core refused the call: its stable
    /// [`ErrorKind`](crate::protocol::ErrorKind) and its message.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use herdrify_hc::HerdrCore;
    /// use herdrify_hc::protocol::ErrorKind;
    ///
    /// # async fn run() -> Result<(), herdrify_hc::HerdrCoreError> {
    /// let core = HerdrCore::local().await?;
    /// match core.terminals().handle("t99")?.get().await {
    ///     Err(error) if error.refusal().is_some_and(|r| r.kind == ErrorKind::NotFound) => {
    ///         println!("no such terminal");
    ///     }
    ///     other => println!("{:?}", other?.state),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn refusal(&self) -> Option<&RpcError> {
        match self {
            Self::Call(error) => match error.as_ref() {
                Error::Rpc { error, .. } => Some(error),
                _ => None,
            },
            _ => None,
        }
    }
}

impl From<Error> for HerdrCoreError {
    fn from(error: Error) -> Self {
        Self::Call(Box::new(error))
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::HerdrCoreError;
    use crate::connection::Error;
    use crate::port::TransportError;
    use crate::protocol::{EmptyValueError, ErrorKind, MethodName, RpcError};

    #[test]
    fn refusal_is_the_core_s_error_and_only_that() -> Result<(), Box<dyn std::error::Error>> {
        let error: RpcError = serde_json::from_str(
            r#"{"code":-32001,"message":"no terminal","data":{"kind":"not_found"}}"#,
        )?;
        let refused = HerdrCoreError::from(Error::Rpc {
            method: MethodName::new("terminal.get")?,
            error,
        });
        assert_eq!(
            refused.refusal().map(|refusal| refusal.kind.clone()),
            Some(ErrorKind::NotFound)
        );
        assert!(HerdrCoreError::from(Error::Closed).refusal().is_none());
        assert!(
            HerdrCoreError::from(EmptyValueError { what: "argv" })
                .refusal()
                .is_none()
        );
        Ok(())
    }

    #[test]
    fn empty_shows_the_value_that_was_empty() {
        let error = HerdrCoreError::from(EmptyValueError {
            what: "session name",
        });
        assert_eq!(error.to_string(), "session name must not be empty");
    }

    #[test]
    fn zero_names_what_was_zero() {
        let error = HerdrCoreError::Zero {
            what: "terminal rows",
        };
        assert_eq!(error.to_string(), "terminal rows must not be zero");
    }

    #[test]
    fn transport_and_call_show_their_cause_unchanged_and_the_call_error_is_kept() {
        let spawn = TransportError::Spawn {
            program: "hc".to_owned(),
            source: io::Error::from(io::ErrorKind::NotFound),
        };
        let shown = spawn.to_string();
        assert_eq!(HerdrCoreError::from(spawn).to_string(), shown);
        let closed = HerdrCoreError::from(Error::Closed);
        assert_eq!(closed.to_string(), "the connection to herdr-core is closed");
        assert!(matches!(closed, HerdrCoreError::Call(error) if matches!(*error, Error::Closed)));
    }
}
