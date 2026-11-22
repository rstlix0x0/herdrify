//! The request that opens a stream of events.
//!
//! The filters narrow which terminals the events are about. With none set, the stream carries
//! events for every terminal.
//!
//! Responsibilities:
//!
//! - [`EventsSubscribe`], with the schema's optional filters as setters
//!
//! Non-responsibilities:
//!
//! - reading the event stream, which the client's subscription does

use serde::Serialize;

use crate::protocol::{Labels, Method, SubscribeResult, TerminalRef};

/// The parameters of `events.subscribe`.
///
/// Nothing is sent that was not set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct EventsSubscribe {
    #[serde(skip_serializing_if = "Option::is_none")]
    children_of: Option<TerminalRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    labels: Option<Labels>,
    #[serde(skip_serializing_if = "Option::is_none")]
    terminal: Option<TerminalRef>,
}

impl EventsSubscribe {
    /// A subscription to every terminal's events.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            children_of: None,
            labels: None,
            terminal: None,
        }
    }

    /// Only events about children of this terminal.
    #[must_use]
    pub fn children_of(mut self, parent: TerminalRef) -> Self {
        self.children_of = Some(parent);
        self
    }

    /// Only events about terminals that carry these labels.
    #[must_use]
    pub fn labels(mut self, labels: Labels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Only events about this terminal.
    #[must_use]
    pub fn terminal(mut self, terminal: TerminalRef) -> Self {
        self.terminal = Some(terminal);
        self
    }
}

impl Method for EventsSubscribe {
    const NAME: &'static str = "events.subscribe";
    type Params = Self;
    type Output = SubscribeResult;
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use serde_json::json;

    use crate::protocol::{EventsSubscribe, LabelKey, LabelValue, Labels, TerminalRef};

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn events_subscribe_serializes_every_filter() -> TestResult {
        let labels: Labels =
            std::iter::once((LabelKey::new("team")?, LabelValue::new("infra")?)).collect();
        let params = EventsSubscribe::new()
            .children_of(TerminalRef::new("t1")?)
            .labels(labels)
            .terminal(TerminalRef::new("t2")?);
        assert_eq!(
            serde_json::to_value(params)?,
            json!({"children_of": "t1", "labels": {"team": "infra"}, "terminal": "t2"})
        );
        assert_eq!(serde_json::to_value(EventsSubscribe::new())?, json!({}));
        Ok(())
    }
}
