//! The requests that read from, attach to, send to and wait on a terminal.
//!
//! Each type holds exactly the parameters the schema lists for its method and sends only the ones
//! a caller set. A `terminal.wait` takes a [`WaitTarget`] instead of a `terminal` and a `children`
//! that could be combined in ways the core refuses.
//!
//! Responsibilities:
//!
//! - the `Params` type of `terminal.read`, `terminal.attach`, `terminal.send`, `terminal.wait`,
//!   `terminal.keys`, `terminal.interrupt` and `terminal.put`, each with its wire name and reply
//!   type
//! - [`WaitTarget`], which names what a wait watches
//!
//! Non-responsibilities:
//!
//! - the requests that start, list, end or label a terminal, which `lifecycle` holds
//! - sending the request or reading the reply, which the client does
//! - deciding which terminals, keys or sizes the core accepts, which the core decides

use std::time::Duration;

use serde::Serialize;

use crate::protocol::methods::seconds::Seconds;
use crate::protocol::{
    AttachResult, CallerId, Children, DisplayFacts, FileName, IdleOrWorking, InterruptResult,
    KeyName, KeysResult, Method, PutResult, ReadResult, SendResult, TerminalRef, WaitResult,
};

/// The parameters of `terminal.read`.
///
/// Nothing is sent that was not set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerminalRead {
    terminal: TerminalRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    caller: Option<CallerId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lines: Option<u64>,
}

impl TerminalRead {
    /// A `terminal.read` request for the screen of `terminal`.
    #[must_use]
    pub const fn new(terminal: TerminalRef) -> Self {
        Self {
            terminal,
            caller: None,
            lines: None,
        }
    }

    /// Who is calling.
    #[must_use]
    pub fn caller(mut self, caller: CallerId) -> Self {
        self.caller = Some(caller);
        self
    }

    /// How many lines of history to include above the screen.
    #[must_use]
    pub const fn lines(mut self, lines: u64) -> Self {
        self.lines = Some(lines);
        self
    }
}

impl Method for TerminalRead {
    const NAME: &'static str = "terminal.read";
    type Params = Self;
    type Output = ReadResult;
}

/// The parameters of `terminal.attach`.
///
/// The size is required, and 0 is accepted: the core takes an attach at 0 by 0. Nothing else is
/// sent that was not set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerminalAttach {
    terminal: TerminalRef,
    cols: u16,
    rows: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    caller: Option<CallerId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    display: Option<DisplayFacts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    history: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observe: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resize_notifications: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resumed: Option<bool>,
}

impl TerminalAttach {
    /// A `terminal.attach` request for `terminal` from a viewer of `cols` by `rows` cells.
    #[must_use]
    pub const fn new(terminal: TerminalRef, cols: u16, rows: u16) -> Self {
        Self {
            terminal,
            cols,
            rows,
            caller: None,
            display: None,
            history: None,
            observe: None,
            resize_notifications: None,
            resumed: None,
        }
    }

    /// Who is calling.
    #[must_use]
    pub fn caller(mut self, caller: CallerId) -> Self {
        self.caller = Some(caller);
        self
    }

    /// What the viewer's terminal looks like.
    #[must_use]
    pub fn display(mut self, display: DisplayFacts) -> Self {
        self.display = Some(display);
        self
    }

    /// How many lines of scrollback to show first, on the main screen only.
    #[must_use]
    pub const fn history(mut self, lines: u64) -> Self {
        self.history = Some(lines);
        self
    }

    /// Watches only: the viewer never types and never resizes.
    #[must_use]
    pub const fn observe(mut self) -> Self {
        self.observe = Some(true);
        self
    }

    /// Tells the core the observer understands a core-to-viewer resize before replacement paints.
    #[must_use]
    pub const fn resize_notifications(mut self) -> Self {
        self.resize_notifications = Some(true);
        self
    }

    /// Marks the attach as a return after a dropped line, so the size is set only once the
    /// viewer is active.
    #[must_use]
    pub const fn resumed(mut self) -> Self {
        self.resumed = Some(true);
        self
    }
}

impl Method for TerminalAttach {
    const NAME: &'static str = "terminal.attach";
    type Params = Self;
    type Output = AttachResult;
}

/// The parameters of `terminal.send`.
///
/// The text may be empty. Nothing else is sent that was not set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerminalSend {
    terminal: TerminalRef,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    caller: Option<CallerId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    force: Option<bool>,
    #[serde(rename = "if", skip_serializing_if = "Option::is_none")]
    only_if: Option<IdleOrWorking>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout: Option<Seconds>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wait: Option<bool>,
}

impl TerminalSend {
    /// A `terminal.send` request that pastes `text` into `terminal` and submits it with Enter.
    #[must_use]
    pub const fn new(terminal: TerminalRef, text: String) -> Self {
        Self {
            terminal,
            text,
            caller: None,
            force: None,
            only_if: None,
            timeout: None,
            wait: None,
        }
    }

    /// Who is sending.
    #[must_use]
    pub fn caller(mut self, caller: CallerId) -> Self {
        self.caller = Some(caller);
        self
    }

    /// Types even if the program is blocked.
    #[must_use]
    pub const fn force(mut self) -> Self {
        self.force = Some(true);
        self
    }

    /// Types only if the terminal's status is exactly `status`. The wire parameter is named `if`.
    #[must_use]
    pub const fn only_if(mut self, status: IdleOrWorking) -> Self {
        self.only_if = Some(status);
        self
    }

    /// With [`wait`](Self::wait), how long to wait before checking in.
    #[must_use]
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(Seconds(timeout));
        self
    }

    /// Waits for the turn this send started.
    #[must_use]
    pub const fn wait(mut self) -> Self {
        self.wait = Some(true);
        self
    }
}

impl Method for TerminalSend {
    const NAME: &'static str = "terminal.send";
    type Params = Self;
    type Output = SendResult;
}

/// What a `terminal.wait` request waits on.
///
/// The core refuses a wait with neither a terminal nor a set of children, so one of these is
/// always given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaitTarget {
    /// This terminal.
    Terminal(TerminalRef),
    /// The caller's own children.
    Children(Children),
    /// The children of this terminal.
    ChildrenOf(TerminalRef, Children),
}

/// The parameters of `terminal.wait`.
///
/// Nothing is sent that was not set. A [`WaitTarget::ChildrenOf`] sends both `terminal` and
/// `children`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerminalWait {
    #[serde(skip_serializing_if = "Option::is_none")]
    terminal: Option<TerminalRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Children>,
    #[serde(skip_serializing_if = "Option::is_none")]
    caller: Option<CallerId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout: Option<Seconds>,
}

impl TerminalWait {
    /// A `terminal.wait` request on `target`.
    #[must_use]
    pub fn new(target: WaitTarget) -> Self {
        let (terminal, children) = match target {
            WaitTarget::Terminal(terminal) => (Some(terminal), None),
            WaitTarget::Children(children) => (None, Some(children)),
            WaitTarget::ChildrenOf(terminal, children) => (Some(terminal), Some(children)),
        };
        Self {
            terminal,
            children,
            caller: None,
            output: None,
            timeout: None,
        }
    }

    /// Who is waiting.
    #[must_use]
    pub fn caller(mut self, caller: CallerId) -> Self {
        self.caller = Some(caller);
        self
    }

    /// Ends the wait when output printed after it started contains this text.
    #[must_use]
    pub fn output(mut self, text: String) -> Self {
        self.output = Some(text);
        self
    }

    /// How long to wait before checking in.
    #[must_use]
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(Seconds(timeout));
        self
    }
}

impl Method for TerminalWait {
    const NAME: &'static str = "terminal.wait";
    type Params = Self;
    type Output = WaitResult;
}

/// The parameters of `terminal.keys`.
///
/// The list of keys may be empty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerminalKeys {
    terminal: TerminalRef,
    keys: Vec<KeyName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    caller: Option<CallerId>,
}

impl TerminalKeys {
    /// A `terminal.keys` request that presses `keys` in `terminal`.
    #[must_use]
    pub const fn new(terminal: TerminalRef, keys: Vec<KeyName>) -> Self {
        Self {
            terminal,
            keys,
            caller: None,
        }
    }

    /// Who is calling.
    #[must_use]
    pub fn caller(mut self, caller: CallerId) -> Self {
        self.caller = Some(caller);
        self
    }
}

impl Method for TerminalKeys {
    const NAME: &'static str = "terminal.keys";
    type Params = Self;
    type Output = KeysResult;
}

/// The parameters of `terminal.interrupt`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerminalInterrupt {
    terminal: TerminalRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout: Option<Seconds>,
}

impl TerminalInterrupt {
    /// A `terminal.interrupt` request for `terminal`.
    #[must_use]
    pub const fn new(terminal: TerminalRef) -> Self {
        Self {
            terminal,
            timeout: None,
        }
    }

    /// How long to watch for an effect.
    #[must_use]
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(Seconds(timeout));
        self
    }
}

impl Method for TerminalInterrupt {
    const NAME: &'static str = "terminal.interrupt";
    type Params = Self;
    type Output = InterruptResult;
}

/// The parameters of `terminal.put`.
///
/// The data is sent as base64 text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TerminalPut {
    terminal: TerminalRef,
    name: FileName,
    #[serde(serialize_with = "crate::protocol::base64::serialize")]
    data: Vec<u8>,
}

impl TerminalPut {
    /// A `terminal.put` request that stores `data` in `terminal` under `name`.
    #[must_use]
    pub const fn new(terminal: TerminalRef, name: FileName, data: Vec<u8>) -> Self {
        Self {
            terminal,
            name,
            data,
        }
    }
}

impl Method for TerminalPut {
    const NAME: &'static str = "terminal.put";
    type Params = Self;
    type Output = PutResult;
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::error::Error;
    use std::time::Duration;

    use serde_json::{Value, json};

    use crate::protocol::{
        CallerId, Children, DisplayFacts, FileName, IdleOrWorking, KeyName, Method, TerminalAttach,
        TerminalInterrupt, TerminalKeys, TerminalPut, TerminalRead, TerminalRef, TerminalSend,
        TerminalWait, WaitTarget,
    };

    type TestResult = Result<(), Box<dyn Error>>;

    fn keys(value: &Value) -> BTreeSet<&str> {
        value
            .as_object()
            .map(|object| object.keys().map(String::as_str).collect())
            .unwrap_or_default()
    }

    fn t7() -> Result<TerminalRef, Box<dyn Error>> {
        Ok(TerminalRef::new("t7")?)
    }

    #[test]
    fn terminal_read_required_and_setters() -> TestResult {
        assert_eq!(
            serde_json::to_value(TerminalRead::new(t7()?))?,
            json!({"terminal": "t7"})
        );
        assert_eq!(
            serde_json::to_value(
                TerminalRead::new(t7()?)
                    .caller(CallerId::new("c"))
                    .lines(50)
            )?,
            json!({"terminal": "t7", "caller": "c", "lines": 50})
        );
        assert_eq!(TerminalRead::NAME, "terminal.read");
        Ok(())
    }

    #[test]
    fn terminal_attach_required_only() -> TestResult {
        assert_eq!(
            serde_json::to_value(TerminalAttach::new(t7()?, 0, 0))?,
            json!({"cols": 0, "rows": 0, "terminal": "t7"})
        );
        assert_eq!(TerminalAttach::NAME, "terminal.attach");
        Ok(())
    }

    #[test]
    fn terminal_send_allows_empty_text() -> TestResult {
        assert_eq!(
            serde_json::to_value(TerminalSend::new(t7()?, String::new()))?,
            json!({"terminal": "t7", "text": ""})
        );
        let full = TerminalSend::new(t7()?, "hi".to_owned())
            .caller(CallerId::new("c"))
            .force()
            .only_if(IdleOrWorking::Idle)
            .timeout(Duration::from_secs(5))
            .wait();
        let value = serde_json::to_value(full)?;
        assert_eq!(value["if"], json!("idle"));
        assert_eq!(
            keys(&value),
            BTreeSet::from([
                "caller", "force", "if", "terminal", "text", "timeout", "wait"
            ])
        );
        assert_eq!(TerminalSend::NAME, "terminal.send");
        Ok(())
    }

    #[test]
    fn terminal_wait_targets() -> TestResult {
        assert_eq!(
            serde_json::to_value(TerminalWait::new(WaitTarget::Terminal(t7()?)))?,
            json!({"terminal": "t7"})
        );
        assert_eq!(
            serde_json::to_value(TerminalWait::new(WaitTarget::Children(Children::Any)))?,
            json!({"children": "any"})
        );
        assert_eq!(
            serde_json::to_value(TerminalWait::new(WaitTarget::ChildrenOf(
                t7()?,
                Children::All
            )))?,
            json!({"terminal": "t7", "children": "all"})
        );
        assert_eq!(TerminalWait::NAME, "terminal.wait");
        Ok(())
    }

    #[test]
    fn terminal_keys_allows_an_empty_list() -> TestResult {
        assert_eq!(
            serde_json::to_value(TerminalKeys::new(t7()?, vec![]))?,
            json!({"terminal": "t7", "keys": []})
        );
        assert_eq!(
            serde_json::to_value(
                TerminalKeys::new(t7()?, vec![KeyName::new("ctrl+c")?]).caller(CallerId::new("c"))
            )?,
            json!({"terminal": "t7", "keys": ["ctrl+c"], "caller": "c"})
        );
        assert_eq!(TerminalKeys::NAME, "terminal.keys");
        Ok(())
    }

    #[test]
    fn terminal_put_encodes_data() -> TestResult {
        let put = TerminalPut::new(t7()?, FileName::new("a.png"), b"x\n".to_vec());
        assert_eq!(
            serde_json::to_value(put)?,
            json!({"terminal": "t7", "name": "a.png", "data": "eAo="})
        );
        assert_eq!(TerminalPut::NAME, "terminal.put");
        Ok(())
    }

    #[test]
    fn interaction_every_setter() -> TestResult {
        let display = DisplayFacts {
            bg: None,
            cell: Some([8, 16]),
            cursor: None,
            fg: None,
        };
        let attach = TerminalAttach::new(t7()?, 80, 24)
            .caller(CallerId::new("c"))
            .display(display)
            .history(100)
            .observe()
            .resize_notifications()
            .resumed();
        assert_eq!(
            keys(&serde_json::to_value(attach)?),
            BTreeSet::from([
                "caller",
                "cols",
                "display",
                "history",
                "observe",
                "resize_notifications",
                "resumed",
                "rows",
                "terminal"
            ])
        );
        let wait = TerminalWait::new(WaitTarget::ChildrenOf(t7()?, Children::Any))
            .caller(CallerId::new("c"))
            .output("done".to_owned())
            .timeout(Duration::from_millis(500));
        let value = serde_json::to_value(wait)?;
        assert_eq!(
            keys(&value),
            BTreeSet::from(["caller", "children", "output", "terminal", "timeout"])
        );
        assert_eq!(value["timeout"], json!(0.5));
        assert_eq!(
            serde_json::to_value(TerminalInterrupt::new(t7()?).timeout(Duration::from_secs(10)))?,
            json!({"terminal": "t7", "timeout": 10.0})
        );
        assert_eq!(
            serde_json::to_value(TerminalInterrupt::new(t7()?))?,
            json!({"terminal": "t7"})
        );
        assert_eq!(TerminalInterrupt::NAME, "terminal.interrupt");
        Ok(())
    }
}
