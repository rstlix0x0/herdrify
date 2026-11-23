//! The error every client operation returns, and the record of why a connection ended.
//!
//! A client call can fail because the core refused it, because a reply did not decode, because the
//! peer broke the protocol, because the transport failed or because the connection is gone. One
//! type names them all, so a caller matches on a single enum. A connection that has ended also
//! keeps one [`ConnectionEnd`], the first reason it ended, so every later call and every caller
//! that asks sees the same answer.
//!
//! Responsibilities:
//!
//! - [`Error`], with one variant per way a client operation fails
//! - [`ConnectionEnd`] and [`ConnectionEndKind`], the reason a connection ended
//! - [`EndRecord`], the place that reason is kept, where the first one recorded wins
//!
//! Non-responsibilities:
//!
//! - deciding when a connection ends, which the writer and the router do
//! - the errors a transport, the codecs and the core report, which `port` and `protocol` define

use std::error::Error as StdError;
use std::fmt;
use std::io;
use std::pin::pin;
use std::sync::OnceLock;

use thiserror::Error;
use tokio::sync::Notify;

use crate::port::TransportError;
use crate::protocol::{CoreVersion, MethodName, ProtocolError, RpcError};

/// A failure of a client operation.
///
/// # Examples
///
/// ```
/// use herdrify_hc::Error;
///
/// assert_eq!(Error::Closed.to_string(), "the connection to herdr-core is closed");
/// ```
#[derive(Debug, Error)]
pub enum Error {
    /// The transport failed while the client was using it.
    #[error("transport failed: {0}")]
    Transport(#[from] TransportError),
    /// The core refused a call.
    #[error("{method} refused by herdr-core: {error}")]
    Rpc {
        /// The method that was refused.
        method: MethodName,
        /// The refusal the core sent.
        #[source]
        error: RpcError,
    },
    /// A reply did not decode into the type its method returns.
    #[error("cannot decode the {method} result: {source}")]
    Decode {
        /// The method whose reply did not decode.
        method: MethodName,
        /// Why it did not decode.
        source: serde_json::Error,
    },
    /// The peer broke the protocol.
    #[error("herdr-core protocol violation: {0}")]
    Protocol(#[from] ProtocolError),
    /// The core does not speak protocol generation 1.
    #[error("herdr-core refused protocol generation 1: {message}")]
    UnsupportedGeneration {
        /// The core's message.
        message: String,
    },
    /// The core is older than the oldest version this client supports.
    #[error("herdr-core {found} is older than the supported minimum {minimum}")]
    UnsupportedVersion {
        /// The version the core reported.
        found: CoreVersion,
        /// The oldest supported version.
        minimum: CoreVersion,
    },
    /// The core reported a version that is not `MAJOR.MINOR.PATCH`.
    #[error("herdr-core reported version \"{found}\", which is not MAJOR.MINOR.PATCH")]
    InvalidVersion {
        /// The text the core reported.
        found: String,
    },
    /// A payload is longer than a frame may carry.
    #[error("payload of {len} bytes exceeds the 16777216-byte frame limit")]
    PayloadTooLarge {
        /// The refused payload length, in bytes.
        len: usize,
    },
    /// The connection to the core has ended.
    #[error("the connection to herdr-core is closed")]
    Closed,
    /// The attach stream has ended while the connection is still open.
    #[error("the terminal.attach stream is closed")]
    StreamClosed,
}

/// Why a connection ended.
///
/// # Examples
///
/// ```
/// use herdrify_hc::connection::ConnectionEndKind;
///
/// assert_eq!(ConnectionEndKind::PeerClosed.to_string(), "peer closed");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionEndKind {
    /// The core closed the connection.
    PeerClosed,
    /// Reading from or writing to the connection failed.
    TransportFailed,
    /// The core sent bytes that do not form a legal frame.
    ProtocolViolation,
}

impl fmt::Display for ConnectionEndKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::PeerClosed => "peer closed",
            Self::TransportFailed => "transport failed",
            Self::ProtocolViolation => "protocol violation",
        })
    }
}

/// The reason a connection ended: its kind and the text of what caused it.
///
/// # Examples
///
/// ```
/// use herdrify_hc::connection::{ConnectionEnd, ConnectionEndKind};
///
/// let end = ConnectionEnd {
///     kind: ConnectionEndKind::PeerClosed,
///     message: "herdr-core closed the connection".to_owned(),
/// };
/// assert_eq!(end.to_string(), "peer closed: herdr-core closed the connection");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionEnd {
    /// The kind of end.
    pub kind: ConnectionEndKind,
    /// The `Display` of the error that caused it, or a fixed sentence when no error did.
    pub message: String,
}

impl fmt::Display for ConnectionEnd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

/// Where a connection keeps the reason it ended.
///
/// The writer and the router can each find the connection broken at the same moment. Only the
/// first reason recorded is kept, so every caller that asks later sees the same one. A caller that
/// waits on something that may never come, such as a write the core has stopped reading, can wait
/// for the end with [`EndRecord::wait_ended`].
pub(super) struct EndRecord {
    end: OnceLock<ConnectionEnd>,
    ended: Notify,
}

impl EndRecord {
    pub(super) const fn new() -> Self {
        Self {
            end: OnceLock::new(),
            ended: Notify::const_new(),
        }
    }

    /// Records the end unless one is already recorded, in which case the new one is dropped.
    pub(super) fn record(&self, kind: ConnectionEndKind, message: String) {
        if self.end.set(ConnectionEnd { kind, message }).is_ok() {
            self.ended.notify_waiters();
        }
    }

    /// Resolves once the end is recorded, at once if it already is.
    pub(super) async fn wait_ended(&self) {
        let mut notified = pin!(self.ended.notified());
        // Registering before the check means an end recorded in between still wakes this call.
        notified.as_mut().enable();
        if !self.is_ended() {
            notified.await;
        }
    }

    /// The recorded end, or `None` while the connection is open.
    pub(super) fn get(&self) -> Option<ConnectionEnd> {
        self.end.get().cloned()
    }

    pub(super) fn is_ended(&self) -> bool {
        self.end.get().is_some()
    }
}

/// A request this crate builds itself that cannot be encoded.
///
/// Every request is built from a name that is never empty and from values that always serialize
/// to a single line, so this does not happen. It is reported as a transport failure, the nearest
/// variant, rather than ending the program.
pub(super) fn unencodable(source: impl StdError + Send + Sync + 'static) -> Error {
    Error::Transport(TransportError::Io(io::Error::other(source)))
}

#[cfg(test)]
mod tests {
    use std::error::Error as StdError;

    use super::{ConnectionEnd, ConnectionEndKind, EndRecord, Error};
    use crate::port::TransportError;
    use crate::protocol::{CoreVersion, ErrorKind, MethodName, ProtocolError, RpcError};

    type TestResult = Result<(), Box<dyn StdError>>;

    fn json_error() -> Result<serde_json::Error, &'static str> {
        serde_json::from_str::<u32>("-1")
            .err()
            .ok_or("`-1` parsed as a u32")
    }

    #[test]
    fn error_display_texts() -> TestResult {
        let decode = json_error()?;
        let decode_text = decode.to_string();
        let cases = [
            (
                Error::Transport(TransportError::Frame(
                    crate::protocol::FrameError::TooLarge { len: 16_777_217 },
                )),
                "transport failed: frame payload of 16777217 bytes exceeds the 16777216-byte limit"
                    .to_owned(),
            ),
            (
                Error::Rpc {
                    method: MethodName::new("terminal.get")?,
                    error: RpcError {
                        code: -32000,
                        kind: ErrorKind::NotFound,
                        message: "no terminal \"t999\"".to_owned(),
                        hint: None,
                        data: serde_json::Value::Null,
                    },
                },
                "terminal.get refused by herdr-core: no terminal \"t999\" [not_found]".to_owned(),
            ),
            (
                Error::Decode {
                    method: MethodName::new("terminal.get")?,
                    source: decode,
                },
                format!("cannot decode the terminal.get result: {decode_text}"),
            ),
            (
                Error::Protocol(ProtocolError::NotJsonRpc),
                "herdr-core protocol violation: message is not a JSON-RPC response or notification"
                    .to_owned(),
            ),
            (
                Error::UnsupportedGeneration {
                    message: "protocol generation 2 not supported".to_owned(),
                },
                "herdr-core refused protocol generation 1: protocol generation 2 not supported"
                    .to_owned(),
            ),
            (
                Error::UnsupportedVersion {
                    found: CoreVersion::parse("0.1.20")?,
                    minimum: CoreVersion::MINIMUM,
                },
                "herdr-core 0.1.20 is older than the supported minimum 0.1.21".to_owned(),
            ),
            (
                Error::InvalidVersion {
                    found: "v1".to_owned(),
                },
                "herdr-core reported version \"v1\", which is not MAJOR.MINOR.PATCH".to_owned(),
            ),
            (
                Error::PayloadTooLarge { len: 16_777_217 },
                "payload of 16777217 bytes exceeds the 16777216-byte frame limit".to_owned(),
            ),
            (
                Error::Closed,
                "the connection to herdr-core is closed".to_owned(),
            ),
            (
                Error::StreamClosed,
                "the terminal.attach stream is closed".to_owned(),
            ),
        ];
        for (error, text) in cases {
            assert_eq!(error.to_string(), text);
        }
        Ok(())
    }

    #[test]
    fn wrapping_variants_chain_their_source() -> TestResult {
        let transport = Error::from(TransportError::Frame(
            crate::protocol::FrameError::TooLarge { len: 16_777_217 },
        ));
        assert!(matches!(transport, Error::Transport(_)));
        assert!(transport.source().is_some());
        let protocol = Error::from(ProtocolError::NotJsonRpc);
        assert!(matches!(protocol, Error::Protocol(_)));
        assert!(protocol.source().is_some());
        let decode = Error::Decode {
            method: MethodName::new("x")?,
            source: json_error()?,
        };
        assert!(decode.source().is_some());
        assert!(Error::Closed.source().is_none());
        Ok(())
    }

    #[test]
    fn connection_end_display() {
        let end = ConnectionEnd {
            kind: ConnectionEndKind::PeerClosed,
            message: "x".to_owned(),
        };
        assert_eq!(end.to_string(), "peer closed: x");
        assert_eq!(
            ConnectionEndKind::TransportFailed.to_string(),
            "transport failed"
        );
        assert_eq!(
            ConnectionEndKind::ProtocolViolation.to_string(),
            "protocol violation"
        );
    }

    #[test]
    fn errors_are_send_sync_static() {
        fn assert_bounds<T: Send + Sync + 'static>() {}
        assert_bounds::<Error>();
        assert_bounds::<ConnectionEnd>();
        assert_bounds::<ConnectionEndKind>();
    }

    #[test]
    fn the_first_recorded_end_wins() {
        let record = EndRecord::new();
        assert_eq!(record.get(), None);
        assert!(!record.is_ended());
        record.record(ConnectionEndKind::PeerClosed, "first".to_owned());
        record.record(ConnectionEndKind::TransportFailed, "second".to_owned());
        assert!(record.is_ended());
        assert_eq!(
            record.get(),
            Some(ConnectionEnd {
                kind: ConnectionEndKind::PeerClosed,
                message: "first".to_owned(),
            })
        );
    }

    #[tokio::test]
    async fn wait_ended_resolves_when_the_end_is_recorded() -> Result<(), Box<dyn StdError>> {
        let record = std::sync::Arc::new(EndRecord::new());
        let waiting = {
            let record = std::sync::Arc::clone(&record);
            tokio::spawn(async move { record.wait_ended().await })
        };
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        assert!(!waiting.is_finished());
        record.record(ConnectionEndKind::PeerClosed, "gone".to_owned());
        tokio::time::timeout(std::time::Duration::from_secs(5), waiting).await??;
        tokio::time::timeout(std::time::Duration::from_secs(5), record.wait_ended()).await?;
        Ok(())
    }
}
