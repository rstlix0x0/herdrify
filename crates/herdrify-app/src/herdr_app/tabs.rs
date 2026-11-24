//! `TabOps`: the operations on the tabs of a space.
//!
//! A tab holds the panes of one space. The operations make, close, rename, move, select, put away
//! and bring back tabs. Each returns a [`Call`] over the command of the same name, built from plain
//! values: a name or an identifier is a string, a position is a number counted from 1.
//!
//! Responsibilities:
//!
//! - [`TabOps`], one method per way of using the `tab.*` commands
//!
//! Non-responsibilities:
//!
//! - the options of each command, which [`Call`] defines as setters
//! - running the commands, which [`Client`] does

use super::call::{Call, arg, example, ops_plumbing};
use crate::adapter::ProcessTransport;
use crate::driver::Client;
use crate::files::ModName;
use crate::port::Transport;
use crate::protocol::{
    Position, Rename, TabClose, TabContent, TabId, TabMove, TabMoveTo, TabName, TabNew, TabNext,
    TabPrev, TabRef, TabRename, TabSelect, TabSettle, TabUnsettle,
};

/// The operations on the tabs of a space.
///
/// Reached through [`HerdrApp::tabs`](super::HerdrApp::tabs). A name or an identifier is a string,
/// and an empty one is reported by the `.await` as [`Error::InvalidArgument`], as is the position
/// `0`.
///
/// # Examples
///
/// ```no_run
/// use herdrify_app::HerdrApp;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), herdrify_app::Error> {
/// let app = HerdrApp::local().await?;
/// app.tabs().new_tab().name("build").await?;
/// app.tabs().move_by(-1).await?;
/// app.tabs().select_index(1).await?;
/// # Ok(())
/// # }
/// ```
///
/// [`Error::InvalidArgument`]: crate::driver::Error::InvalidArgument
pub struct TabOps<T: Transport = ProcessTransport> {
    client: Client<T>,
}

ops_plumbing!(TabOps);

impl<T: Transport> TabOps<T> {
    /// `tab.new`: makes a tab that runs a shell. Its call takes `.argv(..)`, `.task(..)` and
    /// `.folder(..)` to run a program instead, besides `.name(..)`, `.space(..)` and `.focus(..)`.
    ///
    #[doc = example!(
        "app.tabs().new_tab().name(\"build\").await?;",
        "app.tabs().new_tab().argv([\"htop\"]).folder(\"/tmp\").await?;"
    )]
    pub fn new_tab(&self) -> Call<T, TabNew> {
        self.call(Ok(TabNew::new(TabContent::Shell {
            argv: None,
            task: None,
            folder: None,
        })))
    }

    /// `tab.new`: makes a tab that shows the mod called `module`. Its call takes `.props(..)` to
    /// give the mod properties; the program options `.argv(..)`, `.task(..)` and `.folder(..)`
    /// change nothing on it.
    ///
    #[doc = example!(
        "app.tabs().new_mod_tab(\"herdrify-host\").await?;",
        "let mut props = serde_json::Map::new();",
        "props.insert(\"title\".to_owned(), serde_json::json!(\"files\"));",
        "app.tabs().new_mod_tab(\"files\").props(props).await?;"
    )]
    pub fn new_mod_tab(&self, module: impl Into<String>) -> Call<T, TabNew> {
        self.call(arg("mod", ModName::new(module)).map(|module| {
            TabNew::new(TabContent::Mod {
                module,
                props: None,
            })
        }))
    }

    /// `tab.close`: closes the shown tab.
    ///
    #[doc = example!("app.tabs().close().await?;")]
    pub fn close(&self) -> Call<T, TabClose> {
        self.call(Ok(TabClose::new()))
    }

    /// `tab.rename`: renames the shown tab to `name`, which must not be empty. Use
    /// [`rename_automatic`](Self::rename_automatic) to give the naming back to herdr-app.
    ///
    #[doc = example!("app.tabs().rename(\"build\").await?;")]
    pub fn rename(&self, name: impl Into<String>) -> Call<T, TabRename> {
        self.call(arg("name", TabName::new(name)).map(|name| TabRename::new(Rename::To(name))))
    }

    /// `tab.rename`: lets herdr-app name the shown tab again.
    ///
    #[doc = example!("app.tabs().rename_automatic().await?;")]
    pub fn rename_automatic(&self) -> Call<T, TabRename> {
        self.call(Ok(TabRename::new(Rename::Automatic)))
    }

    /// `tab.move`: moves the shown tab to position `index`, counted from 1.
    ///
    #[doc = example!("app.tabs().move_to(1).await?;")]
    pub fn move_to(&self, index: u32) -> Call<T, TabMove> {
        self.call(
            arg("index", Position::new(index)).map(|index| TabMove::new(TabMoveTo::Index(index))),
        )
    }

    /// `tab.move`: moves the shown tab `by` places, towards the end when positive.
    ///
    #[doc = example!("app.tabs().move_by(-1).await?;")]
    pub fn move_by(&self, by: i32) -> Call<T, TabMove> {
        self.call(Ok(TabMove::new(TabMoveTo::By(by))))
    }

    /// `tab.select`: shows the tab at position `index`, counted from 1.
    ///
    #[doc = example!("app.tabs().select_index(1).await?;")]
    pub fn select_index(&self, index: u32) -> Call<T, TabSelect> {
        self.call(
            arg("index", Position::new(index)).map(|index| TabSelect::new(TabRef::Index(index))),
        )
    }

    /// `tab.select`: shows the tab with the identifier `id`.
    ///
    #[doc = example!("app.tabs().select_id(\"t1\").await?;")]
    pub fn select_id(&self, id: impl Into<String>) -> Call<T, TabSelect> {
        self.call(arg("id", TabId::new(id)).map(|id| TabSelect::new(TabRef::Id(id))))
    }

    /// `tab.settle`: puts the shown tab away: out of the lists, still running.
    ///
    #[doc = example!("app.tabs().settle().await?;")]
    pub fn settle(&self) -> Call<T, TabSettle> {
        self.call(Ok(TabSettle::new()))
    }

    /// `tab.unsettle`: brings back a tab that was put away.
    ///
    #[doc = example!("app.tabs().unsettle().id(\"t1\").await?;")]
    pub fn unsettle(&self) -> Call<T, TabUnsettle> {
        self.call(Ok(TabUnsettle::new()))
    }

    /// `tab.next`: shows the next tab.
    ///
    #[doc = example!("app.tabs().next().await?;")]
    pub fn next(&self) -> Call<T, TabNext> {
        self.call(Ok(TabNext::new()))
    }

    /// `tab.prev`: shows the previous tab.
    ///
    #[doc = example!("app.tabs().prev().await?;")]
    pub fn prev(&self) -> Call<T, TabPrev> {
        self.call(Ok(TabPrev::new()))
    }
}

#[cfg(test)]
pub(super) mod tests {
    use serde_json::json;

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
        let tabs = app.tabs();

        tabs.new_tab().await?;
        assert_sent(script, "tab.new", &json!({}));

        tabs.new_mod_tab("herdrify-host").await?;
        assert_sent(script, "tab.new", &json!({"mod": "herdrify-host"}));

        tabs.close().await?;
        assert_sent(script, "tab.close", &json!({}));

        tabs.rename("build").await?;
        assert_sent(script, "tab.rename", &json!({"name": "build"}));

        tabs.rename_automatic().await?;
        assert_sent(script, "tab.rename", &json!({"name": ""}));

        tabs.move_to(2).await?;
        assert_sent(script, "tab.move", &json!({"index": 2}));

        tabs.move_by(-1).await?;
        assert_sent(script, "tab.move", &json!({"by": -1}));

        tabs.select_index(3).await?;
        assert_sent(script, "tab.select", &json!({"index": 3}));

        tabs.select_id("t1").await?;
        assert_sent(script, "tab.select", &json!({"id": "t1"}));

        tabs.settle().await?;
        assert_sent(script, "tab.settle", &json!({}));

        tabs.unsettle().await?;
        assert_sent(script, "tab.unsettle", &json!({}));

        tabs.next().await?;
        assert_sent(script, "tab.next", &json!({}));

        tabs.prev().await?;
        assert_sent(script, "tab.prev", &json!({}));

        assert_eq!(script.argvs().len() - first, 13);
        Ok(())
    }

    #[tokio::test]
    async fn every_tab_method_builds_its_command() -> Result<(), Error> {
        let (app, script) = app().await?;
        run_every_method(&app, &script).await
    }

    #[tokio::test]
    async fn rename_refuses_empty() -> Result<(), Error> {
        let (app, script) = app().await?;
        let error = app.tabs().rename("").await.err();
        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some("invalid name: a tab name cannot be empty")
        );
        assert_eq!(script.argvs(), [["--version"]]);

        app.tabs().rename_automatic().await?;
        assert_sent(&script, "tab.rename", &json!({"name": ""}));
        Ok(())
    }

    #[tokio::test]
    async fn refused_values_name_the_argument_they_fill() -> Result<(), Error> {
        let (app, script) = app().await?;
        let error = app.tabs().select_index(0).await.err();
        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some("invalid index: 0 is out of range: positions start at 1")
        );
        let error = app.tabs().move_to(0).await.err();
        assert!(matches!(
            error,
            Some(Error::InvalidArgument { argument, .. }) if argument == "index"
        ));
        let error = app.tabs().select_id("").await.err();
        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some("invalid id: a tab id cannot be empty")
        );
        let error = app.tabs().new_mod_tab("").await.err();
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
        assert_eq!(format!("{:?}", app.tabs()), "TabOps { .. }");
        Ok(())
    }
}
