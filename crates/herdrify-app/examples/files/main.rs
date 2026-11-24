//! Build a layout, a theme and a keys file in code, write each as JSON and read it back.
//!
//! This builds three values with no herdr-app and no window:
//!
//! - a `Layout` whose one block shows the mod `herdrify-host`, with no props
//! - a `Theme` with a cursor and a named colour
//! - `Keys` with two bindings in the section `layout:herdrify-main`
//!
//! For each it prints the text `to_json` writes, parses that printed text again, and exits with an
//! error when the value read back is not the value written.

use std::collections::BTreeMap;

use herdrify_app::files::{
    Action, Chord, ColorName, CommandName, Content, FileError, HexColor, Keys, Layout, LayoutName,
    ModName, Node, NodeId, Section, Theme,
};

type BoxError = Box<dyn std::error::Error>;

/// The sixteen terminal colours of the theme, in order.
const ANSI: [&str; 16] = [
    "#1d1f21", "#cc6666", "#b5bd68", "#f0c674", "#81a2be", "#b294bb", "#8abeb7", "#c5c8c6",
    "#969896", "#d54e53", "#b9ca4a", "#e7c547", "#7aa6da", "#c397d8", "#70c0b1", "#eaeaea",
];

fn main() -> Result<(), BoxError> {
    show("layout", &layout()?, Layout::to_json, Layout::parse)?;
    show("theme", &theme()?, Theme::to_json, Theme::parse)?;
    show("keys", &keys()?, Keys::to_json, Keys::parse)?;
    Ok(())
}

/// A row with one block that shows the mod `herdrify-host`.
fn layout() -> Result<Layout, BoxError> {
    let mut host = Node::block(Content::module(ModName::new("herdrify-host")?));
    host.id = Some(NodeId::new("host")?);
    Ok(Layout::new(Node::row(vec![host])))
}

/// A dark theme with a cursor colour and one named colour.
fn theme() -> Result<Theme, BoxError> {
    let background = HexColor::new("#1d1f21")?;
    let mut theme = Theme::new(
        background.clone(),
        HexColor::new("#c5c8c6")?,
        HexColor::new("#373b41")?,
        std::array::from_fn(|_| background.clone()),
    );
    for (slot, hex) in theme.ansi.iter_mut().zip(ANSI) {
        *slot = HexColor::new(hex)?;
    }
    theme.cursor = Some(HexColor::new("#c5c8c6")?);
    theme.colors = Some(BTreeMap::from([(
        ColorName::new("accent")?,
        HexColor::new("#81a2be")?,
    )]));
    Ok(theme)
}

/// Two bindings in the section of the layout `herdrify-main`.
fn keys() -> Result<Keys, BoxError> {
    let section = Section::Layout(LayoutName::new("herdrify-main")?);
    let mut keys = Keys::new();
    keys.insert(
        section.clone(),
        Chord::new("ctrl-b")?,
        Action::run_with(CommandName::new("pane.split")?, "right"),
    );
    keys.insert(
        section,
        Chord::new("ctrl-n")?,
        Action::run(CommandName::new("tab.next")?),
    );
    Ok(keys)
}

/// Prints `model` as `to_json` writes it, reads the printed text back with `parse`, and fails
/// when the result differs from `model`.
fn show<M: PartialEq>(
    title: &str,
    model: &M,
    to_json: fn(&M) -> String,
    parse: fn(&str) -> Result<M, FileError>,
) -> Result<(), BoxError> {
    let text = to_json(model);
    println!("--- {title} ---");
    print!("{text}");

    let read_back = parse(&text)?;
    if &read_back != model {
        return Err(format!("the {title} read back differs from the {title} written").into());
    }
    println!("the {title} read back equals the {title} written\n");
    Ok(())
}
