//! The open word sets that say why a terminal ended or why a wait returned.
//!
//! The core documents a list of words for each field and may add more. Only some words have a
//! named variant; every other word, documented or not, is kept as `Other` and written back exactly
//! as it was read.
//!
//! Responsibilities:
//!
//! - [`EndReason`]: the reason a terminal's program ended, as an open set of words
//! - [`WaitReason`]: the reason a wait returned, as an open set of words
//!
//! Non-responsibilities:
//!
//! - deciding what the core accepts or reports, which the core decides

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::protocol::word::impl_word_identity;

/// Why a terminal's program ended or is pending.
///
/// A word outside the documented list is [`EndReason::Other`].
///
/// Equality and hashing follow the word, so `EndReason::Other("exited".to_owned())` equals
/// `EndReason::Exited`: both are written as `"exited"` and both read back as `EndReason::Exited`.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::EndReason;
///
/// let known: EndReason = serde_json::from_str("\"exited\"")?;
/// assert_eq!(known, EndReason::Exited);
/// assert_eq!(EndReason::Other("exited".to_owned()), known);
///
/// let unknown: EndReason = serde_json::from_str("\"paused\"")?;
/// assert_eq!(unknown.as_str(), "paused");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone)]
pub enum EndReason {
    /// The word `exited`.
    Exited,
    /// The word `killed`.
    Killed,
    /// The word `server_stop`.
    ServerStop,
    /// The word `host_shutdown`.
    HostShutdown,
    /// The word `holder_lost`.
    HolderLost,
    /// The word `stopped_by_restart`.
    StoppedByRestart,
    /// The word `signaled`.
    Signaled,
    /// The word `spawn_pending`.
    SpawnPending,
    /// A word the core sent that is not in the documented list, kept as read.
    Other(String),
}

impl EndReason {
    /// The word this value is sent and read as.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Exited => "exited",
            Self::Killed => "killed",
            Self::ServerStop => "server_stop",
            Self::HostShutdown => "host_shutdown",
            Self::HolderLost => "holder_lost",
            Self::StoppedByRestart => "stopped_by_restart",
            Self::Signaled => "signaled",
            Self::SpawnPending => "spawn_pending",
            Self::Other(word) => word,
        }
    }
}

impl_word_identity!(EndReason);

impl From<String> for EndReason {
    fn from(word: String) -> Self {
        match word.as_str() {
            "exited" => Self::Exited,
            "killed" => Self::Killed,
            "server_stop" => Self::ServerStop,
            "host_shutdown" => Self::HostShutdown,
            "holder_lost" => Self::HolderLost,
            "stopped_by_restart" => Self::StoppedByRestart,
            "signaled" => Self::Signaled,
            "spawn_pending" => Self::SpawnPending,
            _ => Self::Other(word),
        }
    }
}

impl Serialize for EndReason {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for EndReason {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::from)
    }
}

/// Why a wait returned.
///
/// A word that is not one of the named variants is [`WaitReason::Other`].
///
/// Equality and hashing follow the word, so `WaitReason::Other("exit".to_owned())` equals
/// `WaitReason::Exit`: both are written as `"exit"` and both read back as `WaitReason::Exit`.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::WaitReason;
///
/// let known: WaitReason = serde_json::from_str("\"exit\"")?;
/// assert_eq!(known, WaitReason::Exit);
/// assert_eq!(WaitReason::Other("exit".to_owned()), known);
///
/// let unknown: WaitReason = serde_json::from_str("\"paused\"")?;
/// assert_eq!(unknown.as_str(), "paused");
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone)]
pub enum WaitReason {
    /// The word `exit`.
    Exit,
    /// The word `timeout`.
    Timeout,
    /// The word `message`.
    Message,
    /// A word the core sent that is not one of the named variants, kept as read.
    Other(String),
}

impl WaitReason {
    /// The word this value is sent and read as.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Exit => "exit",
            Self::Timeout => "timeout",
            Self::Message => "message",
            Self::Other(word) => word,
        }
    }
}

impl_word_identity!(WaitReason);

impl From<String> for WaitReason {
    fn from(word: String) -> Self {
        match word.as_str() {
            "exit" => Self::Exit,
            "timeout" => Self::Timeout,
            "message" => Self::Message,
            _ => Self::Other(word),
        }
    }
}

impl Serialize for WaitReason {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for WaitReason {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::from)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::hash::{BuildHasher, RandomState};

    use super::{EndReason, WaitReason};

    type TestResult = Result<(), Box<dyn Error>>;

    #[test]
    fn end_reason_open_enums_round_trip_every_known_word() -> TestResult {
        let known = [
            ("exited", EndReason::Exited),
            ("killed", EndReason::Killed),
            ("server_stop", EndReason::ServerStop),
            ("host_shutdown", EndReason::HostShutdown),
            ("holder_lost", EndReason::HolderLost),
            ("stopped_by_restart", EndReason::StoppedByRestart),
            ("signaled", EndReason::Signaled),
            ("spawn_pending", EndReason::SpawnPending),
        ];
        for (word, variant) in known {
            let json = format!("\"{word}\"");
            let value: EndReason = serde_json::from_str(&json)?;
            assert_eq!(value, variant);
            assert_eq!(value.as_str(), word);
            assert_eq!(serde_json::to_string(&value)?, json);
        }
        Ok(())
    }

    #[test]
    fn end_reason_open_enums_keep_unknown_words() -> TestResult {
        let value: EndReason = serde_json::from_str(r#""paused""#)?;
        assert_eq!(value, EndReason::Other("paused".to_owned()));
        assert_eq!(value.as_str(), "paused");
        assert_eq!(serde_json::to_string(&value)?, r#""paused""#);
        Ok(())
    }

    #[test]
    fn end_reason_a_named_word_equals_other_holding_the_same_word() {
        let state = RandomState::new();
        for (word, named) in [
            ("exited", EndReason::Exited),
            ("killed", EndReason::Killed),
            ("server_stop", EndReason::ServerStop),
            ("host_shutdown", EndReason::HostShutdown),
            ("holder_lost", EndReason::HolderLost),
            ("stopped_by_restart", EndReason::StoppedByRestart),
            ("signaled", EndReason::Signaled),
            ("spawn_pending", EndReason::SpawnPending),
        ] {
            let other = EndReason::Other(word.to_owned());
            assert_eq!(other, named);
            assert_eq!(named, other);
            assert_eq!(state.hash_one(&other), state.hash_one(&named));
        }
    }

    #[test]
    fn end_reason_other_words_are_equal_only_when_the_words_are() {
        let state = RandomState::new();
        let x = EndReason::Other("x".to_owned());
        let y = EndReason::Other("y".to_owned());
        assert_ne!(x, y);
        assert_eq!(x, EndReason::Other("x".to_owned()));
        assert_eq!(
            state.hash_one(&x),
            state.hash_one(EndReason::Other("x".to_owned()))
        );
    }

    #[test]
    fn wait_reason_open_enums_round_trip_every_known_word() -> TestResult {
        let known = [
            ("exit", WaitReason::Exit),
            ("timeout", WaitReason::Timeout),
            ("message", WaitReason::Message),
        ];
        for (word, variant) in known {
            let json = format!("\"{word}\"");
            let value: WaitReason = serde_json::from_str(&json)?;
            assert_eq!(value, variant);
            assert_eq!(value.as_str(), word);
            assert_eq!(serde_json::to_string(&value)?, json);
        }
        Ok(())
    }

    #[test]
    fn wait_reason_open_enums_keep_unknown_words() -> TestResult {
        let value: WaitReason = serde_json::from_str(r#""paused""#)?;
        assert_eq!(value, WaitReason::Other("paused".to_owned()));
        assert_eq!(value.as_str(), "paused");
        assert_eq!(serde_json::to_string(&value)?, r#""paused""#);
        Ok(())
    }

    #[test]
    fn wait_reason_a_named_word_equals_other_holding_the_same_word() {
        let state = RandomState::new();
        for (word, named) in [
            ("exit", WaitReason::Exit),
            ("timeout", WaitReason::Timeout),
            ("message", WaitReason::Message),
        ] {
            let other = WaitReason::Other(word.to_owned());
            assert_eq!(other, named);
            assert_eq!(named, other);
            assert_eq!(state.hash_one(&other), state.hash_one(&named));
        }
    }

    #[test]
    fn wait_reason_other_words_are_equal_only_when_the_words_are() {
        let state = RandomState::new();
        let x = WaitReason::Other("x".to_owned());
        let y = WaitReason::Other("y".to_owned());
        assert_ne!(x, y);
        assert_eq!(x, WaitReason::Other("x".to_owned()));
        assert_eq!(
            state.hash_one(&x),
            state.hash_one(WaitReason::Other("x".to_owned()))
        );
    }
}
