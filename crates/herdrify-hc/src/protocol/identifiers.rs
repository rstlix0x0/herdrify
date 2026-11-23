//! The identifier newtypes herdr-core names things with, `DocVersion`, and the error an empty one
//! returns.
//!
//! A bare `String` cannot say whether it names a terminal, a machine or a document, so each
//! identifier has its own type and a call cannot pass one where another belongs. They sit apart
//! from the messages because every method parameter and result record shares them.
//!
//! Responsibilities:
//!
//! - the nine identifiers the core refuses when empty, each checked once at construction
//! - the four identifiers the core accepts in any form, which the SDK never checks
//! - [`DocVersion`], the version number a document carries
//! - [`EmptyValueError`], the one error every refusing identifier and the argument vector return,
//!   which names the refused value
//!
//! Non-responsibilities:
//!
//! - rejecting a non-empty value the core would refuse, such as `a b` as a name; the core decides
//!   and the refusal reaches the caller as an RPC error
//! - deciding which values are refused, which each newtype does for itself

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

/// Defines an identifier newtype that refuses the empty string and nothing else.
macro_rules! refusing_identifier {
    ($(#[$meta:meta])* $name:ident, $what:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            #[doc = concat!("Wraps `value` as a ", $what, ".")]
            ///
            /// # Errors
            ///
            #[doc = concat!("Returns [`EmptyValueError`] naming the ", $what, " when `value` is empty.")]
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!("# use herdrify_hc::protocol::{EmptyValueError, ", stringify!($name), "};")]
            /// # fn main() -> Result<(), EmptyValueError> {
            #[doc = concat!("let value = ", stringify!($name), "::new(\"t7\")?;")]
            /// assert_eq!(value.as_str(), "t7");
            #[doc = concat!("assert!(", stringify!($name), "::new(\"\").is_err());")]
            /// # Ok(())
            /// # }
            /// ```
            pub fn new(value: impl Into<String>) -> Result<Self, EmptyValueError> {
                let value = value.into();
                if value.is_empty() {
                    Err(EmptyValueError { what: $what })
                } else {
                    Ok(Self(value))
                }
            }

            /// The wrapped text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<&str> for $name {
            type Error = EmptyValueError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl FromStr for $name {
            type Err = EmptyValueError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::new(value)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

refusing_identifier! {
    /// A terminal as a caller names it: an identifier such as `t7`, a name, or `machine/t7`.
    ///
    /// Only the empty string is refused. The core decides which other values name a terminal.
    TerminalRef, "terminal reference"
}

refusing_identifier! {
    /// The name given to a terminal when it is created.
    ///
    /// Only the empty string is refused. The core decides which other values are valid names.
    TerminalName, "terminal name"
}

refusing_identifier! {
    /// The name of a document in the core's document store.
    ///
    /// Only the empty string is refused. The core decides which other values are valid names.
    DocName, "doc name"
}

refusing_identifier! {
    /// The key of a terminal label.
    ///
    /// Only the empty string is refused. The core decides which other values are valid keys, and
    /// refuses a key such as `a=b`.
    LabelKey, "label key"
}

refusing_identifier! {
    /// The value of a terminal label.
    ///
    /// The empty string is refused because the core uses it to remove a label. Removing a label
    /// is a separate change, not an empty value.
    LabelValue, "label value"
}

refusing_identifier! {
    /// The name of a machine registered with the core.
    ///
    /// Only the empty string is refused. The core decides which machines exist.
    MachineName, "machine name"
}

refusing_identifier! {
    /// The name of a session on a machine.
    ///
    /// Only the empty string is refused. The core decides which other values are valid names.
    SessionName, "session name"
}

refusing_identifier! {
    /// The name of a key sent to a terminal, such as `Enter`.
    ///
    /// Only the empty string is refused. The core decides which key names it knows.
    KeyName, "key name"
}

refusing_identifier! {
    /// The name of a core method, such as `terminal.get`.
    ///
    /// Only the empty string is refused. The core decides which methods exist.
    MethodName, "method name"
}

/// Defines an identifier newtype that accepts every string, empty included.
macro_rules! accepting_identifier {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            #[doc = concat!("Wraps `value` as a [`", stringify!($name), "`] without checking it.")]
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!("use herdrify_hc::protocol::", stringify!($name), ";")]
            ///
            #[doc = concat!("assert_eq!(", stringify!($name), "::new(\"x\").as_str(), \"x\");")]
            #[doc = concat!("assert_eq!(", stringify!($name), "::new(\"\").as_str(), \"\");")]
            /// ```
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// The wrapped text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

accepting_identifier! {
    /// The address of a terminal as the core reports it, such as `t7` or `rohan/t7`.
    ///
    /// The core produces these and the SDK never refuses one it reads, so nothing is checked.
    TerminalAddress
}

accepting_identifier! {
    /// The identity a caller gives the core with a request.
    ///
    /// The core decides what is valid, and it accepts an empty value, so the SDK checks nothing.
    CallerId
}

accepting_identifier! {
    /// A path on the machine the core runs on, not on the local machine.
    ///
    /// The core decides what is valid, and it accepts an empty value, so the SDK checks nothing.
    RemotePath
}

accepting_identifier! {
    /// The name a file is stored under when it is sent to a terminal.
    ///
    /// The core decides what is valid, and it accepts an empty value, so the SDK checks nothing.
    FileName
}

/// The version of a document, which the core advances on every change.
///
/// The field is public so a caller writes `DocVersion(1)`.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::DocVersion;
///
/// assert_eq!(DocVersion(1).get(), 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocVersion(pub u64);

impl DocVersion {
    /// The version number.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// A value that must not be empty was empty.
///
/// `what` names the value in words, such as `terminal reference` or `argv`, and is the first part
/// of the message.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::EmptyValueError;
///
/// let error = EmptyValueError { what: "argv" };
/// assert_eq!(error.to_string(), "argv must not be empty");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{what} must not be empty")]
pub struct EmptyValueError {
    /// The refused value, in words.
    pub what: &'static str,
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashSet};
    use std::error::Error;
    use std::fmt::Display;
    use std::hash::Hash;
    use std::str::FromStr;

    use super::{
        CallerId, DocName, DocVersion, EmptyValueError, FileName, KeyName, LabelKey, LabelValue,
        MachineName, MethodName, RemotePath, SessionName, TerminalAddress, TerminalName,
        TerminalRef,
    };

    type TestResult = Result<(), Box<dyn Error>>;

    /// Runs `$check!(Type, "what word")` for each of the nine refusing newtypes.
    macro_rules! for_each_refusing {
        ($check:ident) => {
            $check!(TerminalRef, "terminal reference");
            $check!(TerminalName, "terminal name");
            $check!(DocName, "doc name");
            $check!(LabelKey, "label key");
            $check!(LabelValue, "label value");
            $check!(MachineName, "machine name");
            $check!(SessionName, "session name");
            $check!(KeyName, "key name");
            $check!(MethodName, "method name");
        };
    }

    #[test]
    fn each_refusing_newtype_refuses_empty_with_its_what_word() {
        macro_rules! check {
            ($ty:ident, $what:literal) => {
                assert_eq!($ty::new(""), Err(EmptyValueError { what: $what }));
            };
        }
        for_each_refusing!(check);
    }

    /// Uses `low` (text `a`) and `high` (text `b`) as set keys and prints them.
    fn assert_usable_as_a_key<T: Hash + Ord + Clone + Display>(low: &T, high: &T) {
        let ordered: BTreeSet<T> = [high.clone(), low.clone(), high.clone()].into();
        let texts: Vec<String> = ordered.iter().map(ToString::to_string).collect();
        assert_eq!(texts, ["a", "b"]);
        let hashed: HashSet<T> = [low.clone(), high.clone(), low.clone()].into();
        assert_eq!(hashed.len(), 2);
        assert!(hashed.contains(low));
    }

    #[test]
    fn refusing_newtypes_are_keys_and_display_their_text() -> TestResult {
        macro_rules! check {
            ($ty:ident, $what:literal) => {
                assert_usable_as_a_key(&$ty::new("a")?, &$ty::new("b")?);
            };
        }
        for_each_refusing!(check);
        Ok(())
    }

    #[test]
    fn accepting_newtypes_are_keys_and_display_their_text() {
        assert_usable_as_a_key(&CallerId::new("a"), &CallerId::new("b"));
        assert_usable_as_a_key(&FileName::new("a"), &FileName::new("b"));
        assert_usable_as_a_key(&RemotePath::new("a"), &RemotePath::new("b"));
        assert_usable_as_a_key(&TerminalAddress::new("a"), &TerminalAddress::new("b"));
    }

    #[test]
    fn refusing_newtypes_accept_values_the_core_refuses() -> TestResult {
        assert_eq!(TerminalName::new("a b")?.as_str(), "a b");
        assert_eq!(LabelKey::new("a=b")?.as_str(), "a=b");
        assert_eq!(TerminalRef::new(" ")?.as_str(), " ");
        Ok(())
    }

    #[test]
    fn refusing_newtype_deserialization_goes_through_new() -> TestResult {
        let reference: TerminalRef = serde_json::from_str("\"t7\"")?;
        assert_eq!(reference.as_str(), "t7");
        assert_eq!(serde_json::to_string(&reference)?, "\"t7\"");

        macro_rules! check {
            ($ty:ident, $what:literal) => {
                let refused = serde_json::from_str::<$ty>("\"\"");
                let message = refused.err().map(|error| error.to_string());
                assert!(
                    message
                        .as_deref()
                        .is_some_and(|text| text.contains(concat!($what, " must not be empty"))),
                    "{}: {message:?}",
                    stringify!($ty),
                );
            };
        }
        for_each_refusing!(check);
        Ok(())
    }

    #[test]
    fn refusing_newtype_parsing_matches_new() {
        macro_rules! check {
            ($ty:ident, $what:literal) => {
                for input in ["", "t7"] {
                    assert_eq!($ty::from_str(input), $ty::new(input));
                    assert_eq!($ty::try_from(input), $ty::new(input));
                }
            };
        }
        for_each_refusing!(check);
    }

    #[test]
    fn accepting_newtypes_accept_empty() -> TestResult {
        assert_eq!(serde_json::to_string(&CallerId::new(""))?, "\"\"");
        assert_eq!(serde_json::to_string(&FileName::new(""))?, "\"\"");
        assert_eq!(serde_json::to_string(&RemotePath::new(""))?, "\"\"");
        assert_eq!(serde_json::to_string(&TerminalAddress::new(""))?, "\"\"");
        assert_eq!(CallerId::from(String::new()).as_str(), "");
        Ok(())
    }

    #[test]
    fn accepting_newtypes_round_trip() -> TestResult {
        let address: TerminalAddress = serde_json::from_str("\"rohan/t7\"")?;
        assert_eq!(address.as_str(), "rohan/t7");
        assert_eq!(address.to_string(), "rohan/t7");
        assert_eq!(serde_json::to_string(&address)?, "\"rohan/t7\"");
        Ok(())
    }

    #[test]
    fn doc_version_is_a_transparent_integer() -> TestResult {
        assert_eq!(serde_json::to_string(&DocVersion(3))?, "3");
        let version: DocVersion = serde_json::from_str("3")?;
        assert_eq!(version, DocVersion(3));
        assert_eq!(version.get(), 3);
        Ok(())
    }

    #[test]
    fn empty_value_error_display_names_the_value() {
        let error = EmptyValueError { what: "argv" };
        assert_eq!(error.to_string(), "argv must not be empty");
    }

    #[test]
    fn empty_value_error_is_send_sync_static() {
        fn assert_bounds<T: Send + Sync + 'static>() {}
        assert_bounds::<EmptyValueError>();
    }
}
