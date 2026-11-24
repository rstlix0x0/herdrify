//! What a successful herdr-app command prints, as typed values.
//!
//! Every command answers with one JSON object, and the shape depends on the command. The shapes
//! here are the ones a command's [`Output`](super::Command::Output) names. A field a shape does not
//! name is ignored, so a newer herdr-app that adds one does not break a decode.
//!
//! Responsibilities:
//!
//! - [`Created`], [`TabMade`], [`Split`], [`Opened`], [`Kept`] and [`Parked`], the replies that
//!   name what a command made or moved, with the fields herdr-app leaves out marked optional
//! - [`Ack`], the plain `{"ok": true}` reply, and the refusal of an object that does not say so
//! - [`RawReply`], any JSON value, for a command whose reply has no recorded shape
//!
//! Non-responsibilities:
//!
//! - telling a refusal from a success, which is decided before the stdout is decoded
//! - deciding which command prints which shape, which each command's `Output` does

use serde::de::Error as _;
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};

use crate::files::TerminalAddress;
use crate::protocol::{PaneId, SpaceId, TabId};

/// The reply to a command that makes a space: its first tab and terminal.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::Created;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let created: Created = serde_json::from_str(r#"{"space":"sp1","tab":"t1","terminal":"t2"}"#)?;
/// assert_eq!(created.space.as_str(), "sp1");
/// assert_eq!(created.terminal.as_str(), "t2");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Created {
    /// The space that was made.
    pub space: SpaceId,
    /// The first tab of the space.
    pub tab: TabId,
    /// The terminal that tab runs.
    pub terminal: TerminalAddress,
}

/// The reply to a command that makes a tab.
///
/// A tab that runs a program has a `terminal`, and a tab that shows a mod has a `pane`.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::TabMade;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let made: TabMade = serde_json::from_str(r#"{"space":"sp1","tab":"t1","terminal":"t2"}"#)?;
/// assert!(made.terminal.is_some());
/// assert!(made.pane.is_none());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TabMade {
    /// The space the tab is in.
    pub space: SpaceId,
    /// The tab that was made.
    pub tab: TabId,
    /// The terminal the tab runs, when it runs a program.
    #[serde(default)]
    pub terminal: Option<TerminalAddress>,
    /// The pane the tab shows, when it shows a mod.
    #[serde(default)]
    pub pane: Option<PaneId>,
}

/// The reply to a command that splits a pane.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::Split;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let split: Split = serde_json::from_str(r#"{"pane":"p1"}"#)?;
/// assert_eq!(split.pane.as_str(), "p1");
/// assert!(split.terminal.is_none());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Split {
    /// The pane that was made.
    pub pane: PaneId,
    /// The terminal the pane runs, when it runs a program rather than a mod.
    #[serde(default)]
    pub terminal: Option<TerminalAddress>,
}

/// The reply to a command that opens a terminal.
///
/// Opening it in a new tab names the space and tab, and opening it beside a pane names the pane.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::Opened;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let opened: Opened = serde_json::from_str(r#"{"pane":"p1","terminal":"t2"}"#)?;
/// assert_eq!(opened.terminal.as_str(), "t2");
/// assert!(opened.space.is_none());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Opened {
    /// The terminal that was opened.
    pub terminal: TerminalAddress,
    /// The space of the new tab, when it was opened in one.
    #[serde(default)]
    pub space: Option<SpaceId>,
    /// The new tab, when it was opened in one.
    #[serde(default)]
    pub tab: Option<TabId>,
    /// The pane the terminal was opened in, when it was opened beside one.
    #[serde(default)]
    pub pane: Option<PaneId>,
}

/// The reply to a command that gives a background terminal a pane.
///
/// The first call names everything. A repeated call names only the pane, because the terminal
/// already has one.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::Kept;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let kept: Kept = serde_json::from_str(r#"{"pane":"p1"}"#)?;
/// assert_eq!(kept.pane.as_str(), "p1");
/// assert_eq!(kept.pending, None);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Kept {
    /// The pane the terminal has.
    pub pane: PaneId,
    /// The tab that pane is in, when herdr-app names it.
    #[serde(default)]
    pub tab: Option<TabId>,
    /// The terminal, when herdr-app names it.
    #[serde(default)]
    pub terminal: Option<TerminalAddress>,
    /// Whether the change is still in progress, when herdr-app says.
    #[serde(default)]
    pub pending: Option<bool>,
}

/// The reply to a command that parks a terminal in the background.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::Parked;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let parked: Parked = serde_json::from_str(r#"{"pending":true,"terminal":"t2"}"#)?;
/// assert_eq!(parked.terminal.as_str(), "t2");
/// assert!(parked.pending);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Parked {
    /// The terminal that was parked.
    pub terminal: TerminalAddress,
    /// Whether the change is still in progress.
    pub pending: bool,
}

/// The reply of a command that only says it worked: an object whose `ok` is `true`.
///
/// Other fields of the object are ignored. An object without `ok: true` is refused with the text
/// `expected "ok": true`, and a value that is not an object with serde's own message for it.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::Ack;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let _ack: Ack = serde_json::from_str(r#"{"ok":true}"#)?;
/// assert!(serde_json::from_str::<Ack>(r#"{"ok":false}"#).is_err());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ack;

impl<'de> Deserialize<'de> for Ack {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let object = Map::<String, Value>::deserialize(deserializer)?;
        match object.get("ok") {
            Some(Value::Bool(true)) => Ok(Self),
            _ => Err(D::Error::custom("expected \"ok\": true")),
        }
    }
}

/// The reply of a command with no recorded shape: any JSON value, as it came.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::RawReply;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let reply: RawReply = serde_json::from_str(r#"{"anything":[1,2]}"#)?;
/// assert!(reply.0.is_object());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct RawReply(pub Value);

#[cfg(test)]
mod tests {
    use super::{Ack, Created, Kept, Opened, Parked, RawReply, Split, TabMade};
    use crate::files::TerminalAddress;
    use crate::protocol::{PaneId, SpaceId, TabId};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn terminal(address: Option<&TerminalAddress>) -> Option<&str> {
        address.map(TerminalAddress::as_str)
    }

    fn pane(id: Option<&PaneId>) -> Option<&str> {
        id.map(PaneId::as_str)
    }

    fn space(id: Option<&SpaceId>) -> Option<&str> {
        id.map(SpaceId::as_str)
    }

    fn tab(id: Option<&TabId>) -> Option<&str> {
        id.map(TabId::as_str)
    }

    /// Each reply is the stdout of `do` as herdr-app 0.1.17 printed it.
    #[test]
    fn outputs_decode_recorded_replies() -> TestResult {
        const SPACE: &str = "sp6c039a6d414cc352015b391a2d6279b7";
        const FIRST_TAB: &str = "tb2a277d2ac143c65c063d5cad51bd15b6";
        const SHELL_TAB: &str = "tbb63e1218cf7f2f97c1c1e5635eabaab7";
        const MOD_TAB: &str = "tb5d44f73fbf622bc82a2047fce2db12ba";
        const MOD_PANE: &str = "pe0e9b9419f5307e4ff1787b9baa09549";
        const SPLIT_PANE: &str = "p1c355abaaadefb36dc823c35187f482f";
        const MOD_SPLIT_PANE: &str = "pfb28ec149feaca1d490376841d34d26a";
        const OPENED_PANE: &str = "p5a7bda7ce45003740be823ba2432f3a8";
        const OPENED_TAB: &str = "tb7034c52158c180a628e5128a41f43af0";
        const KEPT_PANE: &str = "p2710fecc3619dcb2414e51346abb1878";
        const KEPT_TAB: &str = "tb009b7157e3a91ebcf22a6548ec03f8d7";

        let created: Created = serde_json::from_str(
            r#"{"space":"sp6c039a6d414cc352015b391a2d6279b7","tab":"tb2a277d2ac143c65c063d5cad51bd15b6","terminal":"t1"}"#,
        )?;
        assert_eq!(created.space.as_str(), SPACE);
        assert_eq!(created.tab.as_str(), FIRST_TAB);
        assert_eq!(created.terminal.as_str(), "t1");

        let shell: TabMade = serde_json::from_str(
            r#"{"space":"sp6c039a6d414cc352015b391a2d6279b7","tab":"tbb63e1218cf7f2f97c1c1e5635eabaab7","terminal":"t3"}"#,
        )?;
        assert_eq!(shell.space.as_str(), SPACE);
        assert_eq!(shell.tab.as_str(), SHELL_TAB);
        assert_eq!(terminal(shell.terminal.as_ref()), Some("t3"));
        assert_eq!(shell.pane, None);
        let module: TabMade = serde_json::from_str(
            r#"{"pane":"pe0e9b9419f5307e4ff1787b9baa09549","space":"sp6c039a6d414cc352015b391a2d6279b7","tab":"tb5d44f73fbf622bc82a2047fce2db12ba"}"#,
        )?;
        assert_eq!(module.terminal, None);
        assert_eq!(pane(module.pane.as_ref()), Some(MOD_PANE));
        assert_eq!(module.space.as_str(), SPACE);
        assert_eq!(module.tab.as_str(), MOD_TAB);

        let with_terminal: Split = serde_json::from_str(
            r#"{"pane":"p1c355abaaadefb36dc823c35187f482f","terminal":"t4"}"#,
        )?;
        assert_eq!(with_terminal.pane.as_str(), SPLIT_PANE);
        assert_eq!(terminal(with_terminal.terminal.as_ref()), Some("t4"));
        let without: Split =
            serde_json::from_str(r#"{"pane":"pfb28ec149feaca1d490376841d34d26a"}"#)?;
        assert_eq!(without.pane.as_str(), MOD_SPLIT_PANE);
        assert_eq!(without.terminal, None);

        let in_tab: Opened = serde_json::from_str(
            r#"{"space":"sp6c039a6d414cc352015b391a2d6279b7","tab":"tb7034c52158c180a628e5128a41f43af0","terminal":"t7"}"#,
        )?;
        assert_eq!(in_tab.terminal.as_str(), "t7");
        assert_eq!(space(in_tab.space.as_ref()), Some(SPACE));
        assert_eq!(tab(in_tab.tab.as_ref()), Some(OPENED_TAB));
        assert_eq!(in_tab.pane, None);
        let in_pane: Opened = serde_json::from_str(
            r#"{"pane":"p5a7bda7ce45003740be823ba2432f3a8","terminal":"t5"}"#,
        )?;
        assert_eq!(in_pane.terminal.as_str(), "t5");
        assert_eq!(pane(in_pane.pane.as_ref()), Some(OPENED_PANE));
        assert_eq!((in_pane.space, in_pane.tab), (None, None));

        let full: Kept = serde_json::from_str(
            r#"{"pane":"p2710fecc3619dcb2414e51346abb1878","pending":true,"tab":"tb009b7157e3a91ebcf22a6548ec03f8d7","terminal":"t2"}"#,
        )?;
        assert_eq!(full.pane.as_str(), KEPT_PANE);
        assert_eq!(tab(full.tab.as_ref()), Some(KEPT_TAB));
        assert_eq!(terminal(full.terminal.as_ref()), Some("t2"));
        assert_eq!(full.pending, Some(true));
        let repeat: Kept = serde_json::from_str(r#"{"pane":"p2710fecc3619dcb2414e51346abb1878"}"#)?;
        assert_eq!(repeat.pane.as_str(), KEPT_PANE);
        assert_eq!(
            (repeat.tab, repeat.terminal, repeat.pending),
            (None, None, None)
        );

        let parked: Parked = serde_json::from_str(r#"{"pending":true,"terminal":"t2"}"#)?;
        assert_eq!(parked.terminal.as_str(), "t2");
        assert!(parked.pending);
        Ok(())
    }

    #[test]
    fn ack_accepts_extra_fields() -> TestResult {
        let plain: Ack = serde_json::from_str(r#"{"ok":true}"#)?;
        let extra: Ack = serde_json::from_str(r#"{"ok":true,"x":1}"#)?;
        assert_eq!(plain, extra);
        Ok(())
    }

    #[test]
    fn ack_refuses_exactly() {
        for text in [r#"{"ok":false}"#, "{}"] {
            let refused = serde_json::from_str::<Ack>(text).map_err(|error| error.to_string());
            assert_eq!(refused, Err("expected \"ok\": true".to_owned()), "{text}");
        }
        let refused = serde_json::from_str::<Ack>("7").map_err(|error| error.to_string());
        assert_eq!(
            refused,
            Err("invalid type: integer `7`, expected a map at line 1 column 1".to_owned())
        );
    }

    #[test]
    fn raw_reply_takes_any_json() -> TestResult {
        let number: RawReply = serde_json::from_str("7")?;
        assert_eq!(number.0, serde_json::json!(7));
        let null: RawReply = serde_json::from_str("null")?;
        assert!(null.0.is_null());
        let object: RawReply = serde_json::from_str(r#"{"a":1}"#)?;
        assert_eq!(object.0, serde_json::json!({"a": 1}));
        Ok(())
    }
}
