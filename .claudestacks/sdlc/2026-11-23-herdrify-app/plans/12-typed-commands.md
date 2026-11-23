---
status: approved
created: 2026-11-23
depends-on: [03, 11]
built-on: aab842c3c7d3
---

# Typed Commands Implementation Plan

**Goal:** Type all 77 herdr-app 0.1.17 commands as `Command` structs.

**Architecture:** One file per command group in `protocol/commands/`, and `terminal/` with two
role files (spec §2). The source of truth for each command is spec §5.10's table. All tasks share
one structure:
- **Required arguments** (**R**) are `new(...)` parameters.
- **Optional arguments** are argument-less or typed setters returning `Self`.
- **`confirmed`** is an argument-less `.confirmed()` setter.
- **Serialization** is `#[derive(Serialize)]`, with `skip_serializing_if = "Option::is_none"` on
  optional fields and `#[serde(flatten)]` on one-of argument enums.
- **The trait** is implemented with `NAME` as the wire name and `Output` as the table's output.

Each struct is `#[non_exhaustive]`, `Debug`, `Clone` and `PartialEq`. The shared rules of plan
02's Architecture apply verbatim, and each command type's doc comment carries a doctest
building it.

**Tech Stack:** Rust 1.94, serde 1.0.229, serde_json 1.0.154.

**Content authority:** spec §5.10 (the table, requiredness notes, totals).

---

## Files that change

```
crates/herdrify-app/src/protocol/commands/space.rs                 — create: space.* (7)
crates/herdrify-app/src/protocol/commands/tab.rs                   — create: tab.* (9)
crates/herdrify-app/src/protocol/commands/pane.rs                  — create: pane.* (8)
crates/herdrify-app/src/protocol/commands/terminal/mod.rs          — create: export-only table of contents of the terminal commands
crates/herdrify-app/src/protocol/commands/terminal/lifecycle.rs    — create: terminal.open/keep/park/kill/peek/show/pin/right_click
crates/herdrify-app/src/protocol/commands/terminal/selection.rs    — create: terminal.select_all/select/adjust_selection/clear_selection/copy/paste/paste_image/scroll/search
crates/herdrify-app/src/protocol/commands/agent.rs                 — create: agent.* (3)
crates/herdrify-app/src/protocol/commands/layout.rs                — create: layout.* (9)
crates/herdrify-app/src/protocol/commands/app.rs                   — create: mod.*, overlay.close, app.*, window.* (24)
crates/herdrify-app/src/protocol/commands/mod.rs                   — modify: mods and re-exports
```

## Order of work

Every task below has the same proof shape:
- `<group>_commands_serialize_full_and_minimal` — for each command in the file, a full instance
  (every setter used) and a minimal one (`new` only) serialize to exactly the JSON object spec
  §5.10's arguments imply.
- `<group>_commands_name_their_wire_command` — each `NAME` is the wire name.

### Task 1 — Type the space commands

**Files:**
- Create `crates/herdrify-app/src/protocol/commands/space.rs`.
- Modify `crates/herdrify-app/src/protocol/commands/mod.rs`.

**Change:** `SpaceNew`, `SpaceClose`, `SpaceRename`, `SpaceMove`, `SpaceSelect`, `SpaceNext` and
`SpacePrev`: one type per wire name. Their arguments and outputs are §5.10's rows.

**Proof:**
- `space_commands_serialize_full_and_minimal` — e.g. `SpaceNew` full gives
  `{"name","folder","tab","argv","task","machine","focus"}` and minimal gives `{}`;
  `SpaceRename::new(Rename::Automatic)` gives `{"name":""}`.
- `space_commands_name_their_wire_command`.

**Done when:** `cargo test -p herdrify-app protocol::commands::space` passes and the Definition of
Done is green. Commit `feat(herdrify-app): type the space commands`.

### Task 2 — Type the tab commands

**Files:**
- Create `crates/herdrify-app/src/protocol/commands/tab.rs`.
- Modify `crates/herdrify-app/src/protocol/commands/mod.rs`.

**Change:** `TabNew` (`Output = TabMade`, required `TabContent`), `TabClose`, `TabRename`,
`TabMove`, `TabSelect`, `TabSettle`, `TabUnsettle`, `TabNext` and `TabPrev`.

**Proof:**
- `tab_commands_serialize_full_and_minimal` — e.g. `TabNew::new(TabContent::Mod{…})` never writes
  `argv`; `TabMove::new(TabMoveTo::By(-1))` gives `{"by":-1}`.
- `tab_commands_name_their_wire_command`.

**Done when:** `cargo test -p herdrify-app protocol::commands::tab` passes and the Definition of
Done is green. Commit `feat(herdrify-app): type the tab commands`.

### Task 3 — Type the pane commands

**Files:**
- Create `crates/herdrify-app/src/protocol/commands/pane.rs`.
- Modify `crates/herdrify-app/src/protocol/commands/mod.rs`.

**Change:**
- `PaneSplit` (`Split`), `PaneClose`, `PaneRename` (required `Rename<PaneName>`), `PaneZoom`,
  `PaneFocus` (`PaneTarget`), `PaneGo` and `PaneSwap` (`SwapWith`).
- `PaneResize` (`Output = RawReply`, required `Direction`, optional `pane` and `px: Px`). Its doc
  comment says no success was recorded on 0.1.17.

**Proof:**
- `pane_commands_serialize_full_and_minimal` — e.g. `PaneSplit::new(Right, PaneContent::Shell{…})`
  gives `{"direction":"right","argv":[…]}`; `PaneRename::new(Rename::Automatic)` gives
  `{"name":""}`.
- `pane_commands_name_their_wire_command`.

**Done when:** `cargo test -p herdrify-app protocol::commands::pane` passes and the Definition of
Done is green. Commit `feat(herdrify-app): type the pane commands`.

### Task 4 — Type the terminal lifecycle commands

**Files:**
- Create `crates/herdrify-app/src/protocol/commands/terminal/mod.rs`.
- Create `crates/herdrify-app/src/protocol/commands/terminal/lifecycle.rs`.
- Modify `crates/herdrify-app/src/protocol/commands/mod.rs`.

**Change:** `TerminalOpen` (`Opened`), `TerminalKeep` (`Kept`), `TerminalPark` (`Parked`),
`TerminalKill`, `TerminalPeek`, `TerminalShow`, `TerminalPin` (`Pin`) and `TerminalRightClick`
(`ClickTarget`).

**Proof:**
- `terminal_lifecycle_commands_serialize_full_and_minimal`
- `terminal_lifecycle_commands_name_their_wire_command`

**Done when:** `cargo test -p herdrify-app protocol::commands::terminal::lifecycle` passes and the
Definition of Done is green. Commit `feat(herdrify-app): type the terminal lifecycle commands`.

### Task 5 — Type the terminal selection commands

**Files:**
- Create `crates/herdrify-app/src/protocol/commands/terminal/selection.rs`.
- Modify `crates/herdrify-app/src/protocol/commands/terminal/mod.rs`.

**Change:**
- `TerminalSelectAll`, `TerminalSelect` (`At`, `SelectMode`), `TerminalAdjustSelection`
  (`SelectionMove`), `TerminalClearSelection`, `TerminalScroll` (`Scroll`) and `TerminalSearch`.
- `TerminalCopy`, `TerminalPaste` and `TerminalPasteImage` (`Output = RawReply`, each documented
  as touching the system clipboard).

**Proof:**
- `terminal_selection_commands_serialize_full_and_minimal` — e.g.
  `TerminalScroll::new(Scroll::Pages(0.5))` gives `{"pages":0.5}`.
- `terminal_selection_commands_name_their_wire_command`.

**Done when:** `cargo test -p herdrify-app protocol::commands::terminal::selection` passes and the
Definition of Done is green. Commit `feat(herdrify-app): type the terminal selection commands`.

### Task 6 — Type the agent and layout commands

**Files:**
- Create `crates/herdrify-app/src/protocol/commands/agent.rs` — `AgentFocus`, `AgentNext`,
  `AgentPrev`.
- Create `crates/herdrify-app/src/protocol/commands/layout.rs`:
  - `LayoutSwitch`, `LayoutClone`
  - `LayoutSet`, whose required argument is `files::Layout`
  - `LayoutToggle`, `LayoutShow`, `LayoutHide`
  - `LayoutResize` (`Resize`), `LayoutSelect`, `LayoutReset`
- Modify `crates/herdrify-app/src/protocol/commands/mod.rs`.

**Change:** As §5.10's rows. All of these output `Ack`. `LayoutClone`'s optional `as` argument
is the setter `.as_name(LayoutName)`, serialized as `"as"` (spec §8).

**Proof:**
- `agent_and_layout_commands_serialize_full_and_minimal` — `LayoutSet::new(layout)` gives
  `{"layout":<the layout's JSON>}`; `LayoutResize::new(id, Resize::Px(px))` gives
  `{"id":…,"px":…}`.
- `agent_and_layout_commands_name_their_wire_command`.

**Done when:** `cargo test -p herdrify-app protocol::commands` passes and the Definition of Done is
green. Commit `feat(herdrify-app): type the agent and layout commands`.

### Task 7 — Type the mod, overlay, app and window commands

**Files:**
- Create `crates/herdrify-app/src/protocol/commands/app.rs`.
- Modify `crates/herdrify-app/src/protocol/commands/mod.rs`.

**Change:**
- `ModClone` (its `as` setter is `.as_name(ModName)`, serialized as `"as"`), `ModOpen` and
  `OverlayClose`.
- `AppHelp`, `AppWelcome`, `AppWhatsNew`, `AppPalette`, `AppSettings` (`SettingsSection`),
  `AppReset`, `AppTheme`, `AppZoomIn`, `AppZoomOut`, `AppZoomReset` and `AppQuit`, all `Ack`.
- `AppFeedback`, `AppCheckUpdates`, `AppUpdateCore`, `AppUpdate`, `AppHide`, `AppHideOthers` and
  `AppShowAll`, all `RawReply`, each documenting why its output is untyped.
- `WindowClose` (`Ack`; its doc comment says the window stops answering afterwards),
  `WindowMinimize` and `WindowMaximize`.

**Proof:**
- `app_commands_serialize_full_and_minimal`
- `app_commands_name_their_wire_command`
- `command_totals_match_the_spec` — across all command files, the count of types with each
  `Output` is 60 `Ack`, 6 shaped and 11 `RawReply`. The test lists the 77 types.

**Done when:** `cargo test -p herdrify-app protocol::commands` passes and the Definition of Done is
green. Commit `feat(herdrify-app): type the mod, overlay, app and window commands`.

---

## Risks

- Volume: 77 structs with similar shapes. A wrong arg name or required flag is caught by plan 13
  against the fixture.
- `#[non_exhaustive]` stops callers from building with literals, so `new` must take every
  required argument.

## Proof

- Plan 13 checks every command against `herdr-app-0.1.17-commands.json`. `cargo make dod` is
  green.
