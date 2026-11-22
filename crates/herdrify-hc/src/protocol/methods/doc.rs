//! The requests that read and write documents in the core's document store.
//!
//! Responsibilities:
//!
//! - [`DocGet`] and [`DocSet`], each with its wire name and reply type
//!
//! Non-responsibilities:
//!
//! - sending the request or reading the reply, which the client does
//! - checking a document's version or size, which the core decides

use serde::Serialize;
use serde_json::Value;

use crate::protocol::{DocGetResult, DocName, DocSetResult, DocVersion, MachineName, Method};

/// The parameters of `doc.get`.
///
/// Nothing is sent that was not set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DocGet {
    name: DocName,
    #[serde(skip_serializing_if = "Option::is_none")]
    on: Option<MachineName>,
}

impl DocGet {
    /// A `doc.get` request for the document `name`.
    #[must_use]
    pub const fn new(name: DocName) -> Self {
        Self { name, on: None }
    }

    /// Reads from another machine. Without it, the request goes to this core.
    #[must_use]
    pub fn on(mut self, machine: MachineName) -> Self {
        self.on = Some(machine);
        self
    }
}

impl Method for DocGet {
    const NAME: &'static str = "doc.get";
    type Params = Self;
    type Output = DocGetResult;
}

/// The parameters of `doc.set`.
///
/// The value is any JSON value, and `null` deletes the document. Nothing else is sent that was not
/// set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DocSet {
    name: DocName,
    value: Value,
    version: DocVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    on: Option<MachineName>,
}

impl DocSet {
    /// A `doc.set` request that stores `value` as `name`, when `version` is its current version.
    ///
    /// The version is zero for a name never written.
    #[must_use]
    pub const fn new(name: DocName, value: Value, version: DocVersion) -> Self {
        Self {
            name,
            value,
            version,
            on: None,
        }
    }

    /// Writes on another machine. Without it, the request goes to this core.
    #[must_use]
    pub fn on(mut self, machine: MachineName) -> Self {
        self.on = Some(machine);
        self
    }
}

impl Method for DocSet {
    const NAME: &'static str = "doc.set";
    type Params = Self;
    type Output = DocSetResult;
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::{Value, json};

    use crate::protocol::{DocGet, DocName, DocSet, DocVersion, Method};

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn doc_set_allows_null() -> TestResult {
        let set = DocSet::new(DocName::new("n")?, Value::Null, DocVersion(1));
        assert_eq!(
            serde_json::to_value(set)?,
            json!({"name": "n", "value": null, "version": 1})
        );
        assert_eq!(DocSet::NAME, "doc.set");
        Ok(())
    }

    #[test]
    fn doc_get_required_only() -> TestResult {
        assert_eq!(
            serde_json::to_value(DocGet::new(DocName::new("n")?))?,
            json!({"name": "n"})
        );
        assert_eq!(DocGet::NAME, "doc.get");
        Ok(())
    }
}
