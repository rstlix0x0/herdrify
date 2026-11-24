//! `PaneOps`: the operations on the panes of a tab.
//!
//! A pane is one of the areas a tab is split into. The operations split, close, rename, resize,
//! zoom, focus, move between and swap panes. Each returns a [`Call`] over the command of the same
//! name, built from plain values: a name or an identifier is a string, a direction is an enum.
//!
//! Responsibilities:
//!
//! - [`PaneOps`], one method per way of using the `pane.*` commands
//!
//! Non-responsibilities:
//!
//! - the options of each command, which [`Call`] defines as setters
//! - running the commands, which [`Client`] does

use super::call::{Call, arg, example, ops_plumbing};
use crate::adapter::ProcessTransport;
use crate::driver::Client;
use crate::files::{ModName, TerminalAddress};
use crate::port::Transport;
use crate::protocol::{
    Direction, PaneClose, PaneContent, PaneFocus, PaneGo, PaneId, PaneName, PaneRename, PaneResize,
    PaneSplit, PaneSwap, PaneTarget, PaneZoom, Rename, SplitDirection, SwapWith,
};

/// The operations on the panes of a tab.
///
/// Reached through [`HerdrApp::panes`](super::HerdrApp::panes). A name or an identifier is a
/// string, and an empty one is reported by the `.await` as [`Error::InvalidArgument`].
///
/// # Examples
///
/// ```no_run
/// use herdrify_app::HerdrApp;
/// use herdrify_app::protocol::{Direction, SplitDirection};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), herdrify_app::Error> {
/// let app = HerdrApp::local().await?;
/// let split = app.panes().split(SplitDirection::Right).await?;
/// app.panes().rename("logs").pane(split.pane.as_str()).await?;
/// app.panes().go(Direction::Left).await?;
/// # Ok(())
/// # }
/// ```
///
/// [`Error::InvalidArgument`]: crate::driver::Error::InvalidArgument
pub struct PaneOps<T: Transport = ProcessTransport> {
    client: Client<T>,
}

ops_plumbing!(PaneOps);

impl<T: Transport> PaneOps<T> {
    /// `pane.split`: splits the focused pane towards `direction`, the new pane running a shell.
    /// Its call takes `.argv(..)` to run a program instead, besides `.pane(..)`.
    ///
    #[doc = example!(
        "use herdrify_app::protocol::SplitDirection;",
        "app.panes().split(SplitDirection::Right).await?;",
        "app.panes().split(SplitDirection::Down).argv([\"htop\"]).await?;"
    )]
    pub fn split(&self, direction: SplitDirection) -> Call<T, PaneSplit> {
        self.call(Ok(PaneSplit::new(
            direction,
            PaneContent::Shell { argv: None },
        )))
    }

    /// `pane.split`: splits the focused pane towards `direction`, the new pane showing the mod
    /// called `module`. Its call takes `.props(..)` to give the mod properties; `.argv(..)` changes
    /// nothing on it.
    ///
    #[doc = example!(
        "use herdrify_app::protocol::SplitDirection;",
        "app.panes().split_mod(SplitDirection::Down, \"herdrify-host\").await?;",
        "let mut props = serde_json::Map::new();",
        "props.insert(\"title\".to_owned(), serde_json::json!(\"files\"));",
        "app.panes().split_mod(SplitDirection::Down, \"files\").props(props).await?;"
    )]
    pub fn split_mod(
        &self,
        direction: SplitDirection,
        module: impl Into<String>,
    ) -> Call<T, PaneSplit> {
        self.call(arg("mod", ModName::new(module)).map(|module| {
            PaneSplit::new(
                direction,
                PaneContent::Mod {
                    module,
                    props: None,
                },
            )
        }))
    }

    /// `pane.close`: closes the focused pane.
    ///
    #[doc = example!("app.panes().close().await?;")]
    pub fn close(&self) -> Call<T, PaneClose> {
        self.call(Ok(PaneClose::new()))
    }

    /// `pane.rename`: names the focused pane `name`, which must not be empty. Use
    /// [`unname`](Self::unname) to remove the name.
    ///
    #[doc = example!("app.panes().rename(\"logs\").await?;")]
    pub fn rename(&self, name: impl Into<String>) -> Call<T, PaneRename> {
        self.call(arg("name", PaneName::new(name)).map(|name| PaneRename::new(Rename::To(name))))
    }

    /// `pane.rename`: removes the name of the focused pane.
    ///
    #[doc = example!("app.panes().unname().await?;")]
    pub fn unname(&self) -> Call<T, PaneRename> {
        self.call(Ok(PaneRename::new(Rename::Automatic)))
    }

    /// `pane.resize`: moves the focused pane's edge towards `direction`.
    ///
    #[doc = example!(
        "use herdrify_app::protocol::Direction;",
        "app.panes().resize(Direction::Left).px(30.0).await?;"
    )]
    pub fn resize(&self, direction: Direction) -> Call<T, PaneResize> {
        self.call(Ok(PaneResize::new(direction)))
    }

    /// `pane.zoom`: zooms the focused pane, or shows the other panes again when it is zoomed.
    ///
    #[doc = example!("app.panes().zoom().await?;")]
    pub fn zoom(&self) -> Call<T, PaneZoom> {
        self.call(Ok(PaneZoom::new()))
    }

    /// `pane.focus`: focuses the pane with the identifier `pane`.
    ///
    #[doc = example!("app.panes().focus(\"p1\").await?;")]
    pub fn focus(&self, pane: impl Into<String>) -> Call<T, PaneFocus> {
        self.call(arg("pane", PaneId::new(pane)).map(|pane| PaneFocus::new(PaneTarget::Pane(pane))))
    }

    /// `pane.focus`: focuses the pane that shows the terminal at the address `terminal`.
    ///
    #[doc = example!("app.panes().focus_terminal(\"t2\").await?;")]
    pub fn focus_terminal(&self, terminal: impl Into<String>) -> Call<T, PaneFocus> {
        self.call(
            arg("terminal", TerminalAddress::new(terminal))
                .map(|terminal| PaneFocus::new(PaneTarget::Terminal(terminal))),
        )
    }

    /// `pane.go`: focuses the neighbouring pane towards `direction`.
    ///
    #[doc = example!(
        "use herdrify_app::protocol::Direction;",
        "app.panes().go(Direction::Left).await?;"
    )]
    pub fn go(&self, direction: Direction) -> Call<T, PaneGo> {
        self.call(Ok(PaneGo::new(direction)))
    }

    /// `pane.swap`: swaps the focused pane with its neighbour towards `direction`.
    ///
    #[doc = example!(
        "use herdrify_app::protocol::Direction;",
        "app.panes().swap(Direction::Left).await?;"
    )]
    pub fn swap(&self, direction: Direction) -> Call<T, PaneSwap> {
        self.call(Ok(PaneSwap::new(SwapWith::Direction(direction))))
    }

    /// `pane.swap`: swaps the focused pane with the pane with the identifier `target`.
    ///
    #[doc = example!("app.panes().swap_with(\"p2\").await?;")]
    pub fn swap_with(&self, target: impl Into<String>) -> Call<T, PaneSwap> {
        self.call(
            arg("target", PaneId::new(target))
                .map(|target| PaneSwap::new(SwapWith::Target(target))),
        )
    }
}

#[cfg(test)]
pub(super) mod tests {
    use serde_json::json;

    use super::*;
    use crate::driver::Error;
    use crate::herdr_app::HerdrApp;
    use crate::herdr_app::fake::{Script, app, assert_sent};

    /// Runs every method of the group once against `app`, checking the command each one sent.
    ///
    /// Fails when the calls do not send one run each.
    pub(in crate::herdr_app) async fn run_every_method(
        app: &HerdrApp<Script>,
        script: &Script,
    ) -> Result<(), Error> {
        let first = script.argvs().len();
        let panes = app.panes();

        panes.split(SplitDirection::Right).await?;
        assert_sent(script, "pane.split", &json!({"direction": "right"}));

        panes
            .split_mod(SplitDirection::Down, "herdrify-host")
            .await?;
        assert_sent(
            script,
            "pane.split",
            &json!({"direction": "down", "mod": "herdrify-host"}),
        );

        panes.close().await?;
        assert_sent(script, "pane.close", &json!({}));

        panes.rename("logs").await?;
        assert_sent(script, "pane.rename", &json!({"name": "logs"}));

        panes.unname().await?;
        assert_sent(script, "pane.rename", &json!({"name": ""}));

        panes.resize(Direction::Left).await?;
        assert_sent(script, "pane.resize", &json!({"direction": "left"}));

        panes.zoom().await?;
        assert_sent(script, "pane.zoom", &json!({}));

        panes.focus("p1").await?;
        assert_sent(script, "pane.focus", &json!({"pane": "p1"}));

        panes.focus_terminal("t2").await?;
        assert_sent(script, "pane.focus", &json!({"terminal": "t2"}));

        panes.go(Direction::Up).await?;
        assert_sent(script, "pane.go", &json!({"direction": "up"}));

        panes.swap(Direction::Right).await?;
        assert_sent(script, "pane.swap", &json!({"direction": "right"}));

        panes.swap_with("p2").await?;
        assert_sent(script, "pane.swap", &json!({"target": "p2"}));

        assert_eq!(script.argvs().len() - first, 12);
        Ok(())
    }

    #[tokio::test]
    async fn every_pane_method_builds_its_command() -> Result<(), Error> {
        let (app, script) = app().await?;
        run_every_method(&app, &script).await
    }

    #[tokio::test]
    async fn rename_refuses_empty() -> Result<(), Error> {
        let (app, script) = app().await?;
        let error = app.panes().rename("").await.err();
        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some("invalid name: a pane name cannot be empty")
        );
        assert_eq!(script.argvs(), [["--version"]]);

        app.panes().unname().await?;
        assert_sent(&script, "pane.rename", &json!({"name": ""}));
        Ok(())
    }

    #[tokio::test]
    async fn refused_values_name_the_argument_they_fill() -> Result<(), Error> {
        let (app, script) = app().await?;
        let error = app.panes().focus("").await.err();
        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some("invalid pane: a pane id cannot be empty")
        );
        let error = app.panes().focus_terminal("").await.err();
        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some("invalid terminal: a terminal address cannot be empty")
        );
        let error = app.panes().swap_with("").await.err();
        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some("invalid target: a pane id cannot be empty")
        );
        let error = app.panes().split_mod(SplitDirection::Right, "").await.err();
        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some("invalid mod: a mod name cannot be empty")
        );
        assert_eq!(script.argvs(), [["--version"]]);
        Ok(())
    }

    #[tokio::test]
    async fn debug_names_the_group() -> Result<(), Error> {
        let (app, _script) = app().await?;
        assert_eq!(format!("{:?}", app.panes()), "PaneOps { .. }");
        Ok(())
    }
}
