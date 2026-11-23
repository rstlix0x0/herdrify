//! The two maps the core exchanges: environment variables and labels.
//!
//! Each map keeps its entries in sorted order and is read from and written to JSON as one object.
//!
//! Responsibilities:
//!
//! - [`Environment`]: the names and values of environment variables sent to the core
//! - [`Labels`]: the key and value pairs attached to a terminal
//!
//! Non-responsibilities:
//!
//! - deciding what the core accepts or reports, which the core decides

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::protocol::{LabelKey, LabelValue};

/// Environment variables as a map of names to values.
///
/// A name may be empty, because the core accepts it. Names are kept in sorted order.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::Environment;
///
/// let mut environment = Environment::new();
/// environment.insert("PATH", "/bin");
/// assert_eq!(environment.get("PATH"), Some("/bin"));
/// assert_eq!(environment.len(), 1);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Environment(BTreeMap<String, String>);

impl Environment {
    /// An environment with no variables.
    #[must_use]
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Sets `name` to `value`, replacing an earlier value for the same name.
    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.0.insert(name.into(), value.into());
    }

    /// The value of `name`, if it is set.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }

    /// The variables in name order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
    }

    /// How many variables are set.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether no variable is set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<N: Into<String>, V: Into<String>> FromIterator<(N, V)> for Environment {
    fn from_iter<I: IntoIterator<Item = (N, V)>>(entries: I) -> Self {
        Self(
            entries
                .into_iter()
                .map(|(name, value)| (name.into(), value.into()))
                .collect(),
        )
    }
}

/// A terminal's labels as a map of keys to values.
///
/// Reading a label with an empty value is refused. The empty value is how the core removes a
/// label, and that removal is a separate change, `LabelChange::Remove`. Labels are kept in key
/// order.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::{EmptyValueError, LabelKey, LabelValue, Labels};
///
/// # fn main() -> Result<(), EmptyValueError> {
/// let mut labels = Labels::new();
/// labels.insert(LabelKey::new("team")?, LabelValue::new("infra")?);
/// assert_eq!(labels.get(&LabelKey::new("team")?).map(LabelValue::as_str), Some("infra"));
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Labels(BTreeMap<LabelKey, LabelValue>);

impl Labels {
    /// A set of labels with none in it.
    #[must_use]
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Sets the label `key` to `value`, replacing an earlier value for the same key.
    pub fn insert(&mut self, key: LabelKey, value: LabelValue) {
        self.0.insert(key, value);
    }

    /// The value of the label `key`, if it is set.
    #[must_use]
    pub fn get(&self, key: &LabelKey) -> Option<&LabelValue> {
        self.0.get(key)
    }

    /// The labels in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&LabelKey, &LabelValue)> {
        self.0.iter()
    }

    /// How many labels are set.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether no label is set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl FromIterator<(LabelKey, LabelValue)> for Labels {
    fn from_iter<I: IntoIterator<Item = (LabelKey, LabelValue)>>(entries: I) -> Self {
        Self(entries.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::{Environment, Labels};

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn environment_accepts_an_empty_name() -> TestResult {
        let environment: Environment = serde_json::from_str(r#"{"":"x"}"#)?;
        assert_eq!(environment.get(""), Some("x"));
        assert_eq!(serde_json::to_string(&environment)?, r#"{"":"x"}"#);
        Ok(())
    }

    #[test]
    fn labels_refuse_an_empty_value() -> TestResult {
        assert!(serde_json::from_str::<Labels>(r#"{"k":""}"#).is_err());
        let labels: Labels = serde_json::from_str(r#"{"k":"v"}"#)?;
        assert_eq!(serde_json::to_string(&labels)?, r#"{"k":"v"}"#);
        Ok(())
    }
}
