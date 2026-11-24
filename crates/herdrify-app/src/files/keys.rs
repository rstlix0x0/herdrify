//! The keys file: the key chords herdr-app binds, grouped by the section they apply in.
//!
//! The file maps a section, such as `global` or `layout:herdrify-main`, to the chords bound in it.
//! A section has a text form that a plain string would not check, so it is a type of its own. The
//! whole file is a [`Keys`]: sections of chords, each bound to an action.
//!
//! Responsibilities:
//!
//! - [`Keys`], the file: its sections, parsing from text and writing back as JSON
//! - [`Section`], the key of the file's outer map: read from and written as its string
//! - [`Action`], what a chord does: a string or an object, read into the form it has and written
//!   back as it was read
//! - [`KeyActionError`], the refusal of a string action whose command name is empty
//!
//! Non-responsibilities:
//!
//! - deciding which sections herdr-app acts on, which herdr-app does; a section this crate does not
//!   name is kept as written
//! - deciding whether a command exists or what its value means, which herdr-app does; an action this
//!   crate does not recognise is kept as written
//! - reading a keys file from disk or locating it, which a layer above does

use std::collections::{BTreeMap, btree_map};
use std::convert::Infallible;
use std::fmt;
use std::str::FromStr;

use serde::de::{self, Deserializer};
use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

use crate::files::{Chord, CommandName, FileError, FileKind, LayoutName, ModName};

/// The part of herdr-app a group of key bindings applies in: the key of the keys file's outer map.
///
/// A section is read from its string:
/// - `global` is [`Section::Global`] and `terminal` is [`Section::Terminal`]
/// - `layout:` followed by a non-empty name is [`Section::Layout`], and `mod:` followed by a
///   non-empty name is [`Section::Mod`]
/// - anything else is [`Section::Other`], including `layout:`, `mod:`, `weird` and a different case
///   such as `Global`
///
/// The name is everything after the first `:`, so `layout:a:b` names the layout `a:b`. A section
/// is written back as the string it was read from. A [`Section::Other`] built by hand from text
/// that names one of the other forms is written as that text and so reads back as that form.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::{EmptyValueError, LayoutName, Section};
///
/// # fn main() -> Result<(), EmptyValueError> {
/// let section = Section::new("layout:herdrify-main");
/// assert_eq!(section, Section::Layout(LayoutName::new("herdrify-main")?));
/// assert_eq!(section.to_string(), "layout:herdrify-main");
/// assert_eq!(Section::new("layout:"), Section::Other("layout:".to_owned()));
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum Section {
    /// The bindings that apply everywhere: the string `global`.
    Global,
    /// The bindings that apply in a terminal: the string `terminal`.
    Terminal,
    /// The bindings of one layout: `layout:` and the layout's name.
    Layout(LayoutName),
    /// The bindings of one mod: `mod:` and the mod's name.
    Mod(ModName),
    /// Any other string, as written.
    Other(String),
}

impl Section {
    /// Reads a section from its string. Every string is a section, so this never fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::Section;
    ///
    /// assert_eq!(Section::new("global"), Section::Global);
    /// assert_eq!(Section::new("weird"), Section::Other("weird".to_owned()));
    /// ```
    #[must_use]
    pub fn new(value: impl AsRef<str>) -> Self {
        let value = value.as_ref();
        match value {
            "global" => Self::Global,
            "terminal" => Self::Terminal,
            _ => {
                if let Some(Ok(name)) = value.strip_prefix("layout:").map(LayoutName::new) {
                    Self::Layout(name)
                } else if let Some(Ok(name)) = value.strip_prefix("mod:").map(ModName::new) {
                    Self::Mod(name)
                } else {
                    Self::Other(value.to_owned())
                }
            }
        }
    }
}

impl From<String> for Section {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<Section> for String {
    fn from(value: Section) -> Self {
        value.to_string()
    }
}

impl FromStr for Section {
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(value))
    }
}

impl fmt::Display for Section {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Global => formatter.write_str("global"),
            Self::Terminal => formatter.write_str("terminal"),
            Self::Layout(name) => write!(formatter, "layout:{name}"),
            Self::Mod(name) => write!(formatter, "mod:{name}"),
            Self::Other(text) => formatter.write_str(text),
        }
    }
}

/// The refusal of a string action whose command name is empty.
///
/// It displays as `"<value>" is not a key action: its command name is empty`, with the refused
/// string in place of `<value>`.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::{Action, KeyActionError};
///
/// # fn main() -> Result<(), &'static str> {
/// let Err(refused): Result<Action, KeyActionError> = ":x".parse() else {
///     return Err("an empty command name was read");
/// };
/// assert_eq!(refused.value, ":x");
/// assert_eq!(
///     refused.to_string(),
///     r#"":x" is not a key action: its command name is empty"#,
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("\"{value}\" is not a key action: its command name is empty")]
pub struct KeyActionError {
    /// The string that was refused.
    pub value: String,
}

/// What a key chord does: the value of the keys file's inner map.
///
/// An action is read by the first of these rules that fits:
/// - the string `terminal` is [`Action::Terminal`]
/// - a string that starts with `send:` is [`Action::Send`] with the rest, so `send:` alone sends
///   the empty string
/// - any other string is split at its first `:` into a command name and an optional value, giving
///   [`Action::Run`]; an empty command name, as in `:x` or the empty string, is refused with a
///   [`KeyActionError`]
/// - an object whose `command` is a string and whose `args` is absent or an object is
///   [`Action::Command`]; every other key is kept in `extra`, and an empty `command` is refused
///   with an [`EmptyValueError`](crate::files::EmptyValueError)
/// - anything else, including an object with no `command` or a non-object `args`, a number, a
///   boolean, null and an array, is [`Action::Other`] and kept as written
///
/// An action read from a file is written back to text that reads back as an equal action: strings
/// as the same string, and objects with the same keys and values, though the keys of a
/// [`Action::Command`] may change order.
///
/// An action built by hand is written as the text of its own value, and that text is read by the
/// rules above, so it can read back as a different form or be refused:
/// - a [`Action::Run`] whose command name contains `:` is cut at its first `:` on reading, so `a:b`
///   with no value reads back as the command `a` with the value `b`
/// - a [`Action::Run`] of the command `terminal` with no value reads back as [`Action::Terminal`],
///   and one of the command `send` with a value reads back as [`Action::Send`]
/// - an [`Action::Other`] holding a string reads back as the form that string has, and is refused
///   when its command name is empty, as for the strings `:x` and the empty string
/// - an [`Action::Other`] holding an object whose `command` is a string and whose `args` is absent
///   or an object reads back as [`Action::Command`], and is refused when that `command` is empty
/// - a [`Action::Command`] whose `extra` holds a key named `command` or `args` is written without
///   that key, so it reads back with the key gone
///
/// # Examples
///
/// ```
/// use herdrify_app::files::Action;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let action: Action = serde_json::from_str(r#""send:ls -l""#)?;
/// assert_eq!(action, Action::Send("ls -l".to_owned()));
/// assert_eq!(serde_json::to_string(&action)?, r#""send:ls -l""#);
///
/// let empty: Action = serde_json::from_str(r#""send:""#)?;
/// assert_eq!(empty, Action::Send(String::new()));
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// The terminal's own binding: the string `terminal`.
    Terminal,
    /// Text typed into the focused terminal: `send:` and the text.
    Send(String),
    /// A command with an optional value: the command name, and after the first `:` the value.
    Run {
        /// The command name, such as `pane.split`.
        command: CommandName,
        /// The text after the first `:`, which is `Some("")` for `pane.close:`.
        value: Option<String>,
    },
    /// A command with structured arguments: an object with a `command` and optional `args`.
    Command {
        /// The command name, written as `command`.
        command: CommandName,
        /// The command's arguments, written as `args` when present.
        args: Option<Map<String, Value>>,
        /// Every other key of the object, as written. A key named `command` or `args` is not
        /// written, because those keys are the fields above.
        extra: Map<String, Value>,
    },
    /// Any other JSON value, kept whole.
    Other(Value),
}

impl Action {
    /// The action that runs `command` with no value.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{Action, CommandName};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let action = Action::run(CommandName::new("terminal.copy")?);
    /// assert_eq!(serde_json::to_string(&action)?, r#""terminal.copy""#);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub const fn run(command: CommandName) -> Self {
        Self::Run {
            command,
            value: None,
        }
    }

    /// The action that runs `command` with `value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{Action, CommandName};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let action = Action::run_with(CommandName::new("pane.split")?, "right");
    /// assert_eq!(serde_json::to_string(&action)?, r#""pane.split:right""#);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn run_with(command: CommandName, value: impl Into<String>) -> Self {
        Self::Run {
            command,
            value: Some(value.into()),
        }
    }

    /// Reads a string action by the rules for strings.
    fn from_text(text: &str) -> Result<Self, KeyActionError> {
        if text == "terminal" {
            return Ok(Self::Terminal);
        }
        if let Some(rest) = text.strip_prefix("send:") {
            return Ok(Self::Send(rest.to_owned()));
        }
        let (name, value) = match text.split_once(':') {
            Some((name, value)) => (name, Some(value.to_owned())),
            None => (text, None),
        };
        CommandName::new(name)
            .map(|command| Self::Run { command, value })
            .map_err(|_| KeyActionError {
                value: text.to_owned(),
            })
    }

    /// Reads an object action by the rules for objects, or keeps it whole as [`Action::Other`].
    fn from_object<E: de::Error>(mut object: Map<String, Value>) -> Result<Self, E> {
        let name = match (object.get("command"), object.get("args")) {
            (Some(Value::String(name)), None | Some(Value::Object(_))) => name.clone(),
            _ => return Ok(Self::Other(Value::Object(object))),
        };
        let command = CommandName::new(name).map_err(E::custom)?;
        object.remove("command");
        let args = match object.remove("args") {
            Some(Value::Object(args)) => Some(args),
            _ => None,
        };
        Ok(Self::Command {
            command,
            args,
            extra: object,
        })
    }
}

impl FromStr for Action {
    type Err = KeyActionError;

    /// Reads a string action: the same rules as a JSON string, with no quotes or escapes.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::from_text(text)
    }
}

impl<'de> Deserialize<'de> for Action {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match Value::deserialize(deserializer)? {
            Value::String(text) => Self::from_text(&text).map_err(de::Error::custom),
            Value::Object(object) => Self::from_object(object),
            other => Ok(Self::Other(other)),
        }
    }
}

impl Serialize for Action {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Terminal => serializer.serialize_str("terminal"),
            Self::Send(text) => serializer.collect_str(&format_args!("send:{text}")),
            Self::Run {
                command,
                value: None,
            } => serializer.collect_str(command),
            Self::Run {
                command,
                value: Some(value),
            } => serializer.collect_str(&format_args!("{command}:{value}")),
            Self::Command {
                command,
                args,
                extra,
            } => {
                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("command", command)?;
                if let Some(args) = args {
                    map.serialize_entry("args", args)?;
                }
                for (key, value) in extra {
                    if key != "command" && key != "args" {
                        map.serialize_entry(key, value)?;
                    }
                }
                map.end()
            }
            Self::Other(value) => value.serialize(serializer),
        }
    }
}

/// A keys file: the key chords bound in each section.
///
/// The file is a JSON object from a section to an object from a key chord to an action. A section
/// that appears twice in the text keeps only its last object, whole, and a chord that appears twice
/// in a section keeps its last action. An empty chord is refused, and so is a string action with an
/// empty command name. A section or action this crate does not name is kept as written.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::{Action, Keys, Section};
///
/// # fn main() -> Result<(), herdrify_app::files::FileError> {
/// let keys = Keys::parse(r#"{"global": {"ctrl-shift-t": "tab.new"}, "terminal": {"ctrl-1": "terminal"}}"#)?;
/// assert_eq!(keys.sections().count(), 2);
/// let terminal = keys.get(&Section::Terminal);
/// assert_eq!(
///     terminal.map(|chords| chords.values().collect::<Vec<_>>()),
///     Some(vec![&Action::Terminal]),
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Keys(BTreeMap<Section, BTreeMap<Chord, Action>>);

impl Keys {
    /// A keys file with no sections.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::Keys;
    ///
    /// let keys = Keys::new();
    /// assert_eq!(keys.sections().count(), 0);
    /// assert_eq!(keys.to_json(), "{}\n");
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// The chords bound in `section`, or `None` when the file has no such section.
    #[must_use]
    pub fn get(&self, section: &Section) -> Option<&BTreeMap<Chord, Action>> {
        self.0.get(section)
    }

    /// Binds `chord` to `action` in `section`, adding the section when it is absent. A chord that is
    /// already bound in that section is rebound to `action`.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{Action, Chord, Keys, LayoutName, Section};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut keys = Keys::new();
    /// keys.insert(
    ///     Section::Layout(LayoutName::new("herdrify-main")?),
    ///     Chord::new("ctrl-b")?,
    ///     Action::Terminal,
    /// );
    /// assert_eq!(
    ///     keys.to_json(),
    ///     "{\n  \"layout:herdrify-main\": {\n    \"ctrl-b\": \"terminal\"\n  }\n}\n",
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn insert(&mut self, section: Section, chord: Chord, action: Action) {
        self.0.entry(section).or_default().insert(chord, action);
    }

    /// Every section with the chords bound in it, in section order: `global`, `terminal`, layouts,
    /// mods, then any other section.
    pub fn sections(&self) -> impl Iterator<Item = (&Section, &BTreeMap<Chord, Action>)> {
        self.0.iter()
    }

    /// Reads a keys file from its text.
    ///
    /// # Errors
    ///
    /// Returns a [`FileError`] of [`FileKind::Keys`] when the text is empty, is not JSON, has text
    /// after the object, is not an object of objects, holds an empty chord, or holds a string action
    /// whose command name is empty or an object action whose `command` is empty. Its detail is the
    /// text of the JSON reader's error.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{FileKind, Keys};
    ///
    /// let refused = Keys::parse(r#"{"global": {"": "tab.new"}}"#).err();
    /// assert_eq!(refused.as_ref().map(|error| error.file()), Some(FileKind::Keys));
    /// assert!(
    ///     refused
    ///         .map(|error| error.to_string())
    ///         .is_some_and(|text| text.starts_with("invalid keys file: a key chord cannot be empty")),
    /// );
    /// ```
    pub fn parse(text: &str) -> Result<Self, FileError> {
        serde_json::from_str(text).map_err(|source| FileError::from_serde(FileKind::Keys, &source))
    }

    /// The keys file as text: pretty-printed JSON with two-space indentation and a final newline.
    ///
    /// Sections and chords come out in their sorted order, so the result has the meaning of the text
    /// the keys were read from but not necessarily its bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{Action, Chord, Keys, Section};
    ///
    /// # fn main() -> Result<(), herdrify_app::files::EmptyValueError> {
    /// let mut keys = Keys::new();
    /// keys.insert(Section::Terminal, Chord::new("ctrl-1")?, Action::Terminal);
    /// assert_eq!(
    ///     keys.to_json(),
    ///     "{\n  \"terminal\": {\n    \"ctrl-1\": \"terminal\"\n  }\n}\n",
    /// );
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn to_json(&self) -> String {
        // Writing keys cannot fail: every map key is a string, so the fallback is never taken.
        let mut text = serde_json::to_string_pretty(self).unwrap_or_default();
        text.push('\n');
        text
    }
}

impl IntoIterator for Keys {
    type Item = (Section, BTreeMap<Chord, Action>);
    type IntoIter = btree_map::IntoIter<Section, BTreeMap<Chord, Action>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::{Chord, CommandName, FileKind, LayoutName, ModName};
    use serde_json::json;

    const FORMS: [&str; 7] = [
        "global",
        "terminal",
        "layout:herdrify-main",
        "mod:herdrify-host",
        "layout:",
        "mod:",
        "weird",
    ];

    #[test]
    fn section_parses_every_form() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(Section::new("global"), Section::Global);
        assert_eq!(Section::new("terminal"), Section::Terminal);
        assert_eq!(
            Section::new("layout:herdrify-main"),
            Section::Layout(LayoutName::new("herdrify-main")?)
        );
        assert_eq!(
            Section::new("mod:herdrify-host"),
            Section::Mod(ModName::new("herdrify-host")?)
        );
        assert_eq!(
            Section::new("layout:"),
            Section::Other("layout:".to_owned())
        );
        assert_eq!(Section::new("mod:"), Section::Other("mod:".to_owned()));
        assert_eq!(Section::new("weird"), Section::Other("weird".to_owned()));
        Ok(())
    }

    #[test]
    fn section_writes_back_as_read() -> Result<(), Box<dyn std::error::Error>> {
        for text in FORMS {
            let json = serde_json::to_string(text)?;
            let section: Section = serde_json::from_str(&json)?;
            assert_eq!(serde_json::to_string(&section)?, json);
            assert_eq!(section.to_string(), text);
            assert_eq!(String::from(section), text);
        }
        Ok(())
    }

    #[test]
    fn section_nested_colon_and_case_are_kept() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            Section::new("layout:a:b"),
            Section::Layout(LayoutName::new("a:b")?)
        );
        assert_eq!(Section::new("Global"), Section::Other("Global".to_owned()));
        assert_eq!(Section::new(""), Section::Other(String::new()));
        Ok(())
    }

    #[test]
    fn section_orders_by_kind_then_name() {
        let mut sections: Vec<Section> = [
            "weird", "mod:b", "layout:b", "terminal", "global", "layout:a",
        ]
        .into_iter()
        .map(Section::new)
        .collect();
        sections.sort();
        let written: Vec<String> = sections.into_iter().map(String::from).collect();
        assert_eq!(
            written,
            [
                "global", "terminal", "layout:a", "layout:b", "mod:b", "weird"
            ]
        );
    }

    #[test]
    fn section_is_a_json_object_key() -> Result<(), Box<dyn std::error::Error>> {
        let text = r#"{"global":1,"layout:herdrify-main":2,"weird":3}"#;
        let map: std::collections::BTreeMap<Section, u8> = serde_json::from_str(text)?;
        assert_eq!(map.get(&Section::Global), Some(&1));
        assert_eq!(map.get(&Section::Other("weird".to_owned())), Some(&3));
        assert_eq!(serde_json::to_string(&map)?, text);
        Ok(())
    }

    #[test]
    fn section_from_str_never_fails() -> Result<(), Box<dyn std::error::Error>> {
        let section: Section = "mod:herdrify-host".parse()?;
        assert_eq!(section, Section::Mod(ModName::new("herdrify-host")?));
        Ok(())
    }

    const BRACED_ACTION: &str = r#""space.new:{a:b}""#;
    const BRACED_VALUE: &str = "{a:b}";

    const ACTIONS: [&str; 15] = [
        r#""terminal""#,
        r#""send:""#,
        r#""send:ls -l\n""#,
        r#""pane.split:right""#,
        BRACED_ACTION,
        r#""terminal.copy""#,
        r#""pane.close:""#,
        r#"{"command":"layout.resize","args":{"id":"sidebar","px":240}}"#,
        r#"{"command":"x","y":1}"#,
        r#"{"command":"x","args":7}"#,
        r#"{"args":{}}"#,
        "7",
        "null",
        "true",
        r#"["a"]"#,
    ];

    fn read(text: &str) -> Result<Action, serde_json::Error> {
        serde_json::from_str(text)
    }

    fn command(text: &str) -> Result<CommandName, Box<dyn std::error::Error>> {
        Ok(CommandName::new(text)?)
    }

    #[test]
    fn action_parses_strings() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            read(r#""pane.split:right""#)?,
            Action::run_with(command("pane.split")?, "right")
        );
        assert_eq!(
            read(BRACED_ACTION)?,
            Action::run_with(command("space.new")?, BRACED_VALUE)
        );
        assert_eq!(read(r#""send:""#)?, Action::Send(String::new()));
        assert_eq!(read(r#""send:a:b""#)?, Action::Send("a:b".to_owned()));
        assert_eq!(read(r#""terminal""#)?, Action::Terminal);
        assert_eq!(
            read(r#""terminal.copy""#)?,
            Action::run(command("terminal.copy")?)
        );
        assert_eq!(
            read(r#""terminal:x""#)?,
            Action::run_with(command("terminal")?, "x")
        );
        assert_eq!(
            read(r#""pane.close:""#)?,
            Action::run_with(command("pane.close")?, "")
        );
        Ok(())
    }

    #[test]
    fn action_parses_command_objects() -> Result<(), Box<dyn std::error::Error>> {
        let Action::Command {
            command: name,
            args,
            extra,
        } = read(r#"{"command":"layout.resize","args":{"id":"sidebar","px":240}}"#)?
        else {
            return Err("not a command object".into());
        };
        assert_eq!(name, command("layout.resize")?);
        assert_eq!(
            args.map(Value::Object),
            Some(json!({"id": "sidebar", "px": 240}))
        );
        assert!(extra.is_empty());

        let Action::Command {
            command: name,
            args,
            extra,
        } = read(r#"{"command":"x","y":1}"#)?
        else {
            return Err("not a command object".into());
        };
        assert_eq!(name, command("x")?);
        assert_eq!(args, None);
        assert_eq!(Value::Object(extra), json!({"y": 1}));
        Ok(())
    }

    #[test]
    fn action_keeps_other_json() -> Result<(), Box<dyn std::error::Error>> {
        for text in [
            r#"{"command":"x","args":7}"#,
            r#"{"command":"x","args":null}"#,
            r#"{"command":7}"#,
            r#"{"args":{}}"#,
            "7",
            "null",
            "true",
            r#"["a"]"#,
        ] {
            let value: Value = serde_json::from_str(text)?;
            let action = read(text)?;
            assert_eq!(action, Action::Other(value.clone()), "{text}");
            assert_eq!(serde_json::to_value(&action)?, value, "{text}");
        }
        Ok(())
    }

    #[test]
    fn action_refuses_empty_command_names() -> Result<(), Box<dyn std::error::Error>> {
        let Err(error) = read(r#"":x""#) else {
            return Err("an empty command name was read".into());
        };
        assert!(
            error
                .to_string()
                .starts_with(r#"":x" is not a key action: its command name is empty"#),
            "{error}"
        );
        let Err(error) = read(r#""""#) else {
            return Err("an empty string was read".into());
        };
        assert!(
            error
                .to_string()
                .starts_with(r#""" is not a key action: its command name is empty"#),
            "{error}"
        );
        let Err(error) = read(r#"{"command":""}"#) else {
            return Err("an empty command was read".into());
        };
        assert!(
            error
                .to_string()
                .starts_with("a command name cannot be empty"),
            "{error}"
        );

        let error = KeyActionError {
            value: ":x".to_owned(),
        };
        assert_eq!(
            error.to_string(),
            r#"":x" is not a key action: its command name is empty"#
        );
        assert_eq!(
            ":x".parse::<Action>(),
            Err(KeyActionError {
                value: ":x".to_owned()
            })
        );
        assert_eq!(
            "".parse::<Action>().map_err(|error| error.to_string()),
            Err(r#""" is not a key action: its command name is empty"#.to_owned())
        );
        Ok(())
    }

    #[test]
    fn action_writes_back_as_read() -> Result<(), Box<dyn std::error::Error>> {
        for text in ACTIONS {
            let expected: Value = serde_json::from_str(text)?;
            let action = read(text)?;
            assert_eq!(serde_json::to_value(&action)?, expected, "{text}");
            assert_eq!(read(&serde_json::to_string(&action)?)?, action, "{text}");
        }
        Ok(())
    }

    fn written_then_read(action: &Action) -> Result<Result<Action, String>, serde_json::Error> {
        let text = serde_json::to_string(action)?;
        Ok(read(&text).map_err(|error| error.to_string()))
    }

    #[test]
    fn action_built_by_hand_can_read_back_as_another_form() -> Result<(), Box<dyn std::error::Error>>
    {
        let colon = Action::run(command("a:b")?);
        assert_eq!(serde_json::to_string(&colon)?, r#""a:b""#);
        assert_eq!(
            written_then_read(&colon)?,
            Ok(Action::run_with(command("a")?, "b"))
        );

        let colon_with_value = Action::run_with(command("a:b")?, "c");
        assert_eq!(
            written_then_read(&colon_with_value)?,
            Ok(Action::run_with(command("a")?, "b:c"))
        );

        assert_eq!(
            written_then_read(&Action::run(command("terminal")?))?,
            Ok(Action::Terminal)
        );
        assert_eq!(
            written_then_read(&Action::run_with(command("send")?, "x"))?,
            Ok(Action::Send("x".to_owned()))
        );

        assert_eq!(
            written_then_read(&Action::Other(json!("terminal")))?,
            Ok(Action::Terminal)
        );
        assert_eq!(
            written_then_read(&Action::Other(json!("send:x")))?,
            Ok(Action::Send("x".to_owned()))
        );
        assert_eq!(
            written_then_read(&Action::Other(json!("pane.split:right")))?,
            Ok(Action::run_with(command("pane.split")?, "right"))
        );

        assert_eq!(
            written_then_read(&Action::Other(json!({"command": "x"})))?,
            Ok(Action::Command {
                command: command("x")?,
                args: None,
                extra: Map::new(),
            })
        );
        assert_eq!(
            written_then_read(&Action::Other(json!({"command": "x", "args": {}, "y": 1})))?,
            Ok(Action::Command {
                command: command("x")?,
                args: Some(Map::new()),
                extra: Map::from_iter([("y".to_owned(), json!(1))]),
            })
        );

        let mut extra = Map::new();
        extra.insert("command".to_owned(), json!("other"));
        extra.insert("args".to_owned(), json!(1));
        extra.insert("y".to_owned(), json!(2));
        let shadowed = Action::Command {
            command: command("x")?,
            args: None,
            extra,
        };
        assert_eq!(
            written_then_read(&shadowed)?,
            Ok(Action::Command {
                command: command("x")?,
                args: None,
                extra: Map::from_iter([("y".to_owned(), json!(2))]),
            })
        );
        Ok(())
    }

    #[test]
    fn action_built_by_hand_can_be_written_as_text_that_is_refused()
    -> Result<(), Box<dyn std::error::Error>> {
        for text in [":x", ""] {
            let action = Action::Other(json!(text));
            assert_eq!(serde_json::to_value(&action)?, json!(text));
            let Err(message) = written_then_read(&action)? else {
                return Err(format!("{text:?} was read").into());
            };
            assert!(
                message.starts_with(&format!(
                    r#""{text}" is not a key action: its command name is empty"#
                )),
                "{message}"
            );

            let mut keys = Keys::new();
            keys.insert(Section::Global, chord("a")?, action);
            let Err(error) = Keys::parse(&keys.to_json()) else {
                return Err(format!("a keys file with {text:?} was read").into());
            };
            assert!(
                error.to_string().starts_with(&format!(
                    r#"invalid keys file: "{text}" is not a key action: its command name is empty"#
                )),
                "{error}"
            );
        }

        let empty_command = Action::Other(json!({"command": ""}));
        let Err(message) = written_then_read(&empty_command)? else {
            return Err("an empty command was read".into());
        };
        assert!(
            message.starts_with("a command name cannot be empty"),
            "{message}"
        );
        Ok(())
    }

    #[test]
    fn action_from_str_reads_a_string_action() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!("terminal".parse::<Action>()?, Action::Terminal);
        assert_eq!("send:x".parse::<Action>()?, Action::Send("x".to_owned()));
        assert_eq!(
            "pane.split:right".parse::<Action>()?,
            Action::run_with(command("pane.split")?, "right")
        );
        Ok(())
    }

    #[test]
    fn action_command_writes_each_key_once() -> Result<(), Box<dyn std::error::Error>> {
        let mut extra = Map::new();
        extra.insert("command".to_owned(), json!("other"));
        extra.insert("args".to_owned(), json!(1));
        extra.insert("y".to_owned(), json!(2));
        let action = Action::Command {
            command: command("x")?,
            args: None,
            extra,
        };
        assert_eq!(serde_json::to_string(&action)?, r#"{"command":"x","y":2}"#);
        Ok(())
    }

    const DOCUMENTED: &str = r#"{
  "global":   { "ctrl-shift-t": "tab.new", "ctrl-shift-o": "pane.split:right", "alt-1": "space.select:1" },
  "terminal": { "ctrl-shift-c": "terminal.copy", "ctrl-left": "send:\u001bb", "ctrl-1": "terminal" },
  "layout:terminals": { "ctrl-shift-pagedown": "space.next", "ctrl-shift-pageup": "space.prev" },
  "mod:editor": { "ctrl-s": "editor.save" }
}"#;

    fn chord(text: &str) -> Result<Chord, Box<dyn std::error::Error>> {
        Ok(Chord::new(text)?)
    }

    fn parsed(text: &str) -> Result<Keys, Box<dyn std::error::Error>> {
        Ok(Keys::parse(text)?)
    }

    #[test]
    fn keys_parse_the_documented_example() -> Result<(), Box<dyn std::error::Error>> {
        let keys = parsed(DOCUMENTED)?;
        assert_eq!(keys.sections().count(), 4);
        let global = keys.get(&Section::Global).ok_or("no global section")?;
        assert_eq!(global.len(), 3);
        assert_eq!(
            global.get(&chord("ctrl-shift-o")?),
            Some(&Action::run_with(command("pane.split")?, "right"))
        );
        let terminal = keys.get(&Section::Terminal).ok_or("no terminal section")?;
        assert_eq!(terminal.get(&chord("ctrl-1")?), Some(&Action::Terminal));
        assert_eq!(
            terminal.get(&chord("ctrl-left")?),
            Some(&Action::Send("\u{1b}b".to_owned()))
        );
        let layout = keys
            .get(&Section::Layout(LayoutName::new("terminals")?))
            .ok_or("no layout section")?;
        assert_eq!(layout.len(), 2);
        let editor = keys
            .get(&Section::Mod(ModName::new("editor")?))
            .ok_or("no mod section")?;
        assert_eq!(
            editor.get(&chord("ctrl-s")?),
            Some(&Action::run(command("editor.save")?))
        );

        let source: Value = serde_json::from_str(DOCUMENTED)?;
        assert_eq!(serde_json::from_str::<Value>(&keys.to_json())?, source);
        assert_eq!(parsed(&keys.to_json())?, keys);
        Ok(())
    }

    #[test]
    fn keys_refuse_an_empty_chord() -> Result<(), Box<dyn std::error::Error>> {
        let Err(error) = Keys::parse(r#"{"global":{"":"tab.new"}}"#) else {
            return Err("an empty chord was read".into());
        };
        assert!(
            error
                .to_string()
                .starts_with("invalid keys file: a key chord cannot be empty"),
            "{error}"
        );
        assert_eq!(error.file(), FileKind::Keys);
        Ok(())
    }

    #[test]
    fn keys_express_herdrify_sections() -> Result<(), Box<dyn std::error::Error>> {
        let mut keys = Keys::new();
        keys.insert(
            Section::Layout(LayoutName::new("herdrify-main")?),
            chord("ctrl-b")?,
            Action::run(command("tab.new")?),
        );
        keys.insert(
            Section::Mod(ModName::new("herdrify-host")?),
            chord("ctrl-h")?,
            Action::run_with(command("herdrify-host.open")?, "x"),
        );
        let value: Value = serde_json::from_str(&keys.to_json())?;
        assert_eq!(
            value,
            json!({
                "layout:herdrify-main": {"ctrl-b": "tab.new"},
                "mod:herdrify-host": {"ctrl-h": "herdrify-host.open:x"},
            })
        );
        Ok(())
    }

    #[test]
    fn keys_start_empty_and_write_an_empty_object() -> Result<(), Box<dyn std::error::Error>> {
        let keys = Keys::new();
        assert_eq!(keys, Keys::default());
        assert_eq!(keys.sections().count(), 0);
        assert_eq!(keys.get(&Section::Global), None);
        assert_eq!(keys.to_json(), "{}\n");
        assert_eq!(parsed("{}")?, keys);
        Ok(())
    }

    #[test]
    fn keys_insert_replaces_a_bound_chord() -> Result<(), Box<dyn std::error::Error>> {
        let mut keys = Keys::new();
        keys.insert(Section::Global, chord("a")?, Action::Terminal);
        keys.insert(Section::Global, chord("a")?, Action::Send("x".to_owned()));
        keys.insert(Section::Global, chord("b")?, Action::Terminal);
        let global = keys.get(&Section::Global).ok_or("no global section")?;
        assert_eq!(global.len(), 2);
        assert_eq!(
            global.get(&chord("a")?),
            Some(&Action::Send("x".to_owned()))
        );
        Ok(())
    }

    #[test]
    fn keys_repeated_section_and_chord_keep_the_last() -> Result<(), Box<dyn std::error::Error>> {
        let keys = parsed(
            r#"{"global":{"a":"x","b":"y"},"terminal":{"c":"z"},"global":{"a":"w","a":"v"}}"#,
        )?;
        assert_eq!(keys.sections().count(), 2);
        let global = keys.get(&Section::Global).ok_or("no global section")?;
        assert_eq!(global.len(), 1);
        assert_eq!(global.get(&chord("a")?), Some(&Action::run(command("v")?)));
        Ok(())
    }

    #[test]
    fn keys_keep_other_sections_and_actions() -> Result<(), Box<dyn std::error::Error>> {
        let text = r#"{"weird":{"a":7,"b":{"command":"x","args":1}},"layout:":{"c":null}}"#;
        let keys = parsed(text)?;
        assert!(keys.get(&Section::new("weird")).is_some());
        assert!(keys.get(&Section::new("layout:")).is_some());
        let source: Value = serde_json::from_str(text)?;
        assert_eq!(serde_json::from_str::<Value>(&keys.to_json())?, source);
        Ok(())
    }

    #[test]
    fn keys_refuse_a_bad_action_and_bad_text() -> Result<(), Box<dyn std::error::Error>> {
        let Err(error) = Keys::parse(r#"{"global":{"a":":x"}}"#) else {
            return Err("an empty command name was read".into());
        };
        assert!(
            error.to_string().starts_with(
                r#"invalid keys file: ":x" is not a key action: its command name is empty"#
            ),
            "{error}"
        );
        let Err(error) = Keys::parse("") else {
            return Err("an empty text was read".into());
        };
        assert_eq!(
            error.to_string(),
            "invalid keys file: EOF while parsing a value at line 1 column 0"
        );
        let Err(error) = Keys::parse(r#"{"global":[]}"#) else {
            return Err("an array section was read".into());
        };
        assert!(
            error
                .to_string()
                .starts_with("invalid keys file: invalid type: sequence"),
            "{error}"
        );
        Ok(())
    }

    #[test]
    fn keys_iterate_over_sections_in_order() -> Result<(), Box<dyn std::error::Error>> {
        let keys = parsed(r#"{"mod:b":{},"global":{"a":"x"},"terminal":{}}"#)?;
        let by_ref: Vec<String> = keys
            .sections()
            .map(|(section, _)| section.to_string())
            .collect();
        assert_eq!(by_ref, ["global", "terminal", "mod:b"]);
        let owned: Vec<(Section, usize)> = keys
            .into_iter()
            .map(|(section, chords)| (section, chords.len()))
            .collect();
        assert_eq!(owned[0], (Section::Global, 1));
        assert_eq!(owned.len(), 3);
        Ok(())
    }

    #[test]
    fn keys_to_json_is_pretty_with_newline() -> Result<(), Box<dyn std::error::Error>> {
        let json = parsed(r#"{"global":{"a":"x"}}"#)?.to_json();
        assert_eq!(json, "{\n  \"global\": {\n    \"a\": \"x\"\n  }\n}\n");
        Ok(())
    }
}
