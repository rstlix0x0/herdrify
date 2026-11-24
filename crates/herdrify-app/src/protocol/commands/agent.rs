//! The herdr-app commands that act on agents.
//!
//! An agent is a program, such as an AI assistant, that runs in a terminal. Each command here is a
//! value that serializes to the JSON object of arguments herdr-app takes, and names the type its
//! reply decodes into, so a caller that runs one knows what comes back.
//!
//! Responsibilities:
//!
//! - [`AgentFocus`], [`AgentNext`] and [`AgentPrev`], one type per command, with the arguments
//!   herdr-app requires as parameters of `new`
//!
//! Non-responsibilities:
//!
//! - building the command line from a command, which the invocation does
//! - running the command and decoding its reply, which a layer above does
//! - checking that a terminal runs an agent, which herdr-app decides
//!
//! The unit tests below check each command's arguments against the JSON object herdr-app takes, and
//! its name and reply type.

use serde::Serialize;

use crate::files::TerminalAddress;
use crate::protocol::{Ack, Command};

/// Shows an agent, sub-agents included.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::TerminalAddress;
/// use herdrify_app::protocol::AgentFocus;
///
/// let command = AgentFocus::new(TerminalAddress::new("t2")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"terminal":"t2"}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct AgentFocus {
    terminal: TerminalAddress,
}

impl AgentFocus {
    /// A command that shows the agent in `terminal`.
    #[must_use]
    pub const fn new(terminal: TerminalAddress) -> Self {
        Self { terminal }
    }
}

impl Command for AgentFocus {
    const NAME: &'static str = "agent.focus";
    type Output = Ack;
}

/// Shows the next agent, in the order the agents list shows them.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::AgentNext;
///
/// assert_eq!(serde_json::to_string(&AgentNext::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::AgentNext {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct AgentNext {}

impl AgentNext {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for AgentNext {
    const NAME: &'static str = "agent.next";
    type Output = Ack;
}

/// Shows the previous agent.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::AgentPrev;
///
/// assert_eq!(serde_json::to_string(&AgentPrev::new())?, "{}");
/// # Ok::<(), serde_json::Error>(())
/// ```
///
/// A literal is refused outside the crate, because the type is `#[non_exhaustive]`:
///
/// ```compile_fail,E0639
/// let _ = herdrify_app::protocol::AgentPrev {};
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct AgentPrev {}

impl AgentPrev {
    /// The command.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}

impl Command for AgentPrev {
    const NAME: &'static str = "agent.prev";
    type Output = Ack;
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde_json::{Value, json};

    use super::{AgentFocus, AgentNext, AgentPrev};
    use crate::files::TerminalAddress;
    use crate::protocol::{Ack, Command};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn arguments<C: Serialize>(command: &C) -> Result<Value, serde_json::Error> {
        serde_json::to_value(command)
    }

    const fn replies_with<C: Command<Output = O>, O>() {}

    #[test]
    fn agent_commands_serialize_full_and_minimal() -> TestResult {
        assert_eq!(
            arguments(&AgentFocus::new(TerminalAddress::new("t2")?))?,
            json!({"terminal": "t2"}),
        );
        assert_eq!(arguments(&AgentNext::new())?, json!({}));
        assert_eq!(arguments(&AgentPrev::new())?, json!({}));
        Ok(())
    }

    #[test]
    fn agent_commands_name_their_wire_command() {
        assert_eq!(AgentFocus::NAME, "agent.focus");
        assert_eq!(AgentNext::NAME, "agent.next");
        assert_eq!(AgentPrev::NAME, "agent.prev");

        replies_with::<AgentFocus, Ack>();
        replies_with::<AgentNext, Ack>();
        replies_with::<AgentPrev, Ack>();
    }
}
