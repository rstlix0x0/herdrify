//! The window status: zoom, font sizes, focus, the layout in use, floating nodes and problems.
//!
//! `herdr-app status` prints one JSON document. [`WindowStatus`] decodes it, with [`Float`] for
//! each floating node it lists. Each type ignores fields herdr-app adds later, and the numbers
//! are plain `f64` because a status is only read, never written back.
//!
//! Responsibilities:
//!
//! - decoding the `status` document into typed values, reusing the identifier types of
//!   [`protocol`](crate::protocol) and the edge, alignment and names of [`files`](crate::files)
//!
//! Non-responsibilities:
//!
//! - running `herdr-app status` to obtain the document, which a layer above does
//! - reading fields this crate has no type for, which the caller does from the raw output

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::files::{Align, Edge, LayoutName, NodeId, TerminalAddress};
use crate::protocol::{OverlayId, PaneId, SpaceId, TabId};

/// What `herdr-app status` prints about the window.
///
/// Fields herdr-app adds later are ignored.
///
/// # Examples
///
/// ```
/// use herdrify_app::protocol::WindowStatus;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let status: WindowStatus = serde_json::from_str(
///     r#"{
///         "zoom": 1.0, "font_size": 14.0, "ui_font_size": 13.0,
///         "space": null, "tab": null, "focused_pane": null,
///         "layout": "main", "agent": null,
///         "floats": {}, "overlays": ["frame-welcome"], "errors": []
///     }"#,
/// )?;
/// assert_eq!(status.layout.as_str(), "main");
/// assert!(status.space.is_none());
/// assert_eq!(status.overlays.len(), 1);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WindowStatus {
    /// The zoom factor of the window.
    pub zoom: f64,
    /// The font size of the terminals.
    pub font_size: f64,
    /// The font size of the interface around them.
    pub ui_font_size: f64,
    /// The space in focus, when there is one.
    pub space: Option<SpaceId>,
    /// The tab in focus, when there is one.
    pub tab: Option<TabId>,
    /// The pane in focus, when there is one.
    pub focused_pane: Option<PaneId>,
    /// The layout in use.
    pub layout: LayoutName,
    /// The terminal of the focused agent, when one is focused. A fresh window reports it as null, as
    /// every recording does.
    pub agent: Option<TerminalAddress>,
    /// The floating nodes, by node name.
    pub floats: BTreeMap<NodeId, Float>,
    /// The overlays that are active.
    pub overlays: Vec<OverlayId>,
    /// The problems herdr-app reports, as the text it prints.
    pub errors: Vec<String>,
}

/// Where one floating node sits and what it is doing.
///
/// Keys other than the five below are ignored.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::{Align, Edge};
/// use herdrify_app::protocol::Float;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let float: Float = serde_json::from_str(
///     r#"{"edge":"left","align":"end","vertical":true,"dragging":false,"hovered":false}"#,
/// )?;
/// assert_eq!(float.edge, Edge::Left);
/// assert_eq!(float.align, Align::End);
/// assert!(float.vertical);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Float {
    /// The window edge the node is attached to.
    pub edge: Edge,
    /// Where along that edge the node sits.
    pub align: Align,
    /// Whether the node is laid out vertically.
    pub vertical: bool,
    /// Whether the node is being dragged.
    pub dragging: bool,
    /// Whether the pointer is over the node.
    pub hovered: bool,
}

#[cfg(test)]
mod tests {
    use super::{Float, WindowStatus};
    use crate::files::{Align, Edge};
    use crate::protocol::OverlayId;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    // recorded from herdr-app 0.1.17
    const FRESH_WINDOW: &str = r#"
{
  "zoom": 1.0,
  "font_size": 14.0,
  "ui_font_size": 13.0,
  "space": null,
  "tab": null,
  "focused_pane": null,
  "layout": "main",
  "agent": null,
  "floats": {},
  "overlays": [
    "frame-welcome"
  ],
  "errors": []
}
"#;

    // recorded from herdr-app 0.1.17, with local paths replaced
    const FLOATS_AND_ERRORS: &str = r#"
{
  "zoom": 1.0,
  "font_size": 22.86,
  "ui_font_size": 13.0,
  "space": "sp791801470e39e979233f19213e2f5b3b",
  "tab": "tbac537ebad46b69af5db7cafb14f4a078",
  "focused_pane": "p13436eac9efd61a6971094912e82edd7",
  "layout": "win98",
  "agent": null,
  "floats": {
    "clippy": {
      "edge": "left",
      "align": "end",
      "vertical": true,
      "dragging": false,
      "hovered": false
    }
  },
  "overlays": [
    "frame-welcome",
    "broken",
    "win98"
  ],
  "errors": [
    "mod broken: Unexpected token '('\n    at /tmp/hzw/mods/broken/main.js?v=8:1:1\n"
  ]
}
"#;

    #[test]
    fn status_decodes_a_fresh_window() -> TestResult {
        let status: WindowStatus = serde_json::from_str(FRESH_WINDOW)?;
        assert!(status.space.is_none());
        assert!(status.tab.is_none());
        assert!(status.focused_pane.is_none());
        assert!(status.agent.is_none());
        assert_eq!(status.layout.as_str(), "main");
        assert!(status.floats.is_empty());
        let overlays: Vec<&str> = status.overlays.iter().map(OverlayId::as_str).collect();
        assert_eq!(overlays, ["frame-welcome"]);
        assert!(status.errors.is_empty());
        Ok(())
    }

    #[test]
    fn status_decodes_floats_and_errors() -> TestResult {
        let status: WindowStatus = serde_json::from_str(FLOATS_AND_ERRORS)?;
        assert_eq!(status.layout.as_str(), "win98");
        assert_eq!(status.floats.len(), 1);
        let clippy = status
            .floats
            .iter()
            .find(|(name, _)| name.as_str() == "clippy")
            .map(|(_, float)| float)
            .ok_or("no float named clippy")?;
        assert_eq!(clippy.edge, Edge::Left);
        assert_eq!(clippy.align, Align::End);
        assert!(clippy.vertical);
        assert!(!clippy.dragging);
        assert!(!clippy.hovered);
        assert_eq!(status.errors.len(), 1);
        assert!(status.errors[0].starts_with("mod broken:"));
        Ok(())
    }

    #[test]
    fn float_ignores_other_keys() -> TestResult {
        let float: Float = serde_json::from_str(
            r#"{"edge":"left","align":"end","vertical":true,"dragging":false,"hovered":false,"bounds":[1,2,3,4]}"#,
        )?;
        assert_eq!(float.edge, Edge::Left);
        Ok(())
    }

    #[test]
    fn status_ignores_unknown_fields() -> TestResult {
        let mut document: serde_json::Value = serde_json::from_str(FRESH_WINDOW)?;
        document["added_later"] = serde_json::json!({"anything": [1, 2]});
        let status: WindowStatus = serde_json::from_value(document)?;
        assert_eq!(status.layout.as_str(), "main");
        Ok(())
    }
}
