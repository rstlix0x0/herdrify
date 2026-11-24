---
status: done
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

## Review findings

Tasks 1–3 reviewed together: SPEC COMPLIANT-WITH-AMENDMENTS, code 0🔴 3🟡 3🔵 1❓, spec 0🔴 1🟡 1🔵, blocking: none.
- Verified (reviewer, mechanical): `KNOWN_COMMANDS` equals `evidence/herdr-app-0.1.17-commands.json` (jq vs the parsed Rust literal, diffed: 77 entries; names, arg names, required flags and order identical). `RECORDED_LS` equals `probe-views-output.txt:10-46` (cmp). `FRESH_WINDOW` equals `probe-window-output.txt:11-25` minus its `stdout=` prefix (cmp). `FLOATS_AND_ERRORS` equals `probe-views-output.txt:88-114` (cmp).
- 🟡 open: `protocol/catalog.rs:5`, `:11`, `:71`, `protocol/mod.rs:18-19` — the docs call the table "the commands the supported herdr-app release lists"; it is 0.1.17 (`AppVersion::PROVEN`), while `MINIMUM` 0.1.16 lists 75 (no `pane.rename`, `pane.resize`).
- 🟡 open: `protocol/listing.rs:86`, `:88`, `:119`, `:157`, `:179`, `:181` — the `name` and `folder` fields are bare `String` beside the `SpaceName`/`TabName`/`TerminalName`/`Folder` newtypes. The coder followed spec §5.6, which types them `String`; the field docs do not say why.
- 🟡 open: `protocol/listing.rs:122` — the `TabEntry.settled` doc says the tab's terminals have settled; herdr-app 0.1.17's `tab.settle` description says a settled tab is put away (out of the lists, still running).
- 🟡 open (Task 1 Proof): `command_info_decodes_a_listing` decodes a hand-typed `space.new` object (short `about`, plus `"extra":1`), not the fixture's object with its real `about` text.
- 🔵 open: `protocol/catalog.rs:38` — `ArgInfo.name` is a bare `String`.
- 🔵 open: `protocol/catalog.rs:458` — only count, uniqueness, two entries and `space.new`'s arity are tested. The plan's Risks leave the full check to plans 13 and 19.
- 🔵 open: `protocol/listing.rs:229`, `:259`, `protocol/status.rs:159` — the byte-for-byte recorded literals carry a local home path and scratch paths into crate source.
- 🔵 open: `protocol/status.rs:51` — `zoom`, `font_size`, `ui_font_size` are `f64`, not `files::Number`. The `//!` explains why (status is read-only), at the cost of `Eq` on `WindowStatus`.
- ❓ open: `protocol/status.rs:65` — `agent: Option<TerminalAddress>`: every recorded status has `"agent": null`, so no recording shows its type.
- Verified: `cargo test -p herdrify-app --lib protocol::` → `test result: ok. 33 passed; 0 failed` (the eleven new tests included).

## Deviations

- 2026-11-24 — Definition of Done: `cargo make dod` fails only in `herdrify-hc` `tests/live.rs::live_schema_matches_the_fixture` (installed `hc 0.1.22`, environment drift outside this chain). Gate run instead: workspace fmt-check, clippy `-D warnings`, `RUSTDOCFLAGS="-D warnings" cargo doc`, plus `cargo test -p herdrify-app` all-targets and `--doc` → `exit=0`, no warning/error lines; `132 passed`, `3 passed`, `116 passed`.
- 2026-11-24 — The three tasks share `protocol/mod.rs`, so they ran one after the other and had one review.
- 2026-11-24 — Task 3: the plan cites `probe-views-output.txt:88-113`, which is not valid JSON (the closing `}` is line 114). Built with lines 88-114. `probe-window-output.txt:11-25` starts with a `stdout=` prefix, which was removed from the literal.
- 2026-11-24 — Task 3: the coder wrote the tests and the types in one step; no separate failing run was observed.
- 2026-11-24 — Task 1: `KNOWN_COMMANDS` was generated with jq from the evidence file, not typed by hand.

## Fix pass (2026-11-24, on the author's word "fix all of it")

- fixed: the catalog docs and `protocol/mod.rs` name herdr-app 0.1.17 (`AppVersion::PROVEN`) and say once that 0.1.16 (`MINIMUM`) lists 75 commands, without `pane.rename` and `pane.resize`.
- fixed: each plain-`String` field (`SpaceEntry.name`/`folder`, `TabEntry.name`, `TerminalEntry.name`, `RecentSpace.name`/`folder`, `ArgInfo.name`) documents why: spec §5.6 types it `String`, and it holds herdr-app's output unchecked so a value never fails to decode. A remote machine's folder need not be absolute.
- fixed: `TabEntry.settled` doc matches herdr-app 0.1.17's `tab.settle` description: put away on this screen, out of the lists, still running.
- fixed: `command_info_decodes_a_recorded_listing_entry` decodes the real `space.new` object (`jq -c '.[0]'` of the evidence file); new `command_info_ignores_unknown_fields`.
- fixed: in the recorded test literals, `/Users/hiraq` became `/home/user` and `/private/tmp/hzw` became `/tmp/hzw`, and each literal is commented "recorded from herdr-app 0.1.17, with local paths replaced". This deviates from the byte-for-byte copy only in those path prefixes. `grep -rn 'hiraq\|/private/tmp' crates/herdrify-app/` prints nothing.
- resolved ❓: `WindowStatus.agent` stays `Option<TerminalAddress>` (spec §5.7). Its doc says it names the focused agent's terminal and that a fresh window reports null, as every recording does.
- kept (spec): `zoom`, `font_size`, `ui_font_size` stay `f64`. Spec §5.7: "Output numbers are `f64` because outputs are never written back."
- kept (plan): the full `KNOWN_COMMANDS` check stays with plans 13 and 19, per this plan's Risks. The reviewer diffed all 77 entries against the evidence and found them identical.
- kept: `known_commands_has_77_unique_names` returns `()`; it makes no fallible call, and a `Result` return trips clippy `unnecessary_wraps` under `-D warnings`.
- Gate after the pass: `dod-app.sh` → `exit=0`, no warning/error lines; `140 passed`, `3 passed`, `123 passed`.
