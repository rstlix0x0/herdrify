---
status: approved
created: 2026-11-23
depends-on: [17]
built-on: aab842c3c7d3
---

# Client Fake Tests Implementation Plan

**Goal:** Prove the public client surface against a fake transport replaying recorded herdr-app
runs.

**Architecture:** An offline integration test that uses only the public API (spec §12.5).
`tests/support/fake_transport.rs` is included by `#[path]` from `tests/client_fake.rs`, as hc's
`tests/client_fake.rs` does.

**Tech Stack:** Rust 1.94, tokio 1.54.1 (dev).

**Content authority:** spec §12.5.

---

## Files that change

```
crates/herdrify-app/tests/support/fake_transport.rs   — create: FakeTransport (scripted RunOutputs, recorded invocations)
crates/herdrify-app/tests/client_fake.rs              — create: the §12.5 cases
```

## Order of work

### Task 1 — Add FakeTransport and its test file

**Files:**
- Create `crates/herdrify-app/tests/support/fake_transport.rs`.
- Create `crates/herdrify-app/tests/client_fake.rs` — `//!` block, `#[path =
  "support/fake_transport.rs"] mod fake_transport;`, and the first test.

**Change:** `FakeTransport` implements the public `Transport`:
- a queue of `RunOutput`s behind a mutex
- each invocation's argv recorded
- `Clone`, sharing the queue

**Proof:**
- `fake_transport_replays_in_order` — two scripted outputs come back in order, and both argv lists
  are recorded.

**Done when:** `cargo test -p herdrify-app --test client_fake` passes and the Definition of Done is
green. Commit `test(herdrify-app): add the fake transport`.

### Task 2 — Cover the client through the fake

**Files:**
- Modify `crates/herdrify-app/tests/client_fake.rs`.

**Change:** Spec §12.5's cases, using recorded stdout strings from the chain evidence as literals.

**Proof:**
- `every_output_shape_decodes` — each §5.8 output from its recorded reply.
- `every_failure_maps_to_its_error` — `Refused`, `NoWindow`, `Cli`, `Unexpected` (code and
  signal) and `Decode`, each with its exact §10 Display.
- `version_checks` — 0.1.16, 0.1.17 and 0.2.0 accepted; 0.1.15 and 0.1.16-rc.1 refused.
- `raw_reads_unnamed_fields` — a recorded `status` plus an extra field.
- `raw_version_is_decode`.
- `herdr_app_from_client_checks_once` — one `--version` recorded.
- `invalid_plain_values_run_nothing` — an empty name and index 0 each give `InvalidArgument`, and
  the fake records nothing after the version check.

**Done when:** `cargo test -p herdrify-app --test client_fake` passes and the Definition of Done is
green. Commit `test(herdrify-app): cover the client against a fake transport`.

---

## Risks

- None specific.

## Proof

- `cargo make dod` is green.
