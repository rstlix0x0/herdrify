//! The terminal templates the core knows and the reply to `templates.list`.
//!
//! Each type is read from and written to JSON exactly as the schema names its properties.
//!
//! Responsibilities:
//!
//! - [`TemplateInfo`]: one terminal template the core knows
//! - [`TemplatesResult`]: the reply to `templates.list`
//!
//! Non-responsibilities:
//!
//! - deciding what the core accepts or reports, which the core decides

use serde::{Deserialize, Serialize};

/// One terminal template the core knows.
///
/// A template is a file that names a command and a description.
///
/// An optional property that is absent or `null` reads as `None`, and `None` is written as an
/// absent property.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateInfo {
    /// The template's name.
    pub name: String,
    /// What it is for.
    pub description: String,
    /// The command it runs.
    pub command: String,
    /// Set when the template file cannot be used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// The reply to `templates.list`.
///
/// It lists the templates the core found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplatesResult {
    /// The templates.
    pub templates: Vec<TemplateInfo>,
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::Value;

    use super::{TemplateInfo, TemplatesResult};

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn template_info_round_trips_a_populated_value() -> TestResult {
        let wire = r#"{
    "name": "claude",
    "description": "d",
    "command": "claude",
    "error": "bad file"
}"#;
        let value: TemplateInfo = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn optional_fields_read_absent_and_null_as_none() -> TestResult {
        let absent = r#"{
    "name": "claude",
    "description": "d",
    "command": "claude"
}"#;
        let null = r#"{
    "name": "claude",
    "description": "d",
    "command": "claude",
    "error": null
}"#;
        let from_absent: TemplateInfo = serde_json::from_str(absent)?;
        let from_null: TemplateInfo = serde_json::from_str(null)?;
        assert_eq!(from_absent, from_null);
        assert_eq!(
            serde_json::to_value(&from_absent)?,
            serde_json::from_str::<Value>(absent)?
        );
        Ok(())
    }

    #[test]
    fn templates_result_round_trips_a_populated_value() -> TestResult {
        let wire = r#"{
    "templates": [
        {
            "name": "claude",
            "description": "d",
            "command": "claude",
            "error": "e"
        }
    ]
}"#;
        let value: TemplatesResult = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }
}
