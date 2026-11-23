//! The colours a herdr-app layout or theme holds: a hex value or a theme colour name.
//!
//! A layout paints with either a name the theme defines, such as `canvas`, or a literal
//! `#rrggbb` or `#rrggbbaa` value, and a theme holds only the literal form. Both are plain strings
//! in a file, so [`Color`] tells them apart by the leading `#`, and [`HexColor`] checks the literal
//! form once at construction.
//!
//! Responsibilities:
//!
//! - [`HexColor`], a `#` followed by exactly 6 or 8 hex digits, in either case, kept as written
//! - [`Color`], a [`HexColor`] or a [`ColorName`], read from and written as a plain string
//! - [`ColorError`], the refusal of an empty colour or a malformed hex one
//!
//! Non-responsibilities:
//!
//! - deciding that a name refers to a colour the theme defines, which herdr-app does
//! - converting a hex value to channels or comparing colours by value; `#ABCDEF` and `#abcdef`
//!   are different values here

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::identifiers::ColorName;

/// The refusal of a colour that is empty or is a malformed hex value.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::{Color, ColorError};
///
/// let refused: ColorError = Color::new("#12345").unwrap_err();
/// assert_eq!(
///     refused.to_string(),
///     "\"#12345\" is not a hex colour: use #rrggbb or #rrggbbaa",
/// );
/// assert_eq!(Color::new("").unwrap_err().to_string(), "a colour cannot be empty");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ColorError {
    /// The colour was the empty string.
    #[error("a colour cannot be empty")]
    Empty,
    /// The value was not `#` followed by exactly 6 or 8 hex digits.
    #[error("\"{value}\" is not a hex colour: use #rrggbb or #rrggbbaa")]
    NotHex {
        /// The value that was refused, as written.
        value: String,
    },
}

/// A literal colour: `#` followed by exactly 6 or 8 ASCII hex digits, in either case.
///
/// The text is kept as written, so `#ABCDEF` is written back as `#ABCDEF`.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::{ColorError, HexColor};
///
/// # fn main() -> Result<(), ColorError> {
/// assert_eq!(HexColor::new("#ffffff0d")?.as_str(), "#ffffff0d");
/// assert!(HexColor::new("#abcd").is_err());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct HexColor(String);

impl HexColor {
    /// Wraps `value` as a hex colour.
    ///
    /// # Errors
    ///
    /// Returns [`ColorError::NotHex`] when `value`, including the empty string, is not `#`
    /// followed by exactly 6 or 8 ASCII hex digits.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{ColorError, HexColor};
    ///
    /// # fn main() -> Result<(), ColorError> {
    /// assert_eq!(HexColor::new("#ABCDEF")?.as_str(), "#ABCDEF");
    /// assert_eq!(
    ///     HexColor::new("#abcdefg"),
    ///     Err(ColorError::NotHex { value: "#abcdefg".to_owned() }),
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(value: impl Into<String>) -> Result<Self, ColorError> {
        let value = value.into();
        let digits = value.strip_prefix('#').unwrap_or("");
        if matches!(digits.len(), 6 | 8) && digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            Ok(Self(value))
        } else {
            Err(ColorError::NotHex { value })
        }
    }

    /// The wrapped text, as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for HexColor {
    type Error = ColorError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<HexColor> for String {
    fn from(value: HexColor) -> Self {
        value.0
    }
}

impl FromStr for HexColor {
    type Err = ColorError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl fmt::Display for HexColor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A colour in a layout: a literal hex value or the name of a theme colour.
///
/// A value starting with `#` must be a valid [`HexColor`]; any other non-empty value is a
/// [`ColorName`]. It is read from and written as a plain string.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::{Color, ColorError};
///
/// # fn main() -> Result<(), ColorError> {
/// assert!(matches!(Color::new("#ffffff0d")?, Color::Hex(_)));
/// assert!(matches!(Color::new("canvas")?, Color::Named(_)));
/// assert!(Color::new("#12345").is_err());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Color {
    /// A literal `#rrggbb` or `#rrggbbaa` value.
    Hex(HexColor),
    /// The name of a colour the theme defines.
    Named(ColorName),
}

impl Color {
    /// Reads `value` as a hex colour when it starts with `#`, and as a colour name otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`ColorError::Empty`] when `value` is empty, and [`ColorError::NotHex`] when it
    /// starts with `#` but is not a valid [`HexColor`].
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{Color, ColorError};
    ///
    /// # fn main() -> Result<(), ColorError> {
    /// assert_eq!(Color::new("canvas")?.as_str(), "canvas");
    /// assert_eq!(Color::new(""), Err(ColorError::Empty));
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(value: impl Into<String>) -> Result<Self, ColorError> {
        let value = value.into();
        if value.starts_with('#') {
            HexColor::new(value).map(Self::Hex)
        } else {
            // The empty string is the only value a colour name refuses.
            ColorName::new(value)
                .map(Self::Named)
                .map_err(|_| ColorError::Empty)
        }
    }

    /// The colour as written: the hex text or the name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Hex(hex) => hex.as_str(),
            Self::Named(name) => name.as_str(),
        }
    }
}

impl TryFrom<String> for Color {
    type Error = ColorError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Color> for String {
    fn from(value: Color) -> Self {
        match value {
            Color::Hex(hex) => hex.into(),
            Color::Named(name) => name.into(),
        }
    }
}

impl FromStr for Color {
    type Err = ColorError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

impl fmt::Display for Color {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_parses_the_spec_examples() -> Result<(), ColorError> {
        assert!(matches!(Color::new("#ffffff0d")?, Color::Hex(_)));
        assert!(matches!(Color::new("canvas")?, Color::Named(_)));
        assert_eq!(
            Color::new("#12345"),
            Err(ColorError::NotHex {
                value: "#12345".to_owned()
            })
        );
        Ok(())
    }

    #[test]
    fn hex_color_lengths_and_case() -> Result<(), ColorError> {
        assert_eq!(HexColor::new("#ABCDEF")?.as_str(), "#ABCDEF");
        assert_eq!(HexColor::new("#abcdef12")?.as_str(), "#abcdef12");
        for value in [
            "#abcdefg",
            "#abcd",
            "#",
            "",
            "abcdef",
            "#abcdef1",
            "#abcdef123",
        ] {
            assert_eq!(
                HexColor::new(value),
                Err(ColorError::NotHex {
                    value: value.to_owned()
                }),
                "{value:?} was accepted"
            );
        }
        Ok(())
    }

    #[test]
    fn color_refuses_empty() {
        assert_eq!(Color::new(""), Err(ColorError::Empty));
        assert_eq!("".parse::<Color>(), Err(ColorError::Empty));
    }

    #[test]
    fn color_error_displays_its_text() {
        assert_eq!(ColorError::Empty.to_string(), "a colour cannot be empty");
        let not_hex = ColorError::NotHex {
            value: "#12345".to_owned(),
        };
        assert_eq!(
            not_hex.to_string(),
            "\"#12345\" is not a hex colour: use #rrggbb or #rrggbbaa"
        );
    }

    #[test]
    fn color_round_trips_as_a_string() -> Result<(), serde_json::Error> {
        for text in ["\"#ABCDEF\"", "\"#abcdef12\"", "\"canvas\""] {
            let color: Color = serde_json::from_str(text)?;
            assert_eq!(serde_json::to_string(&color)?, text);
        }
        let hex: HexColor = serde_json::from_str("\"#ABCDEF\"")?;
        assert_eq!(serde_json::to_string(&hex)?, "\"#ABCDEF\"");
        Ok(())
    }

    #[test]
    fn color_displays_and_parses_as_written() -> Result<(), ColorError> {
        assert_eq!("#ABCDEF".parse::<Color>()?.to_string(), "#ABCDEF");
        assert_eq!("canvas".parse::<Color>()?.to_string(), "canvas");
        assert_eq!("#abcdef".parse::<HexColor>()?.to_string(), "#abcdef");
        Ok(())
    }

    /// A file shape holding one colour in a direct field.
    #[derive(Debug, Deserialize)]
    struct Holder {
        #[expect(dead_code, reason = "read only to prove the field parses")]
        color: Color,
    }

    #[test]
    fn color_error_in_a_file_keeps_its_text() -> Result<(), String> {
        let error = serde_json::from_str::<Holder>(r##"{"color":"#12345"}"##)
            .err()
            .ok_or("a bad colour was accepted")?;
        assert_eq!(
            error.to_string(),
            "\"#12345\" is not a hex colour: use #rrggbb or #rrggbbaa at line 1 column 18"
        );
        Ok(())
    }
}
