---
status: approved
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
