---
status: done
created: 2026-11-22
depends-on: [06]
built-on: ca0f990f7018
---

# Schema Coverage Implementation Plan

**Goal:** Prove offline that the SDK's 32 methods, their params and the 36 schema types match the hc 0.1.21 OpenRPC fixture exactly.

**Architecture:**
- Integration tests under `crates/herdrify-hc/tests/`, using only the crate's public API.
- The fixture is a byte-for-byte copy inside the crate, so the published package needs no path outside it. No test names a path outside `crates/herdrify-hc`.
- Each test binary includes only the support files it uses, by `#[path = "support/<file>.rs"] mod <name>;`, so no binary carries unused helpers (dead code would fail the gate).
- Tests return `Result`, since the lints deny `unwrap_used` and `panic`.

**Tech Stack:** Rust 1.94.1, serde_json 1.0.154.

**Content authority:** spec §1 (the fixture), §9.2 (`schema_instance.rs`), §9.3.

---

## Files that change

```
crates/herdrify-hc/tests/fixtures/hc-0.1.21.openrpc.json — create: copy of evidence/upstream-v0.1.21-schema-hc.openrpc.json
crates/herdrify-hc/tests/support/schema_instance.rs      — create: fully populated JSON instance of any fixture schema
crates/herdrify-hc/tests/schema_coverage.rs              — create: the three offline coverage checks
```

## Order of work

### Task 1 — Add the fixture and the schema-instance builder

**Files:**
- Create `crates/herdrify-hc/tests/fixtures/hc-0.1.21.openrpc.json` — `cp .claudestacks/sdlc/2026-11-22-herdrify-hc/evidence/upstream-v0.1.21-schema-hc.openrpc.json crates/herdrify-hc/tests/fixtures/hc-0.1.21.openrpc.json`.
- Create `crates/herdrify-hc/tests/support/schema_instance.rs` — `instance(fixture: &Value, schema_name: &str) -> Value`.
- Create `crates/herdrify-hc/tests/schema_coverage.rs` — with the builder's own tests only, for now.

**Change:** `instance` builds a fully populated JSON value by the value-rule table of spec §9.2:
- objects get every property, and `$ref` is resolved
- `anyOf` with null, or `type: [T, "null"]`, takes the non-null branch
- an `enum` takes its first value
- a string is `"x"`
- an integer is its `minimum` if that is ≥ 1, else `1`
- a number is `1.0`, and a boolean is `true`
- an array has `minItems` items if set, else one; an array whose item schema is a `$ref` to a schema already being built (a cycle) gets no items, so `WaitResult.children` is `[]` (spec §9.2)
- a map gets one entry with key `"k"`
- `any` is `"x"`

One override table sets `ExecResult.stdout` and `ExecResult.stderr` to `"eAo="`.

**Proof:**
- `fixture_is_the_upstream_copy` — not a Rust test, but the command `cmp .claudestacks/sdlc/2026-11-22-herdrify-hc/evidence/upstream-v0.1.21-schema-hc.openrpc.json crates/herdrify-hc/tests/fixtures/hc-0.1.21.openrpc.json`, which prints nothing.
- `instance_fills_every_property` — `instance(fixture, "Edit")` is `{"new":"x","old":"x"}`.
- `instance_applies_the_value_rules` — `InitialSize` gets `cols`/`rows` ≥ 1, `Display.cell` gets two items, `Children` gets `"any"`, and `ExecResult.stdout` is `"eAo="`.
- `instance_stops_at_a_cycle` — `instance(fixture, "WaitResult")` finishes with `children: []`, and `instance(fixture, "RunResult")` finishes with `wait.children: []`.

**Done when:** the `cmp` prints nothing, `cargo test -p herdrify-hc --test schema_coverage` passes, and the Definition of Done is green (`cargo make dod` exits 0; the log grep of `CLAUDE.md` prints nothing). Commit `test(herdrify-hc): add the OpenRPC fixture and the schema-instance builder`.

### Task 2 — Check the SDK against the fixture

**Files:**
- Modify `crates/herdrify-hc/tests/schema_coverage.rs` — the three checks of spec §9.3.

**Change:** The test lists the 32 `Method` types explicitly, by their public paths.
1. Their `NAME`s, as a set, equal the fixture's `methods[].name`.
2. Each `Params`, per spec §9.3 item 2:
   - built with every setter, it serializes to exactly the fixture's param names for that method
   - for `TerminalRun` (`Command`), `MachineExec` (`ExecCommand`), `TerminalWait` (`WaitTarget`) and `MachineAdd` (`MachineTarget`), each variant built with every setter gives the fixture's names minus the params only other variants send, and the union over all variants equals the fixture's names
   - built with only `new()` arguments, it serializes to exactly the required names, plus the chosen variant's params for those four
3. For each of the 36 schemas, `instance(fixture, name)` deserializes into the matching SDK type (`DisplayFacts` for `Display`, `HelloResult` from `protocol::hello`), and serializing it back is JSON-equal to the instance.

Any failure names the method or schema that differs.

**Proof:**
- `method_names_equal_the_fixture` — check 1.
- `params_match_the_fixture_names` — check 2, for all 32.
- `schema_types_round_trip_fixture_instances` — check 3, for all 36.

**Done when:** `cargo test -p herdrify-hc --test schema_coverage` passes and the Definition of Done is green. Commit `test(herdrify-hc): check methods, params and types against the fixture`.

---

## Risks

- **A failure here sends the fix back to plans 04–06's files.** The coder fixes the type, not the test, unless the test misreads the fixture.
- **Instances set every optional property.** A type that serializes `None` differently would not be caught here. Unit tests in plan 04 cover absent and `null`.

## Proof

- `cargo test -p herdrify-hc` passes, and the coverage test needs no `hc`.
- `grep -rnE 'ADR [0-9]|\.claudestacks|milestone' crates/herdrify-hc` prints nothing.
- The Definition of Done is green.

## Review findings

Final batch check (2026-11-23): `cargo make dod` exit 0, zero warning lines; herdrify-hc 181 unit + 47 doc + 6 schema_coverage tests pass; `cargo make deny` advisories/bans/licenses/sources ok.

- Reviewer (with plan 08): blocking none; coverage is not tautological (mutating `TerminalRun::rm` and deleting `ExecResult.signal` each fail a check); all 32 methods, all 36 schemas, per-variant setter checks and new()-only required sets are covered.
- open 🔵 variant lists are hand-written (a new variant would go unchecked); "all 36" is guarded by count only; `# Errors` doc omits the anyOf-null-only error; cycle detection handles only a direct array `$ref` (enough for the fixture).
- `cmp` of the fixture against evidence prints nothing; `cargo test -p herdrify-hc --test schema_coverage` → 6 passed.

## Deviations

- 2026-11-23 — Commits skipped: execute commits only when instructed; every task's changes are left uncommitted in the working tree.
- 2026-11-23 — One coder ran each plan's tasks in order (tasks share `protocol/mod.rs`, so no two ran at once); the coder read the plan directly instead of a task-briefer brief.
- 2026-11-23 — `instance()` returns `Result<Value, String>`, not `Value` (unwrap/panic/expect are denied, so an unknown schema or `$ref` is an Err).
- 2026-11-23 — The support module is declared `pub mod schema_instance;` (`unreachable_pub` and `redundant_pub_crate` conflict on a private module).
- 2026-11-23 — The Task 2 tests passed on their first run (the SDK types were already right); red was shown by temporary mutations. No SDK type needed a fix.
- 2026-11-23 — After the build, the files this plan created were moved into the layout of the redesigned spec (§2, redesign #3): a concept and its error share one file, `frame/`, `jsonrpc/` and `methods/terminal/` split by role, and `types/` is grouped by domain. Paths named in the tasks above are the pre-move paths. Behaviour and public paths are unchanged. `cargo make dod` exits 0 with zero warnings (herdrify_hc: 212 unit, 7 schema_coverage, 49 doctests).
