//! The herdr-app commands that act on layouts and the nodes in them.
//!
//! A layout is the tree of nodes that draws the herdr-app window. Each command here is a value that
//! serializes to the JSON object of arguments herdr-app takes, and names the type its reply decodes
//! into, so a caller that runs one knows what comes back.
//!
//! Responsibilities:
//!
//! - [`LayoutSwitch`], [`LayoutClone`], [`LayoutSet`], [`LayoutToggle`], [`LayoutShow`],
//!   [`LayoutHide`], [`LayoutResize`], [`LayoutSelect`] and [`LayoutReset`], one type per command,
//!   with the arguments herdr-app requires as parameters of `new` and the rest as setters
//!
//! Non-responsibilities:
//!
//! - building the command line from a command, which the invocation does
//! - running the command and decoding its reply, which a layer above does
//! - checking that a layout or node exists, which herdr-app decides
//!
//! The unit tests below check each command's arguments against the JSON object herdr-app takes, and
//! its name and reply type.

use serde::Serialize;

use crate::files::{Layout, LayoutName, NodeId};
use crate::protocol::{Ack, Command, Resize};

/// Shows a layout, or the next one.
///
/// Without a name it shows the next layout.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::LayoutName;
/// use herdrify_app::protocol::LayoutSwitch;
///
/// let command = LayoutSwitch::new().name(LayoutName::new("main")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"name":"main"}"#);
/// assert_eq!(serde_json::to_string(&LayoutSwitch::new())?, "{}");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct LayoutSwitch {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<LayoutName>,
}

impl LayoutSwitch {
    /// A command that sets no argument.
    #[must_use]
    pub const fn new() -> Self {
        Self { name: None }
    }

    /// Shows this layout.
    #[must_use]
    pub fn name(mut self, name: LayoutName) -> Self {
        self.name = Some(name);
        self
    }
}

impl Command for LayoutSwitch {
    const NAME: &'static str = "layout.switch";
    type Output = Ack;
}

/// Duplicates a layout, then shows the copy.
///
/// The copy is called after the layout with `-copy` added, unless [`as_name`](Self::as_name) names
/// it.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::LayoutName;
/// use herdrify_app::protocol::LayoutClone;
///
/// let command = LayoutClone::new(LayoutName::new("main")?).as_name(LayoutName::new("mine")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"name":"main","as":"mine"}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct LayoutClone {
    name: LayoutName,
    #[serde(rename = "as", skip_serializing_if = "Option::is_none")]
    as_name: Option<LayoutName>,
}

impl LayoutClone {
    /// A command that duplicates the layout `name`.
    #[must_use]
    pub const fn new(name: LayoutName) -> Self {
        Self {
            name,
            as_name: None,
        }
    }

    /// Calls the copy this, sent as `as`, which is a keyword in Rust.
    #[must_use]
    pub fn as_name(mut self, name: LayoutName) -> Self {
        self.as_name = Some(name);
        self
    }
}

impl Command for LayoutClone {
    const NAME: &'static str = "layout.clone";
    type Output = Ack;
}

/// Replaces a personal layout's tree.
///
/// herdr-app lets a shipped layout take only its click and look fields, not its structure.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::Layout;
/// use herdrify_app::protocol::LayoutSet;
///
/// let layout = Layout::parse(r#"{"type":"column"}"#)?;
/// let command = LayoutSet::new(layout);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"layout":{"type":"column"}}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct LayoutSet {
    layout: Layout,
}

impl LayoutSet {
    /// A command that replaces the shown layout's tree with `layout`.
    #[must_use]
    pub const fn new(layout: Layout) -> Self {
        Self { layout }
    }
}

impl Command for LayoutSet {
    const NAME: &'static str = "layout.set";
    type Output = Ack;
}

/// Hides a layout node that is shown, or shows one that is hidden.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::NodeId;
/// use herdrify_app::protocol::LayoutToggle;
///
/// let command = LayoutToggle::new(NodeId::new("sidebar")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"id":"sidebar"}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct LayoutToggle {
    id: NodeId,
}

impl LayoutToggle {
    /// A command that toggles the node `id`.
    #[must_use]
    pub const fn new(id: NodeId) -> Self {
        Self { id }
    }
}

impl Command for LayoutToggle {
    const NAME: &'static str = "layout.toggle";
    type Output = Ack;
}

/// Shows a layout node.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::NodeId;
/// use herdrify_app::protocol::LayoutShow;
///
/// let command = LayoutShow::new(NodeId::new("sidebar")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"id":"sidebar"}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct LayoutShow {
    id: NodeId,
}

impl LayoutShow {
    /// A command that shows the node `id`.
    #[must_use]
    pub const fn new(id: NodeId) -> Self {
        Self { id }
    }
}

impl Command for LayoutShow {
    const NAME: &'static str = "layout.show";
    type Output = Ack;
}

/// Hides a layout node.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::NodeId;
/// use herdrify_app::protocol::LayoutHide;
///
/// let command = LayoutHide::new(NodeId::new("sidebar")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"id":"sidebar"}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct LayoutHide {
    id: NodeId,
}

impl LayoutHide {
    /// A command that hides the node `id`.
    #[must_use]
    pub const fn new(id: NodeId) -> Self {
        Self { id }
    }
}

impl Command for LayoutHide {
    const NAME: &'static str = "layout.hide";
    type Output = Ack;
}

/// Sizes a layout node, in pixels or as a weight.
///
/// The size is a parameter of `new` because herdr-app answers `layout.resize` without a size with
/// `ok` and changes nothing, so a missing size would be a silent no-op.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::NodeId;
/// use herdrify_app::protocol::{LayoutResize, Px, Resize};
///
/// let command = LayoutResize::new(NodeId::new("sidebar")?, Resize::Px(Px::new(200.0)?));
/// assert_eq!(serde_json::to_string(&command)?, r#"{"id":"sidebar","px":200.0}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct LayoutResize {
    id: NodeId,
    #[serde(flatten)]
    size: Resize,
}

impl LayoutResize {
    /// A command that gives the node `id` the size `size`.
    #[must_use]
    pub const fn new(id: NodeId, size: Resize) -> Self {
        Self { id, size }
    }
}

impl Command for LayoutResize {
    const NAME: &'static str = "layout.resize";
    type Output = Ack;
}

/// Shows one child of a stack.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::NodeId;
/// use herdrify_app::protocol::LayoutSelect;
///
/// let command = LayoutSelect::new(NodeId::new("tabs")?, NodeId::new("second")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"stack":"tabs","child":"second"}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct LayoutSelect {
    stack: NodeId,
    child: NodeId,
}

impl LayoutSelect {
    /// A command that shows `child` in the stack `stack`.
    #[must_use]
    pub const fn new(stack: NodeId, child: NodeId) -> Self {
        Self { stack, child }
    }
}

impl Command for LayoutSelect {
    const NAME: &'static str = "layout.select";
    type Output = Ack;
}

/// Resets a shipped layout and its clicks.
///
/// herdr-app keeps the old file aside. Without a name it resets the shown layout.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::LayoutName;
/// use herdrify_app::protocol::LayoutReset;
///
/// let command = LayoutReset::new().name(LayoutName::new("main")?);
/// assert_eq!(serde_json::to_string(&command)?, r#"{"name":"main"}"#);
/// assert_eq!(serde_json::to_string(&LayoutReset::new())?, "{}");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[non_exhaustive]
pub struct LayoutReset {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<LayoutName>,
}

impl LayoutReset {
    /// A command that sets no argument.
    #[must_use]
    pub const fn new() -> Self {
        Self { name: None }
    }

    /// Resets this layout.
    #[must_use]
    pub fn name(mut self, name: LayoutName) -> Self {
        self.name = Some(name);
        self
    }
}

impl Command for LayoutReset {
    const NAME: &'static str = "layout.reset";
    type Output = Ack;
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde_json::{Value, json};

    use super::{
        LayoutClone, LayoutHide, LayoutReset, LayoutResize, LayoutSelect, LayoutSet, LayoutShow,
        LayoutSwitch, LayoutToggle,
    };
    use crate::files::{Layout, LayoutName, NodeId};
    use crate::protocol::{Ack, Command, Px, Resize, Weight};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn arguments<C: Serialize>(command: &C) -> Result<Value, serde_json::Error> {
        serde_json::to_value(command)
    }

    const fn replies_with<C: Command<Output = O>, O>() {}

    #[test]
    fn layout_commands_serialize_full_and_minimal() -> TestResult {
        assert_eq!(
            arguments(&LayoutSwitch::new().name(LayoutName::new("main")?))?,
            json!({"name": "main"}),
        );
        assert_eq!(arguments(&LayoutSwitch::new())?, json!({}));

        let clone = LayoutClone::new(LayoutName::new("main")?).as_name(LayoutName::new("mine")?);
        assert_eq!(arguments(&clone)?, json!({"name": "main", "as": "mine"}));
        assert_eq!(
            arguments(&LayoutClone::new(LayoutName::new("main")?))?,
            json!({"name": "main"}),
        );

        let layout = Layout::parse(r#"{"type":"row","children":[{"type":"block"}]}"#)?;
        let expected_layout = serde_json::to_value(&layout)?;
        assert_eq!(
            arguments(&LayoutSet::new(layout))?,
            json!({"layout": expected_layout}),
        );

        assert_eq!(
            arguments(&LayoutToggle::new(NodeId::new("sidebar")?))?,
            json!({"id": "sidebar"}),
        );
        assert_eq!(
            arguments(&LayoutShow::new(NodeId::new("sidebar")?))?,
            json!({"id": "sidebar"}),
        );
        assert_eq!(
            arguments(&LayoutHide::new(NodeId::new("sidebar")?))?,
            json!({"id": "sidebar"}),
        );

        assert_eq!(
            arguments(&LayoutResize::new(
                NodeId::new("sidebar")?,
                Resize::Px(Px::new(200.0)?),
            ))?,
            json!({"id": "sidebar", "px": 200.0}),
        );
        assert_eq!(
            arguments(&LayoutResize::new(
                NodeId::new("sidebar")?,
                Resize::Weight(Weight::new(2.0)?),
            ))?,
            json!({"id": "sidebar", "weight": 2.0}),
        );

        assert_eq!(
            arguments(&LayoutSelect::new(
                NodeId::new("tabs")?,
                NodeId::new("second")?
            ))?,
            json!({"stack": "tabs", "child": "second"}),
        );

        assert_eq!(
            arguments(&LayoutReset::new().name(LayoutName::new("main")?))?,
            json!({"name": "main"}),
        );
        assert_eq!(arguments(&LayoutReset::new())?, json!({}));
        Ok(())
    }

    #[test]
    fn layout_commands_name_their_wire_command() {
        assert_eq!(LayoutSwitch::NAME, "layout.switch");
        assert_eq!(LayoutClone::NAME, "layout.clone");
        assert_eq!(LayoutSet::NAME, "layout.set");
        assert_eq!(LayoutToggle::NAME, "layout.toggle");
        assert_eq!(LayoutShow::NAME, "layout.show");
        assert_eq!(LayoutHide::NAME, "layout.hide");
        assert_eq!(LayoutResize::NAME, "layout.resize");
        assert_eq!(LayoutSelect::NAME, "layout.select");
        assert_eq!(LayoutReset::NAME, "layout.reset");

        replies_with::<LayoutSwitch, Ack>();
        replies_with::<LayoutClone, Ack>();
        replies_with::<LayoutSet, Ack>();
        replies_with::<LayoutToggle, Ack>();
        replies_with::<LayoutShow, Ack>();
        replies_with::<LayoutHide, Ack>();
        replies_with::<LayoutResize, Ack>();
        replies_with::<LayoutSelect, Ack>();
        replies_with::<LayoutReset, Ack>();
    }
}
