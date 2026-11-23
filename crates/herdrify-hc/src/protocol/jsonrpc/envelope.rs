//! The JSON-RPC envelope: the request a client sends and the message a peer sends back.
//!
//! A channel-0 payload is one JSON object. It is a response when it carries an `id` and a `result` or
//! an `error`, and a notification when it carries a `method` and no `id`. The `result` and the
//! `params` stay `serde_json::Value` here; the typed reading of each happens afterwards, once the
//! caller knows which method a response answers.
//!
//! Responsibilities:
//!
//! - [`RequestId`], the number that pairs a response with its request
//! - [`Request`], the JSON-RPC request a client sends
//! - [`Incoming`], a received payload classified as a response or a notification
//!
//! Non-responsibilities:
//!
//! - reading a notification's parameters into events, which `notification` does
//! - choosing request ids or matching them to callers, which the client does

use std::fmt;

use serde::Serialize;
use serde_json::Value;

use super::RpcError;
use crate::protocol::ProtocolError;

/// The number that pairs a response with the request it answers.
///
/// The field is public so a caller writes `RequestId(1)`.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::RequestId;
///
/// assert_eq!(RequestId(7).get(), 7);
/// assert_eq!(RequestId(7).to_string(), "7");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct RequestId(pub u64);

impl RequestId {
    /// The request number.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A JSON-RPC 2.0 request.
///
/// It serializes as `{"jsonrpc":"2.0","id":…,"method":…,"params":…}`. The client always sends an
/// object as `params`, so the caller passes one.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::{Request, RequestId};
///
/// let request = Request::new(RequestId(7), "terminal.get", serde_json::json!({"terminal": "t7"}));
/// assert_eq!(
///     serde_json::to_string(&request)?,
///     r#"{"jsonrpc":"2.0","id":7,"method":"terminal.get","params":{"terminal":"t7"}}"#
/// );
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Request<'a> {
    jsonrpc: &'static str,
    id: RequestId,
    method: &'a str,
    params: Value,
}

impl<'a> Request<'a> {
    /// A request for `method` with `params`, tagged `id`.
    #[must_use]
    pub const fn new(id: RequestId, method: &'a str, params: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            method,
            params,
        }
    }
}

/// A payload received on channel 0, classified.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::{Incoming, RequestId};
///
/// let incoming = Incoming::classify(br#"{"id":1,"jsonrpc":"2.0","result":{"x":1}}"#)?;
/// assert_eq!(
///     incoming,
///     Incoming::Response {
///         id: Some(RequestId(1)),
///         outcome: Ok(serde_json::json!({"x": 1})),
///     }
/// );
/// # Ok::<(), herdrify_hc::protocol::ProtocolError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Incoming {
    /// A response to a request.
    Response {
        /// The id of the request it answers, or `None` when the peer sent `id: null`, as it does
        /// for a message it could not parse.
        id: Option<RequestId>,
        /// The `result` of a success, or the refusal of an error reply.
        outcome: Result<Value, RpcError>,
    },
    /// A message with no id: an event, a heartbeat or another notification.
    Notification {
        /// The notification's method word.
        method: String,
        /// The notification's parameters, `null` when it carried none.
        params: Value,
    },
}

impl Incoming {
    /// Classifies one payload.
    ///
    /// # Errors
    ///
    /// - [`ProtocolError::InvalidJson`] when `payload` is not JSON.
    /// - [`ProtocolError::NotJsonRpc`] when it is JSON that is neither a response nor a
    ///   notification: not an object, a request from the peer, a response with both or neither of
    ///   `result` and `error`, or an `error` that is not an error object. This includes a response
    ///   whose `id` is neither `null` nor an integer that fits a `u64`: the client never sends such
    ///   an id, so the reply cannot be its own.
    pub fn classify(payload: &[u8]) -> Result<Self, ProtocolError> {
        let value: Value = serde_json::from_slice(payload)
            .map_err(|source| ProtocolError::InvalidJson { source })?;
        let Value::Object(mut message) = value else {
            return Err(ProtocolError::NotJsonRpc);
        };

        let Some(id) = message.remove("id") else {
            return match message.remove("method") {
                Some(Value::String(method)) => Ok(Self::Notification {
                    method,
                    params: message.remove("params").unwrap_or(Value::Null),
                }),
                _ => Err(ProtocolError::NotJsonRpc),
            };
        };
        if message.contains_key("method") {
            return Err(ProtocolError::NotJsonRpc);
        }
        let id = match id {
            Value::Null => None,
            Value::Number(number) => {
                Some(RequestId(number.as_u64().ok_or(ProtocolError::NotJsonRpc)?))
            }
            _ => return Err(ProtocolError::NotJsonRpc),
        };
        let outcome =
            match (message.remove("result"), message.remove("error")) {
                (Some(result), None) => Ok(result),
                (None, Some(error)) => Err(serde_json::from_value::<RpcError>(error)
                    .map_err(|_| ProtocolError::NotJsonRpc)?),
                _ => return Err(ProtocolError::NotJsonRpc),
            };
        Ok(Self::Response { id, outcome })
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::json;

    use super::{Incoming, Request, RequestId};
    use crate::protocol::{ErrorKind, ProtocolError};

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn request_serializes_with_object_params() -> TestResult {
        let request = Request::new(RequestId(7), "terminal.get", json!({"terminal": "t7"}));
        assert_eq!(
            serde_json::to_value(&request)?,
            json!({"jsonrpc": "2.0", "id": 7, "method": "terminal.get", "params": {"terminal": "t7"}})
        );
        assert_eq!(
            serde_json::to_string(&request)?,
            r#"{"jsonrpc":"2.0","id":7,"method":"terminal.get","params":{"terminal":"t7"}}"#
        );
        Ok(())
    }

    #[test]
    fn classifies_a_result_response() -> TestResult {
        let incoming = Incoming::classify(br#"{"id":1,"jsonrpc":"2.0","result":{"x":1}}"#)?;
        assert_eq!(
            incoming,
            Incoming::Response {
                id: Some(RequestId(1)),
                outcome: Ok(json!({"x": 1})),
            }
        );
        Ok(())
    }

    #[test]
    fn classifies_an_error_response_with_null_id() -> TestResult {
        let wire = br#"{"error":{"code":-32700,"data":{"hint":"see `hc schema`","kind":"parse_error"},"message":"expected ident at line 1 column 2"},"id":null,"jsonrpc":"2.0"}"#;
        let Incoming::Response { id, outcome } = Incoming::classify(wire)? else {
            return Err("not a response".into());
        };
        assert_eq!(id, None);
        let error = outcome.err().ok_or("not an error outcome")?;
        assert_eq!(error.code, -32700);
        assert_eq!(error.kind, ErrorKind::ParseError);
        Ok(())
    }

    #[test]
    fn a_null_result_is_a_success() -> TestResult {
        let incoming = Incoming::classify(br#"{"id":2,"jsonrpc":"2.0","result":null}"#)?;
        assert_eq!(
            incoming,
            Incoming::Response {
                id: Some(RequestId(2)),
                outcome: Ok(json!(null)),
            }
        );
        Ok(())
    }

    #[test]
    fn classifies_a_notification() -> TestResult {
        let incoming =
            Incoming::classify(br#"{"jsonrpc":"2.0","method":"heartbeat","params":{"seq":42}}"#)?;
        assert_eq!(
            incoming,
            Incoming::Notification {
                method: "heartbeat".to_owned(),
                params: json!({"seq": 42}),
            }
        );
        let bare = Incoming::classify(br#"{"jsonrpc":"2.0","method":"heartbeat"}"#)?;
        assert_eq!(
            bare,
            Incoming::Notification {
                method: "heartbeat".to_owned(),
                params: json!(null),
            }
        );
        Ok(())
    }

    #[test]
    fn the_jsonrpc_member_is_not_required() -> TestResult {
        for version in [None, Some("1.0")] {
            let tag = version.map_or_else(String::new, |v| format!(r#""jsonrpc":"{v}","#));
            let response =
                Incoming::classify(format!(r#"{{{tag}"id":1,"result":{{}}}}"#).as_bytes())?;
            assert_eq!(
                response,
                Incoming::Response {
                    id: Some(RequestId(1)),
                    outcome: Ok(json!({})),
                }
            );
            let event = Incoming::classify(format!(r#"{{{tag}"method":"heartbeat"}}"#).as_bytes())?;
            assert_eq!(
                event,
                Incoming::Notification {
                    method: "heartbeat".to_owned(),
                    params: json!(null),
                }
            );
        }
        Ok(())
    }

    #[test]
    fn refuses_json_that_is_not_json_rpc() {
        assert!(matches!(
            Incoming::classify(b"[1,2]"),
            Err(ProtocolError::NotJsonRpc)
        ));
        assert!(matches!(
            Incoming::classify(b"{"),
            Err(ProtocolError::InvalidJson { .. })
        ));
        for wire in [
            &br#"{"jsonrpc":"2.0"}"#[..],
            br#"{"id":1,"jsonrpc":"2.0"}"#,
            br#"{"id":1,"result":1,"error":{"code":1,"message":"m"}}"#,
            br#"{"id":1,"error":"nope"}"#,
            br#"{"result":1}"#,
            br#"{"id":3,"method":"terminal.get"}"#,
            br#"{"method":5}"#,
        ] {
            assert!(
                matches!(Incoming::classify(wire), Err(ProtocolError::NotJsonRpc)),
                "{}",
                String::from_utf8_lossy(wire)
            );
        }
    }

    #[test]
    fn odd_response_ids_are_not_json_rpc() {
        for id in [r#""a""#, "-1", "1.5", "true", "[1]", "18446744073709551616"] {
            let wire = format!(r#"{{"id":{id},"jsonrpc":"2.0","result":{{}}}}"#);
            assert!(
                matches!(
                    Incoming::classify(wire.as_bytes()),
                    Err(ProtocolError::NotJsonRpc)
                ),
                "{wire}"
            );
        }
    }

    #[test]
    fn request_id_is_a_number() {
        assert_eq!(RequestId(7).get(), 7);
        assert_eq!(RequestId(7).to_string(), "7");
    }
}
