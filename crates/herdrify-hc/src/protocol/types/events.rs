//! The reply to `events.subscribe`.
//!
//! The type is read from and written to JSON exactly as the schema names its properties.
//!
//! Responsibilities:
//!
//! - [`SubscribeResult`]: the reply to `events.subscribe`
//!
//! Non-responsibilities:
//!
//! - deciding what the core accepts or reports, which the core decides

use serde::{Deserialize, Serialize};

use crate::protocol::Terminal;

/// The reply to `events.subscribe`.
///
/// It holds the matching terminals at the moment of the subscription and the sequence number events
/// follow.
///
/// An optional property that is absent or `null` reads as `None`, and `None` is written as an
/// absent property.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscribeResult {
    /// Events after this sequence follow.
    pub seq: u64,
    /// The matching terminals here and on connected machines.
    pub terminals: Vec<Terminal>,
    /// In framed mode, the channel events arrive on; absent in plain mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<u32>,
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::Value;

    use super::SubscribeResult;

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn round_trips_a_populated_value() -> TestResult {
        let wire = r#"{"seq":5,"terminals":[{"address":"t7","argv":["sh","-c","sleep 1"],"cols":80,"created_ms":1700000000000,"cwd":"/work","labels":{"a":"1"},"rows":24,"screen":"exact","state":"finished","die_with_parent":true,"exit_code":0,"finished_ms":1700000001000,"first_message":{"sent":true,"reason":"busy","started":true,"turn":2},"foreground":{"pid":4242,"name":"vim","argv":["vim"],"cwd":"/work"},"identity":"ghostty","keep":true,"name":"build","notify":true,"parent":"t3","paste_gap_ms":50,"pid":4242,"reason":"exited","records":{"sub":{"state":"working","progress":5}},"reported":true,"rm":true,"signal":9,"status":{"state":"idle","title":"t"},"status_ms":1700000000500,"stop":"esc","submit":"enter","template":"claude","title":"t","turn":3}],"channel":2}"#;
        let value: SubscribeResult = serde_json::from_str(wire)?;
        assert_eq!(
            serde_json::to_value(&value)?,
            serde_json::from_str::<Value>(wire)?
        );
        Ok(())
    }

    #[test]
    fn optional_fields_read_absent_and_null_as_none() -> TestResult {
        let absent = r#"{"seq":5,"terminals":[{"address":"t7","argv":["sh","-c","sleep 1"],"cols":80,"created_ms":1700000000000,"cwd":"/work","labels":{"a":"1"},"rows":24,"screen":"exact","state":"finished","die_with_parent":true,"exit_code":0,"finished_ms":1700000001000,"first_message":{"sent":true,"reason":"busy","started":true,"turn":2},"foreground":{"pid":4242,"name":"vim","argv":["vim"],"cwd":"/work"},"identity":"ghostty","keep":true,"name":"build","notify":true,"parent":"t3","paste_gap_ms":50,"pid":4242,"reason":"exited","records":{"sub":{"state":"working","progress":5}},"reported":true,"rm":true,"signal":9,"status":{"state":"idle","title":"t"},"status_ms":1700000000500,"stop":"esc","submit":"enter","template":"claude","title":"t","turn":3}]}"#;
        let null = r#"{"seq":5,"terminals":[{"address":"t7","argv":["sh","-c","sleep 1"],"cols":80,"created_ms":1700000000000,"cwd":"/work","labels":{"a":"1"},"rows":24,"screen":"exact","state":"finished","die_with_parent":true,"exit_code":0,"finished_ms":1700000001000,"first_message":{"sent":true,"reason":"busy","started":true,"turn":2},"foreground":{"pid":4242,"name":"vim","argv":["vim"],"cwd":"/work"},"identity":"ghostty","keep":true,"name":"build","notify":true,"parent":"t3","paste_gap_ms":50,"pid":4242,"reason":"exited","records":{"sub":{"state":"working","progress":5}},"reported":true,"rm":true,"signal":9,"status":{"state":"idle","title":"t"},"status_ms":1700000000500,"stop":"esc","submit":"enter","template":"claude","title":"t","turn":3}],"channel":null}"#;
        let from_absent: SubscribeResult = serde_json::from_str(absent)?;
        let from_null: SubscribeResult = serde_json::from_str(null)?;
        assert_eq!(from_absent, from_null);
        assert_eq!(
            serde_json::to_value(&from_absent)?,
            serde_json::from_str::<Value>(absent)?
        );
        Ok(())
    }

    #[test]
    fn subscribe_result_channel_is_optional() -> TestResult {
        let without: SubscribeResult = serde_json::from_str(r#"{"seq":1,"terminals":[]}"#)?;
        assert_eq!(without.channel, None);
        let with: SubscribeResult =
            serde_json::from_str(r#"{"seq":1,"terminals":[],"channel":1}"#)?;
        assert_eq!(with.channel, Some(1));
        Ok(())
    }
}
