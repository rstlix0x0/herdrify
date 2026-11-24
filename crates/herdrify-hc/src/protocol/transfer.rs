//! The identifier of one file transfer, and the rule a value must meet to be one.
//!
//! A transfer id names a transfer that spans several calls, so the core keys its state by it. The
//! core accepts only 16 to 64 hexadecimal digits, so this type enforces that rule when it is built
//! and when it is decoded. A value that breaks it can then never reach the wire.
//!
//! Responsibilities:
//!
//! - [`TransferId`], a validated transfer identifier, which can also make a fresh one
//! - [`InvalidTransferId`], the error a value that breaks the rule returns
//!
//! Non-responsibilities:
//!
//! - sending a transfer or tracking its progress, which the calls that carry the id do
//!
//! Tests here cover the rule through every way of building an id, the exact refusal message, the
//! wire form, the shape and variety of random ids, and the thread-safety bounds of the id and its error.

use std::collections::hash_map::RandomState;
use std::fmt;
use std::hash::BuildHasher;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The fewest characters a transfer id has.
const MIN_LEN: usize = 16;

/// The most characters a transfer id has.
const MAX_LEN: usize = 64;

/// The identifier of one file transfer.
///
/// An id has 16 to 64 characters, each an ASCII hexadecimal digit in either case. Nothing is
/// trimmed and the case is kept, so the id reads back exactly as it was given. Decoding a string
/// that breaks the rule fails, so every `TransferId` is valid.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::TransferId;
///
/// let id = TransferId::new("ABCDEF0123456789")?;
/// assert_eq!(id.as_str(), "ABCDEF0123456789");
/// assert_eq!(serde_json::to_string(&id)?, r#""ABCDEF0123456789""#);
/// assert!(TransferId::new("not hex").is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct TransferId(String);

impl TransferId {
    /// Builds an id from `value`, which must be 16 to 64 ASCII hexadecimal digits.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_hc::protocol::TransferId;
    ///
    /// assert!(TransferId::new("0123456789abcdef").is_ok());
    /// assert!(TransferId::new("0123456789abcde").is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`InvalidTransferId`] holding `value` when it is shorter than 16 or longer than 64
    /// characters, or has a character that is not an ASCII hexadecimal digit.
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidTransferId> {
        let value = value.into();
        if is_valid(&value) {
            Ok(Self(value))
        } else {
            Err(InvalidTransferId { value })
        }
    }

    /// Makes a fresh id of 32 lowercase hexadecimal digits.
    ///
    /// The digits come from the standard library's randomly keyed hasher, so no dependency is
    /// needed. The id is not a secret and is not meant to be unguessable: it only tells open puts
    /// apart.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_hc::protocol::TransferId;
    ///
    /// let id = TransferId::random();
    /// assert_eq!(id.as_str().len(), 32);
    /// assert!(TransferId::new(id.as_str()).is_ok());
    /// assert_ne!(id, TransferId::random());
    /// ```
    #[must_use]
    pub fn random() -> Self {
        // Each `RandomState` is seeded independently, so the two halves are hashes under different
        // keys of different inputs.
        let high = RandomState::new().hash_one(0_u8);
        let low = RandomState::new().hash_one(1_u8);
        Self(format!("{high:016x}{low:016x}"))
    }

    /// The id as it was given.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_hc::protocol::TransferId;
    ///
    /// let id = TransferId::new("0123456789abcdef")?;
    /// assert_eq!(id.as_str(), "0123456789abcdef");
    /// # Ok::<(), herdrify_hc::protocol::InvalidTransferId>(())
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether `value` is 16 to 64 characters, every one an ASCII hexadecimal digit.
///
/// Every accepted character is one byte, so the byte length is the character count for a value
/// that passes, and a multi-byte character fails the digit test.
fn is_valid(value: &str) -> bool {
    (MIN_LEN..=MAX_LEN).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

impl fmt::Display for TransferId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for TransferId {
    type Error = InvalidTransferId;

    /// Builds an id from an owned string, which must be 16 to 64 ASCII hexadecimal digits.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_hc::protocol::TransferId;
    ///
    /// let id = TransferId::try_from(String::from("0123456789abcdef"))?;
    /// assert_eq!(id.as_str(), "0123456789abcdef");
    /// assert!(TransferId::try_from(String::from("xyz")).is_err());
    /// # Ok::<(), herdrify_hc::protocol::InvalidTransferId>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`InvalidTransferId`] holding the string when it breaks that rule.
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for TransferId {
    type Error = InvalidTransferId;

    /// Builds an id from a string slice, which must be 16 to 64 ASCII hexadecimal digits.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_hc::protocol::TransferId;
    ///
    /// let id = TransferId::try_from("0123456789abcdef")?;
    /// assert_eq!(id.as_str(), "0123456789abcdef");
    /// assert!(TransferId::try_from("xyz").is_err());
    /// # Ok::<(), herdrify_hc::protocol::InvalidTransferId>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`InvalidTransferId`] holding the text when it breaks that rule.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl FromStr for TransferId {
    type Err = InvalidTransferId;

    /// Parses an id from text, which must be 16 to 64 ASCII hexadecimal digits.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_hc::protocol::TransferId;
    ///
    /// let id: TransferId = "0123456789abcdef".parse()?;
    /// assert_eq!(id.to_string(), "0123456789abcdef");
    /// assert!("xyz".parse::<TransferId>().is_err());
    /// # Ok::<(), herdrify_hc::protocol::InvalidTransferId>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`InvalidTransferId`] holding the text when it breaks that rule.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(text)
    }
}

/// The error a value that is not a valid transfer id returns.
///
/// It keeps the refused value.
///
/// # Examples
///
/// ```
/// use herdrify_hc::protocol::TransferId;
///
/// let refused = TransferId::new("xyz").unwrap_err();
/// assert_eq!(refused.value, "xyz");
/// assert_eq!(refused.to_string(), r#"transfer id "xyz" must be 16 to 64 hex digits"#);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("transfer id {value:?} must be 16 to 64 hex digits")]
pub struct InvalidTransferId {
    /// The value that was refused.
    pub value: String,
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::{InvalidTransferId, TransferId};

    /// One entry point's name and what it made of the value; a refusal carries its text.
    type Outcome = (&'static str, Result<TransferId, String>);

    /// Builds `value` through every entry point, the serde decode included, and returns each
    /// outcome with the name of the entry that produced it. Encoding `value` as JSON text is the
    /// helper's own step, so a failure there is returned rather than turned into an outcome.
    fn through_every_entry(value: &str) -> Result<Vec<Outcome>, serde_json::Error> {
        let json = serde_json::to_string(value)?;
        Ok(vec![
            ("new", TransferId::new(value).map_err(|e| e.value)),
            (
                "try_from_str",
                <TransferId as TryFrom<&str>>::try_from(value).map_err(|e| e.value),
            ),
            (
                "try_from_string",
                <TransferId as TryFrom<String>>::try_from(value.to_owned()).map_err(|e| e.value),
            ),
            ("from_str", value.parse::<TransferId>().map_err(|e| e.value)),
            (
                "deserialize",
                serde_json::from_str::<TransferId>(&json).map_err(|e| e.to_string()),
            ),
        ])
    }

    #[test]
    fn transfer_id_accepts_the_valid_examples() -> Result<(), Box<dyn Error>> {
        let sixty_four = "a".repeat(64);
        for value in ["0123456789abcdef", sixty_four.as_str(), "ABCDEF0123456789"] {
            for (entry, outcome) in through_every_entry(value)? {
                let id = outcome.map_err(|e| format!("{entry} refused {value:?}: {e}"))?;
                assert_eq!(id.as_str(), value, "{entry} changed the value");
                assert_eq!(id.to_string(), value, "{entry} changed the displayed value");
            }
        }
        Ok(())
    }

    #[test]
    fn transfer_id_refuses_the_invalid_examples() -> Result<(), Box<dyn Error>> {
        let sixty_five = "b".repeat(65);
        let invalid = [
            "0123456789abcde",
            sixty_five.as_str(),
            "zzzzzzzzzzzzzzzz",
            "",
            " 0123456789abcdef",
            "0123456789abcdeé",
        ];
        for value in invalid {
            for (entry, outcome) in through_every_entry(value)? {
                let refusal = outcome.err();
                assert!(refusal.is_some(), "{entry} accepted {value:?}");
                if entry == "deserialize" {
                    let message = refusal.unwrap_or_default();
                    assert!(
                        message.contains("must be 16 to 64 hex digits"),
                        "{entry} refused {value:?} with another message: {message}"
                    );
                } else {
                    assert_eq!(
                        refusal.as_deref(),
                        Some(value),
                        "{entry} refused with another value"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn invalid_transfer_id_message_is_exact() {
        let refused = TransferId::new("xyz").err();
        assert_eq!(
            refused.map(|e| e.to_string()),
            Some(r#"transfer id "xyz" must be 16 to 64 hex digits"#.to_owned())
        );
    }

    #[test]
    fn transfer_id_serializes_as_a_plain_string() -> Result<(), Box<dyn Error>> {
        let id = TransferId::new("0123456789abcdef")?;
        assert_eq!(serde_json::to_string(&id)?, r#""0123456789abcdef""#);
        Ok(())
    }

    #[test]
    fn random_ids_are_32_lowercase_hex_digits_and_valid() -> Result<(), Box<dyn Error>> {
        let id = TransferId::random();
        assert_eq!(id.as_str().len(), 32);
        assert!(
            id.as_str()
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "{id} is not lowercase hex"
        );
        let rebuilt = TransferId::new(id.as_str())?;
        assert_eq!(rebuilt, id);
        Ok(())
    }

    #[test]
    fn random_ids_differ() {
        assert_ne!(TransferId::random(), TransferId::random());
    }

    #[test]
    fn invalid_transfer_id_is_send_sync_static() {
        fn assert_bounds<T: Send + Sync + 'static>() {}
        assert_bounds::<InvalidTransferId>();
    }

    #[test]
    fn transfer_id_is_send_sync_static() {
        fn assert_bounds<T: Send + Sync + 'static>() {}
        assert_bounds::<TransferId>();
    }
}
