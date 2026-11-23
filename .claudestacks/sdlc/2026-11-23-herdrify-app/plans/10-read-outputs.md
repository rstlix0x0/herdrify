---
status: approved
created: 2026-11-23
depends-on: [03, 09]
built-on: aab842c3c7d3
---

# Read Outputs Implementation Plan

**Goal:** Decode the outputs of `commands`, `ls` and `status` into typed values.

**Architecture:** Three pure concept files in `protocol` (spec §2). The shared rules of plan 02's
Architecture apply verbatim, doctests included.
- Output types ignore unknown fields; `Client::raw` reads those.
- Output word lists are open enums.
- `status` reuses `Edge` and `Align` from `files` (plan 03).
- This plan depends on plan 09 only to order the edits of `protocol/mod.rs`.

**Tech Stack:** Rust 1.94, serde 1.0.229, serde_json 1.0.154.

**Content authority:** spec §5.5, §5.6, §5.7; §3 F2, F13, F14, F17.

---

## Files that change

```
crates/herdrify-app/src/protocol/catalog.rs   — create: CommandInfo, ArgInfo, KnownCommand, KNOWN_COMMANDS
crates/herdrify-app/src/protocol/listing.rs   — create: Listing, SpaceEntry, TabEntry, TerminalEntry, RecentSpace, AgentStatus
crates/herdrify-app/src/protocol/status.rs    — create: WindowStatus, Float
crates/herdrify-app/src/protocol/mod.rs       — modify: mods and re-exports
```

## Order of work

### Task 1 — Add the command catalog

**Files:**
- Create `crates/herdrify-app/src/protocol/catalog.rs`.
- Modify `crates/herdrify-app/src/protocol/mod.rs`.

**Change:**
- `CommandInfo { name: CommandName, args: Vec<ArgInfo>, about: String }` and
  `ArgInfo { name: String, required: bool }`, both `Deserialize`.
- `KnownCommand { name: &'static str, args: &'static [(&'static str, bool)] }`.
- `pub const KNOWN_COMMANDS: &[KnownCommand]`: the 77 entries, transcribed in order from
  `.claudestacks/sdlc/2026-11-23-herdrify-app/evidence/herdr-app-0.1.17-commands.json`.

**Proof:**
- `known_commands_has_77_unique_names` — 77 entries, all names distinct.
- `known_commands_include_the_0_1_17_additions` — `pane.rename` has `[("pane", false), ("name",
  false)]`, and `pane.resize` has `[("direction", true), ("pane", false), ("px", false)]`.
- `command_info_decodes_a_listing` — the `space.new` object from the fixture decodes, with seven
  args, none required.
- A doctest finding `space.new` in `KNOWN_COMMANDS`.

**Done when:** `cargo test -p herdrify-app protocol::catalog` passes and the Definition of Done is
green. Commit `feat(herdrify-app): add the command catalog`.

### Task 2 — Add the ls listing

**Files:**
- Create `crates/herdrify-app/src/protocol/listing.rs`.
- Modify `crates/herdrify-app/src/protocol/mod.rs`.

**Change:** The spec §5.6 types, each `Deserialize`. `AgentStatus` is `Working`, `Blocked`,
`Done`, `Idle` or `Other(String)`.

**Proof:**
- `listing_decodes_a_recorded_ls` — the `ls` JSON at
  `.claudestacks/sdlc/2026-11-23-herdrify-app/evidence/probe-views-output.txt:10-46`, embedded as a
  string literal, decodes with one space, `sub_agents == ["t2"]` and `recent` holding one entry
  named `gone`.
- `listing_decodes_an_empty_window` — `{"spaces":[],"recent":[]}` decodes.
- `agent_status_keeps_unknown_words` — `"working"` → `Working`, `"thinking"` →
  `Other("thinking")`, `null` → `None`.
- `listing_ignores_unknown_fields` — an extra field at each level is accepted.
- A doctest decoding a one-space listing.

**Done when:** `cargo test -p herdrify-app protocol::listing` passes and the Definition of Done is
green. Commit `feat(herdrify-app): add the ls listing`.

### Task 3 — Add the window status

**Files:**
- Create `crates/herdrify-app/src/protocol/status.rs`.
- Modify `crates/herdrify-app/src/protocol/mod.rs`.

**Change:** `WindowStatus` and `Float` of spec §5.7, each `Deserialize`. `Float` has exactly
`edge`, `align`, `vertical`, `dragging` and `hovered`.

**Proof:**
- `status_decodes_a_fresh_window` — the object at `probe-window-output.txt:11-25` decodes, with
  `space`, `tab`, `focused_pane` and `agent` all `None`, and `overlays == ["frame-welcome"]`.
- `status_decodes_floats_and_errors` — the ST3 object at `probe-views-output.txt:88-113` decodes,
  with float `clippy` (edge `Left`, align `End`, `vertical == true`, `dragging == false`,
  `hovered == false`), layout `win98` and one error string starting `mod broken:`.
- `float_ignores_other_keys` — a float object with an added `"bounds":[1,2,3,4]` decodes.
- `status_ignores_unknown_fields` — an extra top-level field is accepted.
- A doctest decoding a minimal status.

**Done when:** `cargo test -p herdrify-app protocol::status` passes and the Definition of Done is
green. Commit `feat(herdrify-app): add the window status`.

---

## Risks

- The `KNOWN_COMMANDS` transcription is checked against the fixture (plan 13) and the installed
  herdr-app (plan 19).

## Proof

- `cargo make dod` is green.
