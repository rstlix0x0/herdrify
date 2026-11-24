//! Look at a running herdr-app window without changing it.
//!
//! This asks the `herdr-app` found on `PATH` for:
//!
//! - its version
//! - the spaces, tabs and terminals of the window, from `ls`
//! - the state of the window, from `status`
//!
//! It only reads, so it is safe to run against your everyday window.

use herdrify_app::prelude::*;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    let app = HerdrApp::local().await?;
    println!("herdr-app {}", app.version());

    let listing = app.ls().await?;
    println!("{} spaces open", listing.spaces.len());
    for space in &listing.spaces {
        let shown = if space.active { " (shown)" } else { "" };
        println!("space {} {:?}{shown}", space.id, space.name);
        for tab in &space.tabs {
            let shown = if tab.active { " (shown)" } else { "" };
            println!("  tab {} {:?}{shown}", tab.id, tab.name);
            for terminal in &tab.terminals {
                let pane = terminal.pane.as_ref().map_or("none", |pane| pane.as_str());
                println!("    terminal {} in pane {pane}", terminal.terminal);
            }
        }
    }
    println!("{} spaces closed recently", listing.recent.len());

    let status = app.status().await?;
    println!("layout {}", status.layout);
    println!("zoom {}", status.zoom);
    let shown_space = status.space.as_ref().map_or("none", |space| space.as_str());
    let shown_tab = status.tab.as_ref().map_or("none", |tab| tab.as_str());
    println!("shown space {shown_space}, shown tab {shown_tab}");
    println!("{} overlays open", status.overlays.len());
    for message in &status.errors {
        println!("window error: {message}");
    }
    Ok(())
}
