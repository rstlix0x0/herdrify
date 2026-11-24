//! The request that opens a stream of events.
//!
//! The filters narrow which terminals the events are about, and which documents' events are
//! delivered. With none set, the stream carries events for every terminal.
//!
//! Responsibilities:
//!
//! - [`EventsSubscribe`], with the schema's optional filters as setters
//!
//! Non-responsibilities:
//!
//! - reading the event stream, which the client's subscription does

use serde::Serialize;

use crate::protocol::{DocName, Labels, Method, SubscribeResult, TerminalRef};

/// The parameters of `events.subscribe`.
///
/// Nothing is sent that was not set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct EventsSubscribe {
    #[serde(skip_serializing_if = "Option::is_none")]
    children_of: Option<TerminalRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    docs: Option<Vec<DocName>>,
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
            docs: None,
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

    /// Document events for these names, on top of the terminal filters.
    ///
    /// A name ending in `*` is a prefix, and `*` alone matches every document. The names are sent
    /// in the order given, duplicates included, and a later call replaces the earlier names. An
    /// empty iterator clears them, so nothing is sent.
    ///
    /// On its own, `docs` delivers only the matching document events: no terminal events, and an
    /// empty snapshot. Combined with [`terminal`](Self::terminal), [`labels`](Self::labels) or
    /// [`children_of`](Self::children_of), both kinds of event are delivered. This holds on line
    /// and framed connections alike.
    ///
    /// Needs herdr-core 0.1.22: `Client::events_subscribe` refuses the request on an older core.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_hc::protocol::{DocName, EventsSubscribe};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let params = EventsSubscribe::new().docs([DocName::new("settings")?, DocName::new("ui.*")?]);
    /// assert_eq!(
    ///     serde_json::to_string(&params)?,
    ///     r#"{"docs":["settings","ui.*"]}"#
    /// );
    ///
    /// let cleared = params.docs(Vec::new());
    /// assert_eq!(serde_json::to_string(&cleared)?, "{}");
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn docs(mut self, names: impl IntoIterator<Item = DocName>) -> Self {
        let names: Vec<DocName> = names.into_iter().collect();
        self.docs = if names.is_empty() { None } else { Some(names) };
        self
    }

    /// Whether any document names are set.
    pub(crate) const fn docs_set(&self) -> bool {
        self.docs.is_some()
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

    use crate::protocol::{DocName, EventsSubscribe, LabelKey, LabelValue, Labels, TerminalRef};

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn events_subscribe_serializes_every_filter() -> TestResult {
        let labels: Labels =
            std::iter::once((LabelKey::new("team")?, LabelValue::new("infra")?)).collect();
        let params = EventsSubscribe::new()
            .children_of(TerminalRef::new("t1")?)
            .labels(labels)
            .terminal(TerminalRef::new("t2")?)
            .docs([DocName::new("settings")?]);
        assert_eq!(
            serde_json::to_value(params)?,
            json!({
                "children_of": "t1",
                "docs": ["settings"],
                "labels": {"team": "infra"},
                "terminal": "t2"
            })
        );
        assert_eq!(serde_json::to_value(EventsSubscribe::new())?, json!({}));
        Ok(())
    }

    fn doc(name: &str) -> Result<DocName, Box<dyn Error>> {
        Ok(DocName::new(name)?)
    }

    #[test]
    fn docs_are_sent_in_order_with_duplicates() -> TestResult {
        let params = EventsSubscribe::new().docs([doc("a")?, doc("b")?, doc("a")?]);
        assert!(params.docs_set());
        assert_eq!(
            serde_json::to_value(params)?,
            json!({"docs": ["a", "b", "a"]})
        );
        Ok(())
    }

    #[test]
    fn docs_with_an_empty_iterator_sends_nothing() -> TestResult {
        let params = EventsSubscribe::new().docs(Vec::new());
        assert!(!params.docs_set());
        assert_eq!(serde_json::to_value(params)?, json!({}));
        Ok(())
    }

    #[test]
    fn a_second_docs_call_replaces_the_first() -> TestResult {
        let replaced = EventsSubscribe::new().docs([doc("a")?]).docs([doc("b")?]);
        assert_eq!(serde_json::to_value(replaced)?, json!({"docs": ["b"]}));

        let cleared = EventsSubscribe::new().docs([doc("a")?]).docs(Vec::new());
        assert!(!cleared.docs_set());
        assert_eq!(serde_json::to_value(cleared)?, json!({}));
        Ok(())
    }
}
