---
status: approved
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
