//! The command table and the command types, checked against the command list of herdr-app 0.1.17.
//!
//! The fixture is a byte-for-byte copy of what `herdr-app commands` prints. The tests run offline
//! through the public API: they compare the built-in table with the listing, and check the command
//! types against the table in both directions. Each failing check prints every difference it found,
//! not only the first.
//!
//! Responsibilities:
//!
//! - the table against the listing: the same names, in the same order, and per command the same
//!   argument names in the same order with the same `required` flags
//! - the 77 command types against the table: each type's name is in it, and the types cover all
//!   of it
//! - per command, the keys its instances write: only keys the table lists for it, every key the
//!   table lists written by some instance, and every required key written by the command built
//!   with `new` alone
//! - one instance of every variant of every argument enum, each enum listed through a `match` with
//!   no wildcard arm so that a new variant fails to compile until it is covered; these check the
//!   keys each variant writes, and not a unit variant's value, which the listing does not give
//!
//! Non-responsibilities:
//!
//! - the commands' own behaviour on unusual input, which their unit tests cover
//! - running herdr-app, which the tests never need

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

use herdrify_app::files::{Layout, LayoutName, ModName, NodeId, TerminalAddress, ThemeName};
use herdrify_app::protocol::{
    AgentFocus, AgentNext, AgentPrev, AppCheckUpdates, AppFeedback, AppHelp, AppHide,
    AppHideOthers, AppPalette, AppQuit, AppReset, AppSettings, AppShowAll, AppTheme, AppUpdate,
    AppUpdateCore, AppWelcome, AppWhatsNew, AppZoomIn, AppZoomOut, AppZoomReset, Argv, At,
    ClickTarget, Command, CommandInfo, Direction, FeedbackFrom, FiniteF64, Focus, Folder,
    KNOWN_COMMANDS, KnownCommand, LayoutClone, LayoutHide, LayoutReset, LayoutResize, LayoutSelect,
    LayoutSet, LayoutShow, LayoutSwitch, LayoutToggle, Machine, MachineName, ModClone, ModOpen,
    OverlayClose, OverlayId, PaneClose, PaneContent, PaneFocus, PaneGo, PaneId, PaneName,
    PaneRename, PaneResize, PaneSplit, PaneSwap, PaneTarget, PaneZoom, Pin, Place, Position, Px,
    Rename, Resize, Scroll, ScrollEdge, SelectMode, SelectionMove, SettingsSection, SpaceClose,
    SpaceId, SpaceMove, SpaceName, SpaceNew, SpaceNext, SpacePrev, SpaceRef, SpaceRename,
    SpaceSelect, SplitDirection, SwapWith, TabClose, TabContent, TabId, TabMove, TabMoveTo,
    TabName, TabNew, TabNext, TabPrev, TabRef, TabRename, TabSelect, TabSettle, TabUnsettle, Task,
    TerminalAdjustSelection, TerminalClearSelection, TerminalCopy, TerminalKeep, TerminalKill,
    TerminalName, TerminalOpen, TerminalPark, TerminalPaste, TerminalPasteImage, TerminalPeek,
    TerminalPin, TerminalRightClick, TerminalScroll, TerminalSearch, TerminalSelect,
    TerminalSelectAll, TerminalShow, Weight, WindowClose, WindowMaximize, WindowMinimize,
};
use serde_json::{Map, Value, json};

/// The 0.1.17 command list, as `herdr-app commands` prints it.
const LISTING: &str = include_str!("fixtures/herdr-app-0.1.17-commands.json");

/// The fixture decoded, or the reason it does not decode.
fn listing() -> Result<Vec<CommandInfo>, serde_json::Error> {
    serde_json::from_str(LISTING)
}

/// Every difference between the table and the listing, one line each. Empty when they agree.
///
/// The names come first, as sets, so a missing or extra command is named. The order is compared
/// only when the sets agree, since one missing name would shift every later position. Arguments are
/// compared for the commands both sides have, looked up by name.
fn table_differences(listing: &[CommandInfo]) -> Vec<String> {
    let mut differences = Vec::new();
    let listed_names: Vec<&str> = listing.iter().map(|listed| listed.name.as_str()).collect();
    let known_names: Vec<&str> = KNOWN_COMMANDS.iter().map(|known| known.name).collect();
    let listed_set: BTreeSet<&str> = listed_names.iter().copied().collect();
    let known_set: BTreeSet<&str> = known_names.iter().copied().collect();
    for name in listed_set.difference(&known_set) {
        differences.push(format!("`{name}` is in the listing and not in the table"));
    }
    for name in known_set.difference(&listed_set) {
        differences.push(format!("`{name}` is in the table and not in the listing"));
    }
    if listed_names.len() != listed_set.len() {
        differences.push(format!(
            "the listing has {} names and {} distinct ones",
            listed_names.len(),
            listed_set.len()
        ));
    }
    if known_names.len() != known_set.len() {
        differences.push(format!(
            "the table has {} names and {} distinct ones",
            known_names.len(),
            known_set.len()
        ));
    }
    if listed_set == known_set && listed_names != known_names {
        for (index, (listed, known)) in listed_names.iter().zip(&known_names).enumerate() {
            if listed != known {
                differences.push(format!(
                    "position {index}: the listing has `{listed}` and the table `{known}`"
                ));
            }
        }
    }
    for listed in listing {
        let Some(known) = KNOWN_COMMANDS
            .iter()
            .find(|known| known.name == listed.name.as_str())
        else {
            continue;
        };
        let listed_args: Vec<(&str, bool)> = listed
            .args
            .iter()
            .map(|arg| (arg.name.as_str(), arg.required))
            .collect();
        if listed_args != known.args {
            differences.push(format!(
                "`{}`: the listing has the arguments {listed_args:?} and the table {:?}",
                known.name, known.args
            ));
        }
    }
    differences
}

#[test]
fn known_commands_equal_the_fixture() -> Result<(), serde_json::Error> {
    let listing = listing()?;
    let differences = table_differences(&listing);
    assert!(
        differences.is_empty(),
        "the table differs from the listing:\n{}",
        differences.join("\n")
    );
    assert_eq!(KNOWN_COMMANDS.len(), 77);
    Ok(())
}

/// One built command: the name of its type and the arguments it serializes to.
struct Instance {
    name: &'static str,
    arguments: Value,
}

/// Builds the instance of a command: its type's name and the arguments it serializes to.
fn instance<C: Command>(command: &C) -> Result<Instance, serde_json::Error> {
    Ok(Instance {
        name: C::NAME,
        arguments: serde_json::to_value(command)?,
    })
}

/// The properties of a mod.
fn props() -> Option<Map<String, Value>> {
    json!({"depth": 2}).as_object().cloned()
}

/// One command of every type, each with every setter used and one variant of each argument enum.
#[expect(
    clippy::too_many_lines,
    reason = "one line group per command type, 77 in all"
)]
fn full_instances() -> Result<Vec<Instance>, Box<dyn Error>> {
    let terminal = || TerminalAddress::new("t2");
    let node = || NodeId::new("n1");
    let layout = || LayoutName::new("main");
    Ok(vec![
        instance(
            &SpaceNew::new()
                .name(SpaceName::new("work")?)
                .folder(Folder::new(std::env::temp_dir())?)
                .tab(TabName::new("build")?)
                .argv(Argv::new(["claude"])?)
                .task(Task::new("fix the build")?)
                .machine(Machine::ThisComputer)
                .focus(Focus::Background),
        )?,
        instance(&SpaceClose::new().id(SpaceId::new("sp1")?).confirmed())?,
        instance(&SpaceRename::new(Rename::To(SpaceName::new("docs")?)).id(SpaceId::new("sp1")?))?,
        instance(&SpaceMove::new(SpaceId::new("sp1")?, Position::new(2)?))?,
        instance(&SpaceSelect::new(SpaceRef::Index(Position::new(1)?)))?,
        instance(&SpaceNext::new())?,
        instance(&SpacePrev::new())?,
        instance(
            &TabNew::new(TabContent::Shell {
                argv: Some(Argv::new(["sh"])?),
                task: Some(Task::new("run the tests")?),
                folder: Some(Folder::new(std::env::temp_dir())?),
            })
            .name(TabName::new("tests")?)
            .space(SpaceId::new("sp1")?)
            .focus(Focus::Show),
        )?,
        instance(&TabClose::new().id(TabId::new("tb1")?).confirmed())?,
        instance(&TabRename::new(Rename::Automatic).id(TabId::new("tb1")?))?,
        instance(&TabMove::new(TabMoveTo::Index(Position::new(3)?)).id(TabId::new("tb1")?))?,
        instance(&TabSelect::new(TabRef::Id(TabId::new("tb1")?)))?,
        instance(&TabSettle::new().id(TabId::new("tb1")?))?,
        instance(&TabUnsettle::new().id(TabId::new("tb1")?))?,
        instance(&TabNext::new())?,
        instance(&TabPrev::new())?,
        instance(
            &PaneSplit::new(
                SplitDirection::Right,
                PaneContent::Shell {
                    argv: Some(Argv::new(["sh"])?),
                },
            )
            .pane(PaneId::new("p1")?),
        )?,
        instance(&PaneClose::new().pane(PaneId::new("p1")?))?,
        instance(&PaneRename::new(Rename::To(PaneName::new("logs")?)).pane(PaneId::new("p1")?))?,
        instance(
            &PaneResize::new(Direction::Left)
                .pane(PaneId::new("p1")?)
                .px(Px::new(20.0)?),
        )?,
        instance(&PaneZoom::new().pane(PaneId::new("p1")?))?,
        instance(&PaneFocus::new(PaneTarget::Pane(PaneId::new("p1")?)))?,
        instance(&PaneGo::new(Direction::Up))?,
        instance(&PaneSwap::new(SwapWith::Direction(Direction::Down)).pane(PaneId::new("p1")?))?,
        instance(
            &TerminalOpen::new()
                .argv(Argv::new(["top"])?)
                .folder(Folder::new(std::env::temp_dir())?)
                .place(Place::Right)
                .name(TerminalName::new("monitor")?),
        )?,
        instance(&TerminalKeep::new(terminal()?).place(Place::Tab))?,
        instance(&TerminalPark::new(terminal()?))?,
        instance(&TerminalKill::new(terminal()?))?,
        instance(&TerminalPeek::new(terminal()?))?,
        instance(
            &TerminalShow::new()
                .terminal(terminal()?)
                .pane(PaneId::new("p1")?),
        )?,
        instance(&TerminalPin::new(terminal()?, Pin::Pinned))?,
        instance(&TerminalRightClick::new(ClickTarget::Menu).terminal(terminal()?))?,
        instance(&TerminalSelectAll::new().terminal(terminal()?))?,
        instance(
            &TerminalSelect::new()
                .terminal(terminal()?)
                .at(At::Cell { row: 2, col: 0 })
                .mode(SelectMode::Word),
        )?,
        instance(&TerminalAdjustSelection::new(SelectionMove::LineEnd).terminal(terminal()?))?,
        instance(&TerminalClearSelection::new().terminal(terminal()?))?,
        instance(&TerminalScroll::new(Scroll::To(ScrollEdge::Top)).terminal(terminal()?))?,
        instance(&TerminalSearch::new())?,
        instance(&TerminalCopy::new())?,
        instance(&TerminalPaste::new())?,
        instance(&TerminalPasteImage::new())?,
        instance(&AgentFocus::new(terminal()?))?,
        instance(&AgentNext::new())?,
        instance(&AgentPrev::new())?,
        instance(&LayoutSwitch::new().name(layout()?))?,
        instance(&LayoutClone::new(layout()?).as_name(LayoutName::new("copy")?))?,
        instance(&LayoutSet::new(Layout::parse(r#"{"type":"column"}"#)?))?,
        instance(&LayoutToggle::new(node()?))?,
        instance(&LayoutShow::new(node()?))?,
        instance(&LayoutHide::new(node()?))?,
        instance(&LayoutResize::new(node()?, Resize::Px(Px::new(200.0)?)))?,
        instance(&LayoutSelect::new(node()?, NodeId::new("n2")?))?,
        instance(&LayoutReset::new().name(layout()?))?,
        instance(&ModClone::new(ModName::new("files")?).as_name(ModName::new("files2")?))?,
        instance(&ModOpen::new(ModName::new("files")?))?,
        instance(&OverlayClose::new().id(OverlayId::new("ov1")?))?,
        instance(&AppHelp::new())?,
        instance(&AppWelcome::new())?,
        instance(&AppWhatsNew::new())?,
        instance(&AppPalette::new())?,
        instance(&AppSettings::new().section(SettingsSection::Look))?,
        instance(&AppReset::new())?,
        instance(&AppTheme::new(ThemeName::new("dark")?))?,
        instance(&AppZoomIn::new())?,
        instance(&AppZoomOut::new())?,
        instance(&AppZoomReset::new())?,
        instance(&AppQuit::new())?,
        instance(
            &AppFeedback::new()
                .message("the tab bar flickers")
                .from(FeedbackFrom::Person),
        )?,
        instance(&AppCheckUpdates::new())?,
        instance(&AppUpdateCore::new())?,
        instance(&AppUpdate::new())?,
        instance(&AppHide::new())?,
        instance(&AppHideOthers::new())?,
        instance(&AppShowAll::new())?,
        instance(&WindowClose::new())?,
        instance(&WindowMinimize::new())?,
        instance(&WindowMaximize::new())?,
    ])
}

/// Defines a function that lists the cases of one argument enum.
///
/// Each arm pairs a pattern of the enum with the values that stand for that variant. The generated
/// `match` has no wildcard arm, so a variant added to the enum does not compile until it has an
/// arm here, and an arm cannot be written without a value for its variant.
macro_rules! variant_cases {
    (
        $(#[$meta:meta])*
        fn $name:ident() -> $ty:ty {
            $($pattern:pat => [$($case:expr),+ $(,)?]),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        fn $name() -> Result<Vec<$ty>, Box<dyn Error>> {
            let cases: Vec<$ty> = vec![$($($case),+),+];
            for case in &cases {
                match case {
                    $($pattern => {})+
                }
            }
            Ok(cases)
        }
    };
}

variant_cases! {
    /// Every direction.
    fn directions() -> Direction {
        Direction::Left => [Direction::Left],
        Direction::Right => [Direction::Right],
        Direction::Up => [Direction::Up],
        Direction::Down => [Direction::Down],
    }
}

variant_cases! {
    /// Every way to split a pane.
    fn split_directions() -> SplitDirection {
        SplitDirection::Right => [SplitDirection::Right],
        SplitDirection::Down => [SplitDirection::Down],
    }
}

variant_cases! {
    /// Every place a terminal can take.
    fn places() -> Place {
        Place::Tab => [Place::Tab],
        Place::Right => [Place::Right],
        Place::Down => [Place::Down],
    }
}

variant_cases! {
    /// Every target of a right click.
    fn click_targets() -> ClickTarget {
        ClickTarget::Program => [ClickTarget::Program],
        ClickTarget::Menu => [ClickTarget::Menu],
    }
}

variant_cases! {
    /// Every selection mode.
    fn select_modes() -> SelectMode {
        SelectMode::Cell => [SelectMode::Cell],
        SelectMode::Word => [SelectMode::Word],
        SelectMode::Line => [SelectMode::Line],
    }
}

variant_cases! {
    /// Every move of a selection.
    fn selection_moves() -> SelectionMove {
        SelectionMove::Left => [SelectionMove::Left],
        SelectionMove::Right => [SelectionMove::Right],
        SelectionMove::Up => [SelectionMove::Up],
        SelectionMove::Down => [SelectionMove::Down],
        SelectionMove::PageUp => [SelectionMove::PageUp],
        SelectionMove::PageDown => [SelectionMove::PageDown],
        SelectionMove::Home => [SelectionMove::Home],
        SelectionMove::End => [SelectionMove::End],
        SelectionMove::LineStart => [SelectionMove::LineStart],
        SelectionMove::LineEnd => [SelectionMove::LineEnd],
    }
}

variant_cases! {
    /// Every edge a terminal scrolls to.
    fn scroll_edges() -> ScrollEdge {
        ScrollEdge::Top => [ScrollEdge::Top],
        ScrollEdge::Bottom => [ScrollEdge::Bottom],
    }
}

variant_cases! {
    /// Every section of the settings.
    fn settings_sections() -> SettingsSection {
        SettingsSection::Look => [SettingsSection::Look],
        SettingsSection::Machines => [SettingsSection::Machines],
        SettingsSection::Alerts => [SettingsSection::Alerts],
        SettingsSection::Updates => [SettingsSection::Updates],
        SettingsSection::Privacy => [SettingsSection::Privacy],
    }
}

variant_cases! {
    /// Every sender of feedback.
    fn feedback_froms() -> FeedbackFrom {
        FeedbackFrom::Person => [FeedbackFrom::Person],
        FeedbackFrom::Agent => [FeedbackFrom::Agent],
    }
}

variant_cases! {
    /// Every focus choice.
    fn focuses() -> Focus {
        Focus::Show => [Focus::Show],
        Focus::Background => [Focus::Background],
    }
}

variant_cases! {
    /// Every pin state.
    fn pins() -> Pin {
        Pin::Pinned => [Pin::Pinned],
        Pin::Unpinned => [Pin::Unpinned],
    }
}

variant_cases! {
    /// Every machine choice.
    fn machines() -> Machine {
        Machine::ThisComputer => [Machine::ThisComputer],
        Machine::Named(_) => [Machine::Named(MachineName::new("box")?)],
    }
}

variant_cases! {
    /// Every way to rename a space.
    fn space_renames() -> Rename<SpaceName> {
        Rename::Automatic => [Rename::Automatic],
        Rename::To(_) => [Rename::To(SpaceName::new("docs")?)],
    }
}

variant_cases! {
    /// Every way to rename a tab.
    fn tab_renames() -> Rename<TabName> {
        Rename::Automatic => [Rename::Automatic],
        Rename::To(_) => [Rename::To(TabName::new("docs")?)],
    }
}

variant_cases! {
    /// Every way to rename a pane.
    fn pane_renames() -> Rename<PaneName> {
        Rename::Automatic => [Rename::Automatic],
        Rename::To(_) => [Rename::To(PaneName::new("logs")?)],
    }
}

variant_cases! {
    /// Every way to refer to a space.
    fn space_refs() -> SpaceRef {
        SpaceRef::Index(_) => [SpaceRef::Index(Position::new(1)?)],
        SpaceRef::Id(_) => [SpaceRef::Id(SpaceId::new("sp1")?)],
    }
}

variant_cases! {
    /// Every way to refer to a tab.
    fn tab_refs() -> TabRef {
        TabRef::Index(_) => [TabRef::Index(Position::new(1)?)],
        TabRef::Id(_) => [TabRef::Id(TabId::new("tb1")?)],
    }
}

variant_cases! {
    /// Every place a tab can move to.
    fn tab_move_tos() -> TabMoveTo {
        TabMoveTo::Index(_) => [TabMoveTo::Index(Position::new(3)?)],
        TabMoveTo::By(_) => [TabMoveTo::By(-1)],
    }
}

variant_cases! {
    /// Every target of a pane focus.
    fn pane_targets() -> PaneTarget {
        PaneTarget::Pane(_) => [PaneTarget::Pane(PaneId::new("p1")?)],
        PaneTarget::Terminal(_) => [PaneTarget::Terminal(TerminalAddress::new("t2")?)],
    }
}

variant_cases! {
    /// Every thing a pane can swap with.
    fn swap_withs() -> SwapWith {
        SwapWith::Direction(_) => [SwapWith::Direction(Direction::Down)],
        SwapWith::Target(_) => [SwapWith::Target(PaneId::new("p2")?)],
    }
}

variant_cases! {
    /// Every way to scroll; the edges a scroll can go to are covered by `scroll_edges`.
    fn scrolls() -> Scroll {
        Scroll::Lines(_) => [Scroll::Lines(-3)],
        Scroll::Pages(_) => [Scroll::Pages(FiniteF64::new(0.5)?)],
        Scroll::To(_) => [Scroll::To(ScrollEdge::Top)],
    }
}

variant_cases! {
    /// Every place a selection can start.
    fn ats() -> At {
        At::Cursor => [At::Cursor],
        At::Cell { .. } => [At::Cell { row: 2, col: 0 }],
    }
}

variant_cases! {
    /// Every kind of node size.
    fn resizes() -> Resize {
        Resize::Px(_) => [Resize::Px(Px::new(200.0)?)],
        Resize::Weight(_) => [Resize::Weight(Weight::new(1.5)?)],
    }
}

variant_cases! {
    /// Every content of a new tab, a mod with and without properties.
    fn tab_contents() -> TabContent {
        TabContent::Shell { .. } => [
            TabContent::Shell {
                argv: Some(Argv::new(["sh"])?),
                task: Some(Task::new("run the tests")?),
                folder: Some(Folder::new(std::env::temp_dir())?),
            },
            TabContent::Shell { argv: None, task: None, folder: None }
        ],
        TabContent::Mod { .. } => [
            TabContent::Mod { module: ModName::new("files")?, props: props() },
            TabContent::Mod { module: ModName::new("files")?, props: None }
        ],
    }
}

variant_cases! {
    /// Every content of a new pane, a mod with and without properties.
    fn pane_contents() -> PaneContent {
        PaneContent::Shell { .. } => [
            PaneContent::Shell { argv: Some(Argv::new(["sh"])?) },
            PaneContent::Shell { argv: None }
        ],
        PaneContent::Mod { .. } => [
            PaneContent::Mod { module: ModName::new("files")?, props: props() },
            PaneContent::Mod { module: ModName::new("files")?, props: None }
        ],
    }
}

/// The commands that carry every case of every argument enum, one instance per case.
fn variant_instances() -> Result<Vec<Instance>, Box<dyn Error>> {
    let terminal = || TerminalAddress::new("t2");
    let mut built = Vec::new();
    for machine in machines()? {
        built.push(instance(&SpaceNew::new().machine(machine))?);
    }
    for focus in focuses()? {
        built.push(instance(&SpaceNew::new().focus(focus))?);
        built.push(instance(
            &TabNew::new(TabContent::Shell {
                argv: None,
                task: None,
                folder: None,
            })
            .focus(focus),
        )?);
    }
    for rename in space_renames()? {
        built.push(instance(&SpaceRename::new(rename))?);
    }
    for rename in tab_renames()? {
        built.push(instance(&TabRename::new(rename))?);
    }
    for rename in pane_renames()? {
        built.push(instance(&PaneRename::new(rename))?);
    }
    for space in space_refs()? {
        built.push(instance(&SpaceSelect::new(space))?);
    }
    for tab in tab_refs()? {
        built.push(instance(&TabSelect::new(tab))?);
    }
    for to in tab_move_tos()? {
        built.push(instance(&TabMove::new(to))?);
    }
    for content in tab_contents()? {
        built.push(instance(&TabNew::new(content))?);
    }
    for direction in split_directions()? {
        built.push(instance(&PaneSplit::new(
            direction,
            PaneContent::Shell { argv: None },
        ))?);
    }
    for content in pane_contents()? {
        built.push(instance(&PaneSplit::new(SplitDirection::Right, content))?);
    }
    for target in pane_targets()? {
        built.push(instance(&PaneFocus::new(target))?);
    }
    for with in swap_withs()? {
        built.push(instance(&PaneSwap::new(with))?);
    }
    for direction in directions()? {
        built.push(instance(&PaneGo::new(direction))?);
        built.push(instance(&PaneResize::new(direction))?);
        built.push(instance(&PaneSwap::new(SwapWith::Direction(direction)))?);
    }
    for place in places()? {
        built.push(instance(&TerminalOpen::new().place(place))?);
        built.push(instance(&TerminalKeep::new(terminal()?).place(place))?);
    }
    for pin in pins()? {
        built.push(instance(&TerminalPin::new(terminal()?, pin))?);
    }
    for target in click_targets()? {
        built.push(instance(&TerminalRightClick::new(target))?);
    }
    for mode in select_modes()? {
        built.push(instance(&TerminalSelect::new().mode(mode))?);
    }
    for at in ats()? {
        built.push(instance(&TerminalSelect::new().at(at))?);
    }
    for movement in selection_moves()? {
        built.push(instance(&TerminalAdjustSelection::new(movement))?);
    }
    for scroll in scrolls()? {
        built.push(instance(&TerminalScroll::new(scroll))?);
    }
    for edge in scroll_edges()? {
        built.push(instance(&TerminalScroll::new(Scroll::To(edge)))?);
    }
    for resize in resizes()? {
        built.push(instance(&LayoutResize::new(NodeId::new("n1")?, resize))?);
    }
    for section in settings_sections()? {
        built.push(instance(&AppSettings::new().section(section))?);
    }
    for from in feedback_froms()? {
        built.push(instance(&AppFeedback::new().from(from))?);
    }
    Ok(built)
}

/// One command of every type built with `new` alone, so it writes only what `new` is given.
fn minimal_instances() -> Result<Vec<Instance>, Box<dyn Error>> {
    let terminal = || TerminalAddress::new("t2");
    let node = || NodeId::new("n1");
    let layout = || LayoutName::new("main");
    Ok(vec![
        instance(&SpaceNew::new())?,
        instance(&SpaceClose::new())?,
        instance(&SpaceRename::new(Rename::<SpaceName>::Automatic))?,
        instance(&SpaceMove::new(SpaceId::new("sp1")?, Position::new(2)?))?,
        instance(&SpaceSelect::new(SpaceRef::Index(Position::new(1)?)))?,
        instance(&SpaceNext::new())?,
        instance(&SpacePrev::new())?,
        instance(&TabNew::new(TabContent::Shell {
            argv: None,
            task: None,
            folder: None,
        }))?,
        instance(&TabClose::new())?,
        instance(&TabRename::new(Rename::<TabName>::Automatic))?,
        instance(&TabMove::new(TabMoveTo::By(1)))?,
        instance(&TabSelect::new(TabRef::Index(Position::new(1)?)))?,
        instance(&TabSettle::new())?,
        instance(&TabUnsettle::new())?,
        instance(&TabNext::new())?,
        instance(&TabPrev::new())?,
        instance(&PaneSplit::new(
            SplitDirection::Right,
            PaneContent::Shell { argv: None },
        ))?,
        instance(&PaneClose::new())?,
        instance(&PaneRename::new(Rename::<PaneName>::Automatic))?,
        instance(&PaneResize::new(Direction::Left))?,
        instance(&PaneZoom::new())?,
        instance(&PaneFocus::new(PaneTarget::Pane(PaneId::new("p1")?)))?,
        instance(&PaneGo::new(Direction::Up))?,
        instance(&PaneSwap::new(SwapWith::Direction(Direction::Down)))?,
        instance(&TerminalOpen::new())?,
        instance(&TerminalKeep::new(terminal()?))?,
        instance(&TerminalPark::new(terminal()?))?,
        instance(&TerminalKill::new(terminal()?))?,
        instance(&TerminalPeek::new(terminal()?))?,
        instance(&TerminalShow::new())?,
        instance(&TerminalPin::new(terminal()?, Pin::Pinned))?,
        instance(&TerminalRightClick::new(ClickTarget::Menu))?,
        instance(&TerminalSelectAll::new())?,
        instance(&TerminalSelect::new())?,
        instance(&TerminalAdjustSelection::new(SelectionMove::LineEnd))?,
        instance(&TerminalClearSelection::new())?,
        instance(&TerminalScroll::new(Scroll::To(ScrollEdge::Top)))?,
        instance(&TerminalSearch::new())?,
        instance(&TerminalCopy::new())?,
        instance(&TerminalPaste::new())?,
        instance(&TerminalPasteImage::new())?,
        instance(&AgentFocus::new(terminal()?))?,
        instance(&AgentNext::new())?,
        instance(&AgentPrev::new())?,
        instance(&LayoutSwitch::new())?,
        instance(&LayoutClone::new(layout()?))?,
        instance(&LayoutSet::new(Layout::parse(r#"{"type":"column"}"#)?))?,
        instance(&LayoutToggle::new(node()?))?,
        instance(&LayoutShow::new(node()?))?,
        instance(&LayoutHide::new(node()?))?,
        instance(&LayoutResize::new(node()?, Resize::Px(Px::new(200.0)?)))?,
        instance(&LayoutSelect::new(node()?, NodeId::new("n2")?))?,
        instance(&LayoutReset::new())?,
        instance(&ModClone::new(ModName::new("files")?))?,
        instance(&ModOpen::new(ModName::new("files")?))?,
        instance(&OverlayClose::new())?,
        instance(&AppHelp::new())?,
        instance(&AppWelcome::new())?,
        instance(&AppWhatsNew::new())?,
        instance(&AppPalette::new())?,
        instance(&AppSettings::new())?,
        instance(&AppReset::new())?,
        instance(&AppTheme::new(ThemeName::new("dark")?))?,
        instance(&AppZoomIn::new())?,
        instance(&AppZoomOut::new())?,
        instance(&AppZoomReset::new())?,
        instance(&AppQuit::new())?,
        instance(&AppFeedback::new())?,
        instance(&AppCheckUpdates::new())?,
        instance(&AppUpdateCore::new())?,
        instance(&AppUpdate::new())?,
        instance(&AppHide::new())?,
        instance(&AppHideOthers::new())?,
        instance(&AppShowAll::new())?,
        instance(&WindowClose::new())?,
        instance(&WindowMinimize::new())?,
        instance(&WindowMaximize::new())?,
    ])
}

/// The entry of the table for a command name.
fn known_command(name: &str) -> Option<&'static KnownCommand> {
    KNOWN_COMMANDS.iter().find(|command| command.name == name)
}

/// The keys an instance writes, or the reason it writes none.
fn written_keys(instance: &Instance) -> Result<BTreeSet<&str>, String> {
    instance
        .arguments
        .as_object()
        .map(|written| written.keys().map(String::as_str).collect())
        .ok_or_else(|| {
            format!(
                "`{}` does not write a JSON object: {}",
                instance.name, instance.arguments
            )
        })
}

#[test]
fn every_command_name_is_known() -> Result<(), Box<dyn Error>> {
    let known: BTreeSet<&str> = KNOWN_COMMANDS.iter().map(|command| command.name).collect();
    let mut differences = Vec::new();
    for (kind, built) in [
        ("full", full_instances()?),
        ("minimal", minimal_instances()?),
    ] {
        let used: BTreeSet<&str> = built.iter().map(|instance| instance.name).collect();
        if built.len() != used.len() {
            differences.push(format!(
                "{} {kind} instances name only {} commands",
                built.len(),
                used.len()
            ));
        }
        for name in used.difference(&known) {
            differences.push(format!(
                "`{name}` is a {kind} command type and not in the table"
            ));
        }
        for name in known.difference(&used) {
            differences.push(format!(
                "`{name}` is in the table and has no {kind} command type"
            ));
        }
        assert_eq!(built.len(), 77);
    }
    assert!(
        differences.is_empty(),
        "the command types differ from the table:\n{}",
        differences.join("\n")
    );
    Ok(())
}

#[test]
fn every_command_writes_only_listed_args() -> Result<(), Box<dyn Error>> {
    let mut differences = Vec::new();
    let all = [
        full_instances()?,
        variant_instances()?,
        minimal_instances()?,
    ];
    for instance in all.iter().flatten() {
        let Some(known) = known_command(instance.name) else {
            differences.push(format!("`{}` is not in the table", instance.name));
            continue;
        };
        match written_keys(instance) {
            Ok(written) => {
                for key in written {
                    if !known.args.iter().any(|(listed, _)| *listed == key) {
                        differences.push(format!(
                            "`{}` writes `{key}`, which `KNOWN_COMMANDS` does not list for it \
                             (it lists {:?})",
                            instance.name,
                            known
                                .args
                                .iter()
                                .map(|(listed, _)| listed)
                                .collect::<Vec<_>>()
                        ));
                    }
                }
            }
            Err(difference) => differences.push(difference),
        }
    }
    differences.sort();
    differences.dedup();
    assert!(
        differences.is_empty(),
        "commands write keys the table does not list for them:\n{}",
        differences.join("\n")
    );
    Ok(())
}

#[test]
fn every_required_arg_is_written_by_the_minimal_command() -> Result<(), Box<dyn Error>> {
    let mut differences = Vec::new();
    for instance in &minimal_instances()? {
        let Some(known) = known_command(instance.name) else {
            differences.push(format!("`{}` is not in the table", instance.name));
            continue;
        };
        match written_keys(instance) {
            Ok(written) => {
                for (arg, required) in known.args {
                    if *required && !written.contains(arg) {
                        differences.push(format!(
                            "`{}` requires `{arg}` and its command built with `new` alone does \
                             not write it (it writes {written:?})",
                            instance.name
                        ));
                    }
                }
            }
            Err(difference) => differences.push(difference),
        }
    }
    assert!(
        differences.is_empty(),
        "commands leave required args unwritten:\n{}",
        differences.join("\n")
    );
    Ok(())
}

#[test]
fn every_listed_arg_is_written_by_some_instance() -> Result<(), Box<dyn Error>> {
    let mut written: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    let all = [
        full_instances()?,
        variant_instances()?,
        minimal_instances()?,
    ];
    let mut differences = Vec::new();
    for instance in all.iter().flatten() {
        match written_keys(instance) {
            Ok(keys) => written
                .entry(instance.name)
                .or_default()
                .extend(keys.into_iter().map(str::to_owned)),
            Err(difference) => differences.push(difference),
        }
    }
    for known in KNOWN_COMMANDS {
        let keys = written.get(known.name).cloned().unwrap_or_default();
        for (arg, _) in known.args {
            if !keys.contains(*arg) {
                differences.push(format!(
                    "`{}` lists `{arg}` and no instance of its command writes it",
                    known.name
                ));
            }
        }
    }
    assert!(
        differences.is_empty(),
        "listed args that no instance writes:\n{}",
        differences.join("\n")
    );
    Ok(())
}
