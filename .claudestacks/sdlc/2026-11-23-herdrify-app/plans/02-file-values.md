---
status: approved
created: 2026-11-23
depends-on: [01]
built-on: aab842c3c7d3
---

# File Values Implementation Plan

**Goal:** Give the `files` layer its value types: identifiers, numbers, colours and the file error.

**Architecture:** The first files of the `files` domain layer: pure, with no tokio and no I/O
(spec §2). This plan also sets the **shared rules** every later code plan applies verbatim:
- **Modules.**
  - Every new non-export file opens with the four-question `//!` block of
    `mod-rs-export-only.md`, and has a colocated `#[cfg(test)] mod tests`.
  - `lib.rs`, `prelude.rs` and every `mod.rs` are export-only, with no tests, and carry a `//!`
    line citing unit-test-mandate exemption #1.
  - Inside a layer, submodules are private (`mod x;`) and re-exported flat with
    `#[doc(inline)] pub use`, as `crates/herdrify-hc/src/protocol/mod.rs:32-43` does. `lib.rs`
    declares the layers themselves `pub mod`.
- **Tests.** The workspace denies `unwrap_used` and `panic` and warns on `expect_used`
  (`Cargo.toml:37-39`), so tests return `Result` and use `?`.
- **Public items.**
  - Every `pub` item is documented and re-exported, because `missing_docs` and `unreachable_pub`
    warn (`Cargo.toml:30-31`).
  - Every public item with behaviour carries a `# Examples` doctest (spec §12.7), run by the DoD's
    `test-doc` step.
- **Derives.** Every value and model type derives at least `Debug`, `Clone` and `PartialEq`.
- **No dead code.** A crate-private helper is added in the task that first uses it in non-test
  code, because `clippy --all-targets -D warnings` rejects dead code.
- **Definition of Done.** `(set -o pipefail; mkdir -p target && cargo make dod 2>&1 | tee
  target/dod.log)` exits 0, and `sed $'s/\x1b\\[[0-9;]*m//g' target/dod.log | grep -E '^
  *(warning|error)'` prints nothing.

**Tech Stack:** Rust 1.94, serde 1.0.229, serde_json 1.0.154, thiserror 2.0.21.

**Content authority:** spec §4.2, §4.3, §4.4, §10 (the `FileError`, `ColorError`, `NumberError`
and `EmptyValueError` rows).

---

## Files that change

```
crates/herdrify-app/src/lib.rs                     — modify: pub mod files
crates/herdrify-app/src/files/mod.rs               — create: export-only table of contents of the files layer
crates/herdrify-app/src/files/identifiers.rs       — create: EmptyValueError and the eight name newtypes of spec §4.2
crates/herdrify-app/src/files/number.rs            — create: Number, NumberError
crates/herdrify-app/src/files/color.rs             — create: HexColor, Color, ColorError
crates/herdrify-app/src/files/error.rs             — create: FileError, FileKind
```

## Order of work

### Task 1 — Add the files layer and its name newtypes

**Files:**
- Modify `crates/herdrify-app/src/lib.rs` — `pub mod files;`, plus an exemption #1 line in its
  `//!` if it lacks one.
- Create `crates/herdrify-app/src/files/mod.rs` — the `//!` block (pure file shapes, no I/O),
  `mod identifiers;` and the re-exports.
- Create `crates/herdrify-app/src/files/identifiers.rs`.

**Change:**
- `EmptyValueError { label: &'static str }`, with Display `a <label> cannot be empty`.
- `NodeId`, `ModName`, `LayoutName`, `ThemeName`, `TerminalAddress`, `ColorName`, `CommandName`
  and `Chord`. Each is a `String` newtype with:
  - `new(impl Into<String>) -> Result<Self, EmptyValueError>`, refusing only `""`
  - `as_str`, `Display` and `FromStr`
  - `#[serde(try_from = "String", into = "String")]`
  - derived `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `PartialOrd` and `Ord`

  The labels are exactly spec §4.2's table.

**Proof:**
- `each_name_refuses_empty_with_its_label` — `new("")` on each of the eight is
  `Err(EmptyValueError { label })` with its §4.2 label, displaying `a <label> cannot be empty`.
- `names_accept_whitespace` — `NodeId::new(" ")` is `Ok`.
- `names_round_trip_through_serde` — `"herdrify-host"` into `ModName` and back; `""` refused for
  each.
- `name_parsing_matches_new` — `FromStr` equals `new` for `""` and `"x"`.
- A doctest on `ModName::new` shows the happy path and the empty refusal.

**Done when:** `cargo test -p herdrify-app files::identifiers` passes and the Definition of Done is
green. Commit `feat(herdrify-app): add the files layer and its name newtypes`.

### Task 2 — Add Number

**Files:**
- Create `crates/herdrify-app/src/files/number.rs`.
- Modify `crates/herdrify-app/src/files/mod.rs`.

**Change:**
- `Number` wraps a private `serde_json::Number` and serializes transparently.
- `From<u32>`, `From<i64>` and `From<u64>`.
- `TryFrom<f64>` refusing NaN and the infinities with `NumberError { value: f64 }`, Display
  `<value> is not a finite number`.
- `as_f64()` and `is_integer()`.

**Proof:**
- `number_keeps_the_written_form` — `120.0`, `268`, `4.286`, `0.0`, `-5` and `-0.0` each
  serialize back to the same text.
- `number_writes_exponents_as_floats` — `1e3` serializes as `1000.0`.
- `number_refuses_non_finite` — NaN displays `NaN is not a finite number`; infinity is refused.
- `number_reports_integer_form` — `Number::from(268u32).is_integer()` is true, and
  `Number::try_from(4.5)?.is_integer()` is false.
- Doctests on `Number::try_from` and `as_f64`.

**Done when:** `cargo test -p herdrify-app files::number` passes and the Definition of Done is
green. Commit `feat(herdrify-app): add Number with its exact JSON form`.

### Task 3 — Add the colours

**Files:**
- Create `crates/herdrify-app/src/files/color.rs`.
- Modify `crates/herdrify-app/src/files/mod.rs`.

**Change:**
- `HexColor` is `#` plus exactly 6 or 8 ASCII hex digits, in any case.
- `Color` is `Hex(HexColor)` or `Named(ColorName)`, parsed by spec §4.4's rule. Both (de)serialize
  as strings through `try_from`.
- `ColorError`:
  - `Empty`, Display `a colour cannot be empty`
  - `NotHex { value }`, Display `"<value>" is not a hex colour: use #rrggbb or #rrggbbaa`

**Proof:**
- `color_parses_the_spec_examples` — `"#ffffff0d"` → `Hex`, `"canvas"` → `Named`, `"#12345"` →
  `NotHex`.
- `hex_color_lengths_and_case` — `"#ABCDEF"` and `"#abcdef12"` accepted; `"#abcdefg"`, `"#abcd"`
  and `"#"` refused.
- `color_refuses_empty` — `""` is `Empty`.
- `color_error_in_a_file_keeps_its_text` — a test struct with a `Color` field parsed from
  `{"color":"#12345"}` fails with exactly ``"#12345" is not a hex colour: use #rrggbb or #rrggbbaa
  at line 1 column 18``.
- Doctests on `Color` parsing and `HexColor::new`.

**Done when:** `cargo test -p herdrify-app files::color` passes and the Definition of Done is
green. Commit `feat(herdrify-app): add the colour types`.

### Task 4 — Add FileError

**Files:**
- Create `crates/herdrify-app/src/files/error.rs`.
- Modify `crates/herdrify-app/src/files/mod.rs`.

**Change:**
- `FileKind`, public: `Layout`, `Theme` or `Keys`, with Display words `layout`, `theme`, `keys`.
- `FileError { file: FileKind, detail: String }`, public and `#[non_exhaustive]`, with accessors
  `file()` and `detail()` and Display `invalid <kind> file: <detail>`.
- `FileError::new(FileKind, impl Into<String>)`, public.

The crate-private conversion from a `serde_json::Error` is added by plan 03 Task 5, its first
user.

**Proof:**
- `file_error_display_names_the_file` — `FileError::new(FileKind::Theme, "x")` displays
  `invalid theme file: x`.
- `file_kind_words` — the three Display words.
- A doctest on `FileError::new`.

**Done when:** `cargo test -p herdrify-app files::error` passes and the Definition of Done is
green. Commit `feat(herdrify-app): add FileError`.

---

## Risks

- `try_from`/`into` serde attributes need `Clone` on the newtype; it is derived.

## Proof

- Every rule in spec §4.2–4.4 has a named test. `cargo make dod` is green.
