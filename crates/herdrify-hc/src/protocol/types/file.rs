//! The records and replies of the `file.*` methods.
//!
//! Each type is read from and written to JSON exactly as the schema names its properties.
//!
//! Responsibilities:
//!
//! - [`Edit`]: one text replacement in a file
//! - [`FileReadResult`]: the reply to `file.read`
//! - [`FileResult`]: the reply to `file.write` and `file.edit`
//!
//! Non-responsibilities:
//!
//! - deciding what the core accepts or reports, which the core decides

use serde::{Deserialize, Serialize};

/// One text replacement in a file.
///
/// It is sent to `file.edit` and read back with the same shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edit {
    /// The exact text, found exactly once in the file as it was.
    pub old: String,
    /// The text that replaces it.
    pub new: String,
}

/// The reply to `file.read`.
///
/// It holds the lines asked for and where they sit in the file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileReadResult {
    /// The lines as they are in the file, line ends kept.
    pub text: String,
    /// The first line given.
    pub first: u64,
    /// The last line given; less than `first` when none was.
    pub last: u64,
    /// How many lines the file has.
    pub lines: u64,
}

/// The reply to `file.write` and `file.edit`.
///
/// It names the file written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileResult {
    /// The file written, absolute.
    pub path: String,
    /// How many bytes the file holds.
    pub bytes: u64,
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::Value;

    use super::{Edit, FileReadResult, FileResult};

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn edit_file_types_round_trip() -> TestResult {
        let wire = r#"{
    "old": "a",
    "new": "b"
}"#;
        let value: Edit = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn file_read_result_file_types_round_trip() -> TestResult {
        let wire = r#"{
    "text": "a\n",
    "first": 1,
    "last": 1,
    "lines": 1
}"#;
        let value: FileReadResult = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn file_result_file_types_round_trip() -> TestResult {
        let wire = r#"{
    "path": "/tmp/f",
    "bytes": 3
}"#;
        let value: FileResult = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }
}
