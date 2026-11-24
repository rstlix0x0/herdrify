---
status: done
created: 2026-11-24
depends-on: [05]
built-on: 2db7af9df077
---

# Live Tests for the 0.1.22 Additions Implementation Plan

**Goal:** Prove the 0.1.22 additions against a live, isolated herdr-core.

**Architecture:**
- New tests in `tests/live.rs` use the existing isolated `LiveCore` (`live()`; `core.herdr_core()`
  for the `HerdrCore` layer).
- Each test chooses what to assert from the core's `hello`, so the suite behaves on 0.1.21 as
  well as on 0.1.22 (spec §8).
- A one-off acceptance run against the hc 0.1.21 release closes the plan. It changes no files.

**Tech Stack:** Rust 1.94, tokio 1.54.1 (dev), hc 0.1.22 (installed), hc 0.1.21 (release
binary, acceptance only).

**Content authority:** spec §8 (`tests/live.rs`, Acceptance on 0.1.21), §5.2, §3.4.

---

## Files that change

```
crates/herdrify-hc/tests/live.rs                                                        — modify: live_put_in_pieces, live_put_refusals, live_events_docs
.claudestacks/sdlc/2026-11-24-herdrify-hc-0-1-22/evidence/live-tests-0.1.21-after.txt   — create: the Task 3 acceptance run's output
```

## Order of work

### Task 1 — Add the live put tests

**Files:**
- Modify `crates/herdrify-hc/tests/live.rs`.

**Change:**
- A helper `has_method(&HelloResult, &str) -> bool`.
- `live_put_in_pieces`:
  - when `hello.methods` lacks `terminal.put_begin`, print `hfy-live: skipped, no
    terminal.put_begin on hc <version>` to stderr and return `Ok`
  - otherwise build a `HerdrCore` with `live.core.herdr_core()`, run `["sleep", "300"]`, and
    `put_in_pieces("big.txt", data)` with 2621440 bytes of ASCII text (three pieces)
  - check `PutResult.bytes` = 2621440, then check `machine.exec ["wc", "-c", <path>]` prints
    2621440
  - kill the terminal
- `live_put_refusals`, with the same skip rule. Through `Client`:
  - begin 4 bytes, send a 2-byte piece at offset 0, then a piece at offset 0 again: refused with
    `ErrorKind::InvalidParams`
  - `put_finish` before all bytes: refused with `ErrorKind::InvalidParams`
  - `put_abort` answers `EmptyResult`
- Notices are written to stderr with the `hfy-live:` prefix, never `warning:`, so the gate's
  log check does not trip.

**Proof:**
- `live_put_in_pieces` and `live_put_refusals` pass against hc 0.1.22, run, not skipped: the
  `hfy-live: running against hc core 0.1.22` line shows and no skip line follows.

**Done when:** `cargo test -p herdrify-hc --test live live_put -- --nocapture` passes and shows
no skip line, and the Definition of Done is green. Commit `test(herdrify-hc): prove the put in
pieces against a live core`.

### Task 2 — Add the live docs test

**Files:**
- Modify `crates/herdrify-hc/tests/live.rs`.

**Change:**
- `live_events_docs`. When the core's version is older than 0.1.22, it subscribes with `docs`
  and asserts `Error::NeedsNewerCore { option: "events.subscribe docs", .. }`.
- Otherwise it expects what a framed connection showed on 0.1.22 (spec P16). It subscribes
  through the framed client with `EventsSubscribe::new().docs([wanted])`,
  where `wanted` is a `DocName` unique to the test. It checks that the snapshot has no
  terminals. Then it `doc.set`s `wanted`, then another name, then `wanted` again, and reads
  events until the second `wanted` event arrives, under `PATIENCE`. No event for the other name
  may arrive in between.

**Proof:**
- `live_events_docs` passes against hc 0.1.22, on its 0.1.22 branch.

**Done when:** `cargo test -p herdrify-hc --test live live_events_docs -- --nocapture` passes
and the Definition of Done is green. Commit `test(herdrify-hc): prove the docs filter against a
live core`.

### Task 3 — Run the live suite against hc 0.1.21 once

No file changes of its own.

**Change:** The orchestrator runs this itself; it is not a coder task.
- Put the hc 0.1.21 release first on `PATH`, as in
  `.claudestacks/sdlc/2026-11-24-herdrify-hc-0-1-22/evidence/live-tests-0.1.21.txt:1`.
- If the scratchpad copy is gone, fetch it again and check its sha256 against the manifest, as
  in `evidence/hc-0.1.21-binary.txt`.
- Run `cargo test -p herdrify-hc --test live --test live_herdr_core -- --nocapture`.
- Save the output as `evidence/live-tests-0.1.21-after.txt` in the chain. Then check that no
  `/tmp/hfy*` directory and no `--hc-launchd` core of an `hfy` session is left.
- If running the binary is not permitted, record that in `## Deviations` and go on.

**Proof:** the saved output shows:
- every test passing except `live_schema_matches_the_fixture`, which fails and names the 0.1.21
  differences
- `live_put_in_pieces` and `live_put_refusals` printing their skip lines
- `live_events_docs` passing on its refusal branch

**Done when:**
`grep -E '^test |test result' .claudestacks/sdlc/2026-11-24-herdrify-hc-0-1-22/evidence/live-tests-0.1.21-after.txt`
shows the results above, and `grep 'hfy-live: skipped'` on the same file shows the two put
skips. The run does not belong in the Definition of Done, which runs on 0.1.22. Commit
`test(herdrify-hc): record the live suite on hc 0.1.21`.

---

## Risks

- **Doc events from other tests.** `live_events_docs` could see doc events from tests running in
  parallel, but each `LiveCore` is its own session. The unique doc name guards against the same
  test racing itself.
- **The riskiest step is Task 3.** It runs a downloaded binary, which an earlier permission
  check blocked until the author allowed it. It is recorded, not forced.
- **Leftover cores.** `hc` can leave an isolated core running: one was found after an
  `hc schema` probe in this chain. The `LiveCore` guard stops its core on drop. Task 3 also
  checks for leftovers afterwards.

## Proof

- `cargo test -p herdrify-hc --test live -- --nocapture` passes on hc 0.1.22 with no skip lines.
- After the runs, no `/tmp/hfy*` directory is left.

## Review findings

Reviewer verdict: COMPLIANT, code 0🔴 1🟡 4🔵, plan 0🔴 0🟡 2🔵 1❓, nothing blocking. DoD re-run green on hc 0.1.22 (`-p herdrify-hc`: fmt, clippy -D warnings, doc -D warnings; `--test live` 19 passed, no skip line).

- 🟡 `tests/live.rs:48-60` `put_unsupported` skipped on the method list alone, so a skip on a 0.1.22+ core would pass silently — fixed: it returns `Result<bool, TestError>`; a missing `terminal.put_begin` on a core at 0.1.22 or newer fails the test (`hc <v> must list terminal.put_begin in hello, but does not`), and below 0.1.22 it prints the unchanged skip line. `cargo test -p herdrify-hc --test live -- --nocapture` (hc 0.1.22) → `test result: ok. 19 passed; 0 failed`, no skip line.
- 🔵 `tests/live.rs:12-13` the module doc repeated "with" and said the core refuses with `Error::NeedsNewerCore` — fixed: the client refuses, before sending anything.
- 🔵 `tests/live.rs:49-50` the `put_unsupported` doc said it always prints — fixed: it prints only when the method is missing.
- 🔵 `doc_changes_until` closure parameter `seen` shadowed the outer binding — fixed: the outer binding is `outcome`.
- 🔵 `live_events_docs` asserts the literal `"events.subscribe docs"` while `Error::DOCS_OPTION` exists — kept on purpose: the literal pins the public value.
- 🔵 Task 3 evidence did not name the branch `live_events_docs` ran — fixed: the refusal branch prints `hfy-live: docs refused by the client on hc <version>`; the 0.1.21 run was repeated and `evidence/live-tests-0.1.21-after.txt` now shows that line.
- ❓ Task 3 evidence did not record the leftover check — fixed: the evidence file now lists `/tmp/hf*` before and after the run and the `--hc-launchd` processes.
- 🔵 the three task commits do not exist yet — by instruction (do not commit).

## Deviations

- 2026-11-24: Task 1 uses `tokio::time::timeout(..).await??` for the `HerdrCore` calls, since the file's `within` helper takes only `Result<_, Error>`; it adds a `put_unsupported` helper so the skip notice is written once.
- 2026-11-24: Task 2 compares against `CoreVersion::parse("0.1.22")?` because `CoreVersion::DOCS_MINIMUM` is `pub(crate)`.
- 2026-11-24: Task 3 ran the scratch copy of the hc 0.1.21 release (archive sha256 `1ab38dd9…f05ffed`, equal to `evidence/hc-0.1.21-binary.txt`), with `HC_*` unset, as `cargo test -p herdrify-hc --test live --test live_herdr_core --no-fail-fast -- --nocapture`. Output in `evidence/live-tests-0.1.21-after.txt`: live 18 passed, 1 failed (`live_schema_matches_the_fixture`, naming the 0.1.21 differences); live_herdr_core 5 passed; two `hfy-live: skipped, no terminal.put_begin on hc 0.1.21` lines; `hfy-live: docs refused by the client on hc 0.1.21`.
- 2026-11-24: The plan-level Proof "no `/tmp/hfy*` directory is left" does not hold literally: `/tmp/hfy.4K8V` (with `hfa48416-0`, `hfa48416-1`) dates from 2026-11-23, before this chain, and does not match `LiveCore`'s naming; no run here added one. A `herdr-core --hc-launchd` (pid 99744, started 2026-11-23 20:17, not an `hfy` session) predates this run and was left untouched.
- 2026-11-24: Plan 06 ran while plan 05's review fix was in flight (disjoint files) and plan 07 Tasks 1–2 ran alongside it (disjoint files); plan 07's gate ran after.
- 2026-11-24: Not committed (instruction: do not commit).
