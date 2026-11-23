//! A block that floats over its node, and where it sits.
//!
//! A floating block shows content over a node, anchored to one of the node's edges and aligned along
//! it, and may be a drawer that shows only a strip of itself until it is hovered. It is read from
//! the layout file and written back with every key and word it does not name, so a layout that uses
//! a newer herdr-app feature is not damaged by being read.
//!
//! Responsibilities:
//!
//! - [`FloatingBlock`], the content, edge, alignment and peek of one floating block
//! - [`Edge`] and [`Align`], the words that place a block, keeping any word they do not know
//!
//! Non-responsibilities:
//!
//! - deciding where a block ends up once a person has dragged it, which herdr-app does
//! - drawing anything

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::files::{Content, NodeId, Number};

/// A block that floats over its node.
///
/// Every key other than the named ones is kept in `extra`. An absent field is not written.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::{FloatingBlock, Number};
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let block: FloatingBlock = serde_json::from_str(
///     r#"{"id":"board","content":{"mod":"board"},"edge":"left","align":"center","peek":52}"#,
/// )?;
/// assert_eq!(block.peek, Some(Number::from(52_u32)));
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FloatingBlock {
    /// The block's id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<NodeId>,
    /// What the block shows.
    pub content: Content,
    /// The edge of the node the block sits on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge: Option<Edge>,
    /// Where along the edge the block sits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,
    /// The peek width, as written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peek: Option<Number>,
    /// Every other key of the object, as written.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl FloatingBlock {
    /// A floating block showing `content`, with no id, edge, alignment, peek or other keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{Content, FloatingBlock, ModName};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let block = FloatingBlock::new(Content::module(ModName::new("board")?));
    /// assert_eq!(block.edge, None);
    /// assert_eq!(serde_json::to_string(&block)?, r#"{"content":{"mod":"board"}}"#);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn new(content: Content) -> Self {
        Self {
            id: None,
            content,
            edge: None,
            align: None,
            peek: None,
            extra: Map::new(),
        }
    }
}

/// The edge of its node a floating block sits on.
///
/// A word this crate does not know is kept as [`Edge::Other`] and written back as it was read.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::Edge;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// assert_eq!(serde_json::from_str::<Edge>("\"left\"")?, Edge::Left);
/// assert_eq!(
///     serde_json::from_str::<Edge>("\"diagonal\"")?,
///     Edge::Other("diagonal".to_owned()),
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Edge {
    /// The top edge.
    Top,
    /// The right edge.
    Right,
    /// The bottom edge.
    Bottom,
    /// The left edge.
    Left,
    /// A word this crate does not know, as written.
    #[serde(untagged)]
    Other(String),
}

/// Where along its edge a floating block sits.
///
/// A word this crate does not know is kept as [`Align::Other`] and written back as it was read.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::Align;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// assert_eq!(serde_json::from_str::<Align>("\"center\"")?, Align::Center);
/// assert_eq!(
///     serde_json::from_str::<Align>("\"middle\"")?,
///     Align::Other("middle".to_owned()),
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Align {
    /// The start of the edge.
    Start,
    /// The middle of the edge.
    Center,
    /// The end of the edge.
    End,
    /// A word this crate does not know, as written.
    #[serde(untagged)]
    Other(String),
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::{Value, json};

    use super::*;
    use crate::files::ModName;

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
    fn floating_block_parses_the_documented_example() -> Result<(), serde_json::Error> {
        let block = read_back::<FloatingBlock>(
            r#"{ "id": "board", "content": { "mod": "board" }, "edge": "left",
                 "align": "center", "peek": 52 }"#,
        )?;
        assert_eq!(block.id.as_ref().map(NodeId::as_str), Some("board"));
        let Content::Mod { module, .. } = &block.content else {
            return fail("content is not a mod");
        };
        assert_eq!(module.as_str(), "board");
        assert_eq!(block.edge, Some(Edge::Left));
        assert_eq!(block.align, Some(Align::Center));
        assert_eq!(block.peek, Some(Number::from(52_u32)));
        assert!(block.extra.is_empty());
        Ok(())
    }

    #[test]
    fn floating_block_keeps_extra() -> Result<(), serde_json::Error> {
        let block = read_back::<FloatingBlock>(r#"{"content":{"mod":"board"},"x":1}"#)?;
        assert_eq!(block.extra.get("x"), Some(&json!(1)));
        assert_eq!(block.id, None);
        assert_eq!(block.edge, None);
        Ok(())
    }

    #[test]
    fn edge_and_align_keep_unknown_words() -> Result<(), serde_json::Error> {
        for (word, edge) in [
            ("top", Edge::Top),
            ("right", Edge::Right),
            ("bottom", Edge::Bottom),
            ("left", Edge::Left),
        ] {
            assert_eq!(read_back::<Edge>(&format!("\"{word}\""))?, edge);
        }
        for (word, align) in [
            ("start", Align::Start),
            ("center", Align::Center),
            ("end", Align::End),
        ] {
            assert_eq!(read_back::<Align>(&format!("\"{word}\""))?, align);
        }
        assert_eq!(
            read_back::<Align>("\"middle\"")?,
            Align::Other("middle".to_owned())
        );
        assert_eq!(
            read_back::<Edge>("\"diagonal\"")?,
            Edge::Other("diagonal".to_owned())
        );
        Ok(())
    }

    #[test]
    fn floating_block_new_writes_only_its_content() -> Result<(), Box<dyn std::error::Error>> {
        let block = FloatingBlock::new(Content::module(ModName::new("board")?));
        assert_eq!(
            serde_json::to_value(&block)?,
            json!({"content": {"mod": "board"}})
        );
        Ok(())
    }

    #[test]
    fn floating_block_requires_content() {
        assert!(serde_json::from_str::<FloatingBlock>(r#"{"edge":"left"}"#).is_err());
    }
}
