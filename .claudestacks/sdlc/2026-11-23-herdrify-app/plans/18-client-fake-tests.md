---
status: done
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

## Review findings

Reviewer (one pass, 2026-11-24): plan-compliant, blocking set empty. Scoped gate during review (plan 19's files were mid-edit): `rustfmt --check --edition 2024` on both files exit 0; `cargo clippy -p herdrify-app --test client_fake --all-features -- -D warnings` exit 0; `cargo test -p herdrify-app --test client_fake` → `8 passed; 0 failed`. Full gate after plan 19 landed: `dod-app.sh` → `exit=0`, no warning/error lines; `client_fake` `test result: ok. 8 passed`.

- open 🔵 `tests/client_fake.rs:3`: the header says outputs are recorded "except where a comment says otherwise", but several made-up literals (`{"spaces":[]}`, the `ls` refusal, `"boom\n"`, `"not json"`, the version strings, …) carry no comment.
- open 🔵 `tests/client_fake.rs:130`: `COMMANDS` is not "the first two entries" of the recorded listing.
- open 🔵 `tests/client_fake.rs:148`: `REFUSED` is the compact form of the recorded pretty-printed stdout, not its bytes.
- open 🔵 `tests/client_fake.rs:183`: `sent_arguments` returns `Result` but asserts with `assert_eq!`.
- open ❓ `tests/support/fake_transport.rs:1`: its `//!` has no Responsibilities lists (it matches hc's support file).

## Deviations

- 2026-11-24: `cargo make dod` is replaced by the scoped gate (workspace fmt, clippy and doc; herdrify-app tests and doctests); `cargo make dod` fails only in herdrify-hc's `tests/live.rs` `live_schema_matches_the_fixture` against the installed hc 0.1.22, outside this chain.
- 2026-11-24: Task 2's tests compile against existing code, so fail-first was one deliberately wrong Display expectation (`every_failure_maps_to_its_error` failed, then restored); a run past the scripted outputs returns `TransportError::NotFound` and is checked as the transport case.

## Fix pass (2026-11-24, on the author's word "fix all of open findings")

- fixed 🔵 :3: every made-up literal carries a "made up" comment and the header says so.
- fixed 🔵 :130: `COMMANDS` is documented as the `space.select` and `space.next` entries, checked against `herdr-app-0.1.17-commands.json`.
- fixed 🔵 :148: `REFUSED` is documented as the one-line form of the recorded pretty-printed stdout (`probe-window-output.txt` E4).
- fixed 🔵 :183: `sent_arguments` returns `Err` on a wrong argv shape.
- fixed ❓ fake_transport.rs:1: its `//!` has Responsibilities and Non-responsibilities lists.
- `cargo test -p herdrify-app --test client_fake` → `8 passed`.
- Gate after the pass (all five fixes together): `dod-app.sh` → `exit=0`, no warning/error lines; 253 unit, catalog 5, client_fake 8, files_roundtrip 3, live 10 (ran, not skipped), 447 + 29 doctests; `cargo clippy -p herdrify-app --all-targets --all-features --target x86_64-pc-windows-msvc -- -D warnings` clean; `cargo deny check` → `advisories ok, bans ok, licenses ok, sources ok`; the seven §2 greps print nothing; `pgrep -fl herdr-app` prints nothing.
