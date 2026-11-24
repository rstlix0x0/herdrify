//! The replies to the `terminal.*` methods, each read from JSON exactly as the schema names its
//! properties.
//!
//! They sit together because several of them hold a `Terminal`, a `WaitResult` or a `StatusWord`,
//! and a call and its reply are read side by side.
//!
//! Responsibilities:
//!
//! - [`ListResult`]: the reply to `terminal.list`
//! - [`RemoveResult`]: the reply to `terminal.rm` and `machine.rm`
//! - [`ReadResult`]: the reply to `terminal.read`
//! - [`SendResult`]: the reply to `terminal.send`
//! - [`WaitResult`]: the reply to `terminal.wait`, and the `wait` object of other replies
//! - [`KeysResult`]: the reply to `terminal.keys`
//! - [`InterruptResult`]: the reply to `terminal.interrupt`
//! - [`AttachResult`]: the reply to `terminal.attach`
//! - [`RunResult`]: the reply to `terminal.run`: a terminal record, plus what a wait gathered
//!
//! Non-responsibilities:
//!
//! - the replies of `terminal.put` and of the put in pieces, which `put` holds
//! - deciding what the core accepts or reports, which the core decides

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::protocol::{
    EndReason, FirstMessage, Foreground, Labels, MachineView, Status, StatusWord, Terminal,
    TerminalAddress, TerminalName, TerminalState, WaitReason,
};

/// The reply to `terminal.list`.
///
/// It holds this machine's terminals, then those of each connected machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListResult {
    /// This machine's terminals, then each connected machine's, as `machine/t7`.
    pub terminals: Vec<Terminal>,
    /// How many finished terminals were left out.
    pub hidden_finished: u64,
    /// The machines added here and their state; one that is not connected is listed, not waited on.
    pub machines: Vec<MachineView>,
}

/// The reply to `terminal.rm` and `machine.rm`.
///
/// It names what was removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveResult {
    /// What was removed.
    pub removed: String,
}

/// The reply to `terminal.read`.
///
/// It holds a terminal's text and says how it was read.
///
/// An optional property that is absent or `null` reads as `None`, and `None` is written as an
/// absent property.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadResult {
    /// The terminal's text.
    pub text: String,
    /// False when part of what was asked for may be missing.
    pub complete: bool,
    /// `screen`, `history`, or `stitched` for a full-screen app read by scrolling it.
    pub how: String,
    /// Why only the screen was read: `working`, `blocked` or `error`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

/// The reply to `terminal.send`.
///
/// It says which turn the text went in on and, when the call asked to wait, how that turn ended.
///
/// An optional property that is absent or `null` reads as `None`, and `None` is written as an
/// absent property.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendResult {
    /// The turn when the text went in.
    pub turn: u64,
    /// Whether it went `working` within a few seconds; absent when the program does not report
    /// status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started: Option<bool>,
    /// The program's status word when the text went in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<StatusWord>,
    /// With a wait: how that turn ended.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait: Option<WaitResult>,
}

/// The reply to `terminal.wait`, and the `wait` object of other replies.
///
/// It says why the wait returned. With children, it also holds one result per child, each a
/// `WaitResult` itself.
///
/// An optional property that is absent or `null` reads as `None`, and `None` is written as an
/// absent property.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitResult {
    /// The terminal waited on.
    pub terminal: String,
    /// False on timeout: a check-in, the terminal is still running.
    pub ended: bool,
    /// Why the wait returned.
    pub reason: WaitReason,
    /// The terminal's turn.
    pub turn: u64,
    /// With children: one result per child it reports on.
    pub children: Vec<Self>,
    /// The exit code, when the terminal exited with one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// With a message: the sender's address; absent when a person sent it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// The signal that ended it, if one did.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal: Option<i32>,
    /// The exact word the program reported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<StatusWord>,
}

/// The reply to `terminal.keys`.
///
/// It says which turn the keys went in on.
///
/// An optional property that is absent or `null` reads as `None`, and `None` is written as an
/// absent property.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeysResult {
    /// The turn when the keys went in.
    pub turn: u64,
    /// The program's status word when the keys went in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<StatusWord>,
}

/// The reply to `terminal.interrupt`.
///
/// It says which key was sent and what came of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterruptResult {
    /// `stopped`, `exited`, `still_working` or `unknown`.
    pub outcome: String,
    /// The key sent: `esc` for programs that report status, else `ctrl+c`.
    pub key: String,
}

/// The reply to `terminal.attach`.
///
/// It names the stream channel the terminal's output arrives on, and the terminal as it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachResult {
    /// The stream channel: first paint, then live bytes.
    pub channel: u32,
    /// The terminal as it is.
    pub terminal: Terminal,
    /// True only when the entire path sends a resize before replacement paints.
    pub resize_notifications: bool,
    /// Whether the entire attach path accepts automatic input frames.
    pub automatic_input: bool,
}

/// The reply to `terminal.run`: a terminal record, plus what a wait gathered.
///
/// It has every property of `Terminal`, and adds `output` and `wait`, which the core fills when the
/// call asked to wait. `Terminal` stays a separate type for the other calls.
///
/// An optional property that is absent or `null` reads as `None`, and `None` is written as an
/// absent property.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunResult {
    /// The terminal's address, such as `t7`: this terminal forever on this core.
    pub address: TerminalAddress,
    /// The command line it was started with.
    pub argv: Vec<String>,
    /// The current width in columns.
    pub cols: u16,
    /// When it was created, in milliseconds since the epoch.
    pub created_ms: u64,
    /// The working directory it was started in.
    pub cwd: String,
    /// Its labels.
    pub labels: Labels,
    /// The current height in rows.
    pub rows: u16,
    /// `exact`, or `inexact` after a core crash, when older history may be missing.
    pub screen: String,
    /// Whether it is live or finished.
    pub state: TerminalState,
    /// Whether it ends with its parent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub die_with_parent: Option<bool>,
    /// The exit code, once it exited with one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// When it finished, in milliseconds since the epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_ms: Option<u64>,
    /// Whether a template's first message went in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_message: Option<FirstMessage>,
    /// What runs in it right now; absent once it has finished.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreground: Option<Foreground>,
    /// Which terminal the program believes it runs in, fixed at start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<String>,
    /// Whether a lost program is replaced with the machine's shell, in place.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep: Option<bool>,
    /// Its name, when it was given one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<TerminalName>,
    /// Whether its parent gets notes about it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify: Option<bool>,
    /// The terminal that started it, set once at start.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// The pause between a paste and its submit key, in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paste_gap_ms: Option<u64>,
    /// Its process id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    /// Why its program ended or is pending.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<EndReason>,
    /// Other status records by id, for programs that report on several things.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records: Option<BTreeMap<String, Status>>,
    /// Whether the program ever sent a status report.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reported: Option<bool>,
    /// Whether it is removed right after it exits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rm: Option<bool>,
    /// The signal that ended it, if one did.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal: Option<i32>,
    /// The program's own word for what it is doing; absent if it never said, or after it exited.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
    /// When that word last changed, in milliseconds since the epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_ms: Option<u64>,
    /// The key that stops it, from its template.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<String>,
    /// The key that submits a message, from its template.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submit: Option<String>,
    /// The template that started it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// Its title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The turn count, which goes up each time the status leaves `working`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn: Option<u64>,
    /// With a wait: the terminal's text then, like `terminal.read`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// With a wait: how the wait ended.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait: Option<WaitResult>,
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::Value;

    use super::{
        AttachResult, InterruptResult, KeysResult, ListResult, ReadResult, RemoveResult, RunResult,
        SendResult, WaitResult,
    };
    use crate::protocol::{StatusWord, WaitReason};

    type TestResult = Result<(), Box<dyn Error>>;

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
    fn list_result_round_trips_a_populated_value() -> TestResult {
        let wire = r#"{"terminals":[{"address":"t7","argv":["sh","-c","sleep 1"],"cols":80,"created_ms":1700000000000,"cwd":"/work","labels":{"a":"1"},"rows":24,"screen":"exact","state":"finished","die_with_parent":true,"exit_code":0,"finished_ms":1700000001000,"first_message":{"sent":true,"reason":"busy","started":true,"turn":2},"foreground":{"pid":4242,"name":"vim","argv":["vim"],"cwd":"/work"},"identity":"ghostty","keep":true,"name":"build","notify":true,"parent":"t3","paste_gap_ms":50,"pid":4242,"reason":"exited","records":{"sub":{"state":"working","progress":5}},"reported":true,"rm":true,"signal":9,"status":{"state":"idle","title":"t"},"status_ms":1700000000500,"stop":"esc","submit":"enter","template":"claude","title":"t","turn":3}],"hidden_finished":2,"machines":[{"name":"rohan","target":"rohan.example","id":"id1","host":"rohan","version":"0.1.21","keeps_running_after_logout":true,"logout_linger_user":"me","session":"work","temporary":true,"via":false,"state":"connected","since_ms":1700000000000,"why":"ok"}]}"#;
        let value: ListResult = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn list_result_round_trips_a_live_reply() -> TestResult {
        let reply = r#"{"hidden_finished":0,"machines":[],"terminals":[{"address":"t2","argv":["sh"],"cols":200,"created_ms":1795356738856,"cwd":"/work/probe-methods","die_with_parent":false,"exit_code":null,"finished_ms":null,"identity":"ghostty","keep":false,"labels":{"a":"1","b":"2"},"name":null,"notify":true,"parent":null,"pid":56464,"reason":null,"reported":false,"rm":false,"rows":120,"screen":"exact","signal":null,"state":"live","status":null,"status_ms":null,"template":null,"title":"","turn":0}]}"#;
        let value: ListResult = serde_json::from_str(reply)?;
        let expected = without_nulls(serde_json::from_str::<Value>(reply)?);
        assert_eq!(serde_json::to_value(&value)?, expected);
        Ok(())
    }

    #[test]
    fn remove_result_round_trips_a_populated_value() -> TestResult {
        let wire = r#"{
    "removed": "t7"
}"#;
        let value: RemoveResult = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn read_result_round_trips_a_populated_value() -> TestResult {
        let wire = r#"{
    "text": "hi",
    "complete": true,
    "how": "screen",
    "why": "working"
}"#;
        let value: ReadResult = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn read_result_optional_fields_read_absent_and_null_as_none() -> TestResult {
        let absent = r#"{
    "text": "hi",
    "complete": true,
    "how": "screen"
}"#;
        let null = r#"{
    "text": "hi",
    "complete": true,
    "how": "screen",
    "why": null
}"#;
        let from_absent: ReadResult = serde_json::from_str(absent)?;
        let from_null: ReadResult = serde_json::from_str(null)?;
        assert_eq!(from_absent, from_null);
        assert_eq!(
            serde_json::to_value(&from_absent)?,
            serde_json::from_str::<Value>(absent)?
        );
        Ok(())
    }

    #[test]
    fn send_result_round_trips_a_populated_value() -> TestResult {
        let wire = r#"{
    "turn": 4,
    "started": true,
    "status": "idle",
    "wait": {
        "terminal": "t7",
        "ended": true,
        "reason": "exit",
        "turn": 1,
        "children": [],
        "exit_code": 0,
        "from": "t3",
        "signal": 9,
        "status": "done"
    }
}"#;
        let value: SendResult = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn send_result_optional_fields_read_absent_and_null_as_none() -> TestResult {
        let absent = r#"{
    "turn": 4
}"#;
        let null = r#"{
    "turn": 4,
    "started": null,
    "status": null,
    "wait": null
}"#;
        let from_absent: SendResult = serde_json::from_str(absent)?;
        let from_null: SendResult = serde_json::from_str(null)?;
        assert_eq!(from_absent, from_null);
        assert_eq!(
            serde_json::to_value(&from_absent)?,
            serde_json::from_str::<Value>(absent)?
        );
        Ok(())
    }

    #[test]
    fn send_result_status_is_a_status_word() -> TestResult {
        let value: SendResult = serde_json::from_str(r#"{"turn":4,"status":"done"}"#)?;
        assert_eq!(value.status, Some(StatusWord::Done));
        let value: SendResult = serde_json::from_str(r#"{"turn":4,"status":"paused"}"#)?;
        assert_eq!(value.status, Some(StatusWord::Other("paused".to_owned())));
        assert_eq!(
            serde_json::to_value(&value)?["status"],
            Value::from("paused")
        );
        Ok(())
    }

    #[test]
    fn wait_result_round_trips_a_populated_value() -> TestResult {
        let wire = r#"{
    "terminal": "t7",
    "ended": true,
    "reason": "exit",
    "turn": 1,
    "children": [],
    "exit_code": 0,
    "from": "t3",
    "signal": 9,
    "status": "done"
}"#;
        let value: WaitResult = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn wait_result_optional_fields_read_absent_and_null_as_none() -> TestResult {
        let absent = r#"{
    "terminal": "t7",
    "ended": true,
    "reason": "exit",
    "turn": 1,
    "children": []
}"#;
        let null = r#"{
    "terminal": "t7",
    "ended": true,
    "reason": "exit",
    "turn": 1,
    "children": [],
    "exit_code": null,
    "from": null,
    "signal": null,
    "status": null
}"#;
        let from_absent: WaitResult = serde_json::from_str(absent)?;
        let from_null: WaitResult = serde_json::from_str(null)?;
        assert_eq!(from_absent, from_null);
        assert_eq!(
            serde_json::to_value(&from_absent)?,
            serde_json::from_str::<Value>(absent)?
        );
        Ok(())
    }

    #[test]
    fn wait_result_round_trips_a_live_reply() -> TestResult {
        let reply = r#"{"children":[],"ended":true,"exit_code":0,"reason":"exit","signal":null,"status":null,"terminal":"t1","turn":0}"#;
        let value: WaitResult = serde_json::from_str(reply)?;
        let expected = without_nulls(serde_json::from_str::<Value>(reply)?);
        assert_eq!(serde_json::to_value(&value)?, expected);
        Ok(())
    }

    #[test]
    fn wait_result_children_nest() -> TestResult {
        let wire = r#"{"terminal":"t1","ended":true,"reason":"children","turn":0,"children":[
            {"terminal":"t2","ended":true,"reason":"exit","turn":1,"children":[],"exit_code":0}]}"#;
        let result: WaitResult = serde_json::from_str(wire)?;
        assert_eq!(result.children.len(), 1);
        assert_eq!(result.children[0].terminal, "t2");
        assert_eq!(result.children[0].reason, WaitReason::Exit);
        assert_eq!(
            serde_json::to_value(&result)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn wait_result_status_is_a_status_word() -> TestResult {
        let value: WaitResult = serde_json::from_str(
            r#"{"terminal":"t1","ended":true,"reason":"exit","turn":0,"children":[],"status":"done"}"#,
        )?;
        assert_eq!(value.status, Some(StatusWord::Done));
        let value: WaitResult = serde_json::from_str(
            r#"{"terminal":"t1","ended":true,"reason":"exit","turn":0,"children":[],"status":"paused"}"#,
        )?;
        assert_eq!(value.status, Some(StatusWord::Other("paused".to_owned())));
        assert_eq!(
            serde_json::to_value(&value)?["status"],
            Value::from("paused")
        );
        Ok(())
    }

    #[test]
    fn keys_result_round_trips_a_populated_value() -> TestResult {
        let wire = r#"{
    "turn": 2,
    "status": "idle"
}"#;
        let value: KeysResult = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn keys_result_optional_fields_read_absent_and_null_as_none() -> TestResult {
        let absent = r#"{
    "turn": 2
}"#;
        let null = r#"{
    "turn": 2,
    "status": null
}"#;
        let from_absent: KeysResult = serde_json::from_str(absent)?;
        let from_null: KeysResult = serde_json::from_str(null)?;
        assert_eq!(from_absent, from_null);
        assert_eq!(
            serde_json::to_value(&from_absent)?,
            serde_json::from_str::<Value>(absent)?
        );
        Ok(())
    }

    #[test]
    fn keys_result_status_is_a_status_word() -> TestResult {
        let value: KeysResult = serde_json::from_str(r#"{"turn":2,"status":"idle"}"#)?;
        assert_eq!(value.status, Some(StatusWord::Idle));
        let value: KeysResult = serde_json::from_str(r#"{"turn":2,"status":"paused"}"#)?;
        assert_eq!(value.status, Some(StatusWord::Other("paused".to_owned())));
        assert_eq!(
            serde_json::to_value(&value)?["status"],
            Value::from("paused")
        );
        Ok(())
    }

    #[test]
    fn interrupt_result_round_trips_a_populated_value() -> TestResult {
        let wire = r#"{
    "outcome": "stopped",
    "key": "esc"
}"#;
        let value: InterruptResult = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn attach_result_round_trips_a_populated_value() -> TestResult {
        let wire = r#"{"channel":1,"terminal":{"address":"t7","argv":["sh","-c","sleep 1"],"cols":80,"created_ms":1700000000000,"cwd":"/work","labels":{"a":"1"},"rows":24,"screen":"exact","state":"finished","die_with_parent":true,"exit_code":0,"finished_ms":1700000001000,"first_message":{"sent":true,"reason":"busy","started":true,"turn":2},"foreground":{"pid":4242,"name":"vim","argv":["vim"],"cwd":"/work"},"identity":"ghostty","keep":true,"name":"build","notify":true,"parent":"t3","paste_gap_ms":50,"pid":4242,"reason":"exited","records":{"sub":{"state":"working","progress":5}},"reported":true,"rm":true,"signal":9,"status":{"state":"idle","title":"t"},"status_ms":1700000000500,"stop":"esc","submit":"enter","template":"claude","title":"t","turn":3},"resize_notifications":true,"automatic_input":true}"#;
        let value: AttachResult = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn attach_result_round_trips_a_live_reply() -> TestResult {
        let reply = r#"{"automatic_input":true,"channel":1,"resize_notifications":false,"terminal":{"address":"t1d","argv":["sh"],"cols":80,"created_ms":1795355450242,"cwd":"/work/probe-framed","die_with_parent":false,"exit_code":null,"finished_ms":null,"foreground":{"argv":["sh"],"cwd":"/work/probe-framed","name":"bash","pid":43803},"identity":"ghostty","keep":false,"labels":{},"name":null,"notify":true,"parent":null,"pid":43803,"reason":null,"reported":false,"rm":false,"rows":24,"screen":"exact","signal":null,"state":"live","status":null,"status_ms":null,"template":null,"title":"","turn":0}}"#;
        let value: AttachResult = serde_json::from_str(reply)?;
        let expected = without_nulls(serde_json::from_str::<Value>(reply)?);
        assert_eq!(serde_json::to_value(&value)?, expected);
        Ok(())
    }

    #[test]
    fn run_result_carries_output_and_wait() -> TestResult {
        let wire = r#"{"address":"t7","argv":["sh","-c","sleep 1"],"cols":80,"created_ms":1700000000000,"cwd":"/work","labels":{"a":"1"},"rows":24,"screen":"exact","state":"finished","die_with_parent":true,"exit_code":0,"finished_ms":1700000001000,"first_message":{"sent":true,"reason":"busy","started":true,"turn":2},"foreground":{"pid":4242,"name":"vim","argv":["vim"],"cwd":"/work"},"identity":"ghostty","keep":true,"name":"build","notify":true,"parent":"t3","paste_gap_ms":50,"pid":4242,"reason":"exited","records":{"sub":{"state":"working","progress":5}},"reported":true,"rm":true,"signal":9,"status":{"state":"idle","title":"t"},"status_ms":1700000000500,"stop":"esc","submit":"enter","template":"claude","title":"t","turn":3,"output":"ev","wait":{"terminal":"t7","ended":true,"reason":"exit","turn":1,"children":[],"exit_code":0,"from":"t3","signal":9,"status":"done"}}"#;
        let value: RunResult = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn run_result_optional_fields_read_absent_and_null_as_none() -> TestResult {
        let absent = r#"{
    "address": "t7",
    "argv": [
        "sh",
        "-c",
        "sleep 1"
    ],
    "cols": 80,
    "created_ms": 1700000000000,
    "cwd": "/work",
    "labels": {
        "a": "1"
    },
    "rows": 24,
    "screen": "exact",
    "state": "finished"
}"#;
        let null = r#"{"address":"t7","argv":["sh","-c","sleep 1"],"cols":80,"created_ms":1700000000000,"cwd":"/work","labels":{"a":"1"},"rows":24,"screen":"exact","state":"finished","die_with_parent":null,"exit_code":null,"finished_ms":null,"first_message":null,"foreground":null,"identity":null,"keep":null,"name":null,"notify":null,"parent":null,"paste_gap_ms":null,"pid":null,"reason":null,"records":null,"reported":null,"rm":null,"signal":null,"status":null,"status_ms":null,"stop":null,"submit":null,"template":null,"title":null,"turn":null,"output":null,"wait":null}"#;
        let from_absent: RunResult = serde_json::from_str(absent)?;
        let from_null: RunResult = serde_json::from_str(null)?;
        assert_eq!(from_absent, from_null);
        assert_eq!(
            serde_json::to_value(&from_absent)?,
            serde_json::from_str::<Value>(absent)?
        );
        Ok(())
    }
}
