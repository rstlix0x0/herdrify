---
status: done
created: 2026-11-23
depends-on: [10]
built-on: aab842c3c7d3
---

# Command Vocabulary Implementation Plan

**Goal:** Define the `Command` trait with the argument and output types every command shares.

**Architecture:** `protocol/commands/` starts here:
- `mod.rs` is export-only, and `protocol/mod.rs` declares it as a private `mod commands;` with
  flat re-exports.
- `command.rs` is a pure trait file (unit-test-mandate exemption #3, cited in its `//!`).
- `outputs.rs` and `words.rs` hold the shared types.

The one-of argument enums serialize to the wire keys of their variant, so a command struct
flattens them. The single-value enums (`Focus`, `Pin`, `Machine`, `Rename`, `At`) serialize as
the value of one key. The shared rules of plan 02's Architecture apply verbatim, doctests
included. This plan depends on plan 10 only to order the edits of `protocol/mod.rs`.

**Tech Stack:** Rust 1.94, serde 1.0.229, serde_json 1.0.154, thiserror 2.0.21.

**Content authority:** spec §5.8, §5.9, the `Command` trait in §5.10; §10 (`ArgumentRangeError`);
§11 (the argument rows).

---

## Files that change

```
crates/herdrify-app/src/protocol/commands/mod.rs       — create: export-only table of contents of the commands
crates/herdrify-app/src/protocol/commands/command.rs   — create: trait Command
crates/herdrify-app/src/protocol/commands/outputs.rs   — create: Created, TabMade, Split, Opened, Kept, Parked, Ack, RawReply
crates/herdrify-app/src/protocol/commands/words.rs     — create: input enums, argument enums, Position, Px, Weight, FiniteF64, ArgumentRangeError
crates/herdrify-app/src/protocol/mod.rs                — modify: mod commands and re-exports
```

## Order of work

### Task 1 — Add the Command trait and the outputs

**Files:**
- Create `commands/mod.rs`, `commands/command.rs` (with its exemption #3 line) and
  `commands/outputs.rs`.
- Modify `crates/herdrify-app/src/protocol/mod.rs`.

**Change:**
- `pub trait Command: Serialize { const NAME: &'static str; type Output: DeserializeOwned; }`.
- The spec §5.8 shapes, each `Deserialize`: `Created`, `TabMade`, `Split`, `Opened`, `Kept` (`tab`,
  `terminal` and `pending` optional) and `Parked`.
- `Ack` decodes a `Map` first, then refuses an object whose `ok` is not `true` with
  `serde::de::Error::custom("expected \"ok\": true")`.
- `RawReply(serde_json::Value)`.

**Proof:**
- `outputs_decode_recorded_replies` — each recorded stdout from spec §5.8's evidence decodes:
  - `Created`
  - `TabMade` shell and mod
  - `Split` with and without a terminal
  - `Opened` tab and pane
  - `Kept` full and `{pane}` only
  - `Parked`
- `ack_accepts_extra_fields` — `{"ok":true,"x":1}` decodes.
- `ack_refuses_exactly` — `{"ok":false}` and `{}` fail with exactly `expected "ok": true`, and `7`
  with exactly ``invalid type: integer `7`, expected a map at line 1 column 1``.
- `raw_reply_takes_any_json` — `7`, `null` and an object decode.
- A doctest decoding `Created`.

**Done when:** `cargo test -p herdrify-app protocol::commands` passes and the Definition of Done is
green. Commit `feat(herdrify-app): add the Command trait and its outputs`.

### Task 2 — Add the input words and numbers

**Files:**
- Create `crates/herdrify-app/src/protocol/commands/words.rs`.
- Modify `crates/herdrify-app/src/protocol/commands/mod.rs`.

**Change:**
- The closed input enums of spec §5.9: `SplitDirection`, `Direction`, `Place`, `ClickTarget`,
  `SelectMode`, `SelectionMove`, `ScrollEdge`, `SettingsSection` and `FeedbackFrom`, each
  serializing as its wire word.
- `Position(NonZeroU32)` with `new(u32)`, and `Px`, `Weight` and `FiniteF64` with `new(f64)`.
- Refusals use `ArgumentRangeError { argument, value, rule }`, with spec §5.9's table and Display
  `<value> is out of range: <rule>`.

**Proof:**
- `input_words_serialize_as_wire_words` — every variant of every one of the nine enums serializes
  to its spec word: all 2 + 4 + 3 + 2 + 3 + 10 + 2 + 5 + 2 words, listed in the test.
- `position_refuses_zero` — `Position::new(0)` displays `0 is out of range: positions start at 1`
  with `argument == "index"`; `Position::new(1)` serializes as `1`.
- `px_weight_pages_ranges`:
  - `Px::new(-5.0)` displays `-5 is out of range: must be finite and not negative`
  - `Px::new(0.0)` is `Ok`
  - `Weight::new(0.0)` displays `0 is out of range: must be finite and above 0`
  - `FiniteF64::new(f64::NAN)` displays `NaN is out of range: must be finite`
- Doctests on `Position::new` and `Px::new`.

**Done when:** `cargo test -p herdrify-app protocol::commands::words` passes and the Definition of
Done is green. Commit `feat(herdrify-app): add the command input words and numbers`.

### Task 3 — Add the argument enums

**Files:**
- Modify `crates/herdrify-app/src/protocol/commands/words.rs`.

**Change:** The argument enums of spec §5.9:
- `Focus`, `Pin`, `Machine`, `Rename<N>`
- `SpaceRef`, `TabRef`, `TabMoveTo`, `PaneTarget`, `SwapWith`
- `Scroll`, `At`, `Resize`
- `TabContent`, `PaneContent`

**Proof:**
- `single_value_enums_serialize_their_value` — `Focus::Show` → `true`, `Focus::Background` →
  `false`, `Pin::Pinned` → `true`, `Pin::Unpinned` → `false`, `Machine::ThisComputer` → `""`,
  `Machine::Named(vps)` → `"vps"`, `Rename::<TabName>::Automatic` → `""`, `Rename::To(api)` →
  `"api"`, `At::Cursor` → `"cursor"`, `At::Cell{row:2,col:0}` → `{"col":0,"row":2}`.
- `one_of_groups_write_their_key`, flattened into a wrapper struct and compared as parsed JSON:
  - `SpaceRef::Index(2)` → `{"index":2}`; `SpaceRef::Id(sp1)` → `{"id":"sp1"}`
  - `TabRef` both forms
  - `TabMoveTo::Index(1)` → `{"index":1}`; `TabMoveTo::By(-1)` → `{"by":-1}`
  - `PaneTarget::Pane(p1)` → `{"pane":"p1"}`; `PaneTarget::Terminal(t1)` → `{"terminal":"t1"}`
  - `SwapWith::Direction(Left)` → `{"direction":"left"}`; `SwapWith::Target(p1)` →
    `{"target":"p1"}`
  - `Scroll::Lines(-1)` → `{"lines":-1}`; `Scroll::Pages(0.5)` → `{"pages":0.5}`;
    `Scroll::To(Bottom)` → `{"to":"bottom"}`
  - `Resize::Px(200)` → `{"px":200.0}`; `Resize::Weight(1)` → `{"weight":1.0}`
- `content_enums_write_their_keys`:
  - `TabContent::Mod{files, props {"role":"x"}}` → `{"mod":"files","props":{"role":"x"}}`
  - `TabContent::Shell{argv ["sh"], task "t", folder "/tmp"}` →
    `{"argv":["sh"],"folder":"/tmp","task":"t"}`
  - `TabContent::Shell` with nothing set → `{}`
  - `PaneContent::Mod{files}` → `{"mod":"files"}`; `PaneContent::Shell{argv ["sh"]}` →
    `{"argv":["sh"]}`
- A doctest building a `TabContent::Mod`.

**Done when:** `cargo test -p herdrify-app protocol::commands::words` passes and the Definition of
Done is green. Commit `feat(herdrify-app): add the command argument enums`.

---

## Risks

- `Px::new(200.0)` serializes as `200.0`; herdr-app accepted floats for `px` (spec §5.9 evidence),
  and tests compare parsed JSON.

## Proof

- `cargo make dod` is green.

## Review findings

Reviewer verdict: plan-compliant; blocking: none. Scoped gate re-run by the reviewer: `dod-app.sh` → `exit=0`, no warning/error lines; 154 unit, 3 integration, 166 doctests passed. `cargo test -p herdrify-app protocol::commands` → 11 passed.

- open 🟡 `commands/words.rs:311` — the `Px` doc says a fractional size is refused; `Px::new` refuses only non-finite and negative values (reviewer probe: `Px::new(2.5) = Ok("2.5")`). The doc is the defect: drop "or fractional".
- open 🔵 `commands/words.rs:347` — `value >= 0.0` lets `-0.0` through (`Px::new(-0.0) = Ok`), while `Weight::new(-0.0)` refuses.
- open 🔵 `commands/outputs.rs:258` / Task 1 Proof — `outputs_decode_recorded_replies` uses shortened ids ("tbb6", "p271", "pe0e") for all but `Created`, so it is not the recorded stdout byte for byte. The shapes match.
- open 🔵 `commands/words.rs:272` — the `Position` doc claims herdr-app answers "no such space" for position 0 for tabs too; the evidence records only `space.select {"index":0}`.
- open 🔵 `commands/words.rs:403-437` — `FiniteF64` is generic by name but tied to `pages` (doc and `argument: "pages"`).
- open 🔵 `commands/words.rs:540` — `Rename<N>` has no bound, so `Rename::<String>` compiles.
- open 🔵 `commands/words.rs:616,680,706` — integer widths differ without a stated reason (`TabMoveTo::By(i32)`, `Scroll::Lines(i64)`, `At::Cell` `u32`).
- open 🔵 `commands/mod.rs:1` — the `//!` block has no "why it exists" sentence.

## Deviations

- 2026-11-24 — `cargo make dod` was replaced by the scoped gate (fmt, workspace clippy and doc with `-D warnings`, herdrify-app tests and doctests). The full gate fails only in herdrify-hc `tests/live.rs` `live_schema_matches_the_fixture`, against the installed `hc 0.1.22`, which this plan does not touch.
- 2026-11-24 — Tasks 1–3 ran as one coder in order, since Task 2 edits Task 1's `mod.rs` and Task 3 edits Task 2's `words.rs`. Built in parallel with plan 14 (disjoint files).
- 2026-11-24 — Where the plan was silent, the coder chose: hand-written `Serialize` for `Focus`, `Pin`, `Machine`, `Rename<N>` and `At`; externally tagged `rename_all = "snake_case"` for the one-of enums (flattened into the command); `#[serde(untagged)]` struct variants for `TabContent`/`PaneContent`, with the `module` field renamed to `"mod"`; `Eq` on both content enums (clippy `derive_partial_eq_without_eq`).

## Fix pass (2026-11-24, on the author's word "fix all findings")

- fixed: the `Px` doc no longer says fractional sizes are refused; new test `px_takes_fractional_sizes` pins `Px::new(2.5)` as `Ok`, written `2.5`.
- fixed: `Px::new(-0.0)` now stores `0.0`; new test `px_writes_negative_zero_as_zero`, seen red first (left `"-0.0"`, right `"0.0"`).
- fixed: `outputs_decode_recorded_replies` decodes the full recorded stdout of every shape (probe-commands-output.txt lines 729, 738, 739, 750, 751, 763, 765; probe-gaps-output.txt lines 6, 8, 9); no ids are shortened and no paths appear.
- fixed: the `Position` doc claims only what is recorded: `space.select` with position 0 is refused as "no such space".
- fixed (name kept, as plan and spec fix it): the `FiniteF64` doc says it is the page count of a scroll and that its refusal names `pages`.
- fixed: `Rename<N: sealed::RenameTarget>`; the marker trait sits in a private module, implemented only for `SpaceName`, `TabName`, `PaneName`; a `compile_fail,E0277` doctest shows `Rename::<String>` is refused.
- declined (spec-fixed): integer widths stay `u32` / `i32` / `i64` as spec §5.9 sets them; a `//` reason now sits at `TabMoveTo::By`, `Scroll::Lines` and `At::Cell`.
- fixed: `commands/mod.rs`'s `//!` block opens with why the layer exists.
- Gate after the pass: `dod-app.sh` → `exit=0`, no warning/error lines; 195 unit, 5 + 3 integration, 254 + 29 doctests passed.
