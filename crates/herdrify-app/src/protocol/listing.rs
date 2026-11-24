//! The `ls` listing: the spaces, tabs and terminals of a window and the spaces closed recently.
//!
//! `herdr-app ls` prints one JSON document. [`Listing`] decodes it, with [`SpaceEntry`],
//! [`TabEntry`] and [`TerminalEntry`] for the three nested levels and [`RecentSpace`] for a space
//! that was closed. Each type ignores fields herdr-app adds later, and the status word of a
//! terminal is kept as written when this crate does not know it.
//!
//! Responsibilities:
//!
//! - decoding the `ls` document into typed entries, reusing the identifier types of
//!   [`protocol`](crate::protocol) and [`files`](crate::files)
//! - the agent status of a terminal as an open word list
//!
//! Non-responsibilities:
//!
//! - running `herdr-app ls` to obtain the document, which a layer above does
//! - reading fields this crate has no type for, which the caller does from the raw output

use serde::Deserialize;

use crate::files::TerminalAddress;
use crate::protocol::{AppName, MachineName, PaneId, SpaceId, TabId};

/// What `herdr-app ls` prints: the open spaces and the recently closed ones.
///
/// Fields herdr-app adds later are ignored.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::Listing;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let listing: Listing = serde_json::from_str(
///     r#"{
///         "spaces": [{
///             "id": "sp1", "name": "work", "folder": "/work", "machine": null,
///             "active": true,
///             "tabs": [{
///                 "id": "tb1", "name": "1", "active": true, "settled": true,
///                 "terminals": [{
///                     "terminal": "t1", "pane": "p1", "status": "idle", "app": null,
///                     "name": null, "sub_agents": []
///                 }]
///             }]
///         }],
///         "recent": []
///     }"#,
/// )?;
/// assert_eq!(listing.spaces.len(), 1);
/// assert_eq!(listing.spaces[0].name, "work");
/// assert!(listing.recent.is_empty());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Listing {
    /// The open spaces, in the order herdr-app lists them.
    pub spaces: Vec<SpaceEntry>,
    /// The spaces closed recently, which can be opened again.
    pub recent: Vec<RecentSpace>,
}

/// One open space of the listing.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::SpaceEntry;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let space: SpaceEntry = serde_json::from_str(
///     r#"{"id":"sp1","name":"work","folder":"/work","machine":"box","active":false,"tabs":[]}"#,
/// )?;
/// assert_eq!(space.folder, "/work");
/// assert_eq!(space.machine.as_ref().map(|machine| machine.as_str()), Some("box"));
/// assert!(!space.active);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SpaceEntry {
    /// The space's identifier.
    pub id: SpaceId,
    /// The space's name, as herdr-app printed it. It is a plain `String`, not a newtype: an output
    /// value is kept unchecked, so a listing never fails to decode.
    pub name: String,
    /// The folder the space was opened in, as herdr-app printed it. It is a plain `String`, not a
    /// `Folder`: it is kept unchecked, so a listing never fails to decode, and the folder of a space
    /// on another machine need not be absolute here.
    pub folder: String,
    /// The machine the space runs on, when it is not this one.
    pub machine: Option<MachineName>,
    /// Whether the space is the one in focus.
    pub active: bool,
    /// The space's tabs, in order.
    pub tabs: Vec<TabEntry>,
}

/// One tab of a space.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::TabEntry;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let tab: TabEntry = serde_json::from_str(
///     r#"{"id":"tb1","name":"1","active":true,"settled":false,"terminals":[]}"#,
/// )?;
/// assert_eq!(tab.id.as_str(), "tb1");
/// assert!(tab.active);
/// assert!(!tab.settled);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TabEntry {
    /// The tab's identifier.
    pub id: TabId,
    /// The tab's name, as herdr-app printed it. It is a plain `String`, not a newtype: an output
    /// value is kept unchecked, so a listing never fails to decode.
    pub name: String,
    /// Whether the tab is the one in focus in its space.
    pub active: bool,
    /// Whether the tab is settled: put away on this screen, out of the lists but still running. It
    /// comes back when it is shown or when an agent in it needs you.
    pub settled: bool,
    /// The tab's terminals, in order.
    pub terminals: Vec<TerminalEntry>,
}

/// One terminal of a tab.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::{AgentStatus, TerminalEntry};
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let terminal: TerminalEntry = serde_json::from_str(
///     r#"{"terminal":"t1","pane":null,"status":"blocked","app":"claude","name":"main","sub_agents":["t2"]}"#,
/// )?;
/// assert_eq!(terminal.terminal.as_str(), "t1");
/// assert_eq!(terminal.status, Some(AgentStatus::Blocked));
/// assert_eq!(terminal.name.as_deref(), Some("main"));
/// assert_eq!(terminal.sub_agents.len(), 1);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TerminalEntry {
    /// The terminal's address, such as `t1`.
    pub terminal: TerminalAddress,
    /// The pane showing the terminal, when it is shown in one.
    pub pane: Option<PaneId>,
    /// The status of the agent running in the terminal, when one is.
    pub status: Option<AgentStatus>,
    /// The application running in the terminal, when herdr-app knows it.
    pub app: Option<AppName>,
    /// The terminal's name, when it has one, as herdr-app printed it. It is a plain `String`, not a
    /// newtype: an output value is kept unchecked, so a listing never fails to decode.
    pub name: Option<String>,
    /// The addresses of the sub-agents the terminal started.
    pub sub_agents: Vec<TerminalAddress>,
}

/// A space closed recently.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::RecentSpace;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let recent: RecentSpace = serde_json::from_str(r#"{"name":"gone","folder":"/work"}"#)?;
/// assert_eq!(recent.name, "gone");
/// assert_eq!(recent.folder, "/work");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RecentSpace {
    /// The space's name, as herdr-app printed it. It is a plain `String`, not a newtype: an output
    /// value is kept unchecked, so a listing never fails to decode.
    pub name: String,
    /// The folder the space was opened in, as herdr-app printed it. It is a plain `String`, not a
    /// `Folder`: it is kept unchecked, so a listing never fails to decode, and the folder of a space
    /// on another machine need not be absolute here.
    pub folder: String,
}

/// The status of the agent running in a terminal.
///
/// The word list is open: a word this crate does not know is kept as [`AgentStatus::Other`].
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::AgentStatus;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// assert_eq!(serde_json::from_str::<AgentStatus>("\"done\"")?, AgentStatus::Done);
/// assert_eq!(
///     serde_json::from_str::<AgentStatus>("\"thinking\"")?,
///     AgentStatus::Other("thinking".to_owned()),
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    /// The word `working`: the agent is busy.
    Working,
    /// The word `blocked`: the agent waits for an answer.
    Blocked,
    /// The word `done`: the agent has finished.
    Done,
    /// The word `idle`: the agent has nothing to do.
    Idle,
    /// A word this crate does not know, as written.
    #[serde(untagged)]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::{AgentStatus, Listing};
    use crate::files::TerminalAddress;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    // recorded from herdr-app 0.1.17, with local paths replaced
    const RECORDED_LS: &str = r#"
{
  "spaces": [
    {
      "id": "sp791801470e39e979233f19213e2f5b3b",
      "name": "hzprobe",
      "folder": "/home/user",
      "machine": null,
      "active": true,
      "tabs": [
        {
          "id": "tbac537ebad46b69af5db7cafb14f4a078",
          "name": "1",
          "active": true,
          "settled": false,
          "terminals": [
            {
              "terminal": "t1",
              "pane": "p13436eac9efd61a6971094912e82edd7",
              "status": null,
              "app": null,
              "name": null,
              "sub_agents": [
                "t2"
              ]
            }
          ]
        }
      ]
    }
  ],
  "recent": [
    {
      "name": "gone",
      "folder": "/home/user"
    }
  ]
}
"#;

    #[test]
    fn listing_decodes_a_recorded_ls() -> TestResult {
        let listing: Listing = serde_json::from_str(RECORDED_LS)?;
        assert_eq!(listing.spaces.len(), 1);
        let space = &listing.spaces[0];
        assert_eq!(space.name, "hzprobe");
        assert_eq!(space.machine, None);
        assert!(space.active);
        let tab = &space.tabs[0];
        assert!(!tab.settled);
        let terminal = &tab.terminals[0];
        assert_eq!(terminal.terminal.as_str(), "t1");
        assert_eq!(terminal.status, None);
        let sub_agents: Vec<&str> = terminal
            .sub_agents
            .iter()
            .map(TerminalAddress::as_str)
            .collect();
        assert_eq!(sub_agents, ["t2"]);
        assert_eq!(listing.recent.len(), 1);
        assert_eq!(listing.recent[0].name, "gone");
        assert_eq!(listing.recent[0].folder, "/home/user");
        Ok(())
    }

    #[test]
    fn listing_decodes_an_empty_window() -> TestResult {
        let listing: Listing = serde_json::from_str(r#"{"spaces":[],"recent":[]}"#)?;
        assert!(listing.spaces.is_empty());
        assert!(listing.recent.is_empty());
        Ok(())
    }

    #[test]
    fn agent_status_keeps_unknown_words() -> TestResult {
        let known: Option<AgentStatus> = serde_json::from_str(r#""working""#)?;
        assert_eq!(known, Some(AgentStatus::Working));
        let unknown: Option<AgentStatus> = serde_json::from_str(r#""thinking""#)?;
        assert_eq!(unknown, Some(AgentStatus::Other("thinking".to_owned())));
        let absent: Option<AgentStatus> = serde_json::from_str("null")?;
        assert_eq!(absent, None);
        Ok(())
    }

    #[test]
    fn listing_ignores_unknown_fields() -> TestResult {
        let listing: Listing = serde_json::from_str(
            r#"{"x":1,"spaces":[{"id":"sp1","name":"a","folder":"/a","machine":null,"active":false,"x":1,
            "tabs":[{"id":"tb1","name":"1","active":false,"settled":true,"x":1,
            "terminals":[{"terminal":"t1","pane":null,"status":"idle","app":null,"name":null,"sub_agents":[],"x":1}]}]}],
            "recent":[{"name":"gone","folder":"/a","x":1}]}"#,
        )?;
        assert_eq!(
            listing.spaces[0].tabs[0].terminals[0].status,
            Some(AgentStatus::Idle)
        );
        assert_eq!(listing.recent[0].name, "gone");
        Ok(())
    }
}
