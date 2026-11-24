//! `SpaceOps`: the operations on the window's spaces.
//!
//! A space is a workspace of tabs. The operations make, close, rename, move and select them. Each
//! returns a [`Call`] over the command of the same name, built from plain values: a name or an
//! identifier is a string, a position is a number counted from 1.
//!
//! Responsibilities:
//!
//! - [`SpaceOps`], one method per way of using the `space.*` commands
//!
//! Non-responsibilities:
//!
//! - the options of each command, which [`Call`] defines as setters
//! - running the commands, which [`Client`] does

use super::call::{Call, arg, example, ops_plumbing};
use crate::adapter::ProcessTransport;
use crate::driver::Client;
use crate::port::Transport;
use crate::protocol::{
    Position, Rename, SpaceClose, SpaceId, SpaceMove, SpaceName, SpaceNew, SpaceNext, SpacePrev,
    SpaceRef, SpaceRename, SpaceSelect,
};

/// The operations on the window's spaces.
///
/// Reached through [`HerdrApp::spaces`](super::HerdrApp::spaces). A name or an identifier is a
/// string, and an empty one is reported by the `.await` as [`Error::InvalidArgument`], as is the
/// position `0`.
///
/// # Examples
///
/// ```no_run
/// use herdrify_app::HerdrApp;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), herdrify_app::Error> {
/// let app = HerdrApp::local().await?;
/// let created = app.spaces().new_space().name("api").argv(["claude"]).await?;
/// app.spaces().select_id(created.space.as_str()).await?;
/// app.spaces().rename("backend").id(created.space.as_str()).await?;
/// # Ok(())
/// # }
/// ```
///
/// [`Error::InvalidArgument`]: crate::driver::Error::InvalidArgument
pub struct SpaceOps<T: Transport = ProcessTransport> {
    client: Client<T>,
}

ops_plumbing!(SpaceOps);

impl<T: Transport> SpaceOps<T> {
    /// `space.new`: makes a space.
    ///
    #[doc = example!("app.spaces().new_space().name(\"api\").await?;")]
    pub fn new_space(&self) -> Call<T, SpaceNew> {
        self.call(Ok(SpaceNew::new()))
    }

    /// `space.close`: closes the shown space.
    ///
    #[doc = example!("app.spaces().close().await?;")]
    pub fn close(&self) -> Call<T, SpaceClose> {
        self.call(Ok(SpaceClose::new()))
    }

    /// `space.rename`: renames the shown space to `name`, which must not be empty. Use
    /// [`rename_automatic`](Self::rename_automatic) to give the naming back to herdr-app.
    ///
    #[doc = example!("app.spaces().rename(\"api\").await?;")]
    pub fn rename(&self, name: impl Into<String>) -> Call<T, SpaceRename> {
        self.call(arg("name", SpaceName::new(name)).map(|name| SpaceRename::new(Rename::To(name))))
    }

    /// `space.rename`: lets herdr-app name the shown space again.
    ///
    #[doc = example!("app.spaces().rename_automatic().await?;")]
    pub fn rename_automatic(&self) -> Call<T, SpaceRename> {
        self.call(Ok(SpaceRename::new(Rename::Automatic)))
    }

    /// `space.move`: moves the space with the identifier `id` to position `index`, counted from 1.
    ///
    #[doc = example!("app.spaces().move_to(\"sp1\", 1).await?;")]
    pub fn move_to(&self, id: impl Into<String>, index: u32) -> Call<T, SpaceMove> {
        let id = arg("id", SpaceId::new(id));
        let index = arg("index", Position::new(index));
        self.call(id.and_then(|id| Ok(SpaceMove::new(id, index?))))
    }

    /// `space.select`: shows the space at position `index`, counted from 1.
    ///
    #[doc = example!("app.spaces().select_index(1).await?;")]
    pub fn select_index(&self, index: u32) -> Call<T, SpaceSelect> {
        self.call(
            arg("index", Position::new(index))
                .map(|index| SpaceSelect::new(SpaceRef::Index(index))),
        )
    }

    /// `space.select`: shows the space with the identifier `id`.
    ///
    #[doc = example!("app.spaces().select_id(\"sp1\").await?;")]
    pub fn select_id(&self, id: impl Into<String>) -> Call<T, SpaceSelect> {
        self.call(arg("id", SpaceId::new(id)).map(|id| SpaceSelect::new(SpaceRef::Id(id))))
    }

    /// `space.next`: shows the next space.
    ///
    #[doc = example!("app.spaces().next().await?;")]
    pub fn next(&self) -> Call<T, SpaceNext> {
        self.call(Ok(SpaceNext::new()))
    }

    /// `space.prev`: shows the previous space.
    ///
    #[doc = example!("app.spaces().prev().await?;")]
    pub fn prev(&self) -> Call<T, SpacePrev> {
        self.call(Ok(SpacePrev::new()))
    }
}

#[cfg(test)]
pub(super) mod tests {
    use serde_json::json;

    use crate::driver::Error;
    use crate::herdr_app::HerdrApp;
    use crate::herdr_app::fake::{Script, app, assert_sent};
    use crate::protocol::Focus;

    /// Runs every method of the group once against `app`, checking the command each one sent.
    ///
    /// Fails when the calls do not send one run each.
    pub(in crate::herdr_app) async fn run_every_method(
        app: &HerdrApp<Script>,
        script: &Script,
    ) -> Result<(), Error> {
        let first = script.argvs().len();
        let spaces = app.spaces();

        spaces
            .new_space()
            .name("api")
            .argv(["claude"])
            .focus(Focus::Background)
            .await?;
        assert_sent(
            script,
            "space.new",
            &json!({"argv": ["claude"], "focus": false, "name": "api"}),
        );

        spaces.new_space().await?;
        assert_sent(script, "space.new", &json!({}));

        spaces.close().await?;
        assert_sent(script, "space.close", &json!({}));

        spaces.rename("api").await?;
        assert_sent(script, "space.rename", &json!({"name": "api"}));

        spaces.rename_automatic().await?;
        assert_sent(script, "space.rename", &json!({"name": ""}));

        spaces.move_to("sp2", 3).await?;
        assert_sent(script, "space.move", &json!({"id": "sp2", "index": 3}));

        spaces.select_index(2).await?;
        assert_sent(script, "space.select", &json!({"index": 2}));

        spaces.select_id("sp1").await?;
        assert_sent(script, "space.select", &json!({"id": "sp1"}));

        spaces.next().await?;
        assert_sent(script, "space.next", &json!({}));

        spaces.prev().await?;
        assert_sent(script, "space.prev", &json!({}));

        assert_eq!(script.argvs().len() - first, 10);
        Ok(())
    }

    #[tokio::test]
    async fn every_space_method_builds_its_command() -> Result<(), Error> {
        let (app, script) = app().await?;
        run_every_method(&app, &script).await
    }

    #[tokio::test]
    async fn rename_refuses_empty() -> Result<(), Error> {
        let (app, script) = app().await?;
        let error = app.spaces().rename("").await.err();
        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some("invalid name: a space name cannot be empty")
        );
        assert_eq!(script.argvs(), [["--version"]]);
        Ok(())
    }

    #[tokio::test]
    async fn index_zero_is_invalid_argument() -> Result<(), Error> {
        let (app, script) = app().await?;
        let error = app.spaces().select_index(0).await.err();
        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some("invalid index: 0 is out of range: positions start at 1")
        );
        let error = app.spaces().move_to("sp1", 0).await.err();
        assert!(matches!(
            error,
            Some(Error::InvalidArgument { argument, .. }) if argument == "index"
        ));
        assert_eq!(script.argvs(), [["--version"]]);
        Ok(())
    }

    #[tokio::test]
    async fn empty_ids_name_the_argument_they_fill() -> Result<(), Error> {
        let (app, script) = app().await?;
        let error = app.spaces().select_id("").await.err();
        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some("invalid id: a space id cannot be empty")
        );
        let error = app.spaces().move_to("", 0).await.err();
        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some("invalid id: a space id cannot be empty")
        );
        assert_eq!(script.argvs(), [["--version"]]);
        Ok(())
    }

    #[tokio::test]
    async fn debug_names_the_group() -> Result<(), Error> {
        let (app, _script) = app().await?;
        assert_eq!(format!("{:?}", app.spaces()), "SpaceOps { .. }");
        Ok(())
    }
}
