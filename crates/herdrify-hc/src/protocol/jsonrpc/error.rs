//! `RpcError` and `ErrorKind`: a refusal herdr-core reports in a JSON-RPC error reply.
//!
//! The core answers a call it refuses with `{code, message, data: {kind, hint}}`. The `kind` is a
//! stable word a caller can branch on, and the core may add words, so a word not listed here is kept
//! as [`ErrorKind::Other`] and written back exactly as it was read.
//!
//! Responsibilities:
//!
//! - [`RpcError`], the error object read from a reply, with its whole `data` kept
//! - [`ErrorKind`], one variant per documented kind and `Other` for the rest
//!
//! Non-responsibilities:
//!
//! - telling an error reply from a result reply, which the envelope does
//! - naming the method that was refused, which the client does

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use thiserror::Error;

use crate::protocol::word::impl_word_identity;

/// Defines `ErrorKind` with one variant per wire word, the word accessors and the conversions.
macro_rules! error_kinds {
    ($($variant:ident => $word:literal,)+) => {
        /// The stable kind of a refusal, as an open set of words.
        ///
        /// A word outside the documented list is [`ErrorKind::Other`]. Equality and hashing follow
        /// the word, so `ErrorKind::Other("not_found".to_owned())` equals `ErrorKind::NotFound`.
        ///
        /// # Examples
        ///
        /// ```
        /// use herdrify_hc::protocol::ErrorKind;
        ///
        /// let known: ErrorKind = serde_json::from_str("\"not_found\"")?;
        /// assert_eq!(known, ErrorKind::NotFound);
        /// assert_eq!(ErrorKind::Other("not_found".to_owned()), known);
        ///
        /// let unknown: ErrorKind = serde_json::from_str("\"brand_new\"")?;
        /// assert_eq!(unknown.as_str(), "brand_new");
        /// # Ok::<(), serde_json::Error>(())
        /// ```
        #[derive(Debug, Clone)]
        pub enum ErrorKind {
            $(
                #[doc = concat!("The kind `", $word, "`.")]
                $variant,
            )+
            /// A kind the core sent that is not in the documented list, kept as read.
            Other(String),
        }

        impl ErrorKind {
            /// The word this kind is sent and read as.
            #[must_use]
            pub fn as_str(&self) -> &str {
                match self {
                    $(Self::$variant => $word,)+
                    Self::Other(word) => word,
                }
            }
        }

        impl From<String> for ErrorKind {
            fn from(word: String) -> Self {
                match word.as_str() {
                    $($word => Self::$variant,)+
                    _ => Self::Other(word),
                }
            }
        }
    };
}

impl_word_identity!(ErrorKind);

error_kinds! {
    AlreadyAdded => "already_added",
    Blocked => "blocked",
    Cancelled => "cancelled",
    Config => "config",
    Conflict => "conflict",
    EditFailed => "edit_failed",
    Finished => "finished",
    HcMissing => "hc_missing",
    Incompatible => "incompatible",
    Internal => "internal",
    InvalidParams => "invalid_params",
    InvalidTemplate => "invalid_template",
    Io => "io",
    Live => "live",
    MachineUnknown => "machine_unknown",
    NameTaken => "name_taken",
    NeedsFramed => "needs_framed",
    NeedsTarget => "needs_target",
    NeedsUpdate => "needs_update",
    NotFound => "not_found",
    NotText => "not_text",
    SpawnFailed => "spawn_failed",
    StatusMismatch => "status_mismatch",
    Unavailable => "unavailable",
    Unreachable => "unreachable",
    UnsupportedGeneration => "unsupported_generation",
    UnknownMethod => "unknown_method",
    ParseError => "parse_error",
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for ErrorKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ErrorKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::from)
    }
}

/// A refusal in a JSON-RPC error reply.
///
/// `kind` and `hint` are read from the reply's `data` object, which is also kept whole in `data`.
/// A reply whose `data` has no string `kind` reads as [`ErrorKind::Other`] with an empty word, and
/// a reply with no `data` keeps `Value::Null`, so a refusal is never lost to a missing field.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::{ErrorKind, RpcError};
///
/// let error: RpcError = serde_json::from_str(
///     r#"{"code":-32000,"data":{"hint":"see `hc ls --all`","kind":"not_found"},"message":"no terminal \"t999\""}"#,
/// )?;
/// assert_eq!(error.kind, ErrorKind::NotFound);
/// assert_eq!(error.to_string(), r#"no terminal "t999" [not_found]"#);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message} [{kind}]")]
pub struct RpcError {
    /// The JSON-RPC error code, such as `-32000`.
    pub code: i64,
    /// The stable kind of the refusal.
    pub kind: ErrorKind,
    /// The core's message.
    pub message: String,
    /// What the caller can do about it, when the core said.
    pub hint: Option<String>,
    /// The reply's whole `data` object, or `null` when the reply had none.
    pub data: Value,
}

/// The error object as it is on the wire.
#[derive(Deserialize)]
struct WireError {
    code: i64,
    message: String,
    #[serde(default)]
    data: Value,
}

impl<'de> Deserialize<'de> for RpcError {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let WireError {
            code,
            message,
            data,
        } = WireError::deserialize(deserializer)?;
        let word = |key: &str| data.get(key).and_then(Value::as_str).map(str::to_owned);
        let kind = ErrorKind::from(word("kind").unwrap_or_default());
        let hint = word("hint");
        Ok(Self {
            code,
            kind,
            message,
            hint,
            data,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::hash::{BuildHasher, RandomState};

    use serde_json::json;

    use super::{ErrorKind, RpcError};

    type TestResult = Result<(), Box<dyn Error>>;

    const WORDS: [&str; 28] = [
        "already_added",
        "blocked",
        "cancelled",
        "config",
        "conflict",
        "edit_failed",
        "finished",
        "hc_missing",
        "incompatible",
        "internal",
        "invalid_params",
        "invalid_template",
        "io",
        "live",
        "machine_unknown",
        "name_taken",
        "needs_framed",
        "needs_target",
        "needs_update",
        "not_found",
        "not_text",
        "spawn_failed",
        "status_mismatch",
        "unavailable",
        "unreachable",
        "unsupported_generation",
        "unknown_method",
        "parse_error",
    ];

    #[test]
    fn every_error_kind_decodes() -> TestResult {
        for word in WORDS {
            let json = format!("\"{word}\"");
            let kind: ErrorKind = serde_json::from_str(&json)?;
            assert!(!matches!(kind, ErrorKind::Other(_)), "{word} fell to Other");
            assert_eq!(kind.as_str(), word);
            assert_eq!(kind.to_string(), word);
            assert_eq!(serde_json::to_string(&kind)?, json);
        }
        Ok(())
    }

    #[test]
    fn unknown_error_kind_is_other() -> TestResult {
        let kind: ErrorKind = serde_json::from_str(r#""brand_new""#)?;
        assert_eq!(kind, ErrorKind::Other("brand_new".to_owned()));
        assert_eq!(kind.as_str(), "brand_new");
        assert_eq!(serde_json::to_string(&kind)?, r#""brand_new""#);
        Ok(())
    }

    #[test]
    fn a_named_kind_equals_other_holding_the_same_word() {
        let state = RandomState::new();
        for word in WORDS {
            let named = ErrorKind::from(word.to_owned());
            let other = ErrorKind::Other(word.to_owned());
            assert_eq!(other, named, "{word}");
            assert_eq!(named, other, "{word}");
            assert_eq!(state.hash_one(&other), state.hash_one(&named), "{word}");
        }
    }

    #[test]
    fn other_kinds_are_equal_only_when_the_words_are() {
        let state = RandomState::new();
        let x = ErrorKind::Other("x".to_owned());
        let y = ErrorKind::Other("y".to_owned());
        assert_ne!(x, y);
        assert_eq!(x, ErrorKind::Other("x".to_owned()));
        assert_eq!(
            state.hash_one(&x),
            state.hash_one(ErrorKind::Other("x".to_owned()))
        );
        assert_ne!(x, ErrorKind::NotFound);
    }

    #[test]
    fn rpc_error_decodes_a_live_reply() -> TestResult {
        let wire = r#"{"code":-32000,"data":{"hint":"see `hc ls --all`","kind":"not_found"},"message":"no terminal \"t999\""}"#;
        let error: RpcError = serde_json::from_str(wire)?;
        assert_eq!(error.code, -32000);
        assert_eq!(error.kind, ErrorKind::NotFound);
        assert_eq!(error.message, r#"no terminal "t999""#);
        assert_eq!(error.hint.as_deref(), Some("see `hc ls --all`"));
        assert_eq!(error.to_string(), r#"no terminal "t999" [not_found]"#);
        assert_eq!(
            error.data,
            json!({"hint": "see `hc ls --all`", "kind": "not_found"})
        );
        Ok(())
    }

    #[test]
    fn rpc_error_keeps_the_hint() -> TestResult {
        let wire = r#"{"code":-32000,"data":{"hint":"no machines yet: hc machine add <ssh-target>","kind":"machine_unknown"},"message":"unknown machine \"nope\""}"#;
        let error: RpcError = serde_json::from_str(wire)?;
        assert_eq!(error.kind, ErrorKind::MachineUnknown);
        assert_eq!(
            error.hint.as_deref(),
            Some("no machines yet: hc machine add <ssh-target>")
        );
        Ok(())
    }

    #[test]
    fn rpc_error_without_data_keeps_the_refusal() -> TestResult {
        let error: RpcError =
            serde_json::from_str(r#"{"code":-32601,"message":"no such method"}"#)?;
        assert_eq!(error.code, -32601);
        assert_eq!(error.kind, ErrorKind::Other(String::new()));
        assert_eq!(error.hint, None);
        assert_eq!(error.data, json!(null));
        Ok(())
    }

    #[test]
    fn rpc_error_is_send_sync_static() {
        fn assert_bounds<T: Send + Sync + 'static + Error>() {}
        assert_bounds::<RpcError>();
    }
}
