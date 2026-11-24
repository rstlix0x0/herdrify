//! `LayoutOps`: the operations on the window's layout.
//!
//! A layout is the tree of nodes that arranges the window. The operations switch between layouts,
//! duplicate, replace and reset one, show, hide and resize its nodes, and choose the child a
//! stack shows. Each returns a [`Call`] over the command of the same name, built from plain
//! values: a name or a node identifier is a string, a size is a number.
//!
//! Responsibilities:
//!
//! - [`LayoutOps`], one method per way of using the `layout.*` commands
//!
//! Non-responsibilities:
//!
//! - the options of each command, which [`Call`] defines as setters
//! - running the commands, which [`Client`] does

use super::call::{Call, arg, example, ops_plumbing};
use crate::adapter::ProcessTransport;
use crate::driver::{Client, Error};
use crate::files::{Layout, LayoutName, NodeId};
use crate::port::Transport;
use crate::protocol::{
    LayoutClone, LayoutHide, LayoutReset, LayoutResize, LayoutSelect, LayoutSet, LayoutShow,
    LayoutSwitch, LayoutToggle, Px, Resize, Weight,
};

/// The node `id` of a layout, or the error for refusing it as the argument `argument`.
fn node(argument: &'static str, id: impl Into<String>) -> Result<NodeId, Error> {
    arg(argument, NodeId::new(id))
}

/// The operations on the window's layout.
///
/// Reached through [`HerdrApp::layouts`](super::HerdrApp::layouts). A name or a node identifier is
/// a string, and an empty one is reported by the `.await` as [`Error::InvalidArgument`], as is a
/// size that is out of range.
///
/// # Examples
///
/// ```no_run
/// use herdrify_app::HerdrApp;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), herdrify_app::Error> {
/// let app = HerdrApp::local().await?;
/// app.layouts().clone("main").as_name("mine").await?;
/// app.layouts().hide("sidebar").await?;
/// app.layouts().resize_px("sidebar", 240.0).await?;
/// # Ok(())
/// # }
/// ```
pub struct LayoutOps<T: Transport = ProcessTransport> {
    client: Client<T>,
}

ops_plumbing!(LayoutOps);

impl<T: Transport> LayoutOps<T> {
    /// `layout.switch`: shows the layout called `name`.
    ///
    #[doc = example!("app.layouts().switch(\"main\").await?;")]
    pub fn switch(&self, name: impl Into<String>) -> Call<T, LayoutSwitch> {
        self.call(arg("name", LayoutName::new(name)).map(|name| LayoutSwitch::new().name(name)))
    }

    /// `layout.switch`: shows the layout after the one in use.
    ///
    #[doc = example!("app.layouts().next().await?;")]
    pub fn next(&self) -> Call<T, LayoutSwitch> {
        self.call(Ok(LayoutSwitch::new()))
    }

    /// `layout.clone`: duplicates the layout called `name` and shows the copy. The copy is called
    /// after the layout with `-copy` added, unless its call is given a name with `.as_name(..)`.
    ///
    #[doc = example!("app.layouts().clone(\"main\").as_name(\"mine\").await?;")]
    pub fn clone(&self, name: impl Into<String>) -> Call<T, LayoutClone> {
        self.call(arg("name", LayoutName::new(name)).map(LayoutClone::new))
    }

    /// `layout.set`: replaces the tree of the shown layout, a personal one, with `layout`. A
    /// shipped layout takes only its click and look fields.
    ///
    #[doc = example!(
        "use herdrify_app::files::{Layout, Node};",
        "let layout = Layout::new(Node::column(Vec::new()));",
        "app.layouts().set(layout).await?;"
    )]
    pub fn set(&self, layout: Layout) -> Call<T, LayoutSet> {
        self.call(Ok(LayoutSet::new(layout)))
    }

    /// `layout.toggle`: hides the node `id` when it is shown, and shows it when it is hidden.
    ///
    #[doc = example!("app.layouts().toggle(\"sidebar\").await?;")]
    pub fn toggle(&self, id: impl Into<String>) -> Call<T, LayoutToggle> {
        self.call(node("id", id).map(LayoutToggle::new))
    }

    /// `layout.show`: shows the node `id`.
    ///
    #[doc = example!("app.layouts().show(\"sidebar\").await?;")]
    pub fn show(&self, id: impl Into<String>) -> Call<T, LayoutShow> {
        self.call(node("id", id).map(LayoutShow::new))
    }

    /// `layout.hide`: hides the node `id`.
    ///
    #[doc = example!("app.layouts().hide(\"sidebar\").await?;")]
    pub fn hide(&self, id: impl Into<String>) -> Call<T, LayoutHide> {
        self.call(node("id", id).map(LayoutHide::new))
    }

    /// `layout.resize`: sizes the node `id` to `px` pixels, which must be finite and not negative.
    ///
    #[doc = example!("app.layouts().resize_px(\"sidebar\", 240.0).await?;")]
    pub fn resize_px(&self, id: impl Into<String>, px: f64) -> Call<T, LayoutResize> {
        self.call(
            node("id", id).and_then(|id| {
                arg("px", Px::new(px)).map(|px| LayoutResize::new(id, Resize::Px(px)))
            }),
        )
    }

    /// `layout.resize`: gives the node `id` a share of the space among its siblings, `weight`,
    /// which must be finite and above 0.
    ///
    #[doc = example!("app.layouts().resize_weight(\"sidebar\", 2.5).await?;")]
    pub fn resize_weight(&self, id: impl Into<String>, weight: f64) -> Call<T, LayoutResize> {
        self.call(node("id", id).and_then(|id| {
            arg("weight", Weight::new(weight))
                .map(|weight| LayoutResize::new(id, Resize::Weight(weight)))
        }))
    }

    /// `layout.select`: makes the stack `stack` show its child `child`.
    ///
    #[doc = example!("app.layouts().select(\"tabs\", \"second\").await?;")]
    pub fn select(
        &self,
        stack: impl Into<String>,
        child: impl Into<String>,
    ) -> Call<T, LayoutSelect> {
        self.call(
            node("stack", stack).and_then(|stack| {
                node("child", child).map(|child| LayoutSelect::new(stack, child))
            }),
        )
    }

    /// `layout.reset`: resets the shown layout, a shipped one, and its clicks, and herdr-app keeps
    /// the old file aside. Another layout is named with `.name(..)` on the call.
    ///
    #[doc = example!("app.layouts().reset().name(\"main\").await?;")]
    pub fn reset(&self) -> Call<T, LayoutReset> {
        self.call(Ok(LayoutReset::new()))
    }
}

#[cfg(test)]
pub(super) mod tests {
    use serde_json::json;

    use super::*;
    use crate::files::Layout;
    use crate::herdr_app::HerdrApp;
    use crate::herdr_app::fake::{Script, app, assert_sent};

    /// Runs every method of the layout group once against `app`, checking the command each one
    /// sent.
    ///
    /// Fails when the calls do not send one run each.
    pub(in crate::herdr_app) async fn run_every_method(
        app: &HerdrApp<Script>,
        script: &Script,
    ) -> Result<(), Error> {
        let first = script.argvs().len();
        let layouts = app.layouts();

        layouts.switch("main").await?;
        assert_sent(script, "layout.switch", &json!({"name": "main"}));

        layouts.next().await?;
        assert_sent(script, "layout.switch", &json!({}));

        layouts.clone("main").await?;
        assert_sent(script, "layout.clone", &json!({"name": "main"}));

        let layout = Layout::parse(r#"{"type":"column"}"#).map_err(|error| Error::Cli {
            message: error.to_string(),
        })?;
        layouts.set(layout).await?;
        assert_sent(script, "layout.set", &json!({"layout": {"type": "column"}}));

        layouts.toggle("sidebar").await?;
        assert_sent(script, "layout.toggle", &json!({"id": "sidebar"}));

        layouts.show("sidebar").await?;
        assert_sent(script, "layout.show", &json!({"id": "sidebar"}));

        layouts.hide("sidebar").await?;
        assert_sent(script, "layout.hide", &json!({"id": "sidebar"}));

        layouts.resize_px("sidebar", 200.0).await?;
        assert_sent(
            script,
            "layout.resize",
            &json!({"id": "sidebar", "px": 200.0}),
        );

        layouts.resize_weight("sidebar", 2.5).await?;
        assert_sent(
            script,
            "layout.resize",
            &json!({"id": "sidebar", "weight": 2.5}),
        );

        layouts.select("tabs", "second").await?;
        assert_sent(
            script,
            "layout.select",
            &json!({"stack": "tabs", "child": "second"}),
        );

        layouts.reset().await?;
        assert_sent(script, "layout.reset", &json!({}));

        assert_eq!(script.argvs().len() - first, 11);
        Ok(())
    }

    #[tokio::test]
    async fn every_layout_method_builds_its_command() -> Result<(), Error> {
        let (app, script) = app().await?;
        run_every_method(&app, &script).await
    }

    #[tokio::test]
    async fn out_of_range_numbers_are_invalid_argument() -> Result<(), Error> {
        let (app, script) = app().await?;
        let error = app.layouts().resize_px("sidebar", -5.0).await.err();
        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some("invalid px: -5 is out of range: must be finite and not negative")
        );
        let error = app.layouts().resize_weight("sidebar", 0.0).await.err();
        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some("invalid weight: 0 is out of range: must be finite and above 0")
        );
        assert_eq!(script.argvs(), [["--version"]]);
        Ok(())
    }

    #[tokio::test]
    async fn empty_names_and_ids_name_the_argument_they_fill() -> Result<(), Error> {
        let (app, script) = app().await?;
        let layouts = app.layouts();
        let refused = [
            ("name", layouts.switch("").await.err()),
            ("name", layouts.clone("").await.err()),
            ("id", layouts.toggle("").await.err()),
            ("id", layouts.show("").await.err()),
            ("id", layouts.hide("").await.err()),
            ("id", layouts.resize_px("", 1.0).await.err()),
            ("id", layouts.resize_weight("", 1.0).await.err()),
            ("stack", layouts.select("", "second").await.err()),
            ("child", layouts.select("tabs", "").await.err()),
        ];
        for (argument, error) in refused {
            assert!(
                matches!(
                    &error,
                    Some(Error::InvalidArgument { argument: named, .. }) if named == argument
                ),
                "{argument}: {error:?}"
            );
        }
        assert_eq!(script.argvs(), [["--version"]]);
        Ok(())
    }

    #[tokio::test]
    async fn debug_names_the_group() -> Result<(), Error> {
        let (app, _script) = app().await?;
        assert_eq!(format!("{:?}", app.layouts()), "LayoutOps { .. }");
        Ok(())
    }
}
