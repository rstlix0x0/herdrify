---
status: done
created: 2026-11-23
depends-on: [10, 12]
built-on: aab842c3c7d3
---

# Catalog Coverage Implementation Plan

**Goal:** Prove offline that the SDK's command table and command types match herdr-app 0.1.17's
list.

**Architecture:** An offline integration test over a byte-for-byte copy of the 0.1.17 command
list (spec §12.2, §12.3).

**Tech Stack:** Rust 1.94, serde_json 1.0.154.

**Content authority:** spec §12.3; §5.5.

---

## Files that change

```
crates/herdrify-app/tests/fixtures/herdr-app-0.1.17-commands.json   — create: copy of evidence/herdr-app-0.1.17-commands.json
crates/herdrify-app/tests/catalog.rs                                 — create: the table and per-type checks
```

## Order of work

### Task 1 — Copy the command list and check the table

**Files:**
- Create the fixture, copied with `cp` and verified with `cmp`.
- Create `crates/herdrify-app/tests/catalog.rs`.

**Change:** The fixture decodes as `Vec<CommandInfo>` and is compared with `KNOWN_COMMANDS`.

**Proof:**
- `known_commands_equal_the_fixture` — the same 77 names in order and, per command, the same arg
  names in order with the same `required` flags. On failure the test prints each difference.

**Done when:** `cargo test -p herdrify-app --test catalog` passes and the Definition of Done is
green. Commit `test(herdrify-app): check the command table against herdr-app 0.1.17`.

### Task 2 — Check every command type against the table

**Files:**
- Modify `crates/herdrify-app/tests/catalog.rs`.

**Change:** For each of the 77 command types, the test builds a full instance and serializes it.
The full instance uses every setter and one variant of each argument enum, and every variant of
every argument enum is used somewhere across the test.

**Proof:**
- `every_command_name_is_known` — each type's `NAME` is in `KNOWN_COMMANDS`, and the 77 types
  cover all 77 names.
- `every_command_writes_only_listed_args` — each full instance's JSON keys are a subset of its
  command's listed arg names.

**Done when:** `cargo test -p herdrify-app --test catalog` passes and the Definition of Done is
green. Commit `test(herdrify-app): check every command type against the listed arguments`.

---

## Risks

- None beyond test volume.

## Proof

- `cargo make dod` is green.

## Review findings

Reviewer verdict: plan-compliant, no drift; blocking: none. Scoped gate re-run by the reviewer: `dod-app.sh` → `exit=0`, no warning/error lines. `cargo test -p herdrify-app --test catalog` → 3 passed. `cmp` of the fixture against `evidence/herdr-app-0.1.17-commands.json` prints nothing (identical). Reviewer mutation probes (files restored, `cmp`-checked): an arg-name typo in a type, a wrong `required` flag in the table, a typo in a variant's key and a `NAME` typo are all caught, each printing its differences.

- open 🟡 `tests/catalog.rs:389` — the check is subset-only: dropping a required arg from a type, or removing a setter, fails nothing. Checking that each required arg is written, and that the union of written keys equals the listed args, would catch both (beyond this plan).
- open 🟡 `tests/catalog.rs:272` — "every variant is used" is kept by hand; with no exhaustive `match`, a variant added later is never built and its key goes unchecked.
- open 🔵 `tests/catalog.rs:14` — the `//!` says each variant's written form is checked; only its keys are (a unit variant's value can change unseen).
- open 🔵 `tests/catalog.rs:64` — names are compared by position, so one missing entry misreports every later one; a set difference printed first would name it.
- open 🔵 `tests/catalog.rs:409` — the message says "the listing does not give it" while the check reads `KNOWN_COMMANDS` (true only because the first test holds).

## Deviations

- 2026-11-24 — `cargo make dod` was replaced by the scoped gate (fmt, workspace clippy and doc with `-D warnings`, herdrify-app tests and doctests). The full gate fails only in herdrify-hc `tests/live.rs` `live_schema_matches_the_fixture`, against the installed `hc 0.1.22`, which this plan does not touch.
- 2026-11-24 — Tasks 1–2 ran as one coder in order (both edit `tests/catalog.rs`), in parallel with plan 15 (disjoint files). The coder's last gate run failed only on a doctest in plan 15's then in-progress `driver/client.rs`; the reviewer's re-run after both finished was green.
- 2026-11-24 — `full_instances` carries `#[expect(clippy::too_many_lines)]` with a reason: it builds every command type in full.

## Fix pass (2026-11-24, on the author's word "fix all findings")

- fixed: new `every_required_arg_is_written_by_the_minimal_command` (all 77 types built with `new` alone) and `every_listed_arg_is_written_by_some_instance` (union of written keys equals the listed args). Both pass on the SDK: no listed arg is left unexposed. Mutation: `space.move` without `id` → both fail ("`space.move` requires `id` and its command built with `new` alone does not write it (it writes {"index"})"); restored, `cmp` clean.
- fixed: each of the 23 argument enums is enumerated by a `variant_cases!` function that expands to a `match` with no `_` arm. Mutation: removing the `Direction::Up` pair → `error[E0004]: non-exhaustive patterns: &Direction::Up not covered`; restored.
- fixed (wording): the `//!` says the instances check the keys each variant writes, not a unit variant's value, which the listing does not give.
- fixed: `table_differences` prints set differences first, then duplicates, then position mismatches only when the sets agree. Mutation: dropping entry 5 → one line, "`tab.close` is in the table and not in the listing"; restored.
- fixed: the message names what is read: "`space.new` writes `bogus`, which `KNOWN_COMMANDS` does not list for it (it lists [...])".
- `cargo test -p herdrify-app --test catalog` → 5 passed. Gate after the pass: `dod-app.sh` → `exit=0`, no warning/error lines.
