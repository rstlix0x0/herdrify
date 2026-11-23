//! The words that say what a terminal is doing, as closed or open sets.
//!
//! `IdleOrWorking` and `Children` are closed: the schema fixes their words, so any other word is
//! refused when read. A newer core cannot add a word without breaking these types, because the
//! schema's own `enum` promises the set. `TerminalState` and `StatusWord` are open: the core may add words, so a word
//! not listed is kept as `Other` and written back exactly as it was read.
//!
//! Responsibilities:
//!
//! - [`TerminalState`]: the state of a terminal, as an open set of words
//! - [`StatusWord`]: the word a program reports for what it is doing, as an open set of words
//! - [`IdleOrWorking`]: a closed set of words the schema fixes
//! - [`Children`]: a closed set of words the schema fixes
//!
//! Non-responsibilities:
//!
//! - deciding what the core accepts or reports, which the core decides

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::protocol::word::impl_word_identity;

/// Whether a terminal is still running: `live` or `finished`.
///
/// A word outside the documented list is [`TerminalState::Other`].
///
/// Equality and hashing follow the word, so `TerminalState::Other("live".to_owned())` equals
/// `TerminalState::Live`: both are written as `"live"` and both read back as `TerminalState::Live`.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::TerminalState;
///
/// let known: TerminalState = serde_json::from_str("\"live\"")?;
/// assert_eq!(known, TerminalState::Live);
/// assert_eq!(TerminalState::Other("live".to_owned()), known);
///
/// let unknown: TerminalState = serde_json::from_str("\"paused\"")?;
/// assert_eq!(unknown.as_str(), "paused");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone)]
pub enum TerminalState {
    /// The word `live`.
    Live,
    /// The word `finished`.
    Finished,
    /// A word the core sent that is not in the documented list, kept as read.
    Other(String),
}

impl TerminalState {
    /// The word this value is sent and read as.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Live => "live",
            Self::Finished => "finished",
            Self::Other(word) => word,
        }
    }
}

impl_word_identity!(TerminalState);

impl From<String> for TerminalState {
    fn from(word: String) -> Self {
        match word.as_str() {
            "live" => Self::Live,
            "finished" => Self::Finished,
            _ => Self::Other(word),
        }
    }
}

impl Serialize for TerminalState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for TerminalState {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::from)
    }
}

/// The word a program reports for what it is doing.
///
/// A word outside the documented list is [`StatusWord::Other`].
///
/// Equality and hashing follow the word, so `StatusWord::Other("idle".to_owned())` equals
/// `StatusWord::Idle`: both are written as `"idle"` and both read back as `StatusWord::Idle`.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::StatusWord;
///
/// let known: StatusWord = serde_json::from_str("\"idle\"")?;
/// assert_eq!(known, StatusWord::Idle);
/// assert_eq!(StatusWord::Other("idle".to_owned()), known);
///
/// let unknown: StatusWord = serde_json::from_str("\"paused\"")?;
/// assert_eq!(unknown.as_str(), "paused");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone)]
pub enum StatusWord {
    /// The word `idle`.
    Idle,
    /// The word `working`.
    Working,
    /// The word `done`.
    Done,
    /// The word `blocked`.
    Blocked,
    /// The word `error`.
    Error,
    /// A word the core sent that is not in the documented list, kept as read.
    Other(String),
}

impl StatusWord {
    /// The word this value is sent and read as.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Idle => "idle",
            Self::Working => "working",
            Self::Done => "done",
            Self::Blocked => "blocked",
            Self::Error => "error",
            Self::Other(word) => word,
        }
    }
}

impl_word_identity!(StatusWord);

impl From<String> for StatusWord {
    fn from(word: String) -> Self {
        match word.as_str() {
            "idle" => Self::Idle,
            "working" => Self::Working,
            "done" => Self::Done,
            "blocked" => Self::Blocked,
            "error" => Self::Error,
            _ => Self::Other(word),
        }
    }
}

impl Serialize for StatusWord {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for StatusWord {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::from)
    }
}

/// The status a `terminal.send` call requires before it types: `idle` or `working`.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::IdleOrWorking;
///
/// assert_eq!(IdleOrWorking::Idle.as_str(), "idle");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdleOrWorking {
    /// The word `idle`.
    Idle,
    /// The word `working`.
    Working,
}

impl IdleOrWorking {
    /// The word this variant is sent and read as.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Working => "working",
        }
    }
}

/// Which children a `terminal.wait` call waits on: `all` until none is working, `any` until the next news from one.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::Children;
///
/// assert_eq!(Children::Any.as_str(), "any");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Children {
    /// The word `any`.
    Any,
    /// The word `all`.
    All,
}

impl Children {
    /// The word this variant is sent and read as.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::All => "all",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::hash::{BuildHasher, RandomState};

    use super::{Children, IdleOrWorking, StatusWord, TerminalState};

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn terminal_state_open_enums_round_trip_every_known_word() -> TestResult {
        let known = [
            ("live", TerminalState::Live),
            ("finished", TerminalState::Finished),
        ];
        for (word, variant) in known {
            let json = format!("\"{word}\"");
            let value: TerminalState = serde_json::from_str(&json)?;
            assert_eq!(value, variant);
            assert_eq!(value.as_str(), word);
            assert_eq!(serde_json::to_string(&value)?, json);
        }
        Ok(())
    }

    #[test]
    fn terminal_state_open_enums_keep_unknown_words() -> TestResult {
        let value: TerminalState = serde_json::from_str(r#""paused""#)?;
        assert_eq!(value, TerminalState::Other("paused".to_owned()));
        assert_eq!(value.as_str(), "paused");
        assert_eq!(serde_json::to_string(&value)?, r#""paused""#);
        Ok(())
    }

    #[test]
    fn terminal_state_a_named_word_equals_other_holding_the_same_word() {
        let state = RandomState::new();
        for (word, named) in [
            ("live", TerminalState::Live),
            ("finished", TerminalState::Finished),
        ] {
            let other = TerminalState::Other(word.to_owned());
            assert_eq!(other, named);
            assert_eq!(named, other);
            assert_eq!(state.hash_one(&other), state.hash_one(&named));
        }
    }

    #[test]
    fn terminal_state_other_words_are_equal_only_when_the_words_are() {
        let state = RandomState::new();
        let x = TerminalState::Other("x".to_owned());
        let y = TerminalState::Other("y".to_owned());
        assert_ne!(x, y);
        assert_eq!(x, TerminalState::Other("x".to_owned()));
        assert_eq!(
            state.hash_one(&x),
            state.hash_one(TerminalState::Other("x".to_owned()))
        );
    }

    #[test]
    fn status_word_open_enums_round_trip_every_known_word() -> TestResult {
        let known = [
            ("idle", StatusWord::Idle),
            ("working", StatusWord::Working),
            ("done", StatusWord::Done),
            ("blocked", StatusWord::Blocked),
            ("error", StatusWord::Error),
        ];
        for (word, variant) in known {
            let json = format!("\"{word}\"");
            let value: StatusWord = serde_json::from_str(&json)?;
            assert_eq!(value, variant);
            assert_eq!(value.as_str(), word);
            assert_eq!(serde_json::to_string(&value)?, json);
        }
        Ok(())
    }

    #[test]
    fn status_word_open_enums_keep_unknown_words() -> TestResult {
        let value: StatusWord = serde_json::from_str(r#""paused""#)?;
        assert_eq!(value, StatusWord::Other("paused".to_owned()));
        assert_eq!(value.as_str(), "paused");
        assert_eq!(serde_json::to_string(&value)?, r#""paused""#);
        Ok(())
    }

    #[test]
    fn status_word_a_named_word_equals_other_holding_the_same_word() {
        let state = RandomState::new();
        for (word, named) in [
            ("idle", StatusWord::Idle),
            ("working", StatusWord::Working),
            ("done", StatusWord::Done),
            ("blocked", StatusWord::Blocked),
            ("error", StatusWord::Error),
        ] {
            let other = StatusWord::Other(word.to_owned());
            assert_eq!(other, named);
            assert_eq!(named, other);
            assert_eq!(state.hash_one(&other), state.hash_one(&named));
        }
    }

    #[test]
    fn status_word_other_words_are_equal_only_when_the_words_are() {
        let state = RandomState::new();
        let x = StatusWord::Other("x".to_owned());
        let y = StatusWord::Other("y".to_owned());
        assert_ne!(x, y);
        assert_eq!(x, StatusWord::Other("x".to_owned()));
        assert_eq!(
            state.hash_one(&x),
            state.hash_one(StatusWord::Other("x".to_owned()))
        );
    }

    #[test]
    fn idle_or_working_closed_enums_round_trip_and_refuse_unknown() -> TestResult {
        let value: IdleOrWorking = serde_json::from_str(r#""idle""#)?;
        assert_eq!(value, IdleOrWorking::Idle);
        assert_eq!(value.as_str(), "idle");
        assert_eq!(serde_json::to_string(&value)?, r#""idle""#);
        let value: IdleOrWorking = serde_json::from_str(r#""working""#)?;
        assert_eq!(value, IdleOrWorking::Working);
        assert_eq!(value.as_str(), "working");
        assert_eq!(serde_json::to_string(&value)?, r#""working""#);
        assert!(serde_json::from_str::<IdleOrWorking>(r#""some""#).is_err());
        Ok(())
    }

    #[test]
    fn children_closed_enums_round_trip_and_refuse_unknown() -> TestResult {
        let value: Children = serde_json::from_str(r#""any""#)?;
        assert_eq!(value, Children::Any);
        assert_eq!(value.as_str(), "any");
        assert_eq!(serde_json::to_string(&value)?, r#""any""#);
        let value: Children = serde_json::from_str(r#""all""#)?;
        assert_eq!(value, Children::All);
        assert_eq!(value.as_str(), "all");
        assert_eq!(serde_json::to_string(&value)?, r#""all""#);
        assert!(serde_json::from_str::<Children>(r#""some""#).is_err());
        Ok(())
    }
}
