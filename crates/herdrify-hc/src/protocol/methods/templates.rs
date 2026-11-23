//! The request that lists the core's terminal templates.
//!
//! Responsibilities:
//!
//! - [`TemplatesList`], serialized as the empty parameter object the schema lists
//!
//! Non-responsibilities:
//!
//! - sending the request or reading the reply, which the client does

use serde::Serialize;

use crate::protocol::{Method, TemplatesResult};

/// The parameters of `templates.list`: none.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct TemplatesList {}

impl TemplatesList {
    /// A `templates.list` request.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Method for TemplatesList {
    const NAME: &'static str = "templates.list";
    type Params = Self;
    type Output = TemplatesResult;
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::json;

    use super::TemplatesList;
    use crate::protocol::Method;

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn templates_request_has_no_params_and_its_wire_name() -> TestResult {
        assert_eq!(serde_json::to_value(TemplatesList::new())?, json!({}));
        assert_eq!(TemplatesList::NAME, "templates.list");
        Ok(())
    }
}
