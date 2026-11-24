//! Make a space, a tab and a pane in a running herdr-app window, then close them again.
//!
//! This acts on the window the `herdr-app` found on `PATH` reaches:
//!
//! - makes a space named `herdrify-example`, left in the background so your focus stays put
//! - adds a tab to that space, and splits a pane in its first tab
//! - closes the pane, the tab and the space it made
//!
//! It touches only what it made. The space is closed whether or not the steps in between worked.

use herdrify_app::prelude::*;
use herdrify_app::protocol::{Created, PaneId, SpaceId, TabId};

/// The program every terminal here runs: it stays up until the space is closed.
const SLEEP: [&str; 3] = ["sh", "-c", "sleep 600"];

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    let app = HerdrApp::local().await?;
    println!("herdr-app {}", app.version());

    let created = app
        .spaces()
        .new_space()
        .name("herdrify-example")
        .argv(SLEEP)
        .focus(Focus::Background)
        .await?;
    println!("made space {}", created.space);

    let walked = walk(&app, &created).await;
    // The space goes, whether or not the walk succeeded.
    let closed = app
        .spaces()
        .close()
        .id(created.space.as_str())
        .confirmed()
        .await;
    match &closed {
        Ok(_) => println!("closed space {}", created.space),
        Err(error) => println!(
            "could not close space {}, it is left behind: {error}",
            created.space
        ),
    }
    walked?;
    closed?;
    Ok(())
}

/// Adds a tab to the space, splits a pane in its first tab, then closes the pane and the tab.
async fn walk(app: &HerdrApp, created: &Created) -> Result<(), Error> {
    let tab = app
        .tabs()
        .new_tab()
        .space(created.space.as_str())
        .name("second")
        .argv(SLEEP)
        .focus(Focus::Background)
        .await?;
    println!("made tab {}", tab.tab);
    show(app, &created.space).await?;

    let Some(pane) = first_pane(app, &created.space).await? else {
        println!("the space shows no pane to split");
        return close_tab(app, &tab.tab).await;
    };
    let split = app
        .panes()
        .split(SplitDirection::Right)
        .pane(pane.as_str())
        .argv(SLEEP)
        .await?;
    println!("split pane {} off pane {pane}", split.pane);
    show(app, &created.space).await?;

    app.panes().close().pane(split.pane.as_str()).await?;
    println!("closed pane {}", split.pane);
    close_tab(app, &tab.tab).await
}

/// Closes the tab `tab`, which must be one this program made.
async fn close_tab(app: &HerdrApp, tab: &TabId) -> Result<(), Error> {
    app.tabs().close().id(tab.as_str()).confirmed().await?;
    println!("closed tab {tab}");
    Ok(())
}

/// The pane of the first terminal of the first tab of `space`, when it has one.
async fn first_pane(app: &HerdrApp, space: &SpaceId) -> Result<Option<PaneId>, Error> {
    let listing = app.ls().await?;
    let pane = listing
        .spaces
        .iter()
        .find(|entry| &entry.id == space)
        .and_then(|entry| entry.tabs.first())
        .and_then(|tab| tab.terminals.first())
        .and_then(|terminal| terminal.pane.clone());
    Ok(pane)
}

/// Prints how many tabs and terminals `space` holds.
async fn show(app: &HerdrApp, space: &SpaceId) -> Result<(), Error> {
    let listing = app.ls().await?;
    if let Some(entry) = listing.spaces.iter().find(|entry| &entry.id == space) {
        let terminals: usize = entry.tabs.iter().map(|tab| tab.terminals.len()).sum();
        println!(
            "  it holds {} tabs and {terminals} terminals",
            entry.tabs.len()
        );
    }
    Ok(())
}
