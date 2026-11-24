---
status: approved
created: 2026-11-24
depends-on: [02, 03, 04]
built-on: 2db7af9df077
---

# Drift Checks on 0.1.22 Implementation Plan

**Goal:** Check the SDK against the herdr-core 0.1.22 OpenRPC document instead of 0.1.21's.

**Architecture:** The fixture is replaced by `hc schema` from 0.1.22, copied byte for byte from
the chain's evidence. The two tests that read it follow:
- the offline `tests/schema_coverage.rs`
- the live drift test in `tests/live.rs`

On 0.1.22, `rpc.discover` returns the same document as `hc schema` (spec P15), so both of the
drift test's assertions hold. The instance generator gains a per-property override, so
`PutBeginResult` gets a valid transfer id (spec §8).

**Tech Stack:** Rust 1.94, serde_json 1.0.154, tokio 1.54.1 (dev).

**Content authority:** spec §8 (schema_coverage, the fixture, `live_schema_matches_the_fixture`),
P15.

---

## Files that change

```
crates/herdrify-hc/tests/fixtures/hc-0.1.22.openrpc.json   — create: a byte copy of .claudestacks/sdlc/2026-11-24-herdrify-hc-0-1-22/evidence/hc-0.1.22-schema.json
crates/herdrify-hc/tests/fixtures/hc-0.1.21.openrpc.json   — delete
crates/herdrify-hc/tests/support/schema_instance.rs        — modify: the (schema, property) → value override table
crates/herdrify-hc/tests/schema_coverage.rs                — modify: read the 0.1.22 fixture; 36 methods; the new requests, docs, schemas and replies
crates/herdrify-hc/tests/live.rs                           — modify: the drift test reads the 0.1.22 fixture
```

## Order of work

### Task 1 — Add the 0.1.22 fixture and the instance override

**Files:**
- Create `crates/herdrify-hc/tests/fixtures/hc-0.1.22.openrpc.json`, a byte copy of the evidence
  file. Check with `cmp` that the two are equal. The 0.1.21 fixture stays until Task 2.
- Modify `crates/herdrify-hc/tests/support/schema_instance.rs`.
- Modify `crates/herdrify-hc/tests/schema_coverage.rs`: add the one test below. It reads the new
  fixture with `include_str!("fixtures/hc-0.1.22.openrpc.json")`.

**Change:**
- A `const STRING_OVERRIDES: [(&str, &str, &str); 1]` of `(schema, property, value)`, beside
  `BYTE_PROPERTIES`. Its one entry is `("PutBeginResult", "transfer", "0123456789abcdef")`.
- A property listed there gets that value instead of `"x"`.
- The `//!` block names the table and why it exists: the SDK validates that string.

**Proof:**
- `instance_uses_a_string_override`: against the 0.1.22 fixture,
  `instance(&fixture, "PutBeginResult")` is `{"transfer":"0123456789abcdef"}`.

**Done when:** `cargo test -p herdrify-hc --test schema_coverage instance_` passes. The
Definition of Done is green except `live_schema_matches_the_fixture`, the one known failure on hc
0.1.22 (spec P1), which Task 2 fixes. Commit `test(herdrify-hc): add the hc 0.1.22 fixture and a
valid transfer id for the schema instance`.

### Task 2 — Move both drift checks to the 0.1.22 document

**Files:**
- Delete `crates/herdrify-hc/tests/fixtures/hc-0.1.21.openrpc.json`.
- Modify `crates/herdrify-hc/tests/schema_coverage.rs`.
- Modify `crates/herdrify-hc/tests/live.rs`.

**Change:**
- `schema_coverage.rs`:
  - The `//!` and `fixture()` read `hc-0.1.22.openrpc.json`, and the docs say 0.1.22.
    `instance_uses_a_string_override` uses `fixture()`.
  - "Every one of the SDK's 32 methods" becomes 36. The asserted counts (`:607`, `:800`) become
    36.
  - `terminal_cases` adds a `plain` case for each of `TerminalPutBegin`, `TerminalPutChunk`,
    `TerminalPutFinish` and `TerminalPutAbort`.
  - The `EventsSubscribe` case includes `docs` in its full form.
  - `schema_types_round_trip_fixture_instances` adds `EmptyResult` for `Empty`, plus
    `PutBeginResult` and `PutPieceResult`.
  - The reply list adds `reply_of::<PutBeginResult, TerminalPutBegin>()`,
    `reply_of::<PutPieceResult, TerminalPutChunk>()`, `reply_of::<PutResult,
    TerminalPutFinish>()` and `reply_of::<EmptyResult, TerminalPutAbort>()`.
- `live.rs`: `live_schema_matches_the_fixture` reads `hc-0.1.22.openrpc.json`.

**Proof:**
- `method_names_equal_the_fixture`: the SDK's 36 names equal the document's 36.
- The param-name test (existing) passes with the four new requests and `docs`.
- `schema_types_round_trip_fixture_instances`: every one of the 39 schemas is checked and
  round-trips. Its "every fixture schema was checked" assertion holds.
- The reply-type test (existing) compiles and passes with the four new pairs.
- `live_schema_matches_the_fixture`, with hc 0.1.22 on `PATH`: both asserts pass, `hc schema`
  (P1) and `rpc.discover` (P15). It failed before this task.

**Done when:** `cargo test -p herdrify-hc --test schema_coverage --test live` passes and the
Definition of Done is green with no exception. Commit `test(herdrify-hc): check the SDK against
hc 0.1.22`.

---

## Risks

- The fixture must be a byte copy. Re-running `hc schema` here would start a core, and an
  earlier `hc schema` in this chain left one running. Copy the evidence file; never run `hc` by
  hand.
- `live_schema_matches_the_fixture` fails on any core other than 0.1.22. That is intended: it is
  the drift test (spec §8). CI has no `hc`, so it skips there.

## Proof

- `cargo test -p herdrify-hc --all-targets` passes with hc 0.1.22 on `PATH`.
- `cmp crates/herdrify-hc/tests/fixtures/hc-0.1.22.openrpc.json
  .claudestacks/sdlc/2026-11-24-herdrify-hc-0-1-22/evidence/hc-0.1.22-schema.json` prints
  nothing.
