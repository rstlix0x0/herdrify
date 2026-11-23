//! The core's identity and the replies to the `server.*` methods.
//!
//! Each type is read from and written to JSON exactly as the schema names its properties.
//!
//! Responsibilities:
//!
//! - [`CoreInfo`]: the identity of a core
//! - [`LoginSession`]: the operating system login a process inherited
//! - [`HolderInfo`]: one holder process that runs terminals for the core
//! - [`StatusResult`]: the reply to `server.status`: how the core is doing
//! - [`StopResult`]: the reply to `server.stop` and `server.restart`
//!
//! Non-responsibilities:
//!
//! - deciding what the core accepts or reports, which the core decides

use serde::{Deserialize, Serialize};

/// The identity of a core.
///
/// It is part of the `server.status` reply and says which core answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreInfo {
    /// The permanent random id, reset only when state is wiped.
    pub id: String,
    /// The display name, the hostname by default.
    pub name: String,
}

/// The operating system login a process inherited.
///
/// It is part of the `hello` reply. It records the login the core inherited, not text restored from
/// its environment.
///
/// An optional property that is absent or `null` reads as `None`, and `None` is written as an
/// absent property.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginSession {
    /// The login session id.
    pub id: u32,
    /// The user id the session belongs to.
    pub user: u32,
    /// Whether the login is a graphical one; absent when the provenance is unknown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gui: Option<bool>,
}

/// One holder process that runs terminals for the core.
///
/// After an update, older holders keep running their terminals to the end while the current one
/// takes new terminals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HolderInfo {
    /// The holder's process id.
    pub pid: u32,
    /// The holder's instance id.
    pub instance: String,
    /// Whether it is the same build as the core, so it takes new terminals.
    pub current: bool,
    /// How many live terminals it runs.
    pub terminals: u64,
}

/// The reply to `server.status`: how the core is doing.
///
/// It reports the core, its holders and how many terminals are live and finished.
///
/// An optional property that is absent or `null` reads as `None`, and `None` is written as an
/// absent property.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusResult {
    /// Which core answered.
    pub core: CoreInfo,
    /// The core's version.
    pub version: String,
    /// The core's process id.
    pub pid: u32,
    /// How long it has run, in milliseconds.
    pub uptime_ms: u64,
    /// Every holder; after an update, older ones run their terminals to the end.
    pub holders: Vec<HolderInfo>,
    /// How many terminals are live.
    pub live: u64,
    /// How many terminals have finished.
    pub finished: u64,
    /// The holder new terminals go to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holder: Option<HolderInfo>,
    /// The package manager this installation came from; absent for a replaceable plain install.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_by: Option<String>,
    /// Whether systemd lingering is enabled; absent when unsupported, not applicable or unknown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keeps_running_after_logout: Option<bool>,
    /// The account for the administrator's `loginctl enable-linger` command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logout_linger_user: Option<String>,
}

/// The reply to `server.stop` and `server.restart`.
///
/// It says whether the core is stopping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopResult {
    /// Whether the core stopped.
    pub stopped: bool,
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::Value;

    use super::{CoreInfo, HolderInfo, LoginSession, StatusResult, StopResult};

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn core_info_round_trips_a_populated_value() -> TestResult {
        let wire = r#"{
    "id": "abc123",
    "name": "host"
}"#;
        let value: CoreInfo = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn login_session_round_trips_a_populated_value() -> TestResult {
        let wire = r#"{
    "id": 100002,
    "user": 501,
    "gui": true
}"#;
        let value: LoginSession = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn login_session_optional_fields_read_absent_and_null_as_none() -> TestResult {
        let absent = r#"{
    "id": 100002,
    "user": 501
}"#;
        let null = r#"{
    "id": 100002,
    "user": 501,
    "gui": null
}"#;
        let from_absent: LoginSession = serde_json::from_str(absent)?;
        let from_null: LoginSession = serde_json::from_str(null)?;
        assert_eq!(from_absent, from_null);
        assert_eq!(
            serde_json::to_value(&from_absent)?,
            serde_json::from_str::<Value>(absent)?
        );
        Ok(())
    }

    #[test]
    fn holder_info_round_trips_a_populated_value() -> TestResult {
        let wire = r#"{
    "pid": 77,
    "instance": "i1",
    "current": true,
    "terminals": 3
}"#;
        let value: HolderInfo = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    /// Removes every `null` property, at any depth, since a type writes an absent property
    /// where the core wrote `null`.
    fn without_nulls(value: Value) -> Value {
        match value {
            Value::Object(map) => Value::Object(
                map.into_iter()
                    .filter(|(_, property)| !property.is_null())
                    .map(|(name, property)| (name, without_nulls(property)))
                    .collect(),
            ),
            Value::Array(items) => Value::Array(items.into_iter().map(without_nulls).collect()),
            other => other,
        }
    }

    #[test]
    fn status_result_round_trips_a_populated_value() -> TestResult {
        let wire = r#"{"core":{"id":"abc","name":"host"},"version":"0.1.21","pid":1,"uptime_ms":9,"holders":[{"pid":2,"instance":"i","current":true,"terminals":1}],"live":1,"finished":0,"holder":{"pid":2,"instance":"i","current":true,"terminals":1},"installed_by":"homebrew","keeps_running_after_logout":false,"logout_linger_user":"me"}"#;
        let value: StatusResult = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn status_result_optional_fields_read_absent_and_null_as_none() -> TestResult {
        let absent = r#"{
    "core": {
        "id": "abc",
        "name": "host"
    },
    "version": "0.1.21",
    "pid": 1,
    "uptime_ms": 9,
    "holders": [
        {
            "pid": 2,
            "instance": "i",
            "current": true,
            "terminals": 1
        }
    ],
    "live": 1,
    "finished": 0
}"#;
        let null = r#"{
    "core": {
        "id": "abc",
        "name": "host"
    },
    "version": "0.1.21",
    "pid": 1,
    "uptime_ms": 9,
    "holders": [
        {
            "pid": 2,
            "instance": "i",
            "current": true,
            "terminals": 1
        }
    ],
    "live": 1,
    "finished": 0,
    "holder": null,
    "installed_by": null,
    "keeps_running_after_logout": null,
    "logout_linger_user": null
}"#;
        let from_absent: StatusResult = serde_json::from_str(absent)?;
        let from_null: StatusResult = serde_json::from_str(null)?;
        assert_eq!(from_absent, from_null);
        assert_eq!(
            serde_json::to_value(&from_absent)?,
            serde_json::from_str::<Value>(absent)?
        );
        Ok(())
    }

    #[test]
    fn status_result_round_trips_a_live_reply() -> TestResult {
        let reply = r#"{"core":{"id":"378c1e96a6faafa9cd6ada3bf9c3f2db","name":"host"},"finished":0,"holder":null,"holders":[],"installed_by":null,"keeps_running_after_logout":null,"live":0,"logout_linger_user":null,"pid":55595,"uptime_ms":4,"version":"0.1.21"}"#;
        let value: StatusResult = serde_json::from_str(reply)?;
        let expected = without_nulls(serde_json::from_str::<Value>(reply)?);
        assert_eq!(serde_json::to_value(&value)?, expected);
        Ok(())
    }

    #[test]
    fn stop_result_round_trips_a_populated_value() -> TestResult {
        let wire = r#"{
    "stopped": true
}"#;
        let value: StopResult = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn stop_result_reads_stopped_true() -> TestResult {
        let result: StopResult = serde_json::from_str(r#"{"stopped":true}"#)?;
        assert!(result.stopped);
        Ok(())
    }
}
