//! The name newtypes the herdr-app files use, and the error an empty one returns.
//!
//! A bare `String` cannot say whether it names a node, a mod or a colour, so each name has its own
//! type and a field cannot take one where another belongs. They sit apart from the file shapes
//! because layouts, themes and keys all share them.
//!
//! Responsibilities:
//!
//! - the eight names, each checked once at construction and each read from and written as a plain
//!   string
//! - [`EmptyValueError`], the one error every name returns for an empty value, which carries the
//!   label of the name that was refused
//!
//! Non-responsibilities:
//!
//! - rejecting a non-empty value herdr-app would refuse, such as a mod name with a space in it;
//!   herdr-app decides, and a whitespace-only value is accepted here
//! - deciding where a name is used, which the file shapes do

use thiserror::Error;

/// The refusal of an empty name.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::{EmptyValueError, ModName};
///
/// let refused: EmptyValueError = ModName::new("").unwrap_err();
/// assert_eq!(refused.label, "mod name");
/// assert_eq!(refused.to_string(), "a mod name cannot be empty");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("a {label} cannot be empty")]
pub struct EmptyValueError {
    /// What the refused value was meant to name, such as `mod name`.
    pub label: &'static str,
}

/// Defines a name newtype that refuses the empty string and nothing else.
///
/// `$module` is the public module the type is exported from, which its doctest imports it by, and
/// `$example` is the value that doctest wraps. Every path the expansion names is absolute, so the
/// caller needs no imports for it.
macro_rules! name {
    ($module:literal, $(#[$meta:meta])* $name:ident, $label:literal, $example:literal) => {
        $(#[$meta])*
        #[derive(
            Debug,
            Clone,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            ::serde::Serialize,
            ::serde::Deserialize,
        )]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            #[doc = concat!("Wraps `value` as a ", $label, ".")]
            ///
            /// # Errors
            ///
            #[doc = concat!("Returns [`EmptyValueError`] labelled `", $label, "` when `value` is empty.")]
            ///
            /// # Examples
            ///
            /// ```
            #[doc = concat!("# use herdrify_app::{files::EmptyValueError, ", $module, "::", stringify!($name), "};")]
            /// # fn main() -> Result<(), EmptyValueError> {
            #[doc = concat!("let value = ", stringify!($name), "::new(\"", $example, "\")?;")]
            #[doc = concat!("assert_eq!(value.as_str(), \"", $example, "\");")]
            #[doc = concat!("assert!(", stringify!($name), "::new(\"\").is_err());")]
            /// # Ok(())
            /// # }
            /// ```
            pub fn new(value: impl Into<String>) -> Result<Self, $crate::files::EmptyValueError> {
                let value = value.into();
                if value.is_empty() {
                    Err($crate::files::EmptyValueError { label: $label })
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

        impl TryFrom<String> for $name {
            type Error = $crate::files::EmptyValueError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl ::core::str::FromStr for $name {
            type Err = $crate::files::EmptyValueError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::new(value)
            }
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

pub(crate) use name;

name! {
    "files",
    /// The identifier of a node in a layout.
    ///
    /// Only the empty string is refused.
    NodeId, "node id", "root"
}

name! {
    "files",
    /// The name of a herdr-app mod.
    ///
    /// Only the empty string is refused. herdr-app decides which mods exist.
    ModName, "mod name", "herdrify-host"
}

name! {
    "files",
    /// The name of a layout.
    ///
    /// Only the empty string is refused.
    LayoutName, "layout name", "main"
}

name! {
    "files",
    /// The name of a theme.
    ///
    /// Only the empty string is refused.
    ThemeName, "theme name", "win98"
}

name! {
    "files",
    /// The address of a terminal, as a layout names the terminal it shows.
    ///
    /// Only the empty string is refused. herdr-core decides which addresses exist.
    TerminalAddress, "terminal address", "t2"
}

name! {
    "files",
    /// The name of a colour in a theme.
    ///
    /// Only the empty string is refused.
    ColorName, "colour name", "accent"
}

name! {
    "files",
    /// The name of a herdr-app command a key or a button runs.
    ///
    /// Only the empty string is refused. herdr-app decides which commands exist.
    CommandName, "command name", "tab.new"
}

name! {
    "files",
    /// A key chord in a keys file, such as `ctrl+b`.
    ///
    /// Only the empty string is refused. herdr-app decides which chords it understands.
    Chord, "key chord", "ctrl+b"
}

#[cfg(test)]
mod tests {
    use std::fmt;

    use serde::{Deserialize, Serialize};

    use super::*;

    /// Asserts that `new("")` refuses with `label`, in both the field and the displayed text.
    fn refuses_with<T: fmt::Debug>(
        refused: Result<T, EmptyValueError>,
        label: &str,
    ) -> Result<(), String> {
        let error = refused.err().ok_or("an empty value was accepted")?;
        if error.label != label {
            return Err(format!("label was {:?}, not {label:?}", error.label));
        }
        let shown = error.to_string();
        if shown != format!("a {label} cannot be empty") {
            return Err(format!("displayed {shown:?}"));
        }
        Ok(())
    }

    #[test]
    fn each_name_refuses_empty_with_its_label() -> Result<(), String> {
        refuses_with(NodeId::new(""), "node id")?;
        refuses_with(ModName::new(""), "mod name")?;
        refuses_with(LayoutName::new(""), "layout name")?;
        refuses_with(ThemeName::new(""), "theme name")?;
        refuses_with(TerminalAddress::new(""), "terminal address")?;
        refuses_with(ColorName::new(""), "colour name")?;
        refuses_with(CommandName::new(""), "command name")?;
        refuses_with(Chord::new(""), "key chord")
    }

    #[test]
    fn names_accept_whitespace() -> Result<(), EmptyValueError> {
        assert_eq!(NodeId::new(" ")?.as_str(), " ");
        Ok(())
    }

    /// Reads `"herdrify-host"` as `T`, writes it back, and checks `""` is refused.
    fn round_trips<T>() -> Result<(), serde_json::Error>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        let value: T = serde_json::from_str("\"herdrify-host\"")?;
        assert_eq!(serde_json::to_string(&value)?, "\"herdrify-host\"");
        assert!(serde_json::from_str::<T>("\"\"").is_err());
        Ok(())
    }

    #[test]
    fn names_round_trip_through_serde() -> Result<(), serde_json::Error> {
        round_trips::<NodeId>()?;
        round_trips::<ModName>()?;
        round_trips::<LayoutName>()?;
        round_trips::<ThemeName>()?;
        round_trips::<TerminalAddress>()?;
        round_trips::<ColorName>()?;
        round_trips::<CommandName>()?;
        round_trips::<Chord>()
    }

    #[test]
    fn name_parsing_matches_new() {
        assert_eq!("".parse::<NodeId>(), NodeId::new(""));
        assert_eq!("x".parse::<NodeId>(), NodeId::new("x"));
        assert_eq!("".parse::<Chord>(), Chord::new(""));
        assert_eq!("x".parse::<Chord>(), Chord::new("x"));
    }
}
