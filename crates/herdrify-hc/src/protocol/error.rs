//! The error for a herdr-core peer that breaks the protocol.
//!
//! These are not refusals the core reports in an RPC error; they are messages that cannot be what
//! the protocol says they are: text that is not JSON, JSON that is not JSON-RPC, a handshake reply
//! that does not match what was asked, a stream that never ends a line. One type names them all so
//! the connection can end with a single, precise reason.
//!
//! Responsibilities:
//!
//! - [`ProtocolError`], with one variant per way a peer can break the protocol
//! - [`UnexpectedReply`], which names what arrived in place of the hello reply
//!
//! Non-responsibilities:
//!
//! - detecting each violation, which the decoders, the envelope and the client do
//! - errors the core reports on purpose, which are RPC errors

use thiserror::Error;

use super::{FrameError, MethodName, RequestId};

/// What arrived where the hello reply was expected.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::{RequestId, UnexpectedReply};
///
/// assert_eq!(UnexpectedReply::Response { id: RequestId(3) }.to_string(), "a response with id 3");
/// assert_eq!(UnexpectedReply::NullIdResponse.to_string(), "a response with id null");
/// assert_eq!(UnexpectedReply::Notification.to_string(), "a notification");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum UnexpectedReply {
    /// A response to a different request.
    #[error("a response with id {id}")]
    Response {
        /// The request id the response answers.
        id: RequestId,
    },
    /// A response whose id is `null`.
    #[error("a response with id null")]
    NullIdResponse,
    /// A notification.
    #[error("a notification")]
    Notification,
}

/// A herdr-core message that breaks the protocol.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::ProtocolError;
///
/// assert_eq!(
///     ProtocolError::ResponseOnStream.to_string(),
///     "a JSON-RPC response arrived on an event stream"
/// );
/// ```
#[derive(Debug, Error)]
pub enum ProtocolError {
    /// A message is not valid JSON.
    #[error("invalid JSON from herdr-core: {source}")]
    InvalidJson {
        /// Why the text did not parse.
        source: serde_json::Error,
    },
    /// A message is valid JSON but neither a JSON-RPC response nor a notification.
    #[error("message is not a JSON-RPC response or notification")]
    NotJsonRpc,
    /// The hello reply does not confirm a framed connection at generation 1.
    #[error(
        "hello reply has framed={framed} and generation={generation}, expected framed=true and generation=1"
    )]
    HelloMismatch {
        /// The `framed` value the reply carried.
        framed: bool,
        /// The `generation` value the reply carried.
        generation: u32,
    },
    /// An event stream's unfinished line grew past 16777216 bytes.
    #[error("event line exceeds 16777216 bytes without a newline")]
    EventLineTooLong,
    /// Something other than the hello reply (request id 0) arrived first.
    #[error("expected the hello reply (id 0), got {got}")]
    UnexpectedHelloReply {
        /// What arrived instead.
        got: UnexpectedReply,
    },
    /// A reply that must name a stream channel on a framed connection has none.
    #[error("the {method} reply has no channel on a framed connection")]
    MissingChannel {
        /// The method whose reply lacks the channel.
        method: MethodName,
    },
    /// An event's data does not match the shape its type requires.
    #[error("cannot decode a {kind} event: {source}")]
    InvalidEvent {
        /// The event type word.
        kind: String,
        /// Why the data did not decode.
        source: serde_json::Error,
    },
    /// A notification's parameters do not match the shape its method requires.
    #[error("cannot decode a {method} notification: {source}")]
    InvalidNotification {
        /// The notification method word.
        method: String,
        /// Why the parameters did not decode.
        source: serde_json::Error,
    },
    /// A JSON-RPC response arrived on an event stream, which carries only notifications.
    #[error("a JSON-RPC response arrived on an event stream")]
    ResponseOnStream,
    /// A frame is malformed.
    #[error(transparent)]
    Frame(#[from] FrameError),
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::{ProtocolError, UnexpectedReply};
    use crate::protocol::{FrameError, MethodName, RequestId};

    fn json_error() -> Result<serde_json::Error, &'static str> {
        serde_json::from_str::<u32>("x")
            .err()
            .ok_or("`x` parsed as a u32")
    }

    #[test]
    fn protocol_error_display_texts() -> Result<(), Box<dyn Error>> {
        let source = json_error()?;
        let source_text = source.to_string();
        let cases = [
            (
                ProtocolError::InvalidJson { source },
                format!("invalid JSON from herdr-core: {source_text}"),
            ),
            (
                ProtocolError::NotJsonRpc,
                "message is not a JSON-RPC response or notification".to_owned(),
            ),
            (
                ProtocolError::HelloMismatch {
                    framed: false,
                    generation: 2,
                },
                "hello reply has framed=false and generation=2, expected framed=true and generation=1"
                    .to_owned(),
            ),
            (
                ProtocolError::EventLineTooLong,
                "event line exceeds 16777216 bytes without a newline".to_owned(),
            ),
            (
                ProtocolError::UnexpectedHelloReply {
                    got: UnexpectedReply::Response { id: RequestId(3) },
                },
                "expected the hello reply (id 0), got a response with id 3".to_owned(),
            ),
            (
                ProtocolError::UnexpectedHelloReply {
                    got: UnexpectedReply::NullIdResponse,
                },
                "expected the hello reply (id 0), got a response with id null".to_owned(),
            ),
            (
                ProtocolError::UnexpectedHelloReply {
                    got: UnexpectedReply::Notification,
                },
                "expected the hello reply (id 0), got a notification".to_owned(),
            ),
            (
                ProtocolError::MissingChannel {
                    method: MethodName::new("events.subscribe")?,
                },
                "the events.subscribe reply has no channel on a framed connection".to_owned(),
            ),
            (
                ProtocolError::InvalidEvent {
                    kind: "status".to_owned(),
                    source: json_error()?,
                },
                format!("cannot decode a status event: {source_text}"),
            ),
            (
                ProtocolError::InvalidNotification {
                    method: "heartbeat".to_owned(),
                    source: json_error()?,
                },
                format!("cannot decode a heartbeat notification: {source_text}"),
            ),
            (
                ProtocolError::ResponseOnStream,
                "a JSON-RPC response arrived on an event stream".to_owned(),
            ),
            (
                ProtocolError::Frame(FrameError::TooLarge { len: 16_777_217 }),
                "frame payload of 16777217 bytes exceeds the 16777216-byte limit".to_owned(),
            ),
        ];
        for (error, text) in cases {
            assert_eq!(error.to_string(), text);
        }
        Ok(())
    }

    #[test]
    fn source_chains_to_the_json_error_and_through_frame() -> Result<(), &'static str> {
        let invalid = ProtocolError::InvalidJson {
            source: json_error()?,
        };
        assert!(invalid.source().is_some());
        let framed = ProtocolError::from(FrameError::TooLarge { len: 1 });
        assert!(framed.source().is_none());
        assert!(ProtocolError::NotJsonRpc.source().is_none());
        Ok(())
    }

    #[test]
    fn protocol_error_is_send_sync_static() {
        fn assert_bounds<T: Send + Sync + 'static>() {}
        assert_bounds::<ProtocolError>();
    }
}
