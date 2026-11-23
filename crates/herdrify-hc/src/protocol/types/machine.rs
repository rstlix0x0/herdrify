//! The machine records and the replies to the `machine.*` methods.
//!
//! Each type is read from and written to JSON exactly as the schema names its properties.
//!
//! Responsibilities:
//!
//! - [`Machine`]: a machine registered with the core
//! - [`MachineView`]: a machine as `machine.list` and `terminal.list` show it: its profile and its
//!   state now
//! - [`MachineListResult`]: the reply to `machine.list`
//! - [`ExecResult`]: the reply to `machine.exec`
//!
//! Non-responsibilities:
//!
//! - deciding what the core accepts or reports, which the core decides

use serde::{Deserialize, Serialize};

/// A machine registered with the core.
///
/// It is the reply to `machine.add` and `machine.rename`. `name` stays a plain string here: the
/// machine-name newtype is for what a caller sends.
///
/// An optional property that is absent or `null` reads as `None`, and `None` is written as an
/// absent property.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Machine {
    /// What this core calls it: `rohan` in `rohan/t7`.
    pub name: String,
    /// How it is reached: an ssh target, or with `via` a command.
    pub target: String,
    /// The other core's permanent id.
    pub id: String,
    /// What that core calls itself, its hostname.
    pub host: String,
    /// Its version when last seen.
    pub version: String,
    /// The last observed systemd logout protection; absent for old or unsupported cores.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keeps_running_after_logout: Option<bool>,
    /// The remote account for `loginctl enable-linger`, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logout_linger_user: Option<String>,
    /// The session there; absent means its default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// Whether it is short-lived, such as a sandbox for a task.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temporary: Option<bool>,
    /// Whether `target` is a command that runs a shell line there, given as its last argument.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub via: Option<bool>,
}

/// A machine as `machine.list` and `terminal.list` show it: its profile and its state now.
///
/// It has every property of `Machine` and adds the connection state.
///
/// An optional property that is absent or `null` reads as `None`, and `None` is written as an
/// absent property.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineView {
    /// What this core calls it: `rohan` in `rohan/t7`.
    pub name: String,
    /// How it is reached: an ssh target, or with `via` a command.
    pub target: String,
    /// The other core's permanent id.
    pub id: String,
    /// What that core calls itself, its hostname.
    pub host: String,
    /// Its version when last seen.
    pub version: String,
    /// `connecting`, `connected`, `needs_login` or `unreachable`.
    pub state: String,
    /// The last observed systemd logout protection; absent for old or unsupported cores.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keeps_running_after_logout: Option<bool>,
    /// The remote account for `loginctl enable-linger`, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logout_linger_user: Option<String>,
    /// The session there; absent means its default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// Whether it is short-lived, such as a sandbox for a task.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temporary: Option<bool>,
    /// Whether `target` is a command that runs a shell line there, given as its last argument.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub via: Option<bool>,
    /// Since when it is not connected, in milliseconds since the epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since_ms: Option<u64>,
    /// Why it is not connected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

/// The reply to `machine.list`.
///
/// It lists every machine added to the core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineListResult {
    /// The machines.
    pub machines: Vec<MachineView>,
}

/// The reply to `machine.exec`.
///
/// The output arrives as base64 text and is held as bytes. Reading text that is not valid base64
/// fails.
///
/// An optional property that is absent or `null` reads as `None`, and `None` is written as an
/// absent property.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecResult {
    /// What it wrote to standard output: the first 8 MB.
    #[serde(with = "crate::protocol::base64")]
    pub stdout: Vec<u8>,
    /// What it wrote to standard error: the first 1 MB.
    #[serde(with = "crate::protocol::base64")]
    pub stderr: Vec<u8>,
    /// Whether it wrote more than was kept.
    pub truncated: bool,
    /// Whether it ran out of time and was ended; the output is what came so far.
    pub timed_out: bool,
    /// How long it ran, in milliseconds.
    pub ms: u64,
    /// Its exit code; absent when a signal ended it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i32>,
    /// The signal that ended it, on Unix.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal: Option<i32>,
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::Value;

    use super::{ExecResult, Machine, MachineListResult, MachineView};

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn machine_machine_types_round_trip() -> TestResult {
        let wire = r#"{
    "name": "rohan",
    "target": "rohan.example",
    "id": "id1",
    "host": "rohan",
    "version": "0.1.21",
    "keeps_running_after_logout": true,
    "logout_linger_user": "me",
    "session": "work",
    "temporary": true,
    "via": false
}"#;
        let value: Machine = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn machine_optional_fields_read_absent_and_null_as_none() -> TestResult {
        let absent = r#"{
    "name": "rohan",
    "target": "rohan.example",
    "id": "id1",
    "host": "rohan",
    "version": "0.1.21"
}"#;
        let null = r#"{
    "name": "rohan",
    "target": "rohan.example",
    "id": "id1",
    "host": "rohan",
    "version": "0.1.21",
    "keeps_running_after_logout": null,
    "logout_linger_user": null,
    "session": null,
    "temporary": null,
    "via": null
}"#;
        let from_absent: Machine = serde_json::from_str(absent)?;
        let from_null: Machine = serde_json::from_str(null)?;
        assert_eq!(from_absent, from_null);
        assert_eq!(
            serde_json::to_value(&from_absent)?,
            serde_json::from_str::<Value>(absent)?
        );
        Ok(())
    }

    #[test]
    fn machine_view_machine_types_round_trip() -> TestResult {
        let wire = r#"{
    "name": "rohan",
    "target": "rohan.example",
    "id": "id1",
    "host": "rohan",
    "version": "0.1.21",
    "keeps_running_after_logout": true,
    "logout_linger_user": "me",
    "session": "work",
    "temporary": true,
    "via": false,
    "state": "connected",
    "since_ms": 1700000000000,
    "why": "ok"
}"#;
        let value: MachineView = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn machine_view_optional_fields_read_absent_and_null_as_none() -> TestResult {
        let absent = r#"{
    "name": "rohan",
    "target": "rohan.example",
    "id": "id1",
    "host": "rohan",
    "version": "0.1.21",
    "state": "connected"
}"#;
        let null = r#"{
    "name": "rohan",
    "target": "rohan.example",
    "id": "id1",
    "host": "rohan",
    "version": "0.1.21",
    "state": "connected",
    "keeps_running_after_logout": null,
    "logout_linger_user": null,
    "session": null,
    "temporary": null,
    "via": null,
    "since_ms": null,
    "why": null
}"#;
        let from_absent: MachineView = serde_json::from_str(absent)?;
        let from_null: MachineView = serde_json::from_str(null)?;
        assert_eq!(from_absent, from_null);
        assert_eq!(
            serde_json::to_value(&from_absent)?,
            serde_json::from_str::<Value>(absent)?
        );
        Ok(())
    }

    #[test]
    fn machine_list_result_machine_types_round_trip() -> TestResult {
        let wire = r#"{
    "machines": [
        {
            "name": "rohan",
            "target": "rohan.example",
            "id": "id1",
            "host": "rohan",
            "version": "0.1.21",
            "keeps_running_after_logout": true,
            "logout_linger_user": "me",
            "session": "work",
            "temporary": true,
            "via": false,
            "state": "connected",
            "since_ms": 1700000000000,
            "why": "ok"
        }
    ]
}"#;
        let value: MachineListResult = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn round_trips_a_populated_value() -> TestResult {
        let wire = r#"{
    "stdout": "eAo=",
    "stderr": "",
    "truncated": false,
    "timed_out": false,
    "ms": 12,
    "code": 0,
    "signal": 9
}"#;
        let value: ExecResult = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn exec_result_optional_fields_read_absent_and_null_as_none() -> TestResult {
        let absent = r#"{
    "stdout": "eAo=",
    "stderr": "",
    "truncated": false,
    "timed_out": false,
    "ms": 12
}"#;
        let null = r#"{
    "stdout": "eAo=",
    "stderr": "",
    "truncated": false,
    "timed_out": false,
    "ms": 12,
    "code": null,
    "signal": null
}"#;
        let from_absent: ExecResult = serde_json::from_str(absent)?;
        let from_null: ExecResult = serde_json::from_str(null)?;
        assert_eq!(from_absent, from_null);
        assert_eq!(
            serde_json::to_value(&from_absent)?,
            serde_json::from_str::<Value>(absent)?
        );
        Ok(())
    }

    #[test]
    fn exec_result_decodes_base64_output() -> TestResult {
        let result: ExecResult = serde_json::from_str(
            r#"{"stdout":"eAo=","stderr":"","truncated":false,"timed_out":false,"ms":1}"#,
        )?;
        assert_eq!(result.stdout, b"x\n");
        assert!(result.stderr.is_empty());
        assert_eq!(serde_json::to_value(&result)?["stdout"], "eAo=");
        Ok(())
    }

    #[test]
    fn exec_result_refuses_invalid_base64() {
        let refused = serde_json::from_str::<ExecResult>(
            r#"{"stdout":"@@","stderr":"","truncated":false,"timed_out":false,"ms":1}"#,
        );
        assert!(refused.is_err());
    }
}
