---
status: approved
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
