//! `TerminalOps` and `AgentOps`: the operations on terminals and on the agents running in them.
//!
//! A terminal is a program running in the window, with or without a pane to show it, and an agent
//! is a terminal running a coding agent. The operations start, keep, park, show, pin and kill
//! terminals, select and scroll their screens, and move between agents. Each returns a [`Call`]
//! over the command of the same name, built from plain values: an address is a string, a line
//! count is a number, a place is an enum.
//!
//! Responsibilities:
//!
//! - [`TerminalOps`], one method per way of using the `terminal.*` commands
//! - [`AgentOps`], one method per `agent.*` command
//!
//! Non-responsibilities:
//!
//! - the options of each command, which [`Call`] defines as setters
//! - running the commands, which [`Client`] does

use super::call::{Call, arg, example, ops_plumbing};
use crate::adapter::ProcessTransport;
use crate::driver::{Client, Error};
use crate::files::TerminalAddress;
use crate::port::Transport;
use crate::protocol::{
    AgentFocus, AgentNext, AgentPrev, ClickTarget, FiniteF64, Pin, Scroll, ScrollEdge,
    SelectionMove, TerminalAdjustSelection, TerminalClearSelection, TerminalCopy, TerminalKeep,
    TerminalKill, TerminalOpen, TerminalPark, TerminalPaste, TerminalPasteImage, TerminalPeek,
    TerminalPin, TerminalRightClick, TerminalScroll, TerminalSearch, TerminalSelect,
    TerminalSelectAll, TerminalShow,
};

/// The address `terminal` of a terminal, or the error for refusing it as the argument `terminal`.
fn address(terminal: impl Into<String>) -> Result<TerminalAddress, Error> {
    arg("terminal", TerminalAddress::new(terminal))
}

/// The operations on terminals.
///
/// Reached through [`HerdrApp::terminals`](super::HerdrApp::terminals). An address is a string,
/// and an empty one is reported by the `.await` as [`Error::InvalidArgument`], as is a number
/// that is out of range. The commands that take an address as an argument are `keep`, `park`,
/// `peek`, `kill`, `pin` and `unpin`. For the others that act on one terminal (`show`,
/// `right_click`, `select`, `select_all`, `adjust_selection`, `clear_selection` and the scrolls)
/// the terminal is optional: they act on the focused one unless their call is given an address
/// with `.terminal(..)`.
///
/// # Examples
///
/// ```no_run
/// use herdrify_app::HerdrApp;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), herdrify_app::Error> {
/// let app = HerdrApp::local().await?;
/// let opened = app.terminals().open().argv(["top"]).await?;
/// app.terminals().pin(opened.terminal.as_str()).await?;
/// app.terminals()
///     .scroll_pages(-1.0)
///     .terminal(opened.terminal.as_str())
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct TerminalOps<T: Transport = ProcessTransport> {
    client: Client<T>,
}

ops_plumbing!(TerminalOps);

impl<T: Transport> TerminalOps<T> {
    /// `terminal.open`: starts a terminal, in a new tab unless its call is given a place.
    ///
    #[doc = example!("app.terminals().open().argv([\"top\"]).await?;")]
    pub fn open(&self) -> Call<T, TerminalOpen> {
        self.call(Ok(TerminalOpen::new()))
    }

    /// `terminal.keep`: gives the running terminal at the address `terminal`, which has no pane,
    /// a pane of its own.
    ///
    #[doc = example!("app.terminals().keep(\"t4\").await?;")]
    pub fn keep(&self, terminal: impl Into<String>) -> Call<T, TerminalKeep> {
        self.call(address(terminal).map(TerminalKeep::new))
    }

    /// `terminal.park`: takes the terminal at the address `terminal` out of its pane and keeps it
    /// running.
    ///
    #[doc = example!("app.terminals().park(\"t4\").await?;")]
    pub fn park(&self, terminal: impl Into<String>) -> Call<T, TerminalPark> {
        self.call(address(terminal).map(TerminalPark::new))
    }

    /// `terminal.peek`: peeks at the running terminal at the address `terminal`, over the tab.
    ///
    #[doc = example!("app.terminals().peek(\"t4\").await?;")]
    pub fn peek(&self, terminal: impl Into<String>) -> Call<T, TerminalPeek> {
        self.call(address(terminal).map(TerminalPeek::new))
    }

    /// `terminal.show`: shows a terminal in a pane. Name the terminal and the pane with
    /// `.terminal(..)` and `.pane(..)` on the call.
    ///
    #[doc = example!("app.terminals().show().terminal(\"t4\").pane(\"p1\").await?;")]
    pub fn show(&self) -> Call<T, TerminalShow> {
        self.call(Ok(TerminalShow::new()))
    }

    /// `terminal.kill`: stops the terminal at the address `terminal`.
    ///
    #[doc = example!("app.terminals().kill(\"t4\").await?;")]
    pub fn kill(&self, terminal: impl Into<String>) -> Call<T, TerminalKill> {
        self.call(address(terminal).map(TerminalKill::new))
    }

    /// `terminal.pin`: pins the terminal at the address `terminal`.
    ///
    #[doc = example!("app.terminals().pin(\"t4\").await?;")]
    pub fn pin(&self, terminal: impl Into<String>) -> Call<T, TerminalPin> {
        self.call(address(terminal).map(|terminal| TerminalPin::new(terminal, Pin::Pinned)))
    }

    /// `terminal.pin`: unpins the terminal at the address `terminal`.
    ///
    #[doc = example!("app.terminals().unpin(\"t4\").await?;")]
    pub fn unpin(&self, terminal: impl Into<String>) -> Call<T, TerminalPin> {
        self.call(address(terminal).map(|terminal| TerminalPin::new(terminal, Pin::Unpinned)))
    }

    /// `terminal.right_click`: right-clicks the focused terminal, on its program or on its menu.
    ///
    #[doc = example!(
        "use herdrify_app::protocol::ClickTarget;",
        "app.terminals().right_click(ClickTarget::Menu).await?;"
    )]
    pub fn right_click(&self, to: ClickTarget) -> Call<T, TerminalRightClick> {
        self.call(Ok(TerminalRightClick::new(to)))
    }

    /// `terminal.select_all`: selects everything on the screen of the focused terminal.
    ///
    #[doc = example!("app.terminals().select_all().await?;")]
    pub fn select_all(&self) -> Call<T, TerminalSelectAll> {
        self.call(Ok(TerminalSelectAll::new()))
    }

    /// `terminal.select`: starts a selection in the focused terminal. Where it starts and how it
    /// grows are `.at(..)` and `.mode(..)` on the call.
    ///
    #[doc = example!(
        "use herdrify_app::protocol::{At, SelectMode};",
        "app.terminals().select().at(At::Cell { row: 2, col: 0 }).mode(SelectMode::Line).await?;"
    )]
    pub fn select(&self) -> Call<T, TerminalSelect> {
        self.call(Ok(TerminalSelect::new()))
    }

    /// `terminal.adjust_selection`: moves the end of the selection of the focused terminal towards
    /// `direction`.
    ///
    #[doc = example!(
        "use herdrify_app::protocol::SelectionMove;",
        "app.terminals().adjust_selection(SelectionMove::PageDown).await?;"
    )]
    pub fn adjust_selection(&self, direction: SelectionMove) -> Call<T, TerminalAdjustSelection> {
        self.call(Ok(TerminalAdjustSelection::new(direction)))
    }

    /// `terminal.clear_selection`: clears the selection of the focused terminal.
    ///
    #[doc = example!("app.terminals().clear_selection().await?;")]
    pub fn clear_selection(&self) -> Call<T, TerminalClearSelection> {
        self.call(Ok(TerminalClearSelection::new()))
    }

    /// `terminal.copy`: copies the selection of the focused terminal to the clipboard.
    ///
    #[doc = example!("app.terminals().copy().await?;")]
    pub fn copy(&self) -> Call<T, TerminalCopy> {
        self.call(Ok(TerminalCopy::new()))
    }

    /// `terminal.scroll`: scrolls the focused terminal by `lines` lines, up when negative.
    ///
    #[doc = example!("app.terminals().scroll_lines(-10).await?;")]
    pub fn scroll_lines(&self, lines: i64) -> Call<T, TerminalScroll> {
        self.call(Ok(TerminalScroll::new(Scroll::Lines(lines))))
    }

    /// `terminal.scroll`: scrolls the focused terminal by `pages` pages, up when negative. The
    /// number must be finite.
    ///
    #[doc = example!("app.terminals().scroll_pages(0.5).await?;")]
    pub fn scroll_pages(&self, pages: f64) -> Call<T, TerminalScroll> {
        self.call(
            arg("pages", FiniteF64::new(pages))
                .map(|pages| TerminalScroll::new(Scroll::Pages(pages))),
        )
    }

    /// `terminal.scroll`: scrolls the focused terminal to `edge` of its scrollback.
    ///
    #[doc = example!(
        "use herdrify_app::protocol::ScrollEdge;",
        "app.terminals().scroll_to(ScrollEdge::Bottom).await?;"
    )]
    pub fn scroll_to(&self, edge: ScrollEdge) -> Call<T, TerminalScroll> {
        self.call(Ok(TerminalScroll::new(Scroll::To(edge))))
    }

    /// `terminal.paste`: pastes the clipboard's text into the focused terminal.
    ///
    #[doc = example!("app.terminals().paste().await?;")]
    pub fn paste(&self) -> Call<T, TerminalPaste> {
        self.call(Ok(TerminalPaste::new()))
    }

    /// `terminal.paste_image`: pastes the clipboard's image into the focused terminal.
    ///
    #[doc = example!("app.terminals().paste_image().await?;")]
    pub fn paste_image(&self) -> Call<T, TerminalPasteImage> {
        self.call(Ok(TerminalPasteImage::new()))
    }

    /// `terminal.search`: opens the find bar of the focused terminal.
    ///
    #[doc = example!("app.terminals().search().await?;")]
    pub fn search(&self) -> Call<T, TerminalSearch> {
        self.call(Ok(TerminalSearch::new()))
    }
}

/// The operations on the agents running in the window's terminals.
///
/// Reached through [`HerdrApp::agents`](super::HerdrApp::agents). An address is a string, and an
/// empty one is reported by the `.await` as [`Error::InvalidArgument`].
///
/// # Examples
///
/// ```no_run
/// use herdrify_app::HerdrApp;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), herdrify_app::Error> {
/// let app = HerdrApp::local().await?;
/// app.agents().next().await?;
/// app.agents().focus("t4").await?;
/// # Ok(())
/// # }
/// ```
pub struct AgentOps<T: Transport = ProcessTransport> {
    client: Client<T>,
}

ops_plumbing!(AgentOps);

impl<T: Transport> AgentOps<T> {
    /// `agent.focus`: focuses the agent in the terminal at the address `terminal`.
    ///
    #[doc = example!("app.agents().focus(\"t4\").await?;")]
    pub fn focus(&self, terminal: impl Into<String>) -> Call<T, AgentFocus> {
        self.call(address(terminal).map(AgentFocus::new))
    }

    /// `agent.next`: focuses the next agent.
    ///
    #[doc = example!("app.agents().next().await?;")]
    pub fn next(&self) -> Call<T, AgentNext> {
        self.call(Ok(AgentNext::new()))
    }

    /// `agent.prev`: focuses the previous agent.
    ///
    #[doc = example!("app.agents().prev().await?;")]
    pub fn prev(&self) -> Call<T, AgentPrev> {
        self.call(Ok(AgentPrev::new()))
    }
}

#[cfg(test)]
pub(super) mod tests {
    use serde_json::json;

    use super::*;
    use crate::herdr_app::HerdrApp;
    use crate::herdr_app::fake::{Script, app, assert_sent};
    use crate::protocol::{ClickTarget, ScrollEdge, SelectionMove};

    /// Runs every method of the terminal group once against `app`, checking the command each one
    /// sent.
    ///
    /// Fails when the calls do not send one run each.
    pub(in crate::herdr_app) async fn run_every_terminal_method(
        app: &HerdrApp<Script>,
        script: &Script,
    ) -> Result<(), Error> {
        let first = script.argvs().len();
        let terminals = app.terminals();

        terminals.open().await?;
        assert_sent(script, "terminal.open", &json!({}));

        terminals.keep("t4").await?;
        assert_sent(script, "terminal.keep", &json!({"terminal": "t4"}));

        terminals.park("t4").await?;
        assert_sent(script, "terminal.park", &json!({"terminal": "t4"}));

        terminals.peek("t4").await?;
        assert_sent(script, "terminal.peek", &json!({"terminal": "t4"}));

        terminals.show().await?;
        assert_sent(script, "terminal.show", &json!({}));

        terminals.kill("t4").await?;
        assert_sent(script, "terminal.kill", &json!({"terminal": "t4"}));

        terminals.pin("t4").await?;
        assert_sent(
            script,
            "terminal.pin",
            &json!({"terminal": "t4", "pinned": true}),
        );

        terminals.unpin("t4").await?;
        assert_sent(
            script,
            "terminal.pin",
            &json!({"terminal": "t4", "pinned": false}),
        );

        terminals.right_click(ClickTarget::Menu).await?;
        assert_sent(script, "terminal.right_click", &json!({"to": "menu"}));

        terminals.select_all().await?;
        assert_sent(script, "terminal.select_all", &json!({}));

        terminals.select().await?;
        assert_sent(script, "terminal.select", &json!({}));

        terminals.adjust_selection(SelectionMove::PageUp).await?;
        assert_sent(
            script,
            "terminal.adjust_selection",
            &json!({"direction": "page_up"}),
        );

        terminals.clear_selection().await?;
        assert_sent(script, "terminal.clear_selection", &json!({}));

        terminals.copy().await?;
        assert_sent(script, "terminal.copy", &json!({}));

        terminals.scroll_lines(-3).await?;
        assert_sent(script, "terminal.scroll", &json!({"lines": -3}));

        terminals.scroll_pages(0.5).await?;
        assert_sent(script, "terminal.scroll", &json!({"pages": 0.5}));

        terminals.scroll_to(ScrollEdge::Bottom).await?;
        assert_sent(script, "terminal.scroll", &json!({"to": "bottom"}));

        terminals.paste().await?;
        assert_sent(script, "terminal.paste", &json!({}));

        terminals.paste_image().await?;
        assert_sent(script, "terminal.paste_image", &json!({}));

        terminals.search().await?;
        assert_sent(script, "terminal.search", &json!({}));

        assert_eq!(script.argvs().len() - first, 20);
        Ok(())
    }

    /// Runs every method of the agent group once against `app`, checking the command each one
    /// sent.
    pub(in crate::herdr_app) async fn run_every_agent_method(
        app: &HerdrApp<Script>,
        script: &Script,
    ) -> Result<(), Error> {
        let first = script.argvs().len();
        let agents = app.agents();

        agents.focus("t4").await?;
        assert_sent(script, "agent.focus", &json!({"terminal": "t4"}));

        agents.next().await?;
        assert_sent(script, "agent.next", &json!({}));

        agents.prev().await?;
        assert_sent(script, "agent.prev", &json!({}));

        assert_eq!(script.argvs().len() - first, 3);
        Ok(())
    }

    #[tokio::test]
    async fn every_terminal_method_builds_its_command() -> Result<(), Error> {
        let (app, script) = app().await?;
        run_every_terminal_method(&app, &script).await
    }

    #[tokio::test]
    async fn every_agent_method_builds_its_command() -> Result<(), Error> {
        let (app, script) = app().await?;
        run_every_agent_method(&app, &script).await
    }

    #[tokio::test]
    async fn out_of_range_numbers_are_invalid_argument() -> Result<(), Error> {
        let (app, script) = app().await?;
        let error = app.terminals().scroll_pages(f64::NAN).await.err();
        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some("invalid pages: NaN is out of range: must be finite")
        );
        let error = app.terminals().scroll_pages(f64::INFINITY).await.err();
        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some("invalid pages: inf is out of range: must be finite")
        );
        assert_eq!(script.argvs(), [["--version"]]);
        Ok(())
    }

    #[tokio::test]
    async fn empty_addresses_name_the_argument_they_fill() -> Result<(), Error> {
        let (app, script) = app().await?;
        let terminals = app.terminals();
        let refused = [
            terminals.keep("").await.err(),
            terminals.park("").await.err(),
            terminals.peek("").await.err(),
            terminals.kill("").await.err(),
            terminals.pin("").await.err(),
            terminals.unpin("").await.err(),
            app.agents().focus("").await.err(),
        ];
        for error in refused {
            assert_eq!(
                error.map(|error| error.to_string()).as_deref(),
                Some("invalid terminal: a terminal address cannot be empty")
            );
        }
        assert_eq!(script.argvs(), [["--version"]]);
        Ok(())
    }

    #[tokio::test]
    async fn debug_names_the_groups() -> Result<(), Error> {
        let (app, _script) = app().await?;
        assert_eq!(format!("{:?}", app.terminals()), "TerminalOps { .. }");
        assert_eq!(format!("{:?}", app.agents()), "AgentOps { .. }");
        Ok(())
    }
}
