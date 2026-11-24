//! `Call`: one command with its options, run through herdr-app when it is awaited.
//!
//! An operation of [`HerdrApp`](super::HerdrApp) returns a `Call` holding the command it built from
//! plain values. Each option is a method on the call that returns it again, so options chain. A
//! value an option cannot accept is kept as the call's error, the first one only, and reported when
//! the call is awaited without running anything. Awaiting the call runs the command through the
//! [`Client`] it was made from.
//!
//! Responsibilities:
//!
//! - [`Call`], its [`IntoFuture`] implementation and the plumbing of its options
//! - the options of each command, written as setters on the call for that command
//! - [`arg`], the conversion every operation shares: a refused value becomes an
//!   [`Error::InvalidArgument`] naming the argument
//!
//! Non-responsibilities:
//!
//! - which command an operation builds, which the file of its group decides
//! - running the command, which [`Client`] does

use std::fmt::{self, Display};
use std::future::{Future, IntoFuture};
use std::path::Path;
use std::pin::Pin;

use serde_json::{Map, Value};

use crate::driver::{Client, Error};
use crate::files::{LayoutName, ModName, TerminalAddress};
use crate::port::Transport;
use crate::protocol::{
    AppFeedback, AppSettings, Argv, At, Command, FeedbackFrom, Focus, Folder, LayoutClone,
    LayoutReset, Machine, MachineName, ModClone, OverlayClose, OverlayId, PaneClose, PaneContent,
    PaneId, PaneRename, PaneResize, PaneSplit, PaneSwap, PaneZoom, Place, Px, SelectMode,
    SettingsSection, SpaceClose, SpaceId, SpaceName, SpaceNew, SpaceRename, TabClose, TabContent,
    TabId, TabMove, TabName, TabNew, TabRename, TabSettle, TabUnsettle, Task,
    TerminalAdjustSelection, TerminalClearSelection, TerminalKeep, TerminalName, TerminalOpen,
    TerminalRightClick, TerminalScroll, TerminalSelect, TerminalSelectAll, TerminalShow,
};

/// A doc comment's `# Examples` section for an operation: `lines` run against an app named `app`.
///
/// The example is `no_run`, because it needs a herdr-app window to answer.
macro_rules! example {
    ($($line:literal),+ $(,)?) => {
        concat!(
            "# Examples\n\n",
            "```no_run\n",
            "# use herdrify_app::HerdrApp;\n",
            "# #[tokio::main(flavor = \"current_thread\")]\n",
            "# async fn main() -> Result<(), herdrify_app::Error> {\n",
            "let app = HerdrApp::local().await?;\n",
            $($line, "\n",)+
            "# Ok(())\n",
            "# }\n",
            "```\n",
        )
    };
}

pub(super) use example;

/// Defines what every group of operations shares: its constructor over a [`Client`], the private
/// `call` that wraps a built command into a [`Call`], and a [`Debug`](fmt::Debug) that does not
/// require the transport to be `Debug`. The struct, holding a `client: Client<T>` field, is
/// written by the caller.
macro_rules! ops_plumbing {
    ($ops:ident) => {
        impl<T: $crate::port::Transport> $ops<T> {
            /// The operations over `client`.
            pub(super) const fn new(client: $crate::driver::Client<T>) -> Self {
                Self { client }
            }

            /// A call that runs `command`, or reports the error building it recorded.
            fn call<C>(
                &self,
                command: ::core::result::Result<C, $crate::driver::Error>,
            ) -> $crate::herdr_app::Call<T, C> {
                $crate::herdr_app::Call::new(self.client.clone(), command)
            }
        }

        impl<T: $crate::port::Transport> ::core::fmt::Debug for $ops<T> {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.debug_struct(stringify!($ops)).finish_non_exhaustive()
            }
        }
    };
}

pub(super) use ops_plumbing;

/// A command and its options, run when awaited.
///
/// Every [`HerdrApp`](super::HerdrApp) operation returns one. Chain its options, then `.await` it;
/// nothing runs before that. A value an option could not accept, such as an empty name, is
/// reported by the `.await` as [`Error::InvalidArgument`] and nothing runs. When several are
/// refused, the first is the one reported.
///
/// # Examples
///
/// ```no_run
/// use herdrify_app::HerdrApp;
/// use herdrify_app::protocol::Focus;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), herdrify_app::Error> {
/// let app = HerdrApp::local().await?;
/// let created = app
///     .spaces()
///     .new_space()
///     .name("api")
///     .argv(["claude"])
///     .focus(Focus::Background)
///     .await?;
/// println!("{}", created.space);
/// # Ok(())
/// # }
/// ```
#[must_use = "a call runs nothing until it is awaited"]
pub struct Call<T: Transport, C> {
    client: Client<T>,
    command: Result<C, Error>,
}

impl<T: Transport, C> Call<T, C> {
    /// A call that runs `command` through `client`, or reports its error when awaited.
    pub(super) const fn new(client: Client<T>, command: Result<C, Error>) -> Self {
        Self { client, command }
    }

    /// Applies an option that cannot be refused. A call that already failed stays as it is.
    pub(super) fn with(self, apply: impl FnOnce(C) -> C) -> Self {
        Self {
            client: self.client,
            command: self.command.map(apply),
        }
    }

    /// Applies an option whose `value` may have been refused for the argument `argument`.
    ///
    /// The first refusal is kept: a call that already failed stays as it is, and a refused `value`
    /// on a call that has not failed becomes its error.
    pub(super) fn try_with<V, E: Display>(
        self,
        argument: &'static str,
        value: Result<V, E>,
        apply: impl FnOnce(C, V) -> C,
    ) -> Self {
        let command = match (self.command, value) {
            (Ok(command), Ok(value)) => Ok(apply(command, value)),
            (Err(first), _) => Err(first),
            (Ok(_), Err(refused)) => Err(invalid(argument, &refused)),
        };
        Self {
            client: self.client,
            command,
        }
    }
}

impl<T: Transport, C: fmt::Debug> fmt::Debug for Call<T, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Call")
            .field("command", &self.command)
            .finish_non_exhaustive()
    }
}

// dyn: `IntoFuture` must name its future type, and the future of an `async fn` cannot be named on
// stable Rust, so the future is boxed: the same type erasure as async-in-trait (justified
// exception 5 in the static-dispatch rules). It is one allocation per call, next to a round trip to
// another process.
type BoxedCall<O> = Pin<Box<dyn Future<Output = Result<O, Error>> + Send>>;

impl<T, C> IntoFuture for Call<T, C>
where
    T: Transport,
    C: Command + Send + 'static,
    C::Output: Send,
{
    type Output = Result<C::Output, Error>;
    type IntoFuture = BoxedCall<C::Output>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.client.run(self.command?).await })
    }
}

/// The error for a value refused for the argument `argument`, with the refusal's text as detail.
fn invalid(argument: &'static str, refused: &impl Display) -> Error {
    Error::InvalidArgument {
        argument: argument.to_owned(),
        detail: refused.to_string(),
    }
}

/// `value`, or the error for its refusal as the argument `argument`.
pub(super) fn arg<V, E: Display>(argument: &'static str, value: Result<V, E>) -> Result<V, Error> {
    value.map_err(|refused| invalid(argument, &refused))
}

/// Defines an option that converts its value, which may be refused for the argument `$argument`.
macro_rules! fallible {
    (
        $(#[$doc:meta])*
        $setter:ident($param:ident: $ty:ty) $argument:literal $value:expr => $apply:path;
        $($example:literal),+
    ) => {
        $(#[$doc])*
        ///
        #[doc = example!($($example),+)]
        pub fn $setter(self, $param: $ty) -> Self {
            self.try_with($argument, $value, $apply)
        }
    };
}

/// Defines an option whose value needs no conversion.
macro_rules! direct {
    (
        $(#[$doc:meta])*
        $setter:ident($param:ident: $ty:ty) => $apply:path;
        $($example:literal),+
    ) => {
        $(#[$doc])*
        ///
        #[doc = example!($($example),+)]
        pub fn $setter(self, $param: $ty) -> Self {
            self.with(|command| $apply(command, $param))
        }
    };
}

/// Defines an option that is a switch, with no value.
macro_rules! flag {
    (
        $(#[$doc:meta])*
        $setter:ident() => $apply:path;
        $($example:literal),+
    ) => {
        $(#[$doc])*
        ///
        #[doc = example!($($example),+)]
        pub fn $setter(self) -> Self {
            self.with($apply)
        }
    };
}

impl<T: Transport> Call<T, SpaceNew> {
    fallible! {
        /// Names the space.
        name(name: impl Into<String>) "name" SpaceName::new(name) => SpaceNew::name;
        "app.spaces().new_space().name(\"api\").await?;"
    }

    fallible! {
        /// Starts the space's first terminal in `folder`, which must be an absolute path.
        folder(folder: impl AsRef<Path>) "folder" Folder::new(folder) => SpaceNew::folder;
        "app.spaces().new_space().folder(\"/tmp\").await?;"
    }

    fallible! {
        /// Names the space's first tab.
        tab(tab: impl Into<String>) "tab" TabName::new(tab) => SpaceNew::tab;
        "app.spaces().new_space().tab(\"build\").await?;"
    }

    fallible! {
        /// Runs a program in the space's first terminal: the words of its command line, the first
        /// being the program.
        argv(argv: impl IntoIterator<Item = impl Into<String>>) "argv" Argv::new(argv)
            => SpaceNew::argv;
        "app.spaces().new_space().argv([\"claude\"]).await?;"
    }

    fallible! {
        /// Hands the space's first terminal a task to run.
        task(task: impl Into<String>) "task" Task::new(task) => SpaceNew::task;
        "app.spaces().new_space().task(\"fix the build\").await?;"
    }

    fallible! {
        /// Opens the space on the machine called `machine`. See
        /// [`this_computer`](Self::this_computer) for the local one.
        machine(machine: impl Into<String>) "machine"
            MachineName::new(machine).map(Machine::Named) => SpaceNew::machine;
        "app.spaces().new_space().machine(\"devbox\").await?;"
    }

    flag! {
        /// Opens the space on this computer.
        this_computer() => on_this_computer;
        "app.spaces().new_space().this_computer().await?;"
    }

    direct! {
        /// Whether the new space is shown or stays in the background.
        focus(focus: Focus) => SpaceNew::focus;
        "use herdrify_app::protocol::Focus;",
        "app.spaces().new_space().focus(Focus::Background).await?;"
    }
}

/// The command for `space.new` with its machine set to this computer.
fn on_this_computer(command: SpaceNew) -> SpaceNew {
    command.machine(Machine::ThisComputer)
}

impl<T: Transport> Call<T, SpaceClose> {
    fallible! {
        /// Closes the space with this identifier instead of the shown one.
        id(id: impl Into<String>) "id" SpaceId::new(id) => SpaceClose::id;
        "app.spaces().close().id(\"sp1\").await?;"
    }

    flag! {
        /// Closes the space even when herdr-app would ask the person to confirm first.
        confirmed() => SpaceClose::confirmed;
        "app.spaces().close().confirmed().await?;"
    }
}

impl<T: Transport> Call<T, SpaceRename> {
    fallible! {
        /// Renames the space with this identifier instead of the shown one.
        id(id: impl Into<String>) "id" SpaceId::new(id) => SpaceRename::id;
        "app.spaces().rename(\"api\").id(\"sp1\").await?;"
    }
}

impl<T: Transport> Call<T, TabNew> {
    fallible! {
        /// Names the tab.
        name(name: impl Into<String>) "name" TabName::new(name) => TabNew::name;
        "app.tabs().new_tab().name(\"build\").await?;"
    }

    fallible! {
        /// Makes the tab in the space with this identifier instead of the shown one.
        space(space: impl Into<String>) "space" SpaceId::new(space) => TabNew::space;
        "app.tabs().new_tab().space(\"sp1\").await?;"
    }

    direct! {
        /// Whether the new tab is shown or stays in the background.
        focus(focus: Focus) => TabNew::focus;
        "use herdrify_app::protocol::Focus;",
        "app.tabs().new_tab().focus(Focus::Background).await?;"
    }

    fallible! {
        /// Runs a program in the tab: the words of its command line, the first being the program.
        ///
        /// Changes nothing on a tab that shows a mod, because herdr-app takes the mod over the
        /// program.
        argv(argv: impl IntoIterator<Item = impl Into<String>>) "argv" Argv::new(argv)
            => tab_argv;
        "app.tabs().new_tab().argv([\"htop\"]).await?;"
    }

    fallible! {
        /// Hands the tab's terminal a task to run.
        ///
        /// Changes nothing on a tab that shows a mod, because herdr-app takes the mod over the
        /// program.
        task(task: impl Into<String>) "task" Task::new(task) => tab_task;
        "app.tabs().new_tab().task(\"fix the build\").await?;"
    }

    fallible! {
        /// Starts the tab's terminal in `folder`, which must be an absolute path.
        ///
        /// Changes nothing on a tab that shows a mod, because herdr-app takes the mod over the
        /// program.
        folder(folder: impl AsRef<Path>) "folder" Folder::new(folder) => tab_folder;
        "app.tabs().new_tab().folder(\"/tmp\").await?;"
    }

    direct! {
        /// Gives the mod the tab shows these properties.
        ///
        /// Changes nothing on a tab that runs a program.
        props(props: Map<String, Value>) => tab_props;
        "let mut props = serde_json::Map::new();",
        "props.insert(\"title\".to_owned(), serde_json::json!(\"files\"));",
        "app.tabs().new_mod_tab(\"files\").props(props).await?;"
    }
}

/// The command for `tab.new` with the program's arguments set, when the tab runs a program.
fn tab_argv(mut command: TabNew, argv: Argv) -> TabNew {
    if let TabContent::Shell { argv: slot, .. } = command.content_mut() {
        *slot = Some(argv);
    }
    command
}

/// The command for `tab.new` with the task set, when the tab runs a program.
fn tab_task(mut command: TabNew, task: Task) -> TabNew {
    if let TabContent::Shell { task: slot, .. } = command.content_mut() {
        *slot = Some(task);
    }
    command
}

/// The command for `tab.new` with the folder set, when the tab runs a program.
fn tab_folder(mut command: TabNew, folder: Folder) -> TabNew {
    if let TabContent::Shell { folder: slot, .. } = command.content_mut() {
        *slot = Some(folder);
    }
    command
}

/// The command for `tab.new` with the mod's properties set, when the tab shows a mod.
fn tab_props(mut command: TabNew, props: Map<String, Value>) -> TabNew {
    if let TabContent::Mod { props: slot, .. } = command.content_mut() {
        *slot = Some(props);
    }
    command
}

impl<T: Transport> Call<T, TabClose> {
    fallible! {
        /// Closes the tab with this identifier instead of the shown one.
        id(id: impl Into<String>) "id" TabId::new(id) => TabClose::id;
        "app.tabs().close().id(\"t1\").await?;"
    }

    flag! {
        /// Closes the tab even when herdr-app would ask the person to confirm first.
        confirmed() => TabClose::confirmed;
        "app.tabs().close().confirmed().await?;"
    }
}

impl<T: Transport> Call<T, TabRename> {
    fallible! {
        /// Renames the tab with this identifier instead of the shown one.
        id(id: impl Into<String>) "id" TabId::new(id) => TabRename::id;
        "app.tabs().rename(\"build\").id(\"t1\").await?;"
    }
}

impl<T: Transport> Call<T, TabMove> {
    fallible! {
        /// Moves the tab with this identifier instead of the shown one.
        id(id: impl Into<String>) "id" TabId::new(id) => TabMove::id;
        "app.tabs().move_by(-1).id(\"t1\").await?;"
    }
}

impl<T: Transport> Call<T, TabSettle> {
    fallible! {
        /// Puts the tab with this identifier away instead of the shown one.
        id(id: impl Into<String>) "id" TabId::new(id) => TabSettle::id;
        "app.tabs().settle().id(\"t1\").await?;"
    }
}

impl<T: Transport> Call<T, TabUnsettle> {
    fallible! {
        /// Brings back the tab with this identifier instead of the shown one.
        id(id: impl Into<String>) "id" TabId::new(id) => TabUnsettle::id;
        "app.tabs().unsettle().id(\"t1\").await?;"
    }
}

impl<T: Transport> Call<T, PaneSplit> {
    fallible! {
        /// Splits the pane with this identifier instead of the focused one.
        pane(pane: impl Into<String>) "pane" PaneId::new(pane) => PaneSplit::pane;
        "use herdrify_app::protocol::SplitDirection;",
        "app.panes().split(SplitDirection::Right).pane(\"p1\").await?;"
    }

    fallible! {
        /// Runs a program in the new pane: the words of its command line, the first being the
        /// program.
        ///
        /// Changes nothing on a pane that shows a mod, because herdr-app takes the mod over the
        /// program.
        argv(argv: impl IntoIterator<Item = impl Into<String>>) "argv" Argv::new(argv)
            => pane_argv;
        "use herdrify_app::protocol::SplitDirection;",
        "app.panes().split(SplitDirection::Right).argv([\"htop\"]).await?;"
    }

    direct! {
        /// Gives the mod the new pane shows these properties.
        ///
        /// Changes nothing on a pane that runs a program.
        props(props: Map<String, Value>) => pane_props;
        "use herdrify_app::protocol::SplitDirection;",
        "let mut props = serde_json::Map::new();",
        "props.insert(\"title\".to_owned(), serde_json::json!(\"files\"));",
        "app.panes().split_mod(SplitDirection::Right, \"files\").props(props).await?;"
    }
}

/// The command for `pane.split` with the program's arguments set, when the pane runs a program.
fn pane_argv(mut command: PaneSplit, argv: Argv) -> PaneSplit {
    if let PaneContent::Shell { argv: slot } = command.content_mut() {
        *slot = Some(argv);
    }
    command
}

/// The command for `pane.split` with the mod's properties set, when the pane shows a mod.
fn pane_props(mut command: PaneSplit, props: Map<String, Value>) -> PaneSplit {
    if let PaneContent::Mod { props: slot, .. } = command.content_mut() {
        *slot = Some(props);
    }
    command
}

impl<T: Transport> Call<T, PaneClose> {
    fallible! {
        /// Closes the pane with this identifier instead of the focused one.
        pane(pane: impl Into<String>) "pane" PaneId::new(pane) => PaneClose::pane;
        "app.panes().close().pane(\"p1\").await?;"
    }
}

impl<T: Transport> Call<T, PaneRename> {
    fallible! {
        /// Renames the pane with this identifier instead of the focused one.
        pane(pane: impl Into<String>) "pane" PaneId::new(pane) => PaneRename::pane;
        "app.panes().rename(\"logs\").pane(\"p1\").await?;"
    }
}

impl<T: Transport> Call<T, PaneResize> {
    fallible! {
        /// Resizes the pane with this identifier instead of the focused one.
        pane(pane: impl Into<String>) "pane" PaneId::new(pane) => PaneResize::pane;
        "use herdrify_app::protocol::Direction;",
        "app.panes().resize(Direction::Left).pane(\"p1\").await?;"
    }

    fallible! {
        /// Resizes by this many pixels, which must be finite and not negative.
        px(px: f64) "px" Px::new(px) => PaneResize::px;
        "use herdrify_app::protocol::Direction;",
        "app.panes().resize(Direction::Left).px(30.0).await?;"
    }
}

impl<T: Transport> Call<T, PaneZoom> {
    fallible! {
        /// Zooms the pane with this identifier instead of the focused one.
        pane(pane: impl Into<String>) "pane" PaneId::new(pane) => PaneZoom::pane;
        "app.panes().zoom().pane(\"p1\").await?;"
    }
}

impl<T: Transport> Call<T, PaneSwap> {
    fallible! {
        /// Swaps the pane with this identifier instead of the focused one.
        pane(pane: impl Into<String>) "pane" PaneId::new(pane) => PaneSwap::pane;
        "use herdrify_app::protocol::Direction;",
        "app.panes().swap(Direction::Left).pane(\"p1\").await?;"
    }
}

impl<T: Transport> Call<T, TerminalOpen> {
    fallible! {
        /// Runs this program: the words of its command line, the first being the program.
        argv(argv: impl IntoIterator<Item = impl Into<String>>) "argv" Argv::new(argv)
            => TerminalOpen::argv;
        "app.terminals().open().argv([\"top\"]).await?;"
    }

    fallible! {
        /// Starts the terminal in `folder`, which must be an absolute path.
        folder(folder: impl AsRef<Path>) "folder" Folder::new(folder) => TerminalOpen::folder;
        "app.terminals().open().folder(\"/tmp\").await?;"
    }

    direct! {
        /// Puts the terminal here: in a new tab, or right of or below the focused pane.
        place(place: Place) => TerminalOpen::place;
        "use herdrify_app::protocol::Place;",
        "app.terminals().open().place(Place::Right).await?;"
    }

    fallible! {
        /// Names the terminal.
        name(name: impl Into<String>) "name" TerminalName::new(name) => TerminalOpen::name;
        "app.terminals().open().name(\"monitor\").await?;"
    }
}

impl<T: Transport> Call<T, TerminalKeep> {
    direct! {
        /// Puts the terminal's new pane here: in a new tab, or right of or below the focused
        /// pane.
        place(place: Place) => TerminalKeep::place;
        "use herdrify_app::protocol::Place;",
        "app.terminals().keep(\"t4\").place(Place::Down).await?;"
    }
}

impl<T: Transport> Call<T, TerminalShow> {
    fallible! {
        /// Shows the terminal at this address.
        terminal(terminal: impl Into<String>) "terminal"
            TerminalAddress::new(terminal) => TerminalShow::terminal;
        "app.terminals().show().terminal(\"t4\").await?;"
    }

    fallible! {
        /// Shows it in the pane with this identifier.
        pane(pane: impl Into<String>) "pane" PaneId::new(pane) => TerminalShow::pane;
        "app.terminals().show().pane(\"p1\").await?;"
    }
}

impl<T: Transport> Call<T, TerminalRightClick> {
    fallible! {
        /// Right-clicks the terminal at this address instead of the focused one.
        terminal(terminal: impl Into<String>) "terminal"
            TerminalAddress::new(terminal) => TerminalRightClick::terminal;
        "use herdrify_app::protocol::ClickTarget;",
        "app.terminals().right_click(ClickTarget::Menu).terminal(\"t4\").await?;"
    }
}

impl<T: Transport> Call<T, TerminalSelectAll> {
    fallible! {
        /// Selects in the terminal at this address instead of the focused one.
        terminal(terminal: impl Into<String>) "terminal"
            TerminalAddress::new(terminal) => TerminalSelectAll::terminal;
        "app.terminals().select_all().terminal(\"t4\").await?;"
    }
}

impl<T: Transport> Call<T, TerminalSelect> {
    fallible! {
        /// Selects in the terminal at this address instead of the focused one.
        terminal(terminal: impl Into<String>) "terminal"
            TerminalAddress::new(terminal) => TerminalSelect::terminal;
        "app.terminals().select().terminal(\"t4\").await?;"
    }

    direct! {
        /// Starts the selection here: at the cursor or at a cell.
        at(at: At) => TerminalSelect::at;
        "use herdrify_app::protocol::At;",
        "app.terminals().select().at(At::Cursor).await?;"
    }

    direct! {
        /// Selects by cell, word or line, as a single, double or triple click does.
        mode(mode: SelectMode) => TerminalSelect::mode;
        "use herdrify_app::protocol::SelectMode;",
        "app.terminals().select().mode(SelectMode::Word).await?;"
    }
}

impl<T: Transport> Call<T, TerminalAdjustSelection> {
    fallible! {
        /// Adjusts the selection of the terminal at this address instead of the focused one.
        terminal(terminal: impl Into<String>) "terminal"
            TerminalAddress::new(terminal) => TerminalAdjustSelection::terminal;
        "use herdrify_app::protocol::SelectionMove;",
        "app.terminals().adjust_selection(SelectionMove::End).terminal(\"t4\").await?;"
    }
}

impl<T: Transport> Call<T, TerminalClearSelection> {
    fallible! {
        /// Clears the selection of the terminal at this address instead of the focused one.
        terminal(terminal: impl Into<String>) "terminal"
            TerminalAddress::new(terminal) => TerminalClearSelection::terminal;
        "app.terminals().clear_selection().terminal(\"t4\").await?;"
    }
}

impl<T: Transport> Call<T, TerminalScroll> {
    fallible! {
        /// Scrolls the terminal at this address instead of the focused one.
        terminal(terminal: impl Into<String>) "terminal"
            TerminalAddress::new(terminal) => TerminalScroll::terminal;
        "app.terminals().scroll_lines(-10).terminal(\"t4\").await?;"
    }
}

impl<T: Transport> Call<T, LayoutClone> {
    fallible! {
        /// Calls the copy `name`. The setter is not `as`, which is a Rust keyword, and a refusal
        /// still names the argument `as`.
        as_name(name: impl Into<String>) "as" LayoutName::new(name) => LayoutClone::as_name;
        "app.layouts().clone(\"main\").as_name(\"mine\").await?;"
    }
}

impl<T: Transport> Call<T, LayoutReset> {
    fallible! {
        /// Resets the layout called `name` instead of the shown one.
        name(name: impl Into<String>) "name" LayoutName::new(name) => LayoutReset::name;
        "app.layouts().reset().name(\"main\").await?;"
    }
}

impl<T: Transport> Call<T, ModClone> {
    fallible! {
        /// Calls the copy `name`. The setter is not `as`, which is a Rust keyword, and a refusal
        /// still names the argument `as`.
        as_name(name: impl Into<String>) "as" ModName::new(name) => ModClone::as_name;
        "app.mods().clone(\"files\").as_name(\"mine\").await?;"
    }
}

impl<T: Transport> Call<T, OverlayClose> {
    fallible! {
        /// Closes the overlay with this identifier instead of the top dialog or popup.
        id(id: impl Into<String>) "id" OverlayId::new(id) => OverlayClose::id;
        "app.overlays().close().id(\"frame-welcome\").await?;"
    }
}

impl<T: Transport> Call<T, AppSettings> {
    direct! {
        /// Opens the settings at this section.
        section(section: SettingsSection) => AppSettings::section;
        "use herdrify_app::protocol::SettingsSection;",
        "app.window().settings().section(SettingsSection::Alerts).await?;"
    }
}

impl<T: Transport> Call<T, AppFeedback> {
    direct! {
        /// Starts the draft with this text.
        message(message: impl Into<String>) => AppFeedback::message;
        "app.window().feedback().message(\"the tab bar flickers\").await?;"
    }

    direct! {
        /// Says who the feedback comes from.
        from(from: FeedbackFrom) => AppFeedback::from;
        "use herdrify_app::protocol::FeedbackFrom;",
        "app.window().feedback().from(FeedbackFrom::Person).await?;"
    }
}

#[cfg(test)]
mod tests {
    use std::future::IntoFuture;

    use serde_json::json;

    use super::*;
    use crate::herdr_app::fake::{Script, app, assert_sent};
    use crate::protocol::{
        At, ClickTarget, Direction, FeedbackFrom, Place, SelectMode, SelectionMove,
        SettingsSection, SplitDirection,
    };

    /// The text of the refusal a call returned, or a description of what it did instead.
    async fn refusal<C>(call: Call<Script, C>) -> String
    where
        C: Command + Send + 'static,
        C::Output: Send,
    {
        match call.await {
            Err(error @ Error::InvalidArgument { .. }) => error.to_string(),
            Err(other) => format!("another error: {other}"),
            Ok(_) => "no error".to_owned(),
        }
    }

    /// The text of a refusal of the argument `argument` for the reason `detail`.
    fn refused(argument: &str, detail: &str) -> String {
        format!("invalid {argument}: {detail}")
    }

    fn assert_send_static<F: Send + 'static>(_future: &F) {}

    /// Fails to compile unless the future of a call is `Send + 'static` for any transport and any
    /// command whose output is `Send`.
    fn assert_future_is_send<T: Transport, C>(call: Call<T, C>)
    where
        C: Command + Send + 'static,
        C::Output: Send,
    {
        assert_send_static(&call.into_future());
    }

    #[tokio::test]
    async fn awaiting_runs_the_command_and_returns_its_reply() -> Result<(), Error> {
        let (app, script) = app().await?;
        let created = app
            .spaces()
            .new_space()
            .name("api")
            .argv(["claude"])
            .focus(Focus::Background)
            .await?;
        assert_eq!(created.space.as_str(), "sp1");
        assert_sent(
            &script,
            "space.new",
            &json!({"argv": ["claude"], "focus": false, "name": "api"}),
        );
        Ok(())
    }

    #[tokio::test]
    async fn call_keeps_the_first_refusal() -> Result<(), Error> {
        let (app, script) = app().await?;
        let call = app.spaces().new_space().name("").name("ok");
        assert!(format!("{call:?}").starts_with("Call { command: Err(InvalidArgument"));
        let Err(error) = call.await else {
            return Err(Error::Cli {
                message: "the call should have been refused".to_owned(),
            });
        };
        assert_eq!(
            error.to_string(),
            "invalid name: a space name cannot be empty"
        );
        assert!(matches!(
            error,
            Error::InvalidArgument { argument, detail }
                if argument == "name" && detail == "a space name cannot be empty"
        ));
        assert_eq!(script.argvs(), [["--version"]]);

        let later = refusal(app.spaces().new_space().name("").tab("")).await;
        assert_eq!(later, refused("name", "a space name cannot be empty"));
        let reversed = refusal(app.spaces().new_space().tab("").name("")).await;
        assert_eq!(reversed, refused("tab", "a tab name cannot be empty"));
        assert_eq!(script.argvs(), [["--version"]]);
        Ok(())
    }

    #[tokio::test]
    async fn call_future_is_send() -> Result<(), Error> {
        let (app, script) = app().await?;
        assert_future_is_send(app.spaces().next());
        assert_future_is_send(app.spaces().new_space().name("api"));

        let spawned = tokio::spawn(app.spaces().next().into_future());
        spawned.await.map_err(|error| Error::Cli {
            message: error.to_string(),
        })??;
        assert_sent(&script, "space.next", &json!({}));
        Ok(())
    }

    #[tokio::test]
    async fn relative_folder_is_invalid_argument() -> Result<(), Error> {
        let (app, script) = app().await?;
        let call = app.spaces().new_space().folder("x/y");
        let Err(error) = call.await else {
            return Err(Error::Cli {
                message: "the call should have been refused".to_owned(),
            });
        };
        assert_eq!(
            error.to_string(),
            "invalid folder: \"x/y\" is not an absolute path"
        );
        assert_eq!(script.argvs(), [["--version"]]);
        Ok(())
    }

    #[tokio::test]
    async fn empty_argv_is_invalid_argument() -> Result<(), Error> {
        let (app, script) = app().await?;
        let call = app.spaces().new_space().argv(Vec::<String>::new());
        let Err(error) = call.await else {
            return Err(Error::Cli {
                message: "the call should have been refused".to_owned(),
            });
        };
        assert_eq!(error.to_string(), "invalid argv: a argv cannot be empty");
        assert_eq!(script.argvs(), [["--version"]]);
        Ok(())
    }

    #[tokio::test]
    async fn space_setters_set_their_arguments() -> Result<(), Error> {
        let (app, script) = app().await?;
        let folder = std::env::temp_dir();
        app.spaces()
            .new_space()
            .name("api")
            .folder(&folder)
            .tab("build")
            .argv(["htop", "--tree"])
            .task("fix the build")
            .machine("devbox")
            .focus(Focus::Show)
            .await?;
        assert_sent(
            &script,
            "space.new",
            &json!({
                "name": "api",
                "folder": folder.to_str(),
                "tab": "build",
                "argv": ["htop", "--tree"],
                "task": "fix the build",
                "machine": "devbox",
                "focus": true,
            }),
        );

        app.spaces()
            .new_space()
            .machine("devbox")
            .this_computer()
            .await?;
        assert_sent(&script, "space.new", &json!({"machine": ""}));

        app.spaces().close().id("sp1").confirmed().await?;
        assert_sent(
            &script,
            "space.close",
            &json!({"id": "sp1", "confirmed": true}),
        );
        app.spaces().rename("api").id("sp1").await?;
        assert_sent(
            &script,
            "space.rename",
            &json!({"id": "sp1", "name": "api"}),
        );
        Ok(())
    }

    #[tokio::test]
    async fn tab_setters_set_their_arguments() -> Result<(), Error> {
        let (app, script) = app().await?;
        app.tabs()
            .new_tab()
            .name("build")
            .space("sp1")
            .focus(Focus::Background)
            .await?;
        assert_sent(
            &script,
            "tab.new",
            &json!({"name": "build", "space": "sp1", "focus": false}),
        );

        app.tabs().close().id("t1").confirmed().await?;
        assert_sent(
            &script,
            "tab.close",
            &json!({"id": "t1", "confirmed": true}),
        );
        app.tabs().rename("build").id("t1").await?;
        assert_sent(&script, "tab.rename", &json!({"id": "t1", "name": "build"}));
        app.tabs().move_to(2).id("t1").await?;
        assert_sent(&script, "tab.move", &json!({"id": "t1", "index": 2}));
        app.tabs().settle().id("t1").await?;
        assert_sent(&script, "tab.settle", &json!({"id": "t1"}));
        app.tabs().unsettle().id("t1").await?;
        assert_sent(&script, "tab.unsettle", &json!({"id": "t1"}));
        Ok(())
    }

    #[tokio::test]
    async fn pane_setters_set_their_arguments() -> Result<(), Error> {
        let (app, script) = app().await?;
        app.panes().split(SplitDirection::Right).pane("p1").await?;
        assert_sent(
            &script,
            "pane.split",
            &json!({"direction": "right", "pane": "p1"}),
        );
        app.panes().close().pane("p1").await?;
        assert_sent(&script, "pane.close", &json!({"pane": "p1"}));
        app.panes().rename("logs").pane("p1").await?;
        assert_sent(
            &script,
            "pane.rename",
            &json!({"pane": "p1", "name": "logs"}),
        );
        app.panes()
            .resize(Direction::Left)
            .pane("p1")
            .px(30.0)
            .await?;
        assert_sent(
            &script,
            "pane.resize",
            &json!({"direction": "left", "pane": "p1", "px": 30.0}),
        );
        app.panes().zoom().pane("p1").await?;
        assert_sent(&script, "pane.zoom", &json!({"pane": "p1"}));
        app.panes().swap(Direction::Up).pane("p1").await?;
        assert_sent(
            &script,
            "pane.swap",
            &json!({"pane": "p1", "direction": "up"}),
        );
        Ok(())
    }

    /// The properties `{"title": "files"}`.
    fn props() -> serde_json::Map<String, serde_json::Value> {
        let mut props = serde_json::Map::new();
        props.insert("title".to_owned(), json!("files"));
        props
    }

    #[tokio::test]
    async fn tab_content_setters_set_the_program_and_the_mod_properties() -> Result<(), Error> {
        let (app, script) = app().await?;
        let folder = std::env::temp_dir();
        app.tabs()
            .new_tab()
            .argv(["htop", "--tree"])
            .task("watch the load")
            .folder(&folder)
            .name("load")
            .await?;
        assert_sent(
            &script,
            "tab.new",
            &json!({
                "name": "load",
                "argv": ["htop", "--tree"],
                "task": "watch the load",
                "folder": folder.to_str(),
            }),
        );

        app.tabs().new_mod_tab("files").props(props()).await?;
        assert_sent(
            &script,
            "tab.new",
            &json!({"mod": "files", "props": {"title": "files"}}),
        );
        Ok(())
    }

    #[tokio::test]
    async fn pane_content_setters_set_the_program_and_the_mod_properties() -> Result<(), Error> {
        let (app, script) = app().await?;
        app.panes()
            .split(SplitDirection::Right)
            .argv(["htop"])
            .pane("p1")
            .await?;
        assert_sent(
            &script,
            "pane.split",
            &json!({"direction": "right", "pane": "p1", "argv": ["htop"]}),
        );

        app.panes()
            .split_mod(SplitDirection::Down, "files")
            .props(props())
            .await?;
        assert_sent(
            &script,
            "pane.split",
            &json!({"direction": "down", "mod": "files", "props": {"title": "files"}}),
        );
        Ok(())
    }

    #[tokio::test]
    async fn a_setter_of_the_other_form_changes_nothing() -> Result<(), Error> {
        let (app, script) = app().await?;
        let folder = std::env::temp_dir();
        app.tabs()
            .new_mod_tab("files")
            .argv(["htop"])
            .task("t")
            .folder(&folder)
            .await?;
        assert_sent(&script, "tab.new", &json!({"mod": "files"}));
        app.tabs().new_tab().props(props()).await?;
        assert_sent(&script, "tab.new", &json!({}));

        app.panes()
            .split_mod(SplitDirection::Down, "files")
            .argv(["htop"])
            .await?;
        assert_sent(
            &script,
            "pane.split",
            &json!({"direction": "down", "mod": "files"}),
        );
        app.panes()
            .split(SplitDirection::Down)
            .props(props())
            .await?;
        assert_sent(&script, "pane.split", &json!({"direction": "down"}));
        Ok(())
    }

    #[tokio::test]
    async fn the_refused_content_setters_name_their_wire_argument() -> Result<(), Error> {
        let (app, script) = app().await?;
        assert_eq!(
            refusal(app.tabs().new_tab().argv(Vec::<String>::new())).await,
            refused("argv", "a argv cannot be empty")
        );
        assert_eq!(
            refusal(app.tabs().new_tab().task("")).await,
            refused("task", "a task cannot be empty")
        );
        assert_eq!(
            refusal(app.tabs().new_tab().folder("x/y")).await,
            refused("folder", "\"x/y\" is not an absolute path")
        );
        assert_eq!(
            refusal(
                app.panes()
                    .split(SplitDirection::Right)
                    .argv(Vec::<String>::new())
            )
            .await,
            refused("argv", "a argv cannot be empty")
        );
        assert_eq!(script.argvs(), [["--version"]]);
        Ok(())
    }

    #[tokio::test]
    async fn every_refused_setter_names_its_wire_argument() -> Result<(), Error> {
        let (app, script) = app().await?;
        assert_eq!(
            refusal(app.spaces().new_space().tab("")).await,
            refused("tab", "a tab name cannot be empty")
        );
        assert_eq!(
            refusal(app.spaces().new_space().task("")).await,
            refused("task", "a task cannot be empty")
        );
        assert_eq!(
            refusal(app.spaces().new_space().machine("")).await,
            refused("machine", "a machine name cannot be empty")
        );
        assert_eq!(
            refusal(app.spaces().close().id("")).await,
            refused("id", "a space id cannot be empty")
        );
        assert_eq!(
            refusal(app.spaces().rename("api").id("")).await,
            refused("id", "a space id cannot be empty")
        );
        assert_eq!(
            refusal(app.tabs().new_tab().name("")).await,
            refused("name", "a tab name cannot be empty")
        );
        assert_eq!(
            refusal(app.tabs().new_tab().space("")).await,
            refused("space", "a space id cannot be empty")
        );
        assert_eq!(
            refusal(app.tabs().close().id("")).await,
            refused("id", "a tab id cannot be empty")
        );
        assert_eq!(
            refusal(app.tabs().rename("build").id("")).await,
            refused("id", "a tab id cannot be empty")
        );
        assert_eq!(
            refusal(app.tabs().move_by(1).id("")).await,
            refused("id", "a tab id cannot be empty")
        );
        assert_eq!(
            refusal(app.tabs().settle().id("")).await,
            refused("id", "a tab id cannot be empty")
        );
        assert_eq!(
            refusal(app.tabs().unsettle().id("")).await,
            refused("id", "a tab id cannot be empty")
        );
        assert_eq!(
            refusal(app.panes().split(SplitDirection::Down).pane("")).await,
            refused("pane", "a pane id cannot be empty")
        );
        assert_eq!(
            refusal(app.panes().close().pane("")).await,
            refused("pane", "a pane id cannot be empty")
        );
        assert_eq!(
            refusal(app.panes().rename("logs").pane("")).await,
            refused("pane", "a pane id cannot be empty")
        );
        assert_eq!(
            refusal(app.panes().resize(Direction::Up).pane("")).await,
            refused("pane", "a pane id cannot be empty")
        );
        assert_eq!(
            refusal(app.panes().resize(Direction::Up).px(-5.0)).await,
            refused("px", "-5 is out of range: must be finite and not negative")
        );
        assert_eq!(
            refusal(app.panes().zoom().pane("")).await,
            refused("pane", "a pane id cannot be empty")
        );
        assert_eq!(
            refusal(app.panes().swap(Direction::Up).pane("")).await,
            refused("pane", "a pane id cannot be empty")
        );
        assert_eq!(script.argvs(), [["--version"]]);
        Ok(())
    }

    #[tokio::test]
    async fn terminal_setters_set_their_arguments() -> Result<(), Error> {
        let (app, script) = app().await?;
        let folder = std::env::temp_dir();
        app.terminals()
            .open()
            .argv(["top", "-o", "cpu"])
            .folder(&folder)
            .place(Place::Right)
            .name("monitor")
            .await?;
        assert_sent(
            &script,
            "terminal.open",
            &json!({
                "argv": ["top", "-o", "cpu"],
                "folder": folder.to_str(),
                "place": "right",
                "name": "monitor",
            }),
        );

        app.terminals().keep("t4").place(Place::Down).await?;
        assert_sent(
            &script,
            "terminal.keep",
            &json!({"terminal": "t4", "place": "down"}),
        );
        app.terminals().show().terminal("t4").pane("p1").await?;
        assert_sent(
            &script,
            "terminal.show",
            &json!({"terminal": "t4", "pane": "p1"}),
        );
        app.terminals()
            .right_click(ClickTarget::Program)
            .terminal("t4")
            .await?;
        assert_sent(
            &script,
            "terminal.right_click",
            &json!({"to": "program", "terminal": "t4"}),
        );
        app.terminals().select_all().terminal("t4").await?;
        assert_sent(&script, "terminal.select_all", &json!({"terminal": "t4"}));
        app.terminals()
            .select()
            .terminal("t4")
            .at(At::Cell { row: 2, col: 0 })
            .mode(SelectMode::Line)
            .await?;
        assert_sent(
            &script,
            "terminal.select",
            &json!({"terminal": "t4", "at": {"row": 2, "col": 0}, "mode": "line"}),
        );
        app.terminals()
            .adjust_selection(SelectionMove::PageDown)
            .terminal("t4")
            .await?;
        assert_sent(
            &script,
            "terminal.adjust_selection",
            &json!({"terminal": "t4", "direction": "page_down"}),
        );
        app.terminals().clear_selection().terminal("t4").await?;
        assert_sent(
            &script,
            "terminal.clear_selection",
            &json!({"terminal": "t4"}),
        );
        app.terminals().scroll_pages(0.5).terminal("t4").await?;
        assert_sent(
            &script,
            "terminal.scroll",
            &json!({"terminal": "t4", "pages": 0.5}),
        );
        Ok(())
    }

    #[tokio::test]
    async fn layout_mod_overlay_and_window_setters_set_their_arguments() -> Result<(), Error> {
        let (app, script) = app().await?;
        app.layouts().clone("main").as_name("mine").await?;
        assert_sent(
            &script,
            "layout.clone",
            &json!({"name": "main", "as": "mine"}),
        );
        app.layouts().reset().name("main").await?;
        assert_sent(&script, "layout.reset", &json!({"name": "main"}));
        app.mods().clone("files").as_name("mine").await?;
        assert_sent(
            &script,
            "mod.clone",
            &json!({"name": "files", "as": "mine"}),
        );
        app.overlays().close().id("frame-welcome").await?;
        assert_sent(&script, "overlay.close", &json!({"id": "frame-welcome"}));
        app.window()
            .settings()
            .section(SettingsSection::Alerts)
            .await?;
        assert_sent(&script, "app.settings", &json!({"section": "alerts"}));
        app.window()
            .feedback()
            .message("the tab bar flickers")
            .from(FeedbackFrom::Agent)
            .await?;
        assert_sent(
            &script,
            "app.feedback",
            &json!({"message": "the tab bar flickers", "from": "agent"}),
        );
        Ok(())
    }

    #[tokio::test]
    async fn the_refused_setters_of_the_remaining_groups_name_their_wire_argument()
    -> Result<(), Error> {
        let (app, script) = app().await?;
        assert_eq!(
            refusal(app.terminals().open().argv(Vec::<String>::new())).await,
            refused("argv", "a argv cannot be empty")
        );
        assert_eq!(
            refusal(app.terminals().open().folder("x/y")).await,
            refused("folder", "\"x/y\" is not an absolute path")
        );
        assert_eq!(
            refusal(app.terminals().open().name("")).await,
            refused("name", "a terminal name cannot be empty")
        );
        assert_eq!(
            refusal(app.terminals().show().terminal("")).await,
            refused("terminal", "a terminal address cannot be empty")
        );
        assert_eq!(
            refusal(app.terminals().show().pane("")).await,
            refused("pane", "a pane id cannot be empty")
        );
        assert_eq!(
            refusal(app.terminals().right_click(ClickTarget::Menu).terminal("")).await,
            refused("terminal", "a terminal address cannot be empty")
        );
        assert_eq!(
            refusal(app.terminals().select_all().terminal("")).await,
            refused("terminal", "a terminal address cannot be empty")
        );
        assert_eq!(
            refusal(app.terminals().select().terminal("")).await,
            refused("terminal", "a terminal address cannot be empty")
        );
        assert_eq!(
            refusal(app.terminals().clear_selection().terminal("")).await,
            refused("terminal", "a terminal address cannot be empty")
        );
        assert_eq!(
            refusal(app.terminals().scroll_lines(1).terminal("")).await,
            refused("terminal", "a terminal address cannot be empty")
        );
        assert_eq!(
            refusal(
                app.terminals()
                    .adjust_selection(SelectionMove::Home)
                    .terminal("")
            )
            .await,
            refused("terminal", "a terminal address cannot be empty")
        );
        assert_eq!(
            refusal(app.layouts().clone("main").as_name("")).await,
            refused("as", "a layout name cannot be empty")
        );
        assert_eq!(
            refusal(app.layouts().reset().name("")).await,
            refused("name", "a layout name cannot be empty")
        );
        assert_eq!(
            refusal(app.mods().clone("files").as_name("")).await,
            refused("as", "a mod name cannot be empty")
        );
        assert_eq!(
            refusal(app.overlays().close().id("")).await,
            refused("id", "a overlay id cannot be empty")
        );
        assert_eq!(script.argvs(), [["--version"]]);
        Ok(())
    }
}
