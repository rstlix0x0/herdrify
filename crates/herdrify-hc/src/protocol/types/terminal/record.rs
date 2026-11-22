//! The records that describe a terminal: the terminal itself and the facts attached to it.
//!
//! Each type is read from and written to JSON exactly as the schema names its properties. They sit
//! together because each describes one terminal: `Terminal` holds a `Status`, a `Foreground` and a
//! `FirstMessage`, while `DisplayFacts` and `InitialSize` describe its screen as a viewer reports
//! it and as it starts.
//!
//! Responsibilities:
//!
//! - [`Terminal`]: a terminal as the core reports it
//! - [`Status`]: one 7501 status record a program reports
//! - [`Foreground`]: the process that runs in a terminal right now
//! - [`FirstMessage`]: whether a template run's first message went in
//! - [`DisplayFacts`]: what a viewer's own terminal looks like, as the client reports it
//! - [`InitialSize`]: the size a terminal starts with
//!
//! Non-responsibilities:
//!
//! - deciding what the core accepts or reports, which the core decides

use std::collections::BTreeMap;
use std::num::NonZeroU16;

use serde::{Deserialize, Serialize};

use crate::protocol::{
    EndReason, Labels, StatusWord, TerminalAddress, TerminalName, TerminalState,
};

/// A terminal as the core reports it.
///
/// This is the record `terminal.get` and the other terminal calls return. `RunResult` is a separate
/// type with the same properties plus `output` and `wait`.
///
/// An optional property that is absent or `null` reads as `None`, and `None` is written as an
/// absent property.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Terminal {
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
}

/// One 7501 status record a program reports.
///
/// The words are the program's own and nothing is translated. A terminal carries one record as its
/// status and may carry more by id.
///
/// An optional property that is absent or `null` reads as `None`, and `None` is written as an
/// absent property.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    /// What the program says it is doing.
    pub state: StatusWord,
    /// The program's name for itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    /// What a blocked program waits on: `permission`, `question` or `auth`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// A message from the program.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,
    /// Progress from 0 to 100, when the program said.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<u8>,
    /// A title from the program.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// The process that runs in a terminal right now.
///
/// The core reports it while the terminal is live, and not once it has finished.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Foreground {
    /// The foreground process group's leader.
    pub pid: i32,
    /// The process name as the operating system has it.
    pub name: String,
    /// The process's command line.
    pub argv: Vec<String>,
    /// The process's working directory now.
    pub cwd: String,
}

/// Whether a template run's first message went in.
///
/// A template can send a first message when it starts. This record says whether that happened.
///
/// An optional property that is absent or `null` reads as `None`, and `None` is written as an
/// absent property.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstMessage {
    /// Whether the message went in.
    pub sent: bool,
    /// Why it was not sent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Whether the agent started working on it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started: Option<bool>,
    /// The turn when it went in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn: Option<u64>,
}

/// What a viewer's own terminal looks like, as the client reports it.
///
/// This is the schema type named `Display`. It is called `DisplayFacts` here so it never shadows
/// `std::fmt::Display`.
///
/// An optional property that is absent or `null` reads as `None`, and `None` is written as an
/// absent property.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisplayFacts {
    /// The default background as `#rrggbb`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg: Option<String>,
    /// The cell size in pixels, as width then height.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cell: Option<[u32; 2]>,
    /// The cursor color as `#rrggbb`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// The default foreground as `#rrggbb`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fg: Option<String>,
}

/// The size a terminal starts with.
///
/// Both sides are at least 1, because the core refuses a terminal of no columns or no rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitialSize {
    /// The initial width in columns.
    pub cols: NonZeroU16,
    /// The initial height in rows.
    pub rows: NonZeroU16,
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::Value;

    use super::{DisplayFacts, FirstMessage, Foreground, InitialSize, Status, Terminal};

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
    fn terminal_round_trips_a_populated_value() -> TestResult {
        let wire = r#"{"address":"t7","argv":["sh","-c","sleep 1"],"cols":80,"created_ms":1700000000000,"cwd":"/work","labels":{"a":"1"},"rows":24,"screen":"exact","state":"finished","die_with_parent":true,"exit_code":0,"finished_ms":1700000001000,"first_message":{"sent":true,"reason":"busy","started":true,"turn":2},"foreground":{"pid":4242,"name":"vim","argv":["vim"],"cwd":"/work"},"identity":"ghostty","keep":true,"name":"build","notify":true,"parent":"t3","paste_gap_ms":50,"pid":4242,"reason":"exited","records":{"sub":{"state":"working","progress":5}},"reported":true,"rm":true,"signal":9,"status":{"state":"idle","title":"t"},"status_ms":1700000000500,"stop":"esc","submit":"enter","template":"claude","title":"t","turn":3}"#;
        let value: Terminal = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn terminal_optional_fields_read_absent_and_null_as_none() -> TestResult {
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
        let null = r#"{"address":"t7","argv":["sh","-c","sleep 1"],"cols":80,"created_ms":1700000000000,"cwd":"/work","labels":{"a":"1"},"rows":24,"screen":"exact","state":"finished","die_with_parent":null,"exit_code":null,"finished_ms":null,"first_message":null,"foreground":null,"identity":null,"keep":null,"name":null,"notify":null,"parent":null,"paste_gap_ms":null,"pid":null,"reason":null,"records":null,"reported":null,"rm":null,"signal":null,"status":null,"status_ms":null,"stop":null,"submit":null,"template":null,"title":null,"turn":null}"#;
        let from_absent: Terminal = serde_json::from_str(absent)?;
        let from_null: Terminal = serde_json::from_str(null)?;
        assert_eq!(from_absent, from_null);
        assert_eq!(
            serde_json::to_value(&from_absent)?,
            serde_json::from_str::<Value>(absent)?
        );
        Ok(())
    }

    #[test]
    fn terminal_round_trips_a_live_record() -> TestResult {
        let reply = r#"{"address":"t2","argv":["sh"],"cols":200,"created_ms":1795356738856,"cwd":"/work/probe-methods","die_with_parent":false,"exit_code":null,"finished_ms":null,"identity":"ghostty","keep":false,"labels":{"a":"1","b":"2"},"name":null,"notify":true,"parent":null,"pid":56464,"reason":null,"reported":false,"rm":false,"rows":120,"screen":"exact","signal":null,"state":"live","status":null,"status_ms":null,"template":null,"title":"","turn":0}"#;
        let value: Terminal = serde_json::from_str(reply)?;
        let expected = without_nulls(serde_json::from_str::<Value>(reply)?);
        assert_eq!(serde_json::to_value(&value)?, expected);
        Ok(())
    }

    #[test]
    fn status_round_trips_a_populated_value() -> TestResult {
        let wire = r#"{
    "state": "working",
    "app": "agent",
    "kind": "question",
    "msg": "thinking",
    "progress": 40,
    "title": "task"
}"#;
        let value: Status = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn status_optional_fields_read_absent_and_null_as_none() -> TestResult {
        let absent = r#"{
    "state": "working"
}"#;
        let null = r#"{
    "state": "working",
    "app": null,
    "kind": null,
    "msg": null,
    "progress": null,
    "title": null
}"#;
        let from_absent: Status = serde_json::from_str(absent)?;
        let from_null: Status = serde_json::from_str(null)?;
        assert_eq!(from_absent, from_null);
        assert_eq!(
            serde_json::to_value(&from_absent)?,
            serde_json::from_str::<Value>(absent)?
        );
        Ok(())
    }

    #[test]
    fn foreground_round_trips_a_populated_value() -> TestResult {
        let wire = r#"{
    "pid": 4242,
    "name": "vim",
    "argv": [
        "vim",
        "notes.md"
    ],
    "cwd": "/work"
}"#;
        let value: Foreground = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn first_message_round_trips_a_populated_value() -> TestResult {
        let wire = r#"{
    "sent": true,
    "reason": "busy",
    "started": true,
    "turn": 2
}"#;
        let value: FirstMessage = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn first_message_optional_fields_read_absent_and_null_as_none() -> TestResult {
        let absent = r#"{
    "sent": true
}"#;
        let null = r#"{
    "sent": true,
    "reason": null,
    "started": null,
    "turn": null
}"#;
        let from_absent: FirstMessage = serde_json::from_str(absent)?;
        let from_null: FirstMessage = serde_json::from_str(null)?;
        assert_eq!(from_absent, from_null);
        assert_eq!(
            serde_json::to_value(&from_absent)?,
            serde_json::from_str::<Value>(absent)?
        );
        Ok(())
    }

    #[test]
    fn display_facts_round_trips_a_populated_value() -> TestResult {
        let wire = r##"{
    "bg": "#000000",
    "cell": [
        8,
        16
    ],
    "cursor": "#ffffff",
    "fg": "#cccccc"
}"##;
        let value: DisplayFacts = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn display_facts_optional_fields_read_absent_and_null_as_none() -> TestResult {
        let absent = r"{}";
        let null = r#"{
    "bg": null,
    "cell": null,
    "cursor": null,
    "fg": null
}"#;
        let from_absent: DisplayFacts = serde_json::from_str(absent)?;
        let from_null: DisplayFacts = serde_json::from_str(null)?;
        assert_eq!(from_absent, from_null);
        assert_eq!(
            serde_json::to_value(&from_absent)?,
            serde_json::from_str::<Value>(absent)?
        );
        Ok(())
    }

    #[test]
    fn display_facts_cell_has_two_items() -> TestResult {
        let facts: DisplayFacts = serde_json::from_str(r#"{"cell":[8,16]}"#)?;
        assert_eq!(facts.cell, Some([8, 16]));
        assert!(serde_json::from_str::<DisplayFacts>(r#"{"cell":[8]}"#).is_err());
        assert!(serde_json::from_str::<DisplayFacts>(r#"{"cell":[8,16,2]}"#).is_err());
        Ok(())
    }

    #[test]
    fn initial_size_round_trips_a_populated_value() -> TestResult {
        let wire = r#"{
    "cols": 80,
    "rows": 24
}"#;
        let value: InitialSize = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn initial_size_refuses_zero() {
        assert!(serde_json::from_str::<InitialSize>(r#"{"cols":0,"rows":24}"#).is_err());
        assert!(serde_json::from_str::<InitialSize>(r#"{"cols":80,"rows":0}"#).is_err());
    }
}
