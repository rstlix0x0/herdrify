//! Notifications: the messages herdr-core sends with no id, and the events among them.
//!
//! A subscribed connection carries `event`, `heartbeat` and `missed` notifications. An `event` has
//! a sequence number, a type word, the terminal it is about and a `data` object whose shape the
//! type decides. Upstream adds event types without a new protocol generation, so a type or method
//! this crate does not know is kept as it arrived rather than refused.
//!
//! Responsibilities:
//!
//! - [`Notification`], a received notification read into its typed form
//! - [`Event`] and [`EventKind`], an event and the data its type carries
//! - [`Seq`], the sequence number of a notification
//!
//! Non-responsibilities:
//!
//! - telling a notification from a response, which the envelope does
//! - dropping events a `missed` notice covers, which the subscription does

use std::collections::BTreeMap;
use std::fmt;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    DocName, DocVersion, EndReason, Foreground, Labels, MachineName, ProtocolError, Status,
    StatusWord, Terminal, TerminalAddress,
};

/// The sequence number of a notification.
///
/// It restarts with each core process, so it orders notifications of one core run only.
///
/// The field is public so a caller writes `Seq(42)`.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::Seq;
///
/// assert!(Seq(43) > Seq(42));
/// assert_eq!(Seq(42).get(), 42);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Seq(pub u64);

impl Seq {
    /// The sequence number.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for Seq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// An event: something that happened to a terminal, or to a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// The event's sequence number.
    pub seq: Seq,
    /// The terminal the event is about, addressed as seen from the subscribed core, such as `t7` or
    /// `rohan/t7`. Absent for an event that is not about a terminal.
    pub terminal: Option<TerminalAddress>,
    /// The machine the event came from, when it is not the subscribed core.
    pub machine: Option<MachineName>,
    /// What happened, with the data its type carries.
    pub kind: EventKind,
}

/// What an event says happened.
///
/// A type this crate does not know is [`EventKind::Unknown`], with its data as it arrived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventKind {
    /// A terminal appeared; the data is its whole record.
    Created(Terminal),
    /// A terminal was started again; the data is its whole record.
    Restarted(Terminal),
    /// A kept terminal lost its program and stays live.
    Lost {
        /// Why the program ended.
        reason: EndReason,
        /// The exit code, when the program exited.
        exit_code: Option<i32>,
        /// The signal that ended the program, when one did.
        signal: Option<i32>,
        /// When the terminal finished, in milliseconds; absent while it stays live.
        finished_ms: Option<u64>,
    },
    /// A terminal's program ended.
    Finished {
        /// Why the program ended.
        reason: EndReason,
        /// The exit code, when the program exited.
        exit_code: Option<i32>,
        /// The signal that ended the program, when one did.
        signal: Option<i32>,
        /// When the terminal finished, in milliseconds.
        finished_ms: Option<u64>,
    },
    /// A terminal's keep setting changed.
    Keep {
        /// Whether the terminal is kept.
        keep: bool,
    },
    /// A terminal was removed.
    Removed,
    /// A terminal's reported status changed.
    Status {
        /// The root status; absent when it was cleared.
        status: Option<Status>,
        /// Every status record by key.
        records: BTreeMap<String, Status>,
        /// Whether the terminal ever reported.
        reported: bool,
    },
    /// A turn ended.
    Turn {
        /// The turn counter after the turn ended.
        turn: u64,
        /// The word the turn ended on.
        state: StatusWord,
    },
    /// A terminal's title changed.
    Title {
        /// The new title.
        title: String,
    },
    /// A terminal's labels changed.
    Labels {
        /// Every label now.
        labels: Labels,
        /// The whole record, when the terminal newly matches the subscription's filter.
        record: Option<Terminal>,
    },
    /// A terminal's size changed.
    Size {
        /// The new column count.
        cols: u16,
        /// The new row count.
        rows: u16,
    },
    /// What runs in a terminal now; absent when nothing does.
    Foreground(Option<Foreground>),
    /// A document changed.
    Doc {
        /// The document's name.
        name: DocName,
        /// The document's new version.
        version: DocVersion,
    },
    /// An event type this crate does not know.
    Unknown {
        /// The type word.
        kind: String,
        /// The event's `data`, as it arrived.
        data: Value,
    },
}

/// A notification from herdr-core, read into its typed form.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::{Notification, Seq};
///
/// let notification = Notification::from_parts("heartbeat", serde_json::json!({"seq": 3}))?;
/// assert_eq!(notification, Notification::Heartbeat { seq: Seq(3) });
/// # Ok::<(), herdrify_hc::protocol::ProtocolError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notification {
    /// Something happened to a terminal or a document. The event is boxed because it holds whole
    /// terminal records, which would make every notification as large as one.
    Event(Box<Event>),
    /// The core is alive, and has sent every notification up to `seq`.
    Heartbeat {
        /// The latest sequence number.
        seq: Seq,
    },
    /// The core dropped notifications this subscriber did not take in time.
    Missed {
        /// The sequence number after which delivery resumes; events at or below it are stale.
        seq: Seq,
        /// A fresh snapshot of the matching terminals. It replaces what the subscriber held, rather
        /// than patching it.
        terminals: Vec<Terminal>,
    },
    /// A notification method this crate does not know.
    Other {
        /// The method word.
        method: String,
        /// The parameters, as they arrived.
        params: Value,
    },
}

impl Notification {
    /// Reads the notification `method` with `params`.
    ///
    /// # Errors
    ///
    /// - [`ProtocolError::InvalidNotification`] when `heartbeat` or `missed` parameters do not
    ///   decode, or an `event` whose envelope does not: `seq` missing or not a `u64`, `type`
    ///   missing or not a string, `terminal` or `machine` not a string, or `machine` empty.
    /// - [`ProtocolError::InvalidEvent`] when the `data` of an event type this crate knows does not
    ///   match the shape that type carries.
    pub fn from_parts(method: &str, params: Value) -> Result<Self, ProtocolError> {
        match method {
            "event" => event(params).map(|event| Self::Event(Box::new(event))),
            "heartbeat" => {
                let SeqParams { seq } = notification_params(method, params)?;
                Ok(Self::Heartbeat { seq })
            }
            "missed" => {
                let MissedParams { seq, terminals } = notification_params(method, params)?;
                Ok(Self::Missed { seq, terminals })
            }
            _ => Ok(Self::Other {
                method: method.to_owned(),
                params,
            }),
        }
    }
}

/// The parameters of `heartbeat`.
#[derive(Deserialize)]
struct SeqParams {
    seq: Seq,
}

/// The parameters of `missed`.
#[derive(Deserialize)]
struct MissedParams {
    seq: Seq,
    terminals: Vec<Terminal>,
}

/// The parameters of `event`, before `data` is read by `type`.
#[derive(Deserialize)]
struct EventParams {
    seq: Seq,
    #[serde(rename = "type")]
    kind: String,
    terminal: Option<TerminalAddress>,
    machine: Option<MachineName>,
    #[serde(default)]
    data: Value,
}

/// The `data` of `lost` and `finished`.
#[derive(Deserialize)]
struct EndedData {
    reason: EndReason,
    exit_code: Option<i32>,
    signal: Option<i32>,
    finished_ms: Option<u64>,
}

/// The `data` of `keep`.
#[derive(Deserialize)]
struct KeepData {
    keep: bool,
}

/// The `data` of `status`.
#[derive(Deserialize)]
struct StatusData {
    status: Option<Status>,
    records: BTreeMap<String, Status>,
    reported: bool,
}

/// The `data` of `turn`.
#[derive(Deserialize)]
struct TurnData {
    turn: u64,
    state: StatusWord,
}

/// The `data` of `title`.
#[derive(Deserialize)]
struct TitleData {
    title: String,
}

/// The `data` of `labels`.
#[derive(Deserialize)]
struct LabelsData {
    labels: Labels,
    record: Option<Terminal>,
}

/// The `data` of `size`.
#[derive(Deserialize)]
struct SizeData {
    cols: u16,
    rows: u16,
}

/// The `data` of `doc`.
#[derive(Deserialize)]
struct DocData {
    name: DocName,
    version: DocVersion,
}

fn notification_params<T: DeserializeOwned>(
    method: &str,
    params: Value,
) -> Result<T, ProtocolError> {
    serde_json::from_value(params).map_err(|source| ProtocolError::InvalidNotification {
        method: method.to_owned(),
        source,
    })
}

fn event_data<T: DeserializeOwned>(kind: &str, data: Value) -> Result<T, ProtocolError> {
    serde_json::from_value(data).map_err(|source| ProtocolError::InvalidEvent {
        kind: kind.to_owned(),
        source,
    })
}

fn event(params: Value) -> Result<Event, ProtocolError> {
    let EventParams {
        seq,
        kind,
        terminal,
        machine,
        data,
    } = notification_params("event", params)?;
    let kind = event_kind(kind, data)?;
    Ok(Event {
        seq,
        terminal,
        machine,
        kind,
    })
}

fn event_kind(kind: String, data: Value) -> Result<EventKind, ProtocolError> {
    Ok(match kind.as_str() {
        "created" => EventKind::Created(event_data(&kind, data)?),
        "restarted" => EventKind::Restarted(event_data(&kind, data)?),
        "lost" => {
            let EndedData {
                reason,
                exit_code,
                signal,
                finished_ms,
            } = event_data(&kind, data)?;
            EventKind::Lost {
                reason,
                exit_code,
                signal,
                finished_ms,
            }
        }
        "finished" => {
            let EndedData {
                reason,
                exit_code,
                signal,
                finished_ms,
            } = event_data(&kind, data)?;
            EventKind::Finished {
                reason,
                exit_code,
                signal,
                finished_ms,
            }
        }
        "keep" => EventKind::Keep {
            keep: event_data::<KeepData>(&kind, data)?.keep,
        },
        "removed" => EventKind::Removed,
        "status" => {
            let StatusData {
                status,
                records,
                reported,
            } = event_data(&kind, data)?;
            EventKind::Status {
                status,
                records,
                reported,
            }
        }
        "turn" => {
            let TurnData { turn, state } = event_data(&kind, data)?;
            EventKind::Turn { turn, state }
        }
        "title" => EventKind::Title {
            title: event_data::<TitleData>(&kind, data)?.title,
        },
        "labels" => {
            let LabelsData { labels, record } = event_data(&kind, data)?;
            EventKind::Labels { labels, record }
        }
        "size" => {
            let SizeData { cols, rows } = event_data(&kind, data)?;
            EventKind::Size { cols, rows }
        }
        "foreground" => EventKind::Foreground(event_data(&kind, data)?),
        "doc" => {
            let DocData { name, version } = event_data(&kind, data)?;
            EventKind::Doc { name, version }
        }
        _ => EventKind::Unknown { kind, data },
    })
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::{Value, json};

    use super::{EventKind, Notification, Seq};
    use crate::protocol::{
        DocName, DocVersion, EndReason, MachineName, ProtocolError, Status, StatusWord,
        TerminalAddress,
    };

    type TestResult = Result<(), Box<dyn Error>>;

    fn event(seq: u64, kind: &str, data: Value) -> Value {
        let mut params = json!({"seq": seq, "type": kind, "terminal": "t7"});
        params["data"] = data;
        params
    }

    fn decode_event(kind: &str, data: Value) -> Result<EventKind, Box<dyn Error>> {
        match Notification::from_parts("event", event(1, kind, data))? {
            Notification::Event(event) => Ok(event.kind),
            other => Err(format!("not an event: {other:?}").into()),
        }
    }

    fn working() -> Status {
        Status {
            state: StatusWord::Working,
            app: None,
            kind: None,
            msg: None,
            progress: None,
            title: None,
        }
    }

    fn terminal_record() -> Value {
        json!({"address":"t7","argv":["sh"],"cols":200,"created_ms":1_795_356_785_624_u64,"cwd":"/work","die_with_parent":false,"exit_code":null,"finished_ms":null,"identity":"ghostty","keep":false,"labels":{},"name":null,"notify":true,"parent":null,"pid":56757,"reason":null,"reported":false,"rm":false,"rows":120,"screen":"exact","signal":null,"state":"live","status":null,"status_ms":null,"template":null,"title":"","turn":0})
    }

    #[test]
    fn decodes_observed_events() -> TestResult {
        let status = decode_event(
            "status",
            json!({"records":{},"reported":true,"status":{"state":"working"}}),
        )?;
        assert_eq!(
            status,
            EventKind::Status {
                status: Some(working()),
                records: std::collections::BTreeMap::new(),
                reported: true,
            }
        );
        let blocked = decode_event(
            "status",
            json!({"records":{},"reported":true,"status":{"kind":"question","state":"blocked"}}),
        )?;
        assert_eq!(
            blocked,
            EventKind::Status {
                status: Some(Status {
                    state: StatusWord::Blocked,
                    kind: Some("question".to_owned()),
                    ..working()
                }),
                records: std::collections::BTreeMap::new(),
                reported: true,
            }
        );
        let turn = decode_event("turn", json!({"state":"idle","turn":1}))?;
        assert_eq!(
            turn,
            EventKind::Turn {
                turn: 1,
                state: StatusWord::Idle,
            }
        );
        let title = decode_event("title", json!({"title":"hfy-title"}))?;
        assert_eq!(
            title,
            EventKind::Title {
                title: "hfy-title".to_owned()
            }
        );
        Ok(())
    }

    #[test]
    fn a_removed_event_ignores_its_data() -> TestResult {
        let with_data = json!({"anything": 1, "nested": {"a": [1, 2]}});
        assert_eq!(decode_event("removed", with_data)?, EventKind::Removed);
        assert_eq!(decode_event("removed", json!("text"))?, EventKind::Removed);
        Ok(())
    }

    #[test]
    fn a_foreground_event_without_data_reads_as_no_foreground() -> TestResult {
        let params = json!({"seq": 3, "type": "foreground", "terminal": "t7"});
        let Notification::Event(event) = Notification::from_parts("event", params)? else {
            return Err("not an event".into());
        };
        assert_eq!(event.kind, EventKind::Foreground(None));
        Ok(())
    }

    #[test]
    fn event_carries_its_envelope() -> TestResult {
        let params = json!({"seq":42,"type":"title","terminal":"rohan/t7","machine":"rohan","data":{"title":"x"}});
        let Notification::Event(event) = Notification::from_parts("event", params)? else {
            return Err("not an event".into());
        };
        assert_eq!(event.seq, Seq(42));
        assert_eq!(event.terminal, Some(TerminalAddress::new("rohan/t7")));
        assert_eq!(event.machine, Some(MachineName::new("rohan")?));
        let bare = json!({"seq":43,"type":"doc","data":{"name":"herdr-app","version":2}});
        let Notification::Event(event) = Notification::from_parts("event", bare)? else {
            return Err("not an event".into());
        };
        assert_eq!((event.terminal, event.machine), (None, None));
        Ok(())
    }

    #[test]
    fn decodes_the_doc_event() -> TestResult {
        let params = json!({"seq":43,"type":"doc","data":{"name":"herdr-app","version":2}});
        let Notification::Event(event) = Notification::from_parts("event", params)? else {
            return Err("not an event".into());
        };
        assert_eq!(event.seq, Seq(43));
        assert_eq!(
            event.kind,
            EventKind::Doc {
                name: DocName::new("herdr-app")?,
                version: DocVersion(2),
            }
        );
        Ok(())
    }

    #[test]
    fn status_event_with_null_status() -> TestResult {
        let kind = decode_event(
            "status",
            json!({"records":{},"reported":true,"status":null}),
        )?;
        assert_eq!(
            kind,
            EventKind::Status {
                status: None,
                records: std::collections::BTreeMap::new(),
                reported: true,
            }
        );
        Ok(())
    }

    #[test]
    fn decodes_the_remaining_types() -> TestResult {
        let ended = json!({"exit_code":0,"finished_ms":1_795_356_788_162_u64,"reason":"exited","signal":null});
        assert_eq!(
            decode_event("finished", ended)?,
            EventKind::Finished {
                reason: EndReason::Exited,
                exit_code: Some(0),
                signal: None,
                finished_ms: Some(1_795_356_788_162),
            }
        );
        let lost =
            json!({"reason":"holder_lost","exit_code":null,"signal":null,"finished_ms":null});
        assert_eq!(
            decode_event("lost", lost)?,
            EventKind::Lost {
                reason: EndReason::HolderLost,
                exit_code: None,
                signal: None,
                finished_ms: None,
            }
        );
        assert_eq!(
            decode_event("keep", json!({"keep":true}))?,
            EventKind::Keep { keep: true }
        );
        assert_eq!(decode_event("removed", json!({}))?, EventKind::Removed);
        assert_eq!(
            decode_event("size", json!({"cols":100,"rows":30}))?,
            EventKind::Size {
                cols: 100,
                rows: 30
            }
        );
        assert_eq!(
            decode_event("foreground", json!(null))?,
            EventKind::Foreground(None)
        );
        let foreground = decode_event(
            "foreground",
            json!({"pid":4242,"name":"vim","argv":["vim"],"cwd":"/work"}),
        )?;
        assert!(matches!(foreground, EventKind::Foreground(Some(f)) if f.pid == 4242));
        let created = decode_event("created", terminal_record())?;
        assert!(matches!(created, EventKind::Created(t) if t.address.as_str() == "t7"));
        let restarted = decode_event("restarted", terminal_record())?;
        assert!(matches!(restarted, EventKind::Restarted(_)));
        let labels = decode_event("labels", json!({"labels":{"team":"infra"}}))?;
        assert!(matches!(labels, EventKind::Labels { record: None, .. }));
        let with_record = decode_event(
            "labels",
            json!({"labels":{"team":"infra"},"record":terminal_record()}),
        )?;
        assert!(matches!(
            with_record,
            EventKind::Labels {
                record: Some(_),
                ..
            }
        ));
        Ok(())
    }

    #[test]
    fn unknown_event_type_is_kept() -> TestResult {
        let kind = decode_event("brand_new", json!({"a":1}))?;
        assert_eq!(
            kind,
            EventKind::Unknown {
                kind: "brand_new".to_owned(),
                data: json!({"a":1}),
            }
        );
        Ok(())
    }

    #[test]
    fn mismatched_event_data_is_invalid_event() -> TestResult {
        let error = Notification::from_parts("event", event(1, "title", json!(5)))
            .err()
            .ok_or("a title event with data 5 decoded")?;
        let ProtocolError::InvalidEvent { kind, .. } = &error else {
            return Err(format!("expected InvalidEvent, got {error:?}").into());
        };
        assert_eq!(kind, "title");
        assert!(
            error
                .to_string()
                .starts_with("cannot decode a title event: ")
        );
        Ok(())
    }

    #[test]
    fn heartbeat_missed_and_other() -> TestResult {
        assert_eq!(
            Notification::from_parts("heartbeat", json!({"seq":3}))?,
            Notification::Heartbeat { seq: Seq(3) }
        );
        assert_eq!(
            Notification::from_parts("missed", json!({"seq":9,"terminals":[]}))?,
            Notification::Missed {
                seq: Seq(9),
                terminals: Vec::new(),
            }
        );
        let missed =
            Notification::from_parts("missed", json!({"seq":9,"terminals":[terminal_record()]}))?;
        assert!(matches!(missed, Notification::Missed { terminals, .. } if terminals.len() == 1));
        assert_eq!(
            Notification::from_parts("brand.new", json!({"x":1}))?,
            Notification::Other {
                method: "brand.new".to_owned(),
                params: json!({"x":1}),
            }
        );
        Ok(())
    }

    #[test]
    fn malformed_notifications_are_invalid_notification() -> TestResult {
        let cases = [
            ("heartbeat", json!({})),
            ("missed", json!({"seq":"x"})),
            ("event", json!({"seq":1,"data":{}})),
            (
                "event",
                json!({"seq":1,"type":"title","machine":"","data":{"title":"x"}}),
            ),
            ("event", json!({"type":"title","data":{"title":"x"}})),
            ("event", json!({"seq":-1,"type":"title","data":{}})),
            ("event", json!({"seq":1,"type":5,"data":{}})),
            (
                "event",
                json!({"seq":1,"type":"title","terminal":5,"data":{}}),
            ),
            (
                "event",
                json!({"seq":1,"type":"title","machine":5,"data":{}}),
            ),
            ("event", json!(5)),
        ];
        for (method, params) in cases {
            let error = Notification::from_parts(method, params.clone())
                .err()
                .ok_or_else(|| format!("{method} {params} decoded"))?;
            let ProtocolError::InvalidNotification { method: named, .. } = &error else {
                return Err(format!("{method} {params}: got {error:?}").into());
            };
            assert_eq!(named, method);
            assert!(
                error
                    .to_string()
                    .starts_with(&format!("cannot decode a {method} notification: ")),
                "{error}"
            );
        }
        Ok(())
    }
}
