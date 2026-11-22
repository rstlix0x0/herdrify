//! The requests that act on the herdr-core server itself.
//!
//! `server.status` and `server.stop` take no parameters. `server.restart` takes an optional
//! replacement environment. Each type implements `Method` with its wire name and reply type.
//!
//! Responsibilities:
//!
//! - [`ServerStatus`], [`ServerStop`] and [`ServerRestart`], each serialized as the parameter
//!   object the schema lists
//!
//! Non-responsibilities:
//!
//! - sending the request or reading the reply, which the client does

use serde::Serialize;

use crate::protocol::{Environment, Method, StatusResult, StopResult};

/// The parameters of `server.status`: none.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ServerStatus {}

impl ServerStatus {
    /// A `server.status` request.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Method for ServerStatus {
    const NAME: &'static str = "server.status";
    type Params = Self;
    type Output = StatusResult;
}

/// The parameters of `server.stop`: none.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ServerStop {}

impl ServerStop {
    /// A `server.stop` request.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Method for ServerStop {
    const NAME: &'static str = "server.stop";
    type Params = Self;
    type Output = StopResult;
}

/// The parameters of `server.restart`.
///
/// Nothing is sent that was not set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ServerRestart {
    #[serde(skip_serializing_if = "Option::is_none")]
    environment: Option<Environment>,
}

impl ServerRestart {
    /// A `server.restart` request that keeps the current environment.
    #[must_use]
    pub const fn new() -> Self {
        Self { environment: None }
    }

    /// Replaces the environment the restarted server runs with.
    #[must_use]
    pub fn environment(mut self, environment: Environment) -> Self {
        self.environment = Some(environment);
        self
    }
}

impl Method for ServerRestart {
    const NAME: &'static str = "server.restart";
    type Params = Self;
    type Output = StopResult;
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::{Value, json};

    use crate::protocol::{
        Environment, EventsSubscribe, Method, RpcDiscover, ServerRestart, ServerStatus, ServerStop,
        TemplatesList,
    };

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn server_and_templates_params_serialize_as_empty_objects() -> TestResult {
        let empty = json!({});
        assert_eq!(serde_json::to_value(ServerStatus::new())?, empty);
        assert_eq!(serde_json::to_value(ServerStop::new())?, empty);
        assert_eq!(serde_json::to_value(TemplatesList::new())?, empty);
        assert_eq!(serde_json::to_value(RpcDiscover::new())?, empty);
        assert_eq!(serde_json::to_value(ServerRestart::new())?, empty);
        Ok(())
    }

    #[test]
    fn server_restart_serializes_environment() -> TestResult {
        let environment: Environment = [("PATH", "/bin"), ("", "x")].into_iter().collect();
        let value: Value = serde_json::to_value(ServerRestart::new().environment(environment))?;
        assert_eq!(value, json!({"environment": {"PATH": "/bin", "": "x"}}));
        Ok(())
    }

    #[test]
    fn method_names_match_the_wire() {
        assert_eq!(ServerStatus::NAME, "server.status");
        assert_eq!(ServerStop::NAME, "server.stop");
        assert_eq!(ServerRestart::NAME, "server.restart");
        assert_eq!(TemplatesList::NAME, "templates.list");
        assert_eq!(RpcDiscover::NAME, "rpc.discover");
        assert_eq!(EventsSubscribe::NAME, "events.subscribe");
    }
}
