---
status: done
created: 2026-11-23
depends-on: [04]
built-on: aab842c3c7d3
---

# Keys Model Implementation Plan

**Goal:** Model `keys.json` as typed sections of chords bound to actions.

**Architecture:** One concept file, `files/keys.rs`, holding `Keys`, `Section`, `Action` and
`KeyActionError` (spec §2). The shared rules of plan 02's Architecture apply verbatim. `Section`
and `Action` have hand-written `Serialize` and `Deserialize` that implement spec §4.10's rule
tables. This plan depends on plan 04 only to order the edits of `files/mod.rs`.

**Tech Stack:** Rust 1.94, serde 1.0.229, serde_json 1.0.154.

**Content authority:** spec §4.10; §10 (`KeyActionError`, the key-action detail); §11 (the keys
rows).

---

## Files that change

```
crates/herdrify-app/src/files/keys.rs   — create: Keys, Section, Action, KeyActionError
crates/herdrify-app/src/files/mod.rs    — modify: mod keys and re-exports
```

## Order of work

### Task 1 — Add Section

**Files:**
- Create `crates/herdrify-app/src/files/keys.rs`.
- Modify `crates/herdrify-app/src/files/mod.rs`.

**Change:**
- `Section`: `Global`, `Terminal`, `Layout(LayoutName)`, `Mod(ModName)`, `Other(String)`.
- It parses and writes the key string by spec §4.10's table, derives `Ord`, and (de)serializes as
  a string.

**Proof:**
- `section_parses_every_form` — `global`, `terminal`, `layout:herdrify-main` and
  `mod:herdrify-host` give the four named variants; `layout:`, `mod:` and `weird` give `Other`.
- `section_writes_back_as_read` — each of those strings serializes back unchanged.
- A doctest on `Section` parsing `layout:herdrify-main`.

**Done when:** `cargo test -p herdrify-app files::keys` passes and the Definition of Done is green.
Commit `feat(herdrify-app): add key sections`.

### Task 2 — Add Action

**Files:**
- Modify `crates/herdrify-app/src/files/keys.rs`.

**Change:**
- `Action`:
  - `Terminal`
  - `Send(String)`
  - `Run { command: CommandName, value: Option<String> }`
  - `Command { command: CommandName, args: Option<Map>, extra: Map }`
  - `Other(Value)`
- Deserialization applies spec §4.10's ordered rules.
- `KeyActionError { value }`, Display `"<value>" is not a key action: its command name is empty`.
- An object with an empty `command` is refused with `EmptyValueError` (`command name`).
- Serialization writes each form back as written.
- Constructors: `Action::run(CommandName)` and `Action::run_with(CommandName, impl Into<String>)`.

**Proof:**
- `action_parses_strings` — `"pane.split:right"` → `Run{pane.split, Some("right")}`;
  `"space.new:{a:b}"` → `Run{space.new, Some("{a:b}")}`; `"send:"` → `Send("")`; `"terminal"` →
  `Terminal`; `"terminal.copy"` → `Run{terminal.copy, None}`.
- `action_parses_command_objects` — `{"command":"layout.resize","args":{"id":"sidebar","px":240}}`
  is `Command` with those args; `{"command":"x","y":1}` keeps `y` in `extra`.
- `action_keeps_other_json` — `{"command":"x","args":7}`, `{"args":{}}`, `7`, `null`, `true` and
  `["a"]` are `Other` and serialize back JSON-equal.
- `action_refuses_empty_command_names` — `":x"` displays `":x" is not a key action: its command
  name is empty`, and `""` displays `"" is not a key action: its command name is empty`;
  `{"command":""}` fails with `a command name cannot be empty`.
- `action_writes_back_as_read` — every accepted input above serializes JSON-equal to its input.
- Doctests on `Action::run_with` and on parsing a `send:` action.

**Done when:** `cargo test -p herdrify-app files::keys` passes and the Definition of Done is green.
Commit `feat(herdrify-app): add key actions`.

### Task 3 — Add Keys

**Files:**
- Modify `crates/herdrify-app/src/files/keys.rs`.

**Change:**
- `Keys`, a transparent `BTreeMap<Section, BTreeMap<Chord, Action>>` newtype deriving `Default`,
  with:
  - `new()` (empty)
  - `get(&Section)`, `insert(Section, Chord, Action)` and `sections()`
  - `IntoIterator` over sections
- `Keys::parse(&str) -> Result<Keys, FileError>` with `FileKind::Keys`, and `to_json`, pretty plus
  `"\n"`.

**Proof:**
- `keys_parse_the_agents_md_example` — the object at AGENTS.md:522-527 parses into four sections
  and round-trips JSON-equal.
- `keys_refuse_an_empty_chord` — `{"global":{"":"tab.new"}}` displays a message starting
  `invalid keys file: a key chord cannot be empty`.
- `keys_express_herdrify_sections` — `Keys::new()`, with inserts into `layout:herdrify-main` and
  `mod:herdrify-host`, serializes with those exact section keys.
- Doctests on `Keys::new`, `insert` and `to_json`.

**Done when:** `cargo test -p herdrify-app files::keys` passes and the Definition of Done is green.
Commit `feat(herdrify-app): add the keys model`.

---

## Risks

- Hand-written serde impls are the riskiest code here. Every rule row has a test, and plan 06
  runs the shipped `keys.json`.

## Proof

- `cargo make dod` is green.

## Review findings

Reviewer verdict: compliant with amendments. It re-ran `cargo make dod`: exit 0, warning/error grep
empty; `cargo test -p herdrify-app files::keys` 23/23. `blocking: keys.rs:183-187`.

Fixed:
- 🟡 (blocking) `files/keys.rs:183-187` — the `Action` doc said a hand-built `Other` string
  "reads back as that form". Probes showed `Other(":x")` and `Other("")` are refused on read
  (`invalid keys file: ":x" is not a key action: its command name is empty at line 4 column 3`),
  `Run{a:b, None}` reads back as `Run{a, Some("b")}`, and `Other({"command":"x"})` reads back
  as `Command`. The doc now lists every case where a hand-built action reads back as another
  form or is refused, pinned by `action_built_by_hand_can_read_back_as_another_form` and
  `action_built_by_hand_can_be_written_as_text_that_is_refused`. Re-run:
  `cargo test -p herdrify-app files::keys` → `test result: ok. 25 passed; 0 failed`;
  `cargo make dod` → exit 0, `Build Done in 21.63 seconds`; warning/error grep printed nothing.

Open (non-blocking, not fixed):
- 🟡 `keys.rs:294-299` — rule order untested: `{"command":"","args":7}` is `Other` because the
  `args` check runs before the empty-command refusal; no test pins that order.
- 🟡 `keys.rs:390-392` (with `:45-47`) — `Keys` can hold both `Section::Global` and
  `Section::Other("global")`; `to_json` writes `"global"` twice and re-parsing keeps one
  (`back == k` false). Undocumented.
- 🔵 `keys.rs:89-101` — `Section::new(impl AsRef<str>)` copies an owned `String` again on the
  serde path.
- 🔵 `keys.rs:295-300` — `from_object` clones the name before removing `command`.
- 🔵 `keys.rs:411-415,443-447` — `Keys::get` and `Keys::sections` have no `# Examples` of their own.
- 🔵 `files/mod.rs:17-18` — the keys responsibilities line omits `KeyActionError`.
- 🔵 `Section` uses `#[serde(from = "String", into = "String")]` rather than hand-written
  Serialize/Deserialize; same behaviour, guarded by `section_is_a_json_object_key` and
  `section_writes_back_as_read`.
- 🔵 Unplanned `impl FromStr for Action`: without it the public `KeyActionError` is returned by
  no public API.

## Deviations

- 2026-11-23 — `keys_parse_the_agents_md_example` was built as `keys_parse_the_documented_example`
  (no file references in source). The text matches the 0.1.17 AGENTS.md:522-527 byte for byte.
- 2026-11-23 — `IntoIterator` is implemented for owned `Keys` only: clippy
  `into_iter_without_iter` (-D warnings) rejected `IntoIterator for &Keys` without an `iter`
  method, and `sections()` already iterates by reference.
- 2026-11-23 — One review for the whole plan, and no per-task commits (as in plans 02–04).
