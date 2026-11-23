---
status: done
created: 2026-11-23
depends-on: [03]
built-on: aab842c3c7d3
---

# Theme Model Implementation Plan

**Goal:** Model `themes/<name>.json` as a typed `Theme`.

**Architecture:** One concept file in the `files` layer (spec §2). The shared rules of plan 02's
Architecture apply verbatim. The struct is `#[non_exhaustive]` with public fields and a flattened
`extra`. It depends on plan 03 only to order the edits of `files/mod.rs`; it reuses
`FileError::from_serde` from plan 03 Task 5.

**Tech Stack:** Rust 1.94, serde 1.0.229, serde_json 1.0.154.

**Content authority:** spec §4.9; §4.2 (empty colour names); §10 (theme `FileError`s).

---

## Files that change

```
crates/herdrify-app/src/files/theme.rs   — create: Theme
crates/herdrify-app/src/files/mod.rs     — modify: mod theme and re-exports
```

## Order of work

### Task 1 — Add Theme

**Files:**
- Create `crates/herdrify-app/src/files/theme.rs`.
- Modify `crates/herdrify-app/src/files/mod.rs`.

**Change:**
- `Theme` has:
  - `background`, `foreground`, `selection`: `HexColor`
  - `cursor`: `Option<HexColor>`
  - `ansi`: `[HexColor; 16]`
  - `colors`: `Option<BTreeMap<ColorName, HexColor>>`
  - `extra`
- `Theme::new(background, foreground, selection, ansi)`.
- `Theme::parse(&str) -> Result<Theme, FileError>` with `FileKind::Theme`, and `to_json`, pretty
  plus `"\n"`.

**Proof:**
- `theme_parses_and_round_trips` — a theme with every field, a `colors` map holding `ring`, and an
  unknown key round-trips JSON-equal.
- `theme_requires_sixteen_ansi_colours` — one ansi colour gives a Display starting
  `invalid theme file: invalid length 1, expected an array of length 16 at line 1 column`.
- `theme_requires_background` — a theme with 16 ansi colours but no `background` gives a detail
  starting ``missing field `background` ``.
- `theme_refuses_named_colours` — `"background":"canvas"` fails, because theme colours are hex
  only.
- `theme_refuses_an_empty_colour_name` — `"colors":{"":"#000000"}` fails with a detail starting
  `a colour name cannot be empty`.
- Doctests on `Theme::new`, `Theme::parse` and `to_json`.

**Done when:** `cargo test -p herdrify-app files::theme` passes and the Definition of Done is
green. Commit `feat(herdrify-app): add the theme model`.

---

## Risks

- None specific. Plan 06 runs both shipped themes.

## Proof

- `cargo make dod` is green.

## Review findings

Reviewer verdict: compliant, `blocking: none`. It re-ran `cargo make dod`: exit 0, warning/error
grep empty, herdrify-app 74 unit tests and 62 doctests ok. `cargo test -p herdrify-app
files::theme` gave 10 passed. The probe crate (rustc 1.94.1, serde_json =1.0.154) was in the
session scratchpad.

Open (non-blocking, not fixed):
- 🟡 `files/theme.rs:67` — a 17-colour `ansi` array is refused with detail `trailing characters
  at line 1 column 238`, which does not name the length; no test pins the over-16 case.
- 🔵 `theme.rs:30-31` — the `extra` collision doc says "written twice". That is true only when the
  named field is present; an absent optional field's key is taken over by `extra` (probe:
  `cursor: None` plus `extra["cursor"]` reads back as `Some`).
- 🔵 `theme.rs:55` — nothing guards `#[non_exhaustive]` (same for the layout types).
- 🔵 `theme.rs:70` — a colour name repeated in `colors` keeps its last value silently; no test.
- 🔵 `theme.rs:117` — "a repeated unknown key keeps its last value" is true but untested here.
- 🔵 `theme.rs:184` — redundant `use crate::files::FileKind;` in `mod tests`.

## Deviations

- 2026-11-23 — No commit: the instruction to execute did not ask for commits. The plan's commit
  message is left for the author.
