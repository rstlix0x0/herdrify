---
status: done
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

## Review findings

Reviewer verdict: compliant with amendments; blocking: none. Scoped gate re-run by the reviewer: `dod-app.sh` → `exit=0`, no warning/error lines; 171 unit, 3 integration, 243 doctests passed. `cargo test -p herdrify-app protocol::commands` → 28 passed. The reviewer cross-checked all 77 `NAME`s, argument keys and required flags against `herdr-app-0.1.17-commands.json` (all match, or are stricter where probes record a refusal) and recounted 60 `Ack` / 6 shaped / 11 `RawReply`.

- open 🟡 `commands/app.rs:626` — the `AppHide` doc says it acts on the person's other apps; `app.hide` hides Herdr itself ("Hide the app (macOS)"). That reason fits only `AppHideOthers` and `AppShowAll`.
- open 🟡 `commands/tab.rs:104` — the `TabClose` doc says herdr-app asks first unless confirmed; the evidence records that only for closing the last tab (which closes its space). `tab.close` otherwise returns ok.
- open 🟡 `commands/app.rs:954` — the test name `command_totals_match_the_spec` (dictated by Task 7) carries the workflow word "spec"; e.g. `command_totals_match_the_recorded_list`.
- open 🔵 `commands/space.rs:146` — the `SpaceClose` doc says herdr-app "asks first"; for a scripted caller it refuses with `space.close ends its terminals: pass confirmed: true`.
- open 🔵 `commands/app.rs:532,564,596,626,656,686` — "so it was never run" narrates evidence gathering instead of stating a fact about the command; those lines and `app.rs:12` run past 100 columns.
- open 🔵 `commands/app.rs:24`, `protocol/mod.rs:27` — doc comments state the counts (77; 60/6/11), which can go stale; point to the totals test instead.
- open 🔵 `commands/app.rs:265` — the `AppSettings` doc repeats itself instead of saying what a section does.
- open 🔵 `commands/app.rs:495,512` — `AppFeedback::message` is a bare `String`, while other text arguments are newtypes.
- open 🔵 `commands/layout.rs:244` — the `LayoutResize` doc's reasoning for a required size does not follow as written (the claim holds: `layout.resize {"id":"sidebar"}` returns ok, so a missing size is a silent no-op).
- open 🔵 `commands/space.rs:44,384`, `tab.rs:435` — the doctest/test program name `"claude"`; a neutral program name avoids the AI-name rule.
- open 🔵 `commands/terminal/lifecycle.rs:426` — 101 columns; writes `folder` where the other files write `folder.as_str()`.
- open 🔵 `commands/*.rs` — `Eq` is derived unevenly across structurally similar structs.
- open 🔵 `commands/*.rs` — no reversion guard on `#[non_exhaustive]` for field-less structs (a `compile_fail` doctest would catch its removal).
- open 🔵 `commands/*.rs` — the per-file name tests repeat each `NAME` literal; comparing against `KNOWN_COMMANDS` would catch a typo in both places (plan 13 is the stated guard).
- open 🔵 setters and `new` functions carry no `# Examples` of their own; `PaneSplit::pane` and `TerminalSelect::terminal` are covered by unit tests only.

## Deviations

- 2026-11-24 — `cargo make dod` was replaced by the scoped gate (fmt, workspace clippy and doc with `-D warnings`, herdrify-app tests and doctests). The full gate fails only in herdrify-hc `tests/live.rs` `live_schema_matches_the_fixture`, against the installed `hc 0.1.22`, which this plan does not touch.
- 2026-11-24 — The tasks ran as three coders in series (Tasks 1–3, 4–5, 6–7), not in parallel: every task edits `commands/mod.rs`, glob re-exports are forbidden, and a red test in one file breaks the build for every other coder.
- 2026-11-24 — Task 6: the named tests `agent_and_layout_commands_serialize_full_and_minimal` / `_name_their_wire_command` were split per file into `agent_commands_*` (in `agent.rs`) and `layout_commands_*` (in `layout.rs`), because unit tests live with their file. Both named assertions (`LayoutSet` → `{"layout":…}`, `LayoutResize` with `Px` → `{"id","px"}`) are present.
- 2026-11-24 — Task 7: `command_totals_match_the_spec` lives in `app.rs`'s test module and imports all 77 types through `crate::protocol`.
- 2026-11-24 — Where the plan was silent: `confirmed` is a plain `bool` skipped when false (`std::ops::Not::not`); commands whose arguments are all optional have `const fn new()` and derive `Default` (clippy `new_without_default`); the name tests check the reply type through a const helper `replies_with::<C, O>()`.

## Fix pass (2026-11-24, on the author's word "fix all findings")

- fixed: the `AppHide` doc says it hides Herdr itself; the "other apps" reason stays only on `AppHideOthers` and `AppShowAll`.
- fixed: the `TabClose` doc names the last-tab case (closing the last tab closes its space and asks first unless confirmed; AGENTS.md:594 in the evidence).
- fixed: Task 7's test is now `command_totals_match_the_recorded_list` (`grep -rn command_totals_match crates` finds only it). Task 7's text above still names the old test; this line records the rename.
- fixed: `SpaceClose` documents the refusal `space.close ends its terminals: pass confirmed: true` (probe-commands-output.txt:61).
- fixed: the six "so it was never run" sentences state facts about each command (why its reply has no known shape); `app.rs` lines rewrapped; `awk 'length>100'` over the command files prints nothing outside byte-for-byte fixtures.
- fixed: no doc comment states the counts; `app.rs` describes the invariant and points to `KNOWN_COMMANDS`.
- fixed: the `AppSettings` doc says a section opens the settings at that section.
- fixed (kept `String`): spec §5.10 types `AppFeedback`'s `message` as `String` and the recorded list sets no rule; the doc says any text, the empty one included, is accepted.
- fixed: the `LayoutResize` doc says the size is required because `layout.resize` without one answers ok and changes nothing (probe-gaps-output.txt:33).
- fixed: the program name in doctests and tests is `htop`.
- fixed: `terminal/lifecycle.rs` wraps the long line and uses `folder.as_str()`.
- fixed: `Eq` is derived on every command struct whose fields allow it; `LayoutResize`, `PaneResize` and `TerminalScroll` hold floats (`E0277` when tried).
- fixed: each of the 28 field-less structs carries a `compile_fail,E0639` doctest building `Type {}`; reversion check: with `#[non_exhaustive]` removed from `AppHelp`, `cargo test -p herdrify-app --doc AppHelp` → "compile fail ... FAILED"; restored.
- fixed: the totals test also compares the 77 `NAME`s, as a set, with `KNOWN_COMMANDS`.
- fixed: every struct's doctest uses all its setters (added `PaneSplit::pane`, `PaneSwap::pane`, `TabMove::id`, `TerminalOpen::folder`/`name`, `TerminalSelect`/`AdjustSelection`/`Scroll`/`RightClick::terminal`).
- Gate after the pass: `dod-app.sh` → `exit=0`, no warning/error lines; 195 unit, 5 + 3 integration, 254 + 29 doctests passed.
