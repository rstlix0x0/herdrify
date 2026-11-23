//! How a layout looks, and the options only its root node takes.
//!
//! A layout file names a theme, may override colours and fonts, and tunes how its terminals are
//! drawn. Its root node may also set window options and record the layout it was cloned from. Each
//! is read from the layout file and written back with every word it does not name, so a layout that
//! uses a newer herdr-app feature is not damaged by being read.
//!
//! Responsibilities:
//!
//! - [`Look`], the theme, colour overrides, fonts and terminal drawing options of a layout
//! - [`ThemeChoice`], a theme by name, by light and dark appearance, or an unrecognised value
//! - [`RootOptions`], the window options and clone record of the root node
//! - [`Titlebar`] and [`ClonedFrom`], the title bar setting and the record of the source layout
//!
//! Non-responsibilities:
//!
//! - deciding whether a theme or colour name is defined, which herdr-app does
//! - holding keys it does not name, which the node that flattens these structs does
//! - drawing anything

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::files::{Color, ColorName, LayoutName, ModName, Number, Padding, ThemeName};

/// The theme, colours, fonts and terminal drawing options of a layout.
///
/// Every field is optional and an absent field is not written. This struct has no `extra`: it is
/// flattened into a node, which keeps every key neither names. The default sets nothing.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::Look;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let look: Look = serde_json::from_str(r#"{"theme":"graphite","font_size":13}"#)?;
/// assert!(look.theme.is_some());
/// assert_eq!(serde_json::to_string(&Look::default())?, "{}");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Look {
    /// The theme the layout uses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<ThemeChoice>,
    /// Colour overrides, by colour name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<BTreeMap<ColorName, Color>>,
    /// The terminal font name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    /// The interface font name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui_font: Option<String>,
    /// The font size, as written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<Number>,
    /// The font weight, as written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<Number>,
    /// The terminal opacity, as written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_opacity: Option<Number>,
    /// The terminal line height, as written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_line_height: Option<Number>,
    /// The terminal cell width, as written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_cell_width: Option<Number>,
    /// The terminal scroll speed, as written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_scroll_speed: Option<Number>,
    /// The terminal padding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_padding: Option<Padding>,
}

/// The theme a layout uses.
///
/// Variants are tried in the order declared. A string is [`ThemeChoice::Name`], an object with
/// `light` and `dark` theme names is [`ThemeChoice::ByAppearance`], and any other value, including
/// an object whose names are invalid, is kept whole as [`ThemeChoice::Other`] and written back as it
/// was read.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::{ThemeChoice, ThemeName};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let choice = ThemeChoice::by_appearance(ThemeName::new("paper")?, ThemeName::new("ink")?);
/// assert_eq!(serde_json::to_string(&choice)?, r#"{"light":"paper","dark":"ink"}"#);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ThemeChoice {
    /// One theme, by name.
    Name(ThemeName),
    /// One theme for a light appearance and one for a dark appearance.
    ByAppearance {
        /// The theme used for a light appearance.
        light: ThemeName,
        /// The theme used for a dark appearance.
        dark: ThemeName,
        /// Every other key of the object, as written.
        #[serde(flatten)]
        extra: Map<String, Value>,
    },
    /// A value that is neither of the above, kept whole.
    Other(Value),
}

impl ThemeChoice {
    /// A choice of `light` for a light appearance and `dark` for a dark appearance, with no other
    /// keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{ThemeChoice, ThemeName};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let choice = ThemeChoice::by_appearance(ThemeName::new("paper")?, ThemeName::new("ink")?);
    /// assert!(matches!(choice, ThemeChoice::ByAppearance { .. }));
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn by_appearance(light: ThemeName, dark: ThemeName) -> Self {
        Self::ByAppearance {
            light,
            dark,
            extra: Map::new(),
        }
    }
}

/// The options only the root node of a layout takes.
///
/// Every field is optional and an absent field is not written. This struct has no `extra`: it is
/// flattened into a node, which keeps every key neither names. The default sets nothing.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::{RootOptions, Titlebar};
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let options: RootOptions = serde_json::from_str(r#"{"titlebar":"none"}"#)?;
/// assert_eq!(options.titlebar, Some(Titlebar::None));
/// assert_eq!(serde_json::to_string(&RootOptions::default())?, "{}");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RootOptions {
    /// The title bar setting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub titlebar: Option<Titlebar>,
    /// The traffic-light pair, as written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traffic_lights: Option<[Number; 2]>,
    /// The window blur flag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_blur: Option<bool>,
    /// The preview value, as written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
    /// The mod named for the terminal menu.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_menu: Option<ModName>,
    /// The mod named for toasts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toast: Option<ModName>,
    /// The mods named as link handlers, in order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_handlers: Option<Vec<ModName>>,
    /// The layout this one was cloned from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloned_from: Option<ClonedFrom>,
}

/// The title bar setting.
///
/// A word this crate does not know is kept as [`Titlebar::Other`] and written back as it was read.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::Titlebar;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// assert_eq!(serde_json::from_str::<Titlebar>("\"none\"")?, Titlebar::None);
/// assert_eq!(
///     serde_json::from_str::<Titlebar>("\"custom\"")?,
///     Titlebar::Other("custom".to_owned()),
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Titlebar {
    /// The word `none`.
    None,
    /// A word this crate does not know, as written.
    #[serde(untagged)]
    Other(String),
}

/// The layout a layout was cloned from, and the version it was cloned at.
///
/// The version is kept as written, whatever its JSON type. Every key other than `name` and
/// `version` is kept in `extra`.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::{ClonedFrom, LayoutName};
/// use serde_json::json;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let cloned = ClonedFrom::new(LayoutName::new("main")?, json!("0.1.17"));
/// assert_eq!(serde_json::to_string(&cloned)?, r#"{"name":"main","version":"0.1.17"}"#);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClonedFrom {
    /// The name of the source layout.
    pub name: LayoutName,
    /// The version of the source layout, as written.
    pub version: Value,
    /// Every other key of the object, as written.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl ClonedFrom {
    /// A record of `name` at `version`, with no other keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{ClonedFrom, LayoutName};
    /// use serde_json::json;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let cloned = ClonedFrom::new(LayoutName::new("main")?, json!("0.1.17"));
    /// assert_eq!(cloned.version, json!("0.1.17"));
    /// assert!(cloned.extra.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn new(name: LayoutName, version: Value) -> Self {
        Self {
            name,
            version,
            extra: Map::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::{Value, json};

    use super::*;
    use crate::files::{LayoutName, ModName, Number, Padding, ThemeName};

    /// Reads `text` as `T` and checks that writing it back gives the same JSON.
    fn read_back<T>(text: &str) -> Result<T, serde_json::Error>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        let value: T = serde_json::from_str(text)?;
        let source: Value = serde_json::from_str(text)?;
        assert_eq!(serde_json::to_value(&value)?, source, "{text} changed");
        Ok(value)
    }

    fn fail<T>(message: &str) -> Result<T, serde_json::Error> {
        Err(serde::de::Error::custom(message))
    }

    #[test]
    fn theme_choice_parses_each_form() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            read_back::<ThemeChoice>("\"graphite\"")?,
            ThemeChoice::Name(ThemeName::new("graphite")?)
        );
        let ThemeChoice::ByAppearance { light, dark, extra } =
            read_back::<ThemeChoice>(r#"{"light":"paper","dark":"ink","x":1}"#)?
        else {
            return Ok(fail("not a by-appearance choice")?);
        };
        assert_eq!(light.as_str(), "paper");
        assert_eq!(dark.as_str(), "ink");
        assert_eq!(extra.get("x"), Some(&json!(1)));
        assert_eq!(read_back::<ThemeChoice>("7")?, ThemeChoice::Other(json!(7)));
        Ok(())
    }

    #[test]
    fn root_options_parse_link_handlers_and_cloned_from() -> Result<(), Box<dyn std::error::Error>>
    {
        let options = read_back::<RootOptions>(
            r#"{"link_handlers":["my-browser"],
                "cloned_from":{"name":"main","version":"0.1.17","x":1},
                "titlebar":"none"}"#,
        )?;
        let handlers: Vec<&str> = options
            .link_handlers
            .iter()
            .flatten()
            .map(ModName::as_str)
            .collect();
        assert_eq!(handlers, ["my-browser"]);
        let Some(cloned) = &options.cloned_from else {
            return Ok(fail("cloned_from is missing")?);
        };
        assert_eq!(cloned.name.as_str(), "main");
        assert_eq!(cloned.version, json!("0.1.17"));
        assert_eq!(cloned.extra.get("x"), Some(&json!(1)));
        assert_eq!(options.titlebar, Some(Titlebar::None));
        Ok(())
    }

    #[test]
    fn titlebar_keeps_unknown_words() -> Result<(), serde_json::Error> {
        assert_eq!(read_back::<Titlebar>("\"none\"")?, Titlebar::None);
        assert_eq!(
            read_back::<Titlebar>("\"custom\"")?,
            Titlebar::Other("custom".to_owned())
        );
        Ok(())
    }

    #[test]
    fn look_parses_every_field() -> Result<(), serde_json::Error> {
        let look = read_back::<Look>(
            r##"{"theme":"graphite","colors":{"canvas":"#101010","edge":"panel"},
                "font":"Mono","ui_font":"Sans","font_size":13,"font_weight":400,
                "terminal_opacity":0.9,"terminal_line_height":1.2,
                "terminal_cell_width":1.0,"terminal_scroll_speed":3,
                "terminal_padding":[4,8]}"##,
        )?;
        assert_eq!(look.colors.as_ref().map(BTreeMap::len), Some(2));
        assert_eq!(look.font.as_deref(), Some("Mono"));
        assert_eq!(look.ui_font.as_deref(), Some("Sans"));
        assert_eq!(look.font_size, Some(Number::from(13_u32)));
        assert_eq!(
            look.terminal_padding,
            Some(Padding::Pair([Number::from(4_u32), Number::from(8_u32)]))
        );
        Ok(())
    }

    #[test]
    fn root_options_parse_the_window_fields() -> Result<(), serde_json::Error> {
        let options = read_back::<RootOptions>(
            r#"{"traffic_lights":[12,14],"window_blur":true,"preview":"off",
                "terminal_menu":"menu","toast":"toasts"}"#,
        )?;
        assert_eq!(
            options.traffic_lights,
            Some([Number::from(12_u32), Number::from(14_u32)])
        );
        assert_eq!(options.window_blur, Some(true));
        assert_eq!(options.preview.as_deref(), Some("off"));
        assert_eq!(
            options.terminal_menu.as_ref().map(ModName::as_str),
            Some("menu")
        );
        assert_eq!(options.toast.as_ref().map(ModName::as_str), Some("toasts"));
        Ok(())
    }

    #[test]
    fn defaults_write_nothing() -> Result<(), serde_json::Error> {
        assert_eq!(serde_json::to_string(&Look::default())?, "{}");
        assert_eq!(serde_json::to_string(&RootOptions::default())?, "{}");
        Ok(())
    }

    #[test]
    fn cloned_from_new_writes_name_and_version() -> Result<(), Box<dyn std::error::Error>> {
        let cloned = ClonedFrom::new(LayoutName::new("main")?, json!("0.1.17"));
        assert_eq!(
            serde_json::to_value(&cloned)?,
            json!({"name": "main", "version": "0.1.17"})
        );
        Ok(())
    }
}
