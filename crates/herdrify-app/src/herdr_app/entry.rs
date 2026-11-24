//! `HerdrApp`: the entrypoint that reaches herdr-app from plain values.
//!
//! A [`HerdrApp`] is a [`Client`] whose herdr-app version was checked once, when the app was built.
//! It keeps that version and hands out the client underneath for anything the entrypoint does not
//! cover. The listing, the window status and the command list are its own methods, and each group
//! of commands is reached through a method that hands out the group's operations.
//!
//! Responsibilities:
//!
//! - [`HerdrApp`], how it is built and what it holds
//! - `ls`, `status` and `commands`, which it forwards to the client
//! - reaching the operation groups, each over a clone of the client
//!
//! Non-responsibilities:
//!
//! - the transport it runs over, which [`HerdrAppBuilder`] assembles for `herdr-app`
//! - the operations of a group, which the group's own file defines
//! - running a command and decoding its reply, which [`Client`] does

use super::call::example;
use super::{
    AgentOps, HerdrAppBuilder, LayoutOps, ModOps, OverlayOps, PaneOps, SpaceOps, TabOps,
    TerminalOps, WindowOps,
};
use crate::adapter::ProcessTransport;
use crate::driver::{Client, Error};
use crate::port::Transport;
use crate::protocol::{AppVersion, CommandInfo, Listing, WindowStatus};

/// A connection to herdr-app whose version has been checked.
///
/// Build it with [`local`](HerdrApp::local) or [`builder`](HerdrApp::builder) to run the
/// `herdr-app` program, or with [`from_client`](HerdrApp::from_client) over a transport of your
/// own. The version is checked once, at construction, and is not checked again. Cloning a
/// `HerdrApp` is cheap, and every clone runs through the same transport, whether or not the
/// transport is `Clone` itself.
///
/// # Examples
///
/// ```no_run
/// use herdrify_app::HerdrApp;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), herdrify_app::Error> {
/// let app = HerdrApp::local().await?;
/// println!("herdr-app {}", app.version());
/// let listing = app.ls().await?;
/// println!("{} spaces", listing.spaces.len());
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct HerdrApp<T: Transport = ProcessTransport> {
    client: Client<T>,
    version: AppVersion,
}

impl<T: Transport> Clone for HerdrApp<T> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            version: self.version.clone(),
        }
    }
}

impl HerdrApp {
    /// Connects to the `herdr-app` found on `PATH`, with the environment unchanged.
    ///
    /// This is `HerdrApp::builder().connect()`.
    ///
    /// # Errors
    ///
    /// - [`Error::Transport`] when `herdr-app` cannot be started or run.
    /// - [`Error::UnsupportedVersion`] when it is older than [`AppVersion::MINIMUM`].
    /// - The errors of [`Client::version`] otherwise.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use herdrify_app::HerdrApp;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), herdrify_app::Error> {
    /// let app = HerdrApp::local().await?;
    /// println!("{}", app.status().await?.layout);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn local() -> Result<Self, Error> {
        Self::builder().connect().await
    }

    /// The builder for a `herdr-app` that is not the one on `PATH`, or that needs a home or
    /// environment edits.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use herdrify_app::HerdrApp;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), herdrify_app::Error> {
    /// let app = HerdrApp::builder()
    ///     .program("/opt/herdr/herdr-app")
    ///     .home("/tmp/herdr-app-home")
    ///     .env_remove("HC_SESSION")
    ///     .connect()
    ///     .await?;
    /// println!("herdr-app {}", app.version());
    /// # Ok(())
    /// # }
    /// ```
    pub fn builder() -> HerdrAppBuilder {
        HerdrAppBuilder::new()
    }
}

impl<T: Transport> HerdrApp<T> {
    /// Wraps `client` after checking the version herdr-app reports, once.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnsupportedVersion`] when the version is below [`AppVersion::MINIMUM`], and
    /// the errors of [`Client::version`] otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::future::Future;
    /// # use herdrify_app::port::{Transport, TransportError};
    /// # use herdrify_app::protocol::{Exit, Invocation, RunOutput};
    /// # struct Fixed;
    /// # impl Transport for Fixed {
    /// #     fn run(
    /// #         &self,
    /// #         _invocation: &Invocation,
    /// #     ) -> impl Future<Output = Result<RunOutput, TransportError>> + Send {
    /// #         async {
    /// #             Ok(RunOutput {
    /// #                 exit: Exit::Code(0),
    /// #                 stdout: b"herdr-app 0.1.17\n".to_vec(),
    /// #                 stderr: Vec::new(),
    /// #             })
    /// #         }
    /// #     }
    /// # }
    /// use herdrify_app::driver::{Client, Error};
    /// use herdrify_app::HerdrApp;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), Error> {
    /// let app = HerdrApp::from_client(Client::new(Fixed)).await?;
    /// assert_eq!(app.version().to_string(), "0.1.17");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn from_client(client: Client<T>) -> Result<Self, Error> {
        let version = client.check_version().await?;
        Ok(Self { client, version })
    }

    /// The version herdr-app reported when the app was built.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use herdrify_app::HerdrApp;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), herdrify_app::Error> {
    /// let app = HerdrApp::local().await?;
    /// println!("herdr-app {}", app.version());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub const fn version(&self) -> &AppVersion {
        &self.version
    }

    /// The client underneath, for the typed commands themselves and for any other invocation.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use herdrify_app::HerdrApp;
    /// use herdrify_app::protocol::TabNext;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), herdrify_app::Error> {
    /// let app = HerdrApp::local().await?;
    /// app.client().run(TabNext::new()).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub const fn client(&self) -> &Client<T> {
        &self.client
    }

    /// The spaces, tabs and terminals of the window, from `ls`.
    ///
    /// # Errors
    ///
    /// The errors of [`Client::ls`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use herdrify_app::HerdrApp;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), herdrify_app::Error> {
    /// let app = HerdrApp::local().await?;
    /// for space in app.ls().await?.spaces {
    ///     println!("{}", space.name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn ls(&self) -> Result<Listing, Error> {
        self.client.ls().await
    }

    /// The state of the window, from `status`.
    ///
    /// # Errors
    ///
    /// The errors of [`Client::status`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use herdrify_app::HerdrApp;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), herdrify_app::Error> {
    /// let app = HerdrApp::local().await?;
    /// println!("{}", app.status().await?.layout);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn status(&self) -> Result<WindowStatus, Error> {
        self.client.status().await
    }

    /// The commands herdr-app offers, from `commands`.
    ///
    /// # Errors
    ///
    /// The errors of [`Client::commands`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use herdrify_app::HerdrApp;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), herdrify_app::Error> {
    /// let app = HerdrApp::local().await?;
    /// println!("{} commands", app.commands().await?.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn commands(&self) -> Result<Vec<CommandInfo>, Error> {
        self.client.commands().await
    }

    /// The operations on spaces: make, close, rename, move and select them.
    ///
    #[doc = example!(
        "let created = app.spaces().new_space().name(\"api\").await?;",
        "app.spaces().select_id(created.space.as_str()).await?;"
    )]
    #[must_use]
    pub fn spaces(&self) -> SpaceOps<T> {
        SpaceOps::new(self.client.clone())
    }

    /// The operations on tabs: make, close, rename, move, select, put away and bring back.
    ///
    #[doc = example!(
        "app.tabs().new_tab().name(\"build\").await?;",
        "app.tabs().move_by(-1).await?;"
    )]
    #[must_use]
    pub fn tabs(&self) -> TabOps<T> {
        TabOps::new(self.client.clone())
    }

    /// The operations on panes: split, close, rename, resize, zoom, focus and swap.
    ///
    #[doc = example!(
        "use herdrify_app::protocol::SplitDirection;",
        "app.panes().split(SplitDirection::Right).await?;",
        "app.panes().zoom().await?;"
    )]
    #[must_use]
    pub fn panes(&self) -> PaneOps<T> {
        PaneOps::new(self.client.clone())
    }

    /// The operations on terminals: start, keep, park, pin and kill them, and select and scroll
    /// their screens.
    ///
    #[doc = example!(
        "let opened = app.terminals().open().argv([\"top\"]).await?;",
        "app.terminals().pin(opened.terminal.as_str()).await?;"
    )]
    #[must_use]
    pub fn terminals(&self) -> TerminalOps<T> {
        TerminalOps::new(self.client.clone())
    }

    /// The operations on the agents running in terminals: focus one, or the next or previous.
    ///
    #[doc = example!(
        "app.agents().next().await?;",
        "app.agents().focus(\"t4\").await?;"
    )]
    #[must_use]
    pub fn agents(&self) -> AgentOps<T> {
        AgentOps::new(self.client.clone())
    }

    /// The operations on the layout: switch, duplicate, replace and reset it, and show, hide and
    /// resize its nodes.
    ///
    #[doc = example!(
        "app.layouts().hide(\"sidebar\").await?;",
        "app.layouts().resize_px(\"sidebar\", 240.0).await?;"
    )]
    #[must_use]
    pub fn layouts(&self) -> LayoutOps<T> {
        LayoutOps::new(self.client.clone())
    }

    /// The operations on mods: duplicate and open them.
    ///
    #[doc = example!(
        "app.mods().clone(\"files\").as_name(\"mine\").await?;",
        "app.mods().open(\"mine\").await?;"
    )]
    #[must_use]
    pub fn mods(&self) -> ModOps<T> {
        ModOps::new(self.client.clone())
    }

    /// The operations on overlays: close the dialog or popup on top.
    ///
    #[doc = example!("app.overlays().close().await?;")]
    #[must_use]
    pub fn overlays(&self) -> OverlayOps<T> {
        OverlayOps::new(self.client.clone())
    }

    /// The operations on the window and the app around it: its dialogs, theme, zoom and updates,
    /// and the window frame.
    ///
    #[doc = example!(
        "app.window().theme(\"win98\").await?;",
        "app.window().zoom_in().await?;"
    )]
    #[must_use]
    pub fn window(&self) -> WindowOps<T> {
        WindowOps::new(self.client.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::herdr_app::fake::{Script, app, success};
    use crate::herdr_app::{layouts, panes, spaces, tabs, terminals, window};
    use crate::protocol::KNOWN_COMMANDS;

    const STATUS: &str = r#"{
        "zoom": 1.0, "font_size": 14.0, "ui_font_size": 13.0,
        "space": null, "tab": null, "focused_pane": null,
        "layout": "main", "agent": null,
        "floats": {}, "overlays": [], "errors": []
    }"#;

    #[tokio::test]
    async fn from_client_checks_the_version_once() -> Result<(), Error> {
        let script = Script::new([success("herdr-app 0.1.17\n")]);
        let app = HerdrApp::from_client(Client::new(script.clone())).await?;

        assert_eq!(script.argvs(), [["--version"]]);
        assert_eq!(app.version(), &AppVersion::parse("herdr-app 0.1.17")?);

        let copy = app.clone();
        assert_eq!(copy.version(), app.version());
        assert_eq!(script.argvs().len(), 1);
        Ok(())
    }

    /// A transport that is deliberately not `Clone`.
    struct Unique(Script);

    impl Transport for Unique {
        fn run(
            &self,
            invocation: &crate::protocol::Invocation,
        ) -> impl std::future::Future<
            Output = Result<crate::protocol::RunOutput, crate::port::TransportError>,
        > + Send {
            self.0.run(invocation)
        }
    }

    #[tokio::test]
    async fn a_clone_runs_through_the_same_transport_even_when_the_transport_is_not_clone()
    -> Result<(), Error> {
        let script =
            Script::new([success("herdr-app 0.1.17\n")]).repeating(success(r#"{"ok":true}"#));
        let app = HerdrApp::from_client(Client::new(Unique(script.clone()))).await?;

        let copy = app.clone();
        copy.client().run(crate::protocol::TabNext::new()).await?;

        assert_eq!(
            script.argvs(),
            [vec!["--version"], vec!["do", "tab.next", "{}"]]
        );
        Ok(())
    }

    #[tokio::test]
    async fn from_client_refuses_old_versions() -> Result<(), &'static str> {
        let script = Script::new([success("herdr-app 0.1.15\n")]);
        let Err(error) = HerdrApp::from_client(Client::new(script.clone())).await else {
            return Err("an old version should be refused");
        };
        assert!(matches!(error, Error::UnsupportedVersion { .. }));
        assert_eq!(
            error.to_string(),
            "herdr-app 0.1.15 is older than 0.1.16, the oldest version this crate supports"
        );
        Ok(())
    }

    #[tokio::test]
    async fn ls_status_commands_delegate() -> Result<(), Error> {
        let script = Script::new([
            success("herdr-app 0.1.17\n"),
            success(r#"{"spaces":[],"recent":[]}"#),
            success(STATUS),
            success(r#"[{"name":"tab.next","args":[],"about":"Show the next tab."}]"#),
        ]);
        let app = HerdrApp::from_client(Client::new(script.clone())).await?;

        assert!(app.ls().await?.spaces.is_empty());
        assert_eq!(app.status().await?.layout.as_str(), "main");
        let commands = app.commands().await?;
        assert_eq!(commands[0].name.as_str(), "tab.next");
        assert_eq!(
            script.argvs(),
            [["--version"], ["ls"], ["status"], ["commands"]]
        );
        Ok(())
    }

    #[tokio::test]
    async fn client_runs_through_the_same_transport() -> Result<(), Error> {
        let script = Script::new([success("herdr-app 0.1.17\n"), success(r#"{"ok":true}"#)]);
        let app = HerdrApp::from_client(Client::new(script.clone())).await?;
        app.client().run(crate::protocol::TabNext::new()).await?;
        assert_eq!(script.argvs()[1], ["do", "tab.next", "{}"]);
        Ok(())
    }

    #[tokio::test]
    async fn every_command_is_reachable() -> Result<(), Error> {
        let (app, script) = app().await?;
        spaces::tests::run_every_method(&app, &script).await?;
        tabs::tests::run_every_method(&app, &script).await?;
        panes::tests::run_every_method(&app, &script).await?;
        terminals::tests::run_every_terminal_method(&app, &script).await?;
        terminals::tests::run_every_agent_method(&app, &script).await?;
        layouts::tests::run_every_method(&app, &script).await?;
        window::tests::run_every_mod_method(&app, &script).await?;
        window::tests::run_every_overlay_method(&app, &script).await?;
        window::tests::run_every_window_method(&app, &script).await?;

        let reached: BTreeSet<String> = script
            .argvs()
            .into_iter()
            .filter(|argv| argv.first().is_some_and(|first| first == "do"))
            .filter_map(|argv| argv.get(1).cloned())
            .collect();
        let known: BTreeSet<String> = KNOWN_COMMANDS
            .iter()
            .map(|command| command.name.to_owned())
            .collect();
        assert_eq!(known.len(), 77);
        assert_eq!(reached, known);
        Ok(())
    }
}
