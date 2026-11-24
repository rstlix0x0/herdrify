//! The replies to the methods that send a file to a terminal, read from JSON exactly as the schema
//! names their properties.
//!
//! They sit apart from the other terminal replies because they belong to one task, storing a file
//! in a terminal's directory, and are read beside the requests in the matching `put` module.
//!
//! Responsibilities:
//!
//! - [`PutResult`]: where a stored file landed and how many bytes it holds
//! - [`PutBeginResult`]: the transfer a `terminal.put_begin` opened
//! - [`PutPieceResult`]: how many bytes a transfer holds after a `terminal.put_chunk`
//! - [`EmptyResult`]: the reply that carries nothing
//!
//! Non-responsibilities:
//!
//! - the other replies to the `terminal.*` methods, which `results` holds
//! - deciding what the core accepts or reports, which the core decides

use std::fmt;

use serde::de::{Deserializer, IgnoredAny, MapAccess, Visitor};
use serde::{Deserialize, Serialize};

use crate::protocol::TransferId;

/// The reply to `terminal.put_begin`.
///
/// It names the transfer the core opened, which the later calls of the transfer carry.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::PutBeginResult;
///
/// let reply: PutBeginResult = serde_json::from_str(r#"{"transfer":"0123456789abcdef"}"#)?;
/// assert_eq!(reply.transfer.as_str(), "0123456789abcdef");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PutBeginResult {
    /// The transfer the core opened.
    pub transfer: TransferId,
}

/// The reply to `terminal.put_chunk`.
///
/// It says how much of the file the core holds so far.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::PutPieceResult;
///
/// let reply: PutPieceResult = serde_json::from_str(r#"{"bytes":5}"#)?;
/// assert_eq!(reply.bytes, 5);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PutPieceResult {
    /// How many bytes of the transfer the core has received so far.
    pub bytes: u64,
}

/// A reply that carries nothing, the schema `Empty`.
///
/// Only `terminal.put_abort` returns it today. It reads from a JSON object and from nothing else.
/// A property in the object is ignored, so a later core that adds one does not break the read.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::EmptyResult;
///
/// let reply: EmptyResult = serde_json::from_str("{}")?;
/// assert_eq!(reply, EmptyResult {});
/// assert_eq!(serde_json::to_string(&reply)?, "{}");
/// assert!(serde_json::from_str::<EmptyResult>("[]").is_err());
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EmptyResult {}

impl<'de> Deserialize<'de> for EmptyResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ObjectVisitor;

        impl<'de> Visitor<'de> for ObjectVisitor {
            type Value = EmptyResult;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON object")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
                Ok(EmptyResult {})
            }
        }

        deserializer.deserialize_map(ObjectVisitor)
    }
}

/// The reply to `terminal.put` and `terminal.put_finish`.
///
/// It says where the file landed.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::PutResult;
///
/// let reply: PutResult = serde_json::from_str(r#"{"path":"/tmp/a.png","bytes":5}"#)?;
/// assert_eq!(reply.path, "/tmp/a.png");
/// assert_eq!(reply.bytes, 5);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PutResult {
    /// Where the file landed, as the terminal's machine sees it.
    pub path: String,
    /// How many bytes were stored.
    pub bytes: u64,
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::Value;

    use super::{EmptyResult, PutBeginResult, PutPieceResult, PutResult};

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn put_begin_result_decodes_and_validates_the_id() -> TestResult {
        let value: PutBeginResult = serde_json::from_str(r#"{"transfer":"0123456789abcdef"}"#)?;
        assert_eq!(value.transfer.as_str(), "0123456789abcdef");
        assert!(serde_json::from_str::<PutBeginResult>(r#"{"transfer":"x"}"#).is_err());
        Ok(())
    }

    #[test]
    fn put_piece_result_decodes_bytes() -> TestResult {
        let value: PutPieceResult = serde_json::from_str(r#"{"bytes":5}"#)?;
        assert_eq!(value.bytes, 5);
        Ok(())
    }

    #[test]
    fn empty_result_reads_and_writes_an_empty_object() -> TestResult {
        assert_eq!(serde_json::from_str::<EmptyResult>("{}")?, EmptyResult {});
        assert_eq!(
            serde_json::from_str::<EmptyResult>(r#"{"later":1}"#)?,
            EmptyResult {}
        );
        assert!(serde_json::from_str::<EmptyResult>("null").is_err());
        assert_eq!(serde_json::to_string(&EmptyResult {})?, "{}");
        Ok(())
    }

    #[test]
    fn empty_result_refuses_everything_but_an_object() {
        for wire in ["[]", "[1]", "null", "true", "0", r#""x""#] {
            assert!(
                serde_json::from_str::<EmptyResult>(wire).is_err(),
                "{wire} must be refused"
            );
        }
    }

    #[test]
    fn put_result_round_trips_a_populated_value() -> TestResult {
        let wire = r#"{
    "path": "/tmp/f",
    "bytes": 3
}"#;
        let value: PutResult = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }
}
