---
status: done
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

## Review findings

- Reviewer (batch with plan 07 Task 1): SPEC COMPLIANT, 0🔴 0🟡, blocking: none.
- 🔵 open: `tests/files_roundtrip.rs:60-91` — each fixture's display name repeats its `include_str!` path by hand, so the two can drift.
- 🔵 open: no DoD step re-checks that `tests/fixtures/` stay byte-identical to `evidence/fresh-home-0.1.17/`; checked once at build time with `cmp` (all six identical).
- Verified: `cargo test -p herdrify-app --test files_roundtrip` → `test result: ok. 3 passed; 0 failed`. No model change was needed.

## Deviations

- 2026-11-24 — Definition of Done. `cargo make dod` fails in `herdrify-hc` `tests/live.rs::live_schema_matches_the_fixture` (`crates/herdrify-hc/tests/live.rs:626`) because the installed `hc --version` is `hc 0.1.22`, whose schema differs from the herdrify-hc fixture (doc.set, events.subscribe, terminal.keep/kill/label/put*/rm/wait). This is environment drift outside this chain; herdrify-hc was not touched. The gate run instead: `cargo fmt --all -- --check`, workspace `clippy -D warnings`, workspace `RUSTDOCFLAGS="-D warnings" cargo doc`, `cargo test -p herdrify-app --all-targets --all-features`, `cargo test -p herdrify-app --all-features --doc` → `exit=0`, no warning/error lines; `101 passed`, `3 passed`, `84 passed`.

## Fix pass (2026-11-24, on the author's word "fix all of it")

- fixed: `tests/files_roundtrip.rs` — a `fixture!` macro writes each path once and yields `(path, include_str!(…))`, so the name and the file cannot drift.
- declined: a DoD re-check that `tests/fixtures/` equals the evidence copies. It would put planning-evidence paths into crate source, which the source rules forbid, and the published crate does not ship the evidence. The fixtures are tracked by git, so any edit to them shows in review.
- Gate after the pass: `dod-app.sh` → `exit=0`, no warning/error lines; `140 passed`, `3 passed`, `123 passed`.
