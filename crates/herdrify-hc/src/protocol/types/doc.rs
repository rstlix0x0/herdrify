//! The replies to `doc.get` and `doc.set`.
//!
//! Each type is read from and written to JSON exactly as the schema names its properties.
//!
//! Responsibilities:
//!
//! - [`DocGetResult`]: the reply to `doc.get`
//! - [`DocSetResult`]: the reply to `doc.set`
//!
//! Non-responsibilities:
//!
//! - deciding what the core accepts or reports, which the core decides

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::protocol::DocVersion;

/// The reply to `doc.get`.
///
/// A document that was never written reads as `null` at version 0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocGetResult {
    /// The document's value; `null` when it was never written or was deleted.
    pub value: Value,
    /// Zero when never written. Deletions retain their version.
    pub version: DocVersion,
}

/// The reply to `doc.set`.
///
/// It carries the version the document now has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocSetResult {
    /// The document's new version.
    pub version: DocVersion,
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::Value;

    use super::{DocGetResult, DocSetResult};
    use crate::protocol::DocVersion;

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn doc_get_result_round_trips_a_populated_value() -> TestResult {
        let wire = r#"{
    "value": {
        "k": [
            1,
            2
        ]
    },
    "version": 3
}"#;
        let value: DocGetResult = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn doc_get_result_reads_an_unwritten_doc() -> TestResult {
        let result: DocGetResult = serde_json::from_str(r#"{"value":null,"version":0}"#)?;
        assert_eq!(result.value, Value::Null);
        assert_eq!(result.version, DocVersion(0));
        assert_eq!(
            serde_json::to_string(&result)?,
            r#"{"value":null,"version":0}"#
        );
        Ok(())
    }

    #[test]
    fn doc_set_result_round_trips_a_populated_value() -> TestResult {
        let wire = r#"{
    "version": 4
}"#;
        let value: DocSetResult = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }
}
