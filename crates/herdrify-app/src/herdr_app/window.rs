//! `ModOps`, `OverlayOps` and `WindowOps`: the operations on mods, overlays and the window.
//!
//! A mod is a program that draws part of the window, an overlay is a dialog or popup shown above
//! it, and the window is the app itself: its help, settings, theme, zoom and updates, and the
//! window frame. Each operation returns a [`Call`] over the command of the same name, built from
//! plain values: a name is a string.
//!
//! Responsibilities:
//!
//! - [`ModOps`], one method per `mod.*` command
//! - [`OverlayOps`], the one method for `overlay.close`
//! - [`WindowOps`], one method per `app.*` and `window.*` command
//!
//! Non-responsibilities:
//!
//! - the options of each command, which [`Call`] defines as setters
//! - running the commands, which [`Client`] does

use super::call::{Call, arg, example, ops_plumbing};
use crate::adapter::ProcessTransport;
use crate::driver::{Client, Error};
use crate::files::{ModName, ThemeName};
use crate::port::Transport;
use crate::protocol::{
    AppCheckUpdates, AppFeedback, AppHelp, AppHide, AppHideOthers, AppPalette, AppQuit, AppReset,
    AppSettings, AppShowAll, AppTheme, AppUpdate, AppUpdateCore, AppWelcome, AppWhatsNew,
    AppZoomIn, AppZoomOut, AppZoomReset, ModClone, ModOpen, OverlayClose, WindowClose,
    WindowMaximize, WindowMinimize,
};

/// The name `name` of a mod, or the error for refusing it as the argument `name`.
fn module(name: impl Into<String>) -> Result<ModName, Error> {
    arg("name", ModName::new(name))
}

/// The operations on mods.
///
/// Reached through [`HerdrApp::mods`](super::HerdrApp::mods). A name is a string, and an empty one
/// is reported by the `.await` as [`Error::InvalidArgument`].
///
/// # Examples
///
/// ```no_run
/// use herdrify_app::HerdrApp;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), herdrify_app::Error> {
/// let app = HerdrApp::local().await?;
/// app.mods().clone("files").as_name("mine").await?;
/// app.mods().open("mine").await?;
/// # Ok(())
/// # }
/// ```
pub struct ModOps<T: Transport = ProcessTransport> {
    client: Client<T>,
}

ops_plumbing!(ModOps);

impl<T: Transport> ModOps<T> {
    /// `mod.clone`: duplicates the mod called `name`. The copy is called after the mod with
    /// `-copy` added, unless its call is given a name with `.as_name(..)`.
    ///
    #[doc = example!("app.mods().clone(\"files\").as_name(\"mine\").await?;")]
    pub fn clone(&self, name: impl Into<String>) -> Call<T, ModClone> {
        self.call(module(name).map(ModClone::new))
    }

    /// `mod.open`: opens the mod called `name` as the overlay its `mod.json` describes. Running it
    /// again closes the overlay.
    ///
    #[doc = example!("app.mods().open(\"files\").await?;")]
    pub fn open(&self, name: impl Into<String>) -> Call<T, ModOpen> {
        self.call(module(name).map(ModOpen::new))
    }
}

/// The operations on overlays.
///
/// Reached through [`HerdrApp::overlays`](super::HerdrApp::overlays).
///
/// # Examples
///
/// ```no_run
/// use herdrify_app::HerdrApp;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), herdrify_app::Error> {
/// let app = HerdrApp::local().await?;
/// app.overlays().close().await?;
/// # Ok(())
/// # }
/// ```
pub struct OverlayOps<T: Transport = ProcessTransport> {
    client: Client<T>,
}

ops_plumbing!(OverlayOps);

impl<T: Transport> OverlayOps<T> {
    /// `overlay.close`: closes the top dialog or popup. Another overlay is named with `.id(..)` on
    /// the call.
    ///
    #[doc = example!("app.overlays().close().id(\"frame-welcome\").await?;")]
    pub fn close(&self) -> Call<T, OverlayClose> {
        self.call(Ok(OverlayClose::new()))
    }
}

/// The operations on the window and the app around it.
///
/// Reached through [`HerdrApp::window`](super::HerdrApp::window). A name is a string, and an empty
/// one is reported by the `.await` as [`Error::InvalidArgument`].
///
/// Three commands end the running app or restart it: [`quit`](Self::quit) quits it on macOS,
/// [`close`](Self::close) closes the window, which on Linux quits the app, and
/// [`update`](Self::update) restarts into a downloaded update.
///
/// # Examples
///
/// ```no_run
/// use herdrify_app::HerdrApp;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), herdrify_app::Error> {
/// let app = HerdrApp::local().await?;
/// app.window().theme("win98").await?;
/// app.window().zoom_in().await?;
/// # Ok(())
/// # }
/// ```
pub struct WindowOps<T: Transport = ProcessTransport> {
    client: Client<T>,
}

ops_plumbing!(WindowOps);

impl<T: Transport> WindowOps<T> {
    /// `app.help`: toggles the keys dialog.
    ///
    #[doc = example!("app.window().help().await?;")]
    pub fn help(&self) -> Call<T, AppHelp> {
        self.call(Ok(AppHelp::new()))
    }

    /// `app.welcome`: shows the welcome again.
    ///
    #[doc = example!("app.window().welcome().await?;")]
    pub fn welcome(&self) -> Call<T, AppWelcome> {
        self.call(Ok(AppWelcome::new()))
    }

    /// `app.whats_new`: shows what this release brought.
    ///
    #[doc = example!("app.window().whats_new().await?;")]
    pub fn whats_new(&self) -> Call<T, AppWhatsNew> {
        self.call(Ok(AppWhatsNew::new()))
    }

    /// `app.feedback`: drafts feedback for the person to review and send. The text and who it
    /// comes from are `.message(..)` and `.from(..)` on the call.
    ///
    #[doc = example!(
        "use herdrify_app::protocol::FeedbackFrom;",
        "app.window().feedback().message(\"the tab bar flickers\").from(FeedbackFrom::Agent).await?;"
    )]
    pub fn feedback(&self) -> Call<T, AppFeedback> {
        self.call(Ok(AppFeedback::new()))
    }

    /// `app.palette`: toggles the command palette.
    ///
    #[doc = example!("app.window().palette().await?;")]
    pub fn palette(&self) -> Call<T, AppPalette> {
        self.call(Ok(AppPalette::new()))
    }

    /// `app.settings`: opens the settings, at a section when its call is given one with
    /// `.section(..)`.
    ///
    #[doc = example!(
        "use herdrify_app::protocol::SettingsSection;",
        "app.window().settings().section(SettingsSection::Alerts).await?;"
    )]
    pub fn settings(&self) -> Call<T, AppSettings> {
        self.call(Ok(AppSettings::new()))
    }

    /// `app.check_updates`: looks for a newer Herdr and herdr-core now.
    ///
    #[doc = example!("app.window().check_updates().await?;")]
    pub fn check_updates(&self) -> Call<T, AppCheckUpdates> {
        self.call(Ok(AppCheckUpdates::new()))
    }

    /// `app.update_core`: updates herdr-core to its latest release of this generation. The
    /// terminals keep running.
    ///
    #[doc = example!("app.window().update_core().await?;")]
    pub fn update_core(&self) -> Call<T, AppUpdateCore> {
        self.call(Ok(AppUpdateCore::new()))
    }

    /// `app.reset`: sets the shown layout and keys aside and goes back to the default screen.
    ///
    #[doc = example!("app.window().reset().await?;")]
    pub fn reset(&self) -> Call<T, AppReset> {
        self.call(Ok(AppReset::new()))
    }

    /// `app.theme`: uses the theme called `name`, a preset or one of the person's own.
    ///
    #[doc = example!("app.window().theme(\"win98\").await?;")]
    pub fn theme(&self, name: impl Into<String>) -> Call<T, AppTheme> {
        self.call(arg("name", ThemeName::new(name)).map(AppTheme::new))
    }

    /// `app.zoom_in`: makes everything bigger.
    ///
    #[doc = example!("app.window().zoom_in().await?;")]
    pub fn zoom_in(&self) -> Call<T, AppZoomIn> {
        self.call(Ok(AppZoomIn::new()))
    }

    /// `app.zoom_out`: makes everything smaller.
    ///
    #[doc = example!("app.window().zoom_out().await?;")]
    pub fn zoom_out(&self) -> Call<T, AppZoomOut> {
        self.call(Ok(AppZoomOut::new()))
    }

    /// `app.zoom_reset`: goes back to the normal size.
    ///
    #[doc = example!("app.window().zoom_reset().await?;")]
    pub fn zoom_reset(&self) -> Call<T, AppZoomReset> {
        self.call(Ok(AppZoomReset::new()))
    }

    /// `app.update`: restarts into the update Herdr downloaded. The terminals keep running.
    ///
    #[doc = example!("app.window().update().await?;")]
    pub fn update(&self) -> Call<T, AppUpdate> {
        self.call(Ok(AppUpdate::new()))
    }

    /// `app.quit`: quits the app, on macOS.
    ///
    #[doc = example!("app.window().quit().await?;")]
    pub fn quit(&self) -> Call<T, AppQuit> {
        self.call(Ok(AppQuit::new()))
    }

    /// `app.hide`: hides the app, on macOS.
    ///
    #[doc = example!("app.window().hide().await?;")]
    pub fn hide(&self) -> Call<T, AppHide> {
        self.call(Ok(AppHide::new()))
    }

    /// `app.hide_others`: hides the other apps, on macOS.
    ///
    #[doc = example!("app.window().hide_others().await?;")]
    pub fn hide_others(&self) -> Call<T, AppHideOthers> {
        self.call(Ok(AppHideOthers::new()))
    }

    /// `app.show_all`: shows all apps, on macOS.
    ///
    #[doc = example!("app.window().show_all().await?;")]
    pub fn show_all(&self) -> Call<T, AppShowAll> {
        self.call(Ok(AppShowAll::new()))
    }

    /// `window.close`: closes the window, which on Linux quits the app.
    ///
    #[doc = example!("app.window().close().await?;")]
    pub fn close(&self) -> Call<T, WindowClose> {
        self.call(Ok(WindowClose::new()))
    }

    /// `window.minimize`: minimizes the window.
    ///
    #[doc = example!("app.window().minimize().await?;")]
    pub fn minimize(&self) -> Call<T, WindowMinimize> {
        self.call(Ok(WindowMinimize::new()))
    }

    /// `window.maximize`: maximizes the window, or puts it back.
    ///
    #[doc = example!("app.window().maximize().await?;")]
    pub fn maximize(&self) -> Call<T, WindowMaximize> {
        self.call(Ok(WindowMaximize::new()))
    }
}

#[cfg(test)]
pub(super) mod tests {
    use serde_json::json;

    use super::*;
    use crate::herdr_app::HerdrApp;
    use crate::herdr_app::fake::{Script, app, assert_sent};

    /// Runs every method of the mod group once against `app`, checking the command each one sent.
    pub(in crate::herdr_app) async fn run_every_mod_method(
        app: &HerdrApp<Script>,
        script: &Script,
    ) -> Result<(), Error> {
        let first = script.argvs().len();
        let mods = app.mods();

        mods.clone("files").await?;
        assert_sent(script, "mod.clone", &json!({"name": "files"}));

        mods.open("files").await?;
        assert_sent(script, "mod.open", &json!({"name": "files"}));

        assert_eq!(script.argvs().len() - first, 2);
        Ok(())
    }

    /// Runs every method of the overlay group once against `app`, checking the command it sent.
    pub(in crate::herdr_app) async fn run_every_overlay_method(
        app: &HerdrApp<Script>,
        script: &Script,
    ) -> Result<(), Error> {
        let first = script.argvs().len();

        app.overlays().close().await?;
        assert_sent(script, "overlay.close", &json!({}));

        assert_eq!(script.argvs().len() - first, 1);
        Ok(())
    }

    /// Runs every method of the window group once against `app`, checking the command each one
    /// sent. The script answers; no window is reached.
    pub(in crate::herdr_app) async fn run_every_window_method(
        app: &HerdrApp<Script>,
        script: &Script,
    ) -> Result<(), Error> {
        let first = script.argvs().len();
        let window = app.window();

        window.help().await?;
        assert_sent(script, "app.help", &json!({}));

        window.welcome().await?;
        assert_sent(script, "app.welcome", &json!({}));

        window.whats_new().await?;
        assert_sent(script, "app.whats_new", &json!({}));

        window.feedback().await?;
        assert_sent(script, "app.feedback", &json!({}));

        window.palette().await?;
        assert_sent(script, "app.palette", &json!({}));

        window.settings().await?;
        assert_sent(script, "app.settings", &json!({}));

        window.check_updates().await?;
        assert_sent(script, "app.check_updates", &json!({}));

        window.update_core().await?;
        assert_sent(script, "app.update_core", &json!({}));

        window.reset().await?;
        assert_sent(script, "app.reset", &json!({}));

        window.theme("win98").await?;
        assert_sent(script, "app.theme", &json!({"name": "win98"}));

        window.zoom_in().await?;
        assert_sent(script, "app.zoom_in", &json!({}));

        window.zoom_out().await?;
        assert_sent(script, "app.zoom_out", &json!({}));

        window.zoom_reset().await?;
        assert_sent(script, "app.zoom_reset", &json!({}));

        window.update().await?;
        assert_sent(script, "app.update", &json!({}));

        window.quit().await?;
        assert_sent(script, "app.quit", &json!({}));

        window.hide().await?;
        assert_sent(script, "app.hide", &json!({}));

        window.hide_others().await?;
        assert_sent(script, "app.hide_others", &json!({}));

        window.show_all().await?;
        assert_sent(script, "app.show_all", &json!({}));

        window.close().await?;
        assert_sent(script, "window.close", &json!({}));

        window.minimize().await?;
        assert_sent(script, "window.minimize", &json!({}));

        window.maximize().await?;
        assert_sent(script, "window.maximize", &json!({}));

        assert_eq!(script.argvs().len() - first, 21);
        Ok(())
    }

    #[tokio::test]
    async fn every_mod_method_builds_its_command() -> Result<(), Error> {
        let (app, script) = app().await?;
        run_every_mod_method(&app, &script).await
    }

    #[tokio::test]
    async fn every_overlay_method_builds_its_command() -> Result<(), Error> {
        let (app, script) = app().await?;
        run_every_overlay_method(&app, &script).await
    }

    #[tokio::test]
    async fn every_window_method_builds_its_command() -> Result<(), Error> {
        let (app, script) = app().await?;
        run_every_window_method(&app, &script).await
    }

    #[tokio::test]
    async fn empty_names_name_the_argument_they_fill() -> Result<(), Error> {
        let (app, script) = app().await?;
        let error = app.window().theme("").await.err();
        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some("invalid name: a theme name cannot be empty")
        );
        let error = app.mods().clone("").await.err();
        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some("invalid name: a mod name cannot be empty")
        );
        let error = app.mods().open("").await.err();
        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some("invalid name: a mod name cannot be empty")
        );
        assert_eq!(script.argvs(), [["--version"]]);
        Ok(())
    }

    #[tokio::test]
    async fn debug_names_the_groups() -> Result<(), Error> {
        let (app, _script) = app().await?;
        assert_eq!(format!("{:?}", app.mods()), "ModOps { .. }");
        assert_eq!(format!("{:?}", app.overlays()), "OverlayOps { .. }");
        assert_eq!(format!("{:?}", app.window()), "WindowOps { .. }");
        Ok(())
    }
}
