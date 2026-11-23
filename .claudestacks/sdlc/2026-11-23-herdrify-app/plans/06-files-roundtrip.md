---
status: approved
created: 2026-11-23
depends-on: [03, 04, 05]
built-on: aab842c3c7d3
---

# Files Round Trip Implementation Plan

**Goal:** Prove every shipped herdr-app 0.1.17 layout, theme and keys file round-trips through the
models.

**Architecture:** An offline integration test over byte-for-byte fixture copies (spec §12.2,
§12.4). It needs no herdr-app.

**Tech Stack:** Rust 1.94, serde_json 1.0.154.

**Content authority:** spec §12.2 (the file fixtures), §12.4.

---

## Files that change

```
crates/herdrify-app/tests/fixtures/layouts/{main,terminals,win98}.json      — create: copies of evidence/fresh-home-0.1.17/layouts
crates/herdrify-app/tests/fixtures/themes/{atom-one-dark,win98}.json        — create: copies of evidence/fresh-home-0.1.17/themes
crates/herdrify-app/tests/fixtures/keys.json                                 — create: copy of evidence/fresh-home-0.1.17/keys.json
crates/herdrify-app/tests/files_roundtrip.rs                                 — create: the four checks per fixture
```

## Order of work

### Task 1 — Copy the file fixtures and test the round trip

**Files:**
- Create the six fixture files, copied with `cp` from
  `.claudestacks/sdlc/2026-11-23-herdrify-app/evidence/fresh-home-0.1.17/`, then verified with
  `cmp`.
- Create `crates/herdrify-app/tests/files_roundtrip.rs` — a `//!` block and one test per file
  kind.

**Change:** For each fixture, the four checks of spec §12.4:
1. it parses
2. `to_json` gives a value JSON-equal to the source
3. there is no duplicate key: the `to_json` text has the same length as
   `serde_json::to_string_pretty` of the same value after a `serde_json::Value` pass, plus `"\n"`
4. re-parsing and re-serializing gives the same text

**Proof:**
- `shipped_layouts_round_trip` — the four checks for main, terminals and win98 through `Layout`.
- `shipped_themes_round_trip` — the four checks for both themes through `Theme`.
- `shipped_keys_round_trip` — the four checks for `keys.json` through `Keys`.

**Done when:** `cargo test -p herdrify-app --test files_roundtrip` passes and the Definition of
Done is green. Commit `test(herdrify-app): round-trip the shipped layout, theme and keys files`.

---

## Risks

- A failure here points at a model field missing `skip_serializing_if`, or at a wrong
  untagged-variant order in plans 03–05. The fix goes there.

## Proof

- `cargo make dod` is green.
