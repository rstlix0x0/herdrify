//! The theme file: the colours a layout's `theme` names.
//!
//! A theme gives a background, a foreground, a selection colour, the sixteen terminal colours and,
//! optionally, a cursor colour and named colours that a layout's colour names refer to. Every colour
//! in a theme is a literal hex value, never a colour name.
//!
//! Responsibilities:
//!
//! - [`Theme`], the file: its colours, parsing from text and writing back as JSON
//!
//! Non-responsibilities:
//!
//! - deciding which colour names a theme must define, which herdr-app does
//! - reading a theme from disk or locating it, which a layer above does

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::files::{ColorName, FileError, FileKind, HexColor};

/// A theme file: the colours a layout's theme gives it.
///
/// `background`, `foreground`, `selection` and `ansi` are required, and `ansi` holds exactly sixteen
/// colours. `cursor` and `colors` are optional: an absent one is not written, and a `null` one is
/// read as absent, so it is dropped when the theme is written back. A present but empty `colors` map
/// is kept and written as `{}`. Every key none of the named fields takes is kept in `extra`.
///
/// `extra` must not hold a key that a named field writes (`background`, `foreground`, `selection`,
/// `cursor`, `ansi` or `colors`): the key would be written twice.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::Theme;
///
/// # fn main() -> Result<(), herdrify_app::files::FileError> {
/// let text = r##"{
///   "background": "#101010",
///   "foreground": "#f0f0f0",
///   "selection": "#303030",
///   "cursor": "#ff00ff",
///   "ansi": ["#000000", "#111111", "#222222", "#333333", "#444444", "#555555",
///            "#666666", "#777777", "#888888", "#999999", "#aaaaaa", "#bbbbbb",
///            "#cccccc", "#dddddd", "#eeeeee", "#ffffff"]
/// }"##;
/// let theme = Theme::parse(text)?;
/// assert_eq!(theme.background.as_str(), "#101010");
/// assert_eq!(theme.ansi.len(), 16);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Theme {
    /// The terminal background colour.
    pub background: HexColor,
    /// The terminal foreground colour.
    pub foreground: HexColor,
    /// The selection colour.
    pub selection: HexColor,
    /// The cursor colour.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<HexColor>,
    /// The sixteen terminal colours, in order.
    pub ansi: [HexColor; 16],
    /// Named colours, by colour name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<BTreeMap<ColorName, HexColor>>,
    /// Every other key of the file, as written.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Theme {
    /// A theme with the required colours and no cursor, named colours or other keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{ColorError, HexColor, Theme};
    ///
    /// # fn main() -> Result<(), ColorError> {
    /// let black = HexColor::new("#000000")?;
    /// let theme = Theme::new(
    ///     black.clone(),
    ///     HexColor::new("#ffffff")?,
    ///     HexColor::new("#303030")?,
    ///     std::array::from_fn(|_| black.clone()),
    /// );
    /// assert_eq!(theme.cursor, None);
    /// assert_eq!(theme.ansi.len(), 16);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn new(
        background: HexColor,
        foreground: HexColor,
        selection: HexColor,
        ansi: [HexColor; 16],
    ) -> Self {
        Self {
            background,
            foreground,
            selection,
            cursor: None,
            ansi,
            colors: None,
            extra: Map::new(),
        }
    }

    /// Reads a theme from the text of a theme file.
    ///
    /// A key repeated in the file that this crate does not name keeps its last value.
    ///
    /// # Errors
    ///
    /// Returns a [`FileError`] of [`FileKind::Theme`] when the text is empty, is not JSON, has text
    /// after the theme, repeats a named field, lacks a required field, holds `ansi` with other than
    /// sixteen colours, holds a colour that is not a hex value (a colour name included), has an
    /// empty name in `colors`, or holds a value of the wrong type. Its detail is the text of the
    /// JSON reader's error.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{FileKind, Theme};
    ///
    /// let refused = Theme::parse("").err();
    /// assert_eq!(refused.as_ref().map(|error| error.file()), Some(FileKind::Theme));
    /// assert_eq!(
    ///     refused.map(|error| error.to_string()).as_deref(),
    ///     Some("invalid theme file: EOF while parsing a value at line 1 column 0"),
    /// );
    /// ```
    pub fn parse(text: &str) -> Result<Self, FileError> {
        serde_json::from_str(text).map_err(|source| FileError::from_serde(FileKind::Theme, &source))
    }

    /// The theme as the text of a theme file: pretty-printed JSON with two-space indentation and a
    /// final newline.
    ///
    /// Named fields come out in the order they are declared on [`Theme`], then the keys kept in
    /// `extra` in alphabetical order, so the result has the meaning of the text the theme was read
    /// from but not necessarily its bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{ColorError, HexColor, Theme};
    ///
    /// # fn main() -> Result<(), ColorError> {
    /// let white = HexColor::new("#ffffff")?;
    /// let theme = Theme::new(
    ///     white.clone(),
    ///     white.clone(),
    ///     white.clone(),
    ///     std::array::from_fn(|_| white.clone()),
    /// );
    /// let text = theme.to_json();
    /// assert!(text.starts_with("{\n  \"background\": \"#ffffff\",\n"));
    /// assert!(text.ends_with("}\n"));
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn to_json(&self) -> String {
        // Writing a theme cannot fail: every map key is a string and every number is finite, so the
        // fallback is never taken.
        let mut text = serde_json::to_string_pretty(self).unwrap_or_default();
        text.push('\n');
        text
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;
    use crate::files::FileKind;

    const ANSI: &str = r##"["#000000","#111111","#222222","#333333","#444444","#555555","#666666","#777777","#888888","#999999","#aaaaaa","#bbbbbb","#cccccc","#dddddd","#eeeeee","#ffffff"]"##;

    fn theme_text(extra: &str) -> String {
        format!(
            r##"{{"background":"#101010","foreground":"#f0f0f0","selection":"#303030","ansi":{ANSI}{extra}}}"##
        )
    }

    fn fail<T>(message: &str) -> Result<T, serde_json::Error> {
        Err(serde::de::Error::custom(message))
    }

    fn refusal(text: &str) -> Result<FileError, serde_json::Error> {
        match Theme::parse(text) {
            Ok(_) => fail("the theme was accepted"),
            Err(error) => Ok(error),
        }
    }

    fn written(theme: &Theme) -> Result<Value, serde_json::Error> {
        serde_json::from_str(&theme.to_json())
    }

    #[test]
    fn theme_parses_and_round_trips() -> Result<(), serde_json::Error> {
        let text = theme_text(
            r##","cursor":"#ff00ff","colors":{"ring":"#123456","canvas":"#000000"},"zz":[1,2.0]"##,
        );
        let theme = match Theme::parse(&text) {
            Ok(theme) => theme,
            Err(error) => return fail(&error.to_string()),
        };
        assert_eq!(theme.background.as_str(), "#101010");
        assert_eq!(theme.cursor.as_ref().map(HexColor::as_str), Some("#ff00ff"));
        assert_eq!(theme.ansi[15].as_str(), "#ffffff");
        assert_eq!(theme.colors.as_ref().map(BTreeMap::len), Some(2));
        assert!(theme.extra.contains_key("zz"));
        assert!(!theme.extra.contains_key("cursor"));
        assert!(!theme.extra.contains_key("colors"));

        let source: Value = serde_json::from_str(&text)?;
        assert_eq!(written(&theme)?, source);
        assert!(theme.to_json().contains("2.0"));
        Ok(())
    }

    #[test]
    fn theme_new_builds_the_required_fields_only() -> Result<(), serde_json::Error> {
        let color = match HexColor::new("#000000") {
            Ok(color) => color,
            Err(error) => return fail(&error.to_string()),
        };
        let theme = Theme::new(
            color.clone(),
            color.clone(),
            color.clone(),
            std::array::from_fn(|_| color.clone()),
        );
        assert_eq!(theme.cursor, None);
        assert_eq!(theme.colors, None);
        assert!(theme.extra.is_empty());
        let text = theme.to_json();
        let positions: Vec<Option<usize>> = ["background", "foreground", "selection", "ansi"]
            .iter()
            .map(|key| text.find(&format!("\"{key}\"")))
            .collect();
        assert!(positions.iter().all(Option::is_some), "{text}");
        assert!(positions.is_sorted(), "{text}");
        assert!(
            !text.contains("cursor") && !text.contains("colors"),
            "{text}"
        );
        Ok(())
    }

    #[test]
    fn theme_reads_null_cursor_and_colors_as_absent() -> Result<(), serde_json::Error> {
        let theme = match Theme::parse(&theme_text(r#","cursor":null,"colors":null"#)) {
            Ok(theme) => theme,
            Err(error) => return fail(&error.to_string()),
        };
        assert_eq!(theme.cursor, None);
        assert_eq!(theme.colors, None);
        let value = written(&theme)?;
        assert_eq!(value.get("cursor"), None);
        assert_eq!(value.get("colors"), None);
        Ok(())
    }

    #[test]
    fn theme_keeps_an_empty_colors_map() -> Result<(), serde_json::Error> {
        let theme = match Theme::parse(&theme_text(r#","colors":{}"#)) {
            Ok(theme) => theme,
            Err(error) => return fail(&error.to_string()),
        };
        assert_eq!(theme.colors, Some(BTreeMap::new()));
        assert_eq!(written(&theme)?.get("colors"), Some(&json!({})));
        Ok(())
    }

    #[test]
    fn theme_requires_sixteen_ansi_colours() -> Result<(), serde_json::Error> {
        let error = refusal(
            r##"{"background":"#101010","foreground":"#f0f0f0","selection":"#303030","ansi":["#000000"]}"##,
        )?;
        assert!(
            error.to_string().starts_with(
                "invalid theme file: invalid length 1, expected an array of length 16 at line 1 column"
            ),
            "{error}"
        );
        assert_eq!(error.file(), FileKind::Theme);
        Ok(())
    }

    #[test]
    fn theme_requires_background() -> Result<(), serde_json::Error> {
        let text = format!(r##"{{"foreground":"#f0f0f0","selection":"#303030","ansi":{ANSI}}}"##);
        let error = refusal(&text)?;
        assert!(
            error.detail().starts_with("missing field `background`"),
            "{error}"
        );
        Ok(())
    }

    #[test]
    fn theme_refuses_named_colours() -> Result<(), serde_json::Error> {
        let text = format!(
            r##"{{"background":"canvas","foreground":"#f0f0f0","selection":"#303030","ansi":{ANSI}}}"##
        );
        let error = refusal(&text)?;
        assert!(
            error.detail().contains("\"canvas\" is not a hex colour"),
            "{error}"
        );
        Ok(())
    }

    #[test]
    fn theme_refuses_an_empty_colour_name() -> Result<(), serde_json::Error> {
        let error = refusal(&theme_text(r##","colors":{"":"#000000"}"##))?;
        assert!(
            error.detail().starts_with("a colour name cannot be empty"),
            "{error}"
        );
        Ok(())
    }

    #[test]
    fn theme_refuses_a_repeated_named_field() -> Result<(), serde_json::Error> {
        let error = refusal(&theme_text(r##","background":"#000000""##))?;
        assert!(
            error.detail().starts_with("duplicate field `background`"),
            "{error}"
        );
        Ok(())
    }

    #[test]
    fn theme_to_json_is_pretty_with_newline() -> Result<(), serde_json::Error> {
        let theme = match Theme::parse(&theme_text("")) {
            Ok(theme) => theme,
            Err(error) => return fail(&error.to_string()),
        };
        let json = theme.to_json();
        assert!(json.ends_with("}\n"));
        assert!(json.starts_with("{\n  \"background\": \"#101010\",\n"));
        Ok(())
    }
}
