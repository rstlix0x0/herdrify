//! What a layout block shows, and the frame a pane draws around a tab.
//!
//! A block shows a mod, one terminal, the shown tab or one agent. A tab's panes can be framed with a
//! gap, a radius, a background, borders and a header. Each is read from the layout file and written
//! back with every key and word it does not name, so a layout that uses a newer herdr-app feature is
//! not damaged by being read.
//!
//! Responsibilities:
//!
//! - [`Content`], a mod, a terminal, a tab, an agent or an unrecognised object, tried in that order
//! - [`PaneFrame`] and [`PaneHeader`], the frame of a tab's panes and the header it may carry
//! - [`HeaderPosition`] and [`Reveal`], where a header sits and when it shows, keeping any word
//!   they do not know
//!
//! Non-responsibilities:
//!
//! - deciding whether a mod folder, terminal address or agent exists, which herdr-app does
//! - drawing anything

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::files::{Line, ModName, Number, Paint, TerminalAddress};

/// What a block shows.
///
/// Variants are tried in the order declared. An object that fits none of the first four, including
/// one whose known key holds an invalid value such as an empty mod name, is kept whole as
/// [`Content::Other`] and written back as it was read. A value that is not an object is refused.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::Content;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let content: Content = serde_json::from_str(r#"{"agent":"focused"}"#)?;
/// assert!(matches!(content, Content::Agent { .. }));
/// let content: Content = serde_json::from_str(r#"{"board":"x"}"#)?;
/// assert!(matches!(content, Content::Other(_)));
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    /// A mod, by folder name, with optional props.
    Mod {
        /// The mod folder name, written as `mod`.
        #[serde(rename = "mod")]
        module: ModName,
        /// The props handed to the mod.
        #[serde(skip_serializing_if = "Option::is_none")]
        props: Option<Map<String, Value>>,
        /// Every other key of the object, as written.
        #[serde(flatten)]
        extra: Map<String, Value>,
    },
    /// One terminal, by address.
    Terminal {
        /// The terminal address.
        terminal: TerminalAddress,
        /// Every other key of the object, as written.
        #[serde(flatten)]
        extra: Map<String, Value>,
    },
    /// A tab, with an optional frame for its panes.
    Tab {
        /// Which tab, as written.
        tab: String,
        /// The frame of the tab's panes, boxed to keep the enum small.
        #[serde(skip_serializing_if = "Option::is_none")]
        pane: Option<Box<PaneFrame>>,
        /// Every other key of the object, as written.
        #[serde(flatten)]
        extra: Map<String, Value>,
    },
    /// One agent's terminal.
    Agent {
        /// Which agent, as written.
        agent: String,
        /// Every other key of the object, as written.
        #[serde(flatten)]
        extra: Map<String, Value>,
    },
    /// An object that is none of the above, kept whole.
    Other(Map<String, Value>),
}

impl Content {
    /// Content that shows the mod `module`, with no props and no other keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{Content, ModName};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let content = Content::module(ModName::new("files")?);
    /// assert_eq!(serde_json::to_string(&content)?, r#"{"mod":"files"}"#);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn module(module: ModName) -> Self {
        Self::Mod {
            module,
            props: None,
            extra: Map::new(),
        }
    }

    /// Content that shows the terminal `terminal`, with no other keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{Content, TerminalAddress};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let content = Content::terminal(TerminalAddress::new("t7")?);
    /// assert_eq!(serde_json::to_string(&content)?, r#"{"terminal":"t7"}"#);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn terminal(terminal: TerminalAddress) -> Self {
        Self::Terminal {
            terminal,
            extra: Map::new(),
        }
    }
}

/// The frame drawn around a tab's panes.
///
/// Every key other than the named ones is kept in `extra`. An absent field is not written. The
/// default is an empty frame.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::PaneFrame;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let frame = PaneFrame::default();
/// assert_eq!(frame.header, None);
/// assert_eq!(serde_json::to_string(&frame)?, "{}");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PaneFrame {
    /// The gap setting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap: Option<Number>,
    /// The radius setting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius: Option<Number>,
    /// The background fill.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<Paint>,
    /// The border stroke.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<Line>,
    /// The focus stroke.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<Line>,
    /// The header of a pane.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<PaneHeader>,
    /// The `lone` flag, as written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lone: Option<bool>,
    /// Every other key of the object, as written.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// The header of a framed pane: its content, height, position and reveal.
///
/// The content is boxed because a header holds [`Content`] and [`Content::Tab`] can hold a frame
/// that holds a header. Every key other than the named ones is kept in `extra`. An absent field is
/// not written.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::{Content, ModName, PaneHeader};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let header = PaneHeader::new(Content::module(ModName::new("pane")?));
/// assert_eq!(serde_json::to_string(&header)?, r#"{"content":{"mod":"pane"}}"#);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PaneHeader {
    /// What the header shows.
    pub content: Box<Content>,
    /// The header height.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<Number>,
    /// The header position.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<HeaderPosition>,
    /// The header reveal setting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reveal: Option<Reveal>,
    /// Every other key of the object, as written.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl PaneHeader {
    /// A header showing `content`, with no height, position, reveal or other keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{Content, ModName, PaneHeader};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let header = PaneHeader::new(Content::module(ModName::new("pane")?));
    /// assert_eq!(header.height, None);
    /// assert_eq!(header.position, None);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn new(content: Content) -> Self {
        Self {
            content: Box::new(content),
            height: None,
            position: None,
            reveal: None,
            extra: Map::new(),
        }
    }
}

/// The position of a pane header.
///
/// A word this crate does not know is kept as [`HeaderPosition::Other`] and written back as it was
/// read.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::HeaderPosition;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// assert_eq!(serde_json::from_str::<HeaderPosition>("\"top\"")?, HeaderPosition::Top);
/// assert_eq!(
///     serde_json::from_str::<HeaderPosition>("\"left\"")?,
///     HeaderPosition::Other("left".to_owned()),
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HeaderPosition {
    /// The header is at the top.
    Top,
    /// The header is at the bottom.
    Bottom,
    /// A word this crate does not know, as written.
    #[serde(untagged)]
    Other(String),
}

/// How a pane header is revealed.
///
/// A word this crate does not know is kept as [`Reveal::Other`] and written back as it was read.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::Reveal;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// assert_eq!(serde_json::from_str::<Reveal>("\"hover\"")?, Reveal::Hover);
/// assert_eq!(
///     serde_json::from_str::<Reveal>("\"never\"")?,
///     Reveal::Other("never".to_owned()),
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Reveal {
    /// The header is always revealed.
    Always,
    /// The header is revealed on hover.
    Hover,
    /// A word this crate does not know, as written.
    #[serde(untagged)]
    Other(String),
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::{Value, json};

    use super::*;
    use crate::files::{ModName, Number, TerminalAddress};

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
    fn content_parses_each_variant() -> Result<(), serde_json::Error> {
        let Content::Mod { module, props, .. } =
            read_back::<Content>(r#"{"mod":"files","props":{"role":"x"}}"#)?
        else {
            return fail("not a mod");
        };
        assert_eq!(module.as_str(), "files");
        assert_eq!(
            props.as_ref().and_then(|p| p.get("role")),
            Some(&json!("x"))
        );

        let Content::Terminal { terminal, .. } = read_back::<Content>(r#"{"terminal":"t7"}"#)?
        else {
            return fail("not a terminal");
        };
        assert_eq!(terminal.as_str(), "t7");

        let Content::Tab { tab, pane, .. } =
            read_back::<Content>(r#"{"tab":"active","pane":{"gap":6,"lone":false,"zz":1}}"#)?
        else {
            return fail("not a tab");
        };
        assert_eq!(tab, "active");
        let Some(frame) = pane else {
            return fail("no pane frame");
        };
        assert_eq!(frame.gap, Some(Number::from(6_u32)));
        assert_eq!(frame.lone, Some(false));
        assert_eq!(frame.extra.get("zz"), Some(&json!(1)));

        let Content::Agent { agent, .. } = read_back::<Content>(r#"{"agent":"focused"}"#)? else {
            return fail("not an agent");
        };
        assert_eq!(agent, "focused");
        Ok(())
    }

    #[test]
    fn content_variants_keep_extra() -> Result<(), serde_json::Error> {
        let Content::Mod { extra, .. } = read_back::<Content>(r#"{"mod":"files","x":1}"#)? else {
            return fail("not a mod");
        };
        assert_eq!(extra.get("x"), Some(&json!(1)));
        let Content::Terminal { extra, .. } = read_back::<Content>(r#"{"terminal":"t7","x":1}"#)?
        else {
            return fail("not a terminal");
        };
        assert_eq!(extra.get("x"), Some(&json!(1)));
        let Content::Tab { extra, .. } = read_back::<Content>(r#"{"tab":"active","x":1}"#)? else {
            return fail("not a tab");
        };
        assert_eq!(extra.get("x"), Some(&json!(1)));
        let Content::Agent { extra, .. } = read_back::<Content>(r#"{"agent":"focused","x":1}"#)?
        else {
            return fail("not an agent");
        };
        assert_eq!(extra.get("x"), Some(&json!(1)));
        Ok(())
    }

    #[test]
    fn content_falls_back_to_other() -> Result<(), serde_json::Error> {
        assert!(matches!(
            read_back::<Content>(r#"{"mod":""}"#)?,
            Content::Other(_)
        ));
        assert!(matches!(
            read_back::<Content>(r#"{"board":"x"}"#)?,
            Content::Other(_)
        ));
        Ok(())
    }

    #[test]
    fn pane_frame_parses_the_documented_example() -> Result<(), serde_json::Error> {
        let frame = read_back::<PaneFrame>(
            r#"{ "gap": 6, "radius": 6, "background": "surface",
                 "border": { "width": 1, "color": "border" },
                 "focus": { "width": 1, "color": "accent" },
                 "header": { "content": { "mod": "pane" }, "height": 30,
                             "position": "top", "reveal": "always" },
                 "lone": false }"#,
        )?;
        let Some(header) = frame.header else {
            return fail("no header");
        };
        let Content::Mod { module, .. } = *header.content else {
            return fail("header content is not a mod");
        };
        assert_eq!(module.as_str(), "pane");
        assert_eq!(header.height, Some(Number::from(30_u32)));
        assert_eq!(header.position, Some(HeaderPosition::Top));
        assert_eq!(header.reveal, Some(Reveal::Always));
        Ok(())
    }

    #[test]
    fn pane_header_keeps_extra() -> Result<(), serde_json::Error> {
        let header = read_back::<PaneHeader>(r#"{"content":{"mod":"pane"},"x":1}"#)?;
        assert_eq!(header.extra.get("x"), Some(&json!(1)));
        Ok(())
    }

    #[test]
    fn header_words_keep_unknown_words() -> Result<(), serde_json::Error> {
        assert_eq!(
            read_back::<HeaderPosition>("\"bottom\"")?,
            HeaderPosition::Bottom
        );
        assert_eq!(
            read_back::<HeaderPosition>("\"left\"")?,
            HeaderPosition::Other("left".to_owned())
        );
        assert_eq!(read_back::<Reveal>("\"hover\"")?, Reveal::Hover);
        assert_eq!(
            read_back::<Reveal>("\"never\"")?,
            Reveal::Other("never".to_owned())
        );
        Ok(())
    }

    #[test]
    fn module_constructor_writes_no_props() -> Result<(), Box<dyn std::error::Error>> {
        let content = Content::module(ModName::new("files")?);
        assert_eq!(serde_json::to_string(&content)?, r#"{"mod":"files"}"#);
        Ok(())
    }

    #[test]
    fn terminal_constructor_writes_the_address() -> Result<(), Box<dyn std::error::Error>> {
        let content = Content::terminal(TerminalAddress::new("t7")?);
        assert_eq!(serde_json::to_string(&content)?, r#"{"terminal":"t7"}"#);
        Ok(())
    }

    #[test]
    fn empty_frame_writes_nothing() -> Result<(), serde_json::Error> {
        assert_eq!(serde_json::to_string(&PaneFrame::default())?, "{}");
        Ok(())
    }
}
