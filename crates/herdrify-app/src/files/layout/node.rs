//! The layout file itself and the node every block, row, column and stack of it is written as.
//!
//! A layout file is one root node, and a node may hold child nodes. A node is a flat object with an
//! explicit `type`, and it keeps every field written on it whatever its type, so a layout is
//! written back as it was read, in meaning if not in byte order.
//!
//! Responsibilities:
//!
//! - [`Layout`], the file: its root [`Node`], parsing from text and writing back as JSON
//! - [`Node`] with [`NodeKind`], one node and the kind its `type` names
//! - [`Size`], [`Shadow`] and [`FocusStyle`], the values a node takes that are not blocks of content
//!   or fills
//! - [`Padding`], a padding value, as one number or as a pair of numbers
//!
//! Non-responsibilities:
//!
//! - deciding which fields suit which kind of node, which herdr-app does
//! - reading a layout from disk or locating it, which a layer above does
//! - drawing anything

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::files::{
    Content, FileError, FileKind, FloatingBlock, Line, Look, NodeId, Number, Paint, RootOptions,
};

/// A layout file: one root [`Node`].
///
/// It reads and writes as its root node and nothing more.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::{Layout, NodeKind};
///
/// # fn main() -> Result<(), herdrify_app::files::FileError> {
/// let layout = Layout::parse(r#"{"type":"row","children":[{"type":"block"}]}"#)?;
/// assert_eq!(layout.root().kind, NodeKind::Row);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Layout(Node);

impl Layout {
    /// A layout whose root is `root`.
    #[must_use]
    pub const fn new(root: Node) -> Self {
        Self(root)
    }

    /// Reads a layout from the text of a layout file.
    ///
    /// A key repeated on a node that this crate does not name keeps its last value. Fields that do
    /// not suit a node's kind are kept as written.
    ///
    /// # Errors
    ///
    /// Returns a [`FileError`] of [`FileKind::Layout`] when the text is empty, is not JSON, has text
    /// after the layout, repeats a named field, lacks the root's `type`, or holds a value of the
    /// wrong type. Its detail is the text of the JSON reader's error.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{FileKind, Layout};
    ///
    /// # fn main() -> Result<(), herdrify_app::files::FileError> {
    /// let layout = Layout::parse(r#"{"type":"column"}"#)?;
    /// assert_eq!(layout.to_json(), "{\n  \"type\": \"column\"\n}\n");
    ///
    /// let refused = Layout::parse("").unwrap_err();
    /// assert_eq!(refused.file(), FileKind::Layout);
    /// assert_eq!(
    ///     refused.to_string(),
    ///     "invalid layout file: EOF while parsing a value at line 1 column 0",
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn parse(text: &str) -> Result<Self, FileError> {
        serde_json::from_str(text)
            .map_err(|source| FileError::from_serde(FileKind::Layout, &source))
    }

    /// The layout as the text of a layout file: pretty-printed JSON with two-space indentation and
    /// a final newline.
    ///
    /// Named fields come out in the order they are declared on [`Node`], then the keys kept in
    /// `extra` in alphabetical order, so the result has the meaning of the text the layout was read
    /// from but not necessarily its bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{Layout, Node};
    ///
    /// let layout = Layout::new(Node::row(Vec::new()));
    /// assert_eq!(
    ///     layout.to_json(),
    ///     "{\n  \"type\": \"row\",\n  \"children\": []\n}\n",
    /// );
    /// ```
    #[must_use]
    pub fn to_json(&self) -> String {
        // Writing a node cannot fail: every map key is a string and every number is finite, so the
        // fallback is never taken.
        let mut text = serde_json::to_string_pretty(self).unwrap_or_default();
        text.push('\n');
        text
    }

    /// The root node.
    #[must_use]
    pub const fn root(&self) -> &Node {
        &self.0
    }

    /// The root node, for changing.
    #[must_use]
    pub const fn root_mut(&mut self) -> &mut Node {
        &mut self.0
    }
}

/// One node of a layout: a block, a row, a column or a stack.
///
/// Every field but `kind` is optional and an absent field is not written. An absent `children` and
/// an empty one are different and are written back as they were read. A node keeps the fields it
/// has whatever its kind, so a block with `children` is not changed by being read.
///
/// The [`Look`] and [`RootOptions`] fields are flattened: their keys sit on the node itself. Every
/// key none of the named fields takes is kept in `extra`.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::{Node, NodeKind};
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let node: Node = serde_json::from_str(r#"{"type":"row","gap":8,"children":[]}"#)?;
/// assert_eq!(node.kind, NodeKind::Row);
/// assert_eq!(node.children, Some(Vec::new()));
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Node {
    /// The node's id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<NodeId>,
    /// What kind of node this is, written as `type`.
    #[serde(rename = "type")]
    pub kind: NodeKind,
    /// What a block shows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Content>,
    /// The nodes inside a row, a column or a stack. An absent list and an empty one are different.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Self>>,
    /// The child of a stack that is showing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<NodeId>,
    /// A stack's header, as written: a word or an object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<Value>,
    /// The node's size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<Size>,
    /// The smallest size, as written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<Number>,
    /// The largest size, as written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<Number>,
    /// The corner radius, as written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius: Option<Number>,
    /// The gap between children, as written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap: Option<Number>,
    /// The padding inside the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding: Option<Padding>,
    /// The node's fill.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<Paint>,
    /// The node's border.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<Line>,
    /// The divider drawn between children.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub divider: Option<Line>,
    /// The node's shadow.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadow: Option<Shadow>,
    /// How the node looks when it has focus.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<FocusStyle>,
    /// Whether the node is hidden.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    /// Whether the node can be resized.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resizable: Option<bool>,
    /// Whether the node can be moved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub movable: Option<bool>,
    /// Whether dragging the node drags the window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_drag: Option<bool>,
    /// The blocks that float over the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub floating: Option<Vec<FloatingBlock>>,
    /// The theme, colours, fonts and terminal options, flattened onto the node.
    #[serde(flatten)]
    pub look: Look,
    /// The options only the root node takes, flattened onto the node.
    #[serde(flatten)]
    pub root: RootOptions,
    /// Every other key of the node, as written.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Node {
    /// A node of `kind` with nothing else set.
    fn of(kind: NodeKind) -> Self {
        Self {
            id: None,
            kind,
            content: None,
            children: None,
            active: None,
            header: None,
            size: None,
            min: None,
            max: None,
            radius: None,
            gap: None,
            padding: None,
            background: None,
            border: None,
            divider: None,
            shadow: None,
            focus: None,
            hidden: None,
            resizable: None,
            movable: None,
            window_drag: None,
            floating: None,
            look: Look::default(),
            root: RootOptions::default(),
            extra: Map::new(),
        }
    }

    /// A block showing `content`, with nothing else set.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{Content, ModName, Node};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let block = Node::block(Content::module(ModName::new("board")?));
    /// assert_eq!(
    ///     serde_json::to_string(&block)?,
    ///     r#"{"type":"block","content":{"mod":"board"}}"#,
    /// );
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn block(content: Content) -> Self {
        Self {
            content: Some(content),
            ..Self::of(NodeKind::Block)
        }
    }

    /// A row of `children`, with nothing else set.
    #[must_use]
    pub fn row(children: Vec<Self>) -> Self {
        Self {
            children: Some(children),
            ..Self::of(NodeKind::Row)
        }
    }

    /// A column of `children`, with nothing else set.
    #[must_use]
    pub fn column(children: Vec<Self>) -> Self {
        Self {
            children: Some(children),
            ..Self::of(NodeKind::Column)
        }
    }

    /// A stack of `children` that shows the child `active`, with nothing else set.
    #[must_use]
    pub fn stack(children: Vec<Self>, active: NodeId) -> Self {
        Self {
            children: Some(children),
            active: Some(active),
            ..Self::of(NodeKind::Stack)
        }
    }
}

/// What a node's `type` names.
///
/// A word this crate does not know is kept as [`NodeKind::Other`] and written back as it was read.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::NodeKind;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// assert_eq!(serde_json::from_str::<NodeKind>("\"stack\"")?, NodeKind::Stack);
/// assert_eq!(
///     serde_json::from_str::<NodeKind>("\"grid\"")?,
///     NodeKind::Other("grid".to_owned()),
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    /// The word `block`.
    Block,
    /// The word `row`.
    Row,
    /// The word `column`.
    Column,
    /// The word `stack`.
    Stack,
    /// A word this crate does not know, as written.
    #[serde(untagged)]
    Other(String),
}

/// A node's size: a pixel size, a weight, or an object this crate does not recognise.
///
/// Variants are tried in the order declared. An object with a valid `px` is [`Size::Px`], one with
/// a valid `weight` is [`Size::Weight`], and any other object, including one whose `px` or `weight`
/// is invalid, is kept whole as [`Size::Other`] and written back as it was read. Keys the matching
/// variant does not name are kept in its `extra`.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::{Number, Size};
///
/// # fn main() -> Result<(), serde_json::Error> {
/// assert_eq!(
///     serde_json::from_str::<Size>(r#"{"px":240}"#)?,
///     Size::px(Number::from(240_u32)),
/// );
/// assert!(matches!(
///     serde_json::from_str::<Size>(r#"{"px":"wide"}"#)?,
///     Size::Other(_),
/// ));
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Size {
    /// A size in pixels.
    Px {
        /// The size in pixels, as written.
        px: Number,
        /// Every other key of the object, as written.
        #[serde(flatten)]
        extra: Map<String, Value>,
    },
    /// A share of the space left over, as a weight.
    Weight {
        /// The weight, as written.
        weight: Number,
        /// Every other key of the object, as written.
        #[serde(flatten)]
        extra: Map<String, Value>,
    },
    /// An object that is neither of the above, kept whole.
    Other(Map<String, Value>),
}

impl Size {
    /// A size of `px` pixels, with no other keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{Number, Size};
    ///
    /// # fn main() -> Result<(), serde_json::Error> {
    /// assert_eq!(serde_json::to_string(&Size::px(Number::from(240_u32)))?, r#"{"px":240}"#);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn px(px: Number) -> Self {
        Self::Px {
            px,
            extra: Map::new(),
        }
    }

    /// A size of `weight`, with no other keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{Number, Size};
    ///
    /// # fn main() -> Result<(), serde_json::Error> {
    /// assert_eq!(serde_json::to_string(&Size::weight(Number::from(2_u32)))?, r#"{"weight":2}"#);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn weight(weight: Number) -> Self {
        Self::Weight {
            weight,
            extra: Map::new(),
        }
    }
}

/// The size of a node's shadow.
///
/// A word this crate does not know is kept as [`Shadow::Other`] and written back as it was read.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::Shadow;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// assert_eq!(serde_json::from_str::<Shadow>("\"md\"")?, Shadow::Md);
/// assert_eq!(
///     serde_json::from_str::<Shadow>("\"xl\"")?,
///     Shadow::Other("xl".to_owned()),
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Shadow {
    /// The word `sm`.
    Sm,
    /// The word `md`.
    Md,
    /// The word `lg`.
    Lg,
    /// A word this crate does not know, as written.
    #[serde(untagged)]
    Other(String),
}

/// How a node looks when it has focus.
///
/// Every key other than `border` is kept in `extra`. An absent field is not written, and the
/// default sets nothing.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::FocusStyle;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let focus: FocusStyle = serde_json::from_str(r#"{"border":{"width":1},"glow":true}"#)?;
/// assert!(focus.border.is_some());
/// assert_eq!(serde_json::to_string(&FocusStyle::default())?, "{}");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FocusStyle {
    /// The border drawn around the focused node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<Line>,
    /// Every other key of the object, as written.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A padding value: one number or two.
///
/// A single number is [`Padding::Even`] and a two-number array is [`Padding::Pair`]. Either is
/// written back in the form it was read.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::{Number, Padding};
///
/// # fn main() -> Result<(), serde_json::Error> {
/// assert_eq!(
///     serde_json::from_str::<Padding>("6")?,
///     Padding::Even(Number::from(6_u32)),
/// );
/// assert_eq!(
///     serde_json::from_str::<Padding>("[4,8]")?,
///     Padding::Pair([Number::from(4_u32), Number::from(8_u32)]),
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Padding {
    /// One number, as written.
    Even(Number),
    /// Two numbers, as written.
    Pair([Number; 2]),
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::{Value, json};

    use super::*;
    use crate::files::{Content, FileError, FileKind, ModName, NodeId, Number, Titlebar};

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
    fn padding_parses_one_number_and_a_pair() -> Result<(), serde_json::Error> {
        assert_eq!(
            read_back::<Padding>("6")?,
            Padding::Even(Number::from(6_u32))
        );
        assert_eq!(
            read_back::<Padding>("[4,8]")?,
            Padding::Pair([Number::from(4_u32), Number::from(8_u32)])
        );
        Ok(())
    }

    #[test]
    fn node_keeps_children_absent_and_empty_apart() -> Result<(), serde_json::Error> {
        let absent = read_back::<Node>(r#"{"type":"row"}"#)?;
        assert_eq!(absent.children, None);
        assert_eq!(serde_json::to_string(&absent)?, r#"{"type":"row"}"#);
        let empty = read_back::<Node>(r#"{"type":"row","children":[]}"#)?;
        assert_eq!(empty.children, Some(Vec::new()));
        assert_eq!(
            serde_json::to_string(&empty)?,
            r#"{"type":"row","children":[]}"#
        );
        Ok(())
    }

    #[test]
    fn node_keeps_unknown_type_and_keys() -> Result<(), serde_json::Error> {
        let text = r#"{"type":"grid","link_handlers":["my-browser"],"zz":[1,2.0]}"#;
        let node = read_back::<Node>(text)?;
        assert_eq!(node.kind, NodeKind::Other("grid".to_owned()));
        assert_eq!(
            node.root.link_handlers.as_ref().map(Vec::len),
            Some(1),
            "link_handlers belongs to the root options"
        );
        assert_eq!(node.extra.get("zz"), Some(&json!([1, 2.0])));
        assert_eq!(node.extra.len(), 1);
        assert_eq!(serde_json::to_string(&node)?, text);
        Ok(())
    }

    #[test]
    fn node_keeps_fields_that_do_not_fit_its_kind() -> Result<(), serde_json::Error> {
        let node = read_back::<Node>(r#"{"type":"block","children":[]}"#)?;
        assert_eq!(node.kind, NodeKind::Block);
        assert_eq!(node.children, Some(Vec::new()));
        Ok(())
    }

    #[test]
    fn node_flattened_structs_take_their_keys() -> Result<(), serde_json::Error> {
        let node = read_back::<Node>(
            r#"{"type":"row","theme":"graphite","font_size":13,"titlebar":"none"}"#,
        )?;
        assert!(node.extra.is_empty());
        assert!(node.look.theme.is_some());
        assert_eq!(node.look.font_size, Some(Number::from(13_u32)));
        assert_eq!(node.root.titlebar, Some(Titlebar::None));
        Ok(())
    }

    #[test]
    fn node_parses_every_named_field() -> Result<(), serde_json::Error> {
        let node = read_back::<Node>(
            r#"{"id":"main","type":"stack","children":[{"type":"block"}],"active":"a",
                "header":"tabs","size":{"px":240},"min":10,"max":900,"radius":6,"gap":4,
                "padding":[2,4],"background":"panel","border":{"width":1},
                "divider":{"width":2},"shadow":"md","focus":{"border":{"width":1}},
                "hidden":false,"resizable":true,"movable":true,"window_drag":false,
                "floating":[{"content":{"mod":"board"}}]}"#,
        )?;
        assert_eq!(node.kind, NodeKind::Stack);
        assert_eq!(node.id.as_ref().map(NodeId::as_str), Some("main"));
        assert_eq!(node.active.as_ref().map(NodeId::as_str), Some("a"));
        assert_eq!(node.header, Some(json!("tabs")));
        assert_eq!(node.size, Some(Size::px(Number::from(240_u32))));
        assert_eq!(node.min, Some(Number::from(10_u32)));
        assert_eq!(node.max, Some(Number::from(900_u32)));
        assert_eq!(node.radius, Some(Number::from(6_u32)));
        assert_eq!(node.gap, Some(Number::from(4_u32)));
        assert_eq!(
            node.padding,
            Some(Padding::Pair([Number::from(2_u32), Number::from(4_u32)]))
        );
        assert!(node.background.is_some());
        assert!(node.border.is_some());
        assert!(node.divider.is_some());
        assert_eq!(node.shadow, Some(Shadow::Md));
        assert!(node.focus.is_some());
        assert_eq!(node.hidden, Some(false));
        assert_eq!(node.resizable, Some(true));
        assert_eq!(node.movable, Some(true));
        assert_eq!(node.window_drag, Some(false));
        assert_eq!(node.floating.as_ref().map(Vec::len), Some(1));
        assert!(node.extra.is_empty());
        Ok(())
    }

    #[test]
    fn node_kind_words_keep_unknown_words() -> Result<(), serde_json::Error> {
        assert_eq!(read_back::<NodeKind>("\"block\"")?, NodeKind::Block);
        assert_eq!(read_back::<NodeKind>("\"row\"")?, NodeKind::Row);
        assert_eq!(read_back::<NodeKind>("\"column\"")?, NodeKind::Column);
        assert_eq!(read_back::<NodeKind>("\"stack\"")?, NodeKind::Stack);
        assert_eq!(
            read_back::<NodeKind>("\"grid\"")?,
            NodeKind::Other("grid".to_owned())
        );
        Ok(())
    }

    #[test]
    fn shadow_words_keep_unknown_words() -> Result<(), serde_json::Error> {
        assert_eq!(read_back::<Shadow>("\"sm\"")?, Shadow::Sm);
        assert_eq!(read_back::<Shadow>("\"md\"")?, Shadow::Md);
        assert_eq!(read_back::<Shadow>("\"lg\"")?, Shadow::Lg);
        assert_eq!(
            read_back::<Shadow>("\"xl\"")?,
            Shadow::Other("xl".to_owned())
        );
        Ok(())
    }

    #[test]
    fn size_keeps_extra_and_falls_back() -> Result<(), serde_json::Error> {
        let Size::Px { px, extra } = read_back::<Size>(r#"{"px":1,"weight":2}"#)? else {
            return fail("not a px size");
        };
        assert_eq!(px, Number::from(1_u32));
        assert_eq!(extra.get("weight"), Some(&json!(2)));

        let Size::Weight { weight, extra } = read_back::<Size>(r#"{"weight":2,"x":1}"#)? else {
            return fail("not a weight size");
        };
        assert_eq!(weight, Number::from(2_u32));
        assert_eq!(extra.get("x"), Some(&json!(1)));

        let Size::Other(map) = read_back::<Size>(r#"{"px":"x"}"#)? else {
            return fail("not an other size");
        };
        assert_eq!(map.get("px"), Some(&json!("x")));
        Ok(())
    }

    #[test]
    fn size_helpers_write_one_key() -> Result<(), serde_json::Error> {
        assert_eq!(
            serde_json::to_string(&Size::px(Number::from(240_u32)))?,
            r#"{"px":240}"#
        );
        assert_eq!(
            serde_json::to_string(&Size::weight(Number::from(2_u32)))?,
            r#"{"weight":2}"#
        );
        Ok(())
    }

    #[test]
    fn focus_style_keeps_extra() -> Result<(), serde_json::Error> {
        let focus = read_back::<FocusStyle>(r#"{"border":{"width":1},"x":1}"#)?;
        assert!(focus.border.is_some());
        assert_eq!(focus.extra.get("x"), Some(&json!(1)));
        assert_eq!(serde_json::to_string(&FocusStyle::default())?, "{}");
        Ok(())
    }

    #[test]
    fn node_repeated_unknown_key_keeps_last() -> Result<(), serde_json::Error> {
        let node: Node = serde_json::from_str(r#"{"type":"row","x":1,"x":2}"#)?;
        assert_eq!(node.extra.get("x"), Some(&json!(2)));
        Ok(())
    }

    #[test]
    fn node_constructors_build_documented_shapes() -> Result<(), Box<dyn std::error::Error>> {
        let block = Node::block(Content::module(ModName::new("board")?));
        assert_eq!(
            serde_json::to_string(&block)?,
            r#"{"type":"block","content":{"mod":"board"}}"#
        );
        let row = Node::row(vec![block.clone()]);
        assert_eq!(
            serde_json::to_string(&row)?,
            r#"{"type":"row","children":[{"type":"block","content":{"mod":"board"}}]}"#
        );
        let column = Node::column(Vec::new());
        assert_eq!(
            serde_json::to_string(&column)?,
            r#"{"type":"column","children":[]}"#
        );
        let stack = Node::stack(vec![block], NodeId::new("a")?);
        assert_eq!(
            serde_json::to_string(&stack)?,
            r#"{"type":"stack","children":[{"type":"block","content":{"mod":"board"}}],"active":"a"}"#
        );
        Ok(())
    }

    /// The refusal `text` meets as a layout, or an error when it is accepted.
    fn refusal(text: &str) -> Result<FileError, serde_json::Error> {
        match Layout::parse(text) {
            Ok(_) => fail("the layout was accepted"),
            Err(error) => Ok(error),
        }
    }

    #[test]
    fn layout_parse_reports_file_errors() -> Result<(), serde_json::Error> {
        let empty = refusal("")?;
        assert_eq!(
            empty.to_string(),
            "invalid layout file: EOF while parsing a value at line 1 column 0"
        );
        assert_eq!(empty.file(), FileKind::Layout);

        assert_eq!(
            refusal("\u{feff}{\"type\":\"row\"}")?.to_string(),
            "invalid layout file: expected value at line 1 column 1"
        );

        let trailing = refusal(r#"{"type":"row"} x"#)?;
        assert!(
            trailing
                .detail()
                .starts_with("trailing characters at line 1 column"),
            "{trailing}"
        );

        let duplicate = refusal(r#"{"type":"row","type":"row"}"#)?;
        assert!(
            duplicate.detail().starts_with("duplicate field `type`"),
            "{duplicate}"
        );
        Ok(())
    }

    #[test]
    fn layout_parse_requires_a_type() -> Result<(), serde_json::Error> {
        let missing = refusal("{}")?;
        assert!(
            missing.detail().starts_with("missing field `type`"),
            "{missing}"
        );
        Ok(())
    }

    #[test]
    fn layout_to_json_is_pretty_with_newline() -> Result<(), FileError> {
        let layout = Layout::parse(r#"{"type":"row","children":[{"type":"block"}]}"#)?;
        let expected = concat!(
            "{\n",
            "  \"type\": \"row\",\n",
            "  \"children\": [\n",
            "    {\n",
            "      \"type\": \"block\"\n",
            "    }\n",
            "  ]\n",
            "}\n",
        );
        assert_eq!(layout.to_json(), expected);
        Ok(())
    }

    #[test]
    fn layout_gives_access_to_its_root() -> Result<(), FileError> {
        let mut layout = Layout::parse(r#"{"type":"row"}"#)?;
        assert_eq!(layout.root().kind, NodeKind::Row);
        layout.root_mut().gap = Some(Number::from(4_u32));
        assert_eq!(
            layout.to_json(),
            "{\n  \"type\": \"row\",\n  \"gap\": 4\n}\n"
        );
        assert_eq!(
            Layout::new(Node::row(Vec::new())).root().children,
            Some(Vec::new())
        );
        Ok(())
    }
}
