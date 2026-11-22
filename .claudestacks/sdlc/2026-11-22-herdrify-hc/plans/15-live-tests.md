---
status: approved
created: 2026-11-22
depends-on: [07, 10, 12, 14]
built-on: ca0f990f7018
---

# Live Tests Implementation Plan

**Goal:** Exercise the SDK against a real, isolated hc 0.1.21 core (all 32 methods plus the schema drift check), skipping where `hc` is absent.

**Architecture:**
- `tests/live.rs` drives the SDK through `ProcessTransport`. Its helper `tests/support/live_core.rs` owns the skip rule and the isolated-core guard, and is included with `#[path]`.
- **No test addresses the user's default session.** Every test gets its own core (spec §1 "Live tests are isolated", §9.5):
  - session `hfy-<pid>-<n>`, where `n` is a process-wide atomic counter
  - a fresh short directory with the same name: `/tmp/hfy<pid>-<n>` on Unix (W24), `std::env::temp_dir()` joined with it on Windows. If the path already exists, the guard first runs `hc server stop` under that session and directory, removes it, then creates it fresh (spec §9.5)
  - `XDG_STATE_HOME`, `XDG_CONFIG_HOME` and `XDG_RUNTIME_DIR` set under it
  - every `HC_*` variable removed
- `hc connect` starts the core (W23). The guard's `Drop` runs `hc server stop` with the same environment plus `HC_SESSION=<session>`, then removes the directory (spec §9.5; `evidence/probe-exec-output.txt` shows that stop exiting 0).
- Test output is captured, so the skip path prints nothing to the gate log (`evidence/probe-crates-output.txt:1277-1336`).

**Tech Stack:** Rust 1.94.1, tokio 1.54.1 (dev: `macros`, `rt-multi-thread`, `time`), hc 0.1.21.

**Content authority:** spec §1, §3 (W21–W24), §9.5, §12 (live run).

---

## Files that change

```
crates/herdrify-hc/tests/support/live_core.rs — create: skip rule; isolated core guard and its ProcessTransport builder
crates/herdrify-hc/tests/live.rs              — create: all 32 methods live, plus the drift check
```

## Order of work

### Task 1 — Add the isolated-core harness and the terminal lifecycle tests

**Files:**
- Create `crates/herdrify-hc/tests/support/live_core.rs`.
- Create `crates/herdrify-hc/tests/live.rs`.

**Change:**
- `live_core.rs`:
  - `hc_available()` spawns `hc --version` and returns false if it cannot spawn
  - `LiveCore::start()` builds the isolated session, directory and environment above, and returns the guard
  - `LiveCore::transport()` returns `ProcessTransport::hc_connect()` with `.session(..)`, the three `XDG_*` values and an `env_remove` for every `HC_*` variable present in the test's environment
  - on the run path, it prints `hfy-live: running against hc <version>` to stderr (captured by the test harness unless `--nocapture`), so a run can be told from a skip
  - `Drop` runs `hc server stop` under the same environment plus `HC_SESSION=<session>`, and removes the directory
- Every live test returns early when `hc_available()` is false.
- The tests cover `hello` (via `connect`), `server.status`, `templates.list` and `terminal.run`/`list`/`get`/`read`/`send`/`wait`/`keys`/`interrupt` (with a short `timeout`)/`kill`/`keep`/`label`/`put`/`rm`, each expecting success (spec §9.5 table).

**Proof:**
- `live_connect_and_server_status` — `connect` succeeds with version ≥ 0.1.21, and `server_status` returns a `StatusResult`.
- `live_templates_list` — succeeds.
- `live_terminal_lifecycle` — the sequence the spec's probe M1 ran on an `sh` terminal (`evidence/probe-methods-output.txt:73-185`). Each step succeeds:
  1. `terminal_run` with `Command::Argv(["sh"])`
  2. list, get, `keep` `On` then `Off`
  3. label `a` set, then removed
  4. send `echo sent-ok`, then read
  5. keys `["enter"]`, interrupt with a 1 s timeout
  6. put (bytes `x\n`), wait with a 1 s timeout
  7. kill, then rm of the finished terminal

**Done when:** `cargo test -p herdrify-hc --test live -- --nocapture 2>&1 | grep -c 'hfy-live: running'` prints 3 or more here (with `hc` on `PATH`, all tests passing) and, with `hc` absent, `env PATH="$HOME/.cargo/bin:/usr/bin:/bin" cargo test -p herdrify-hc --test live` passes with every test skipped. The Definition of Done is green (`cargo make dod` exits 0; the log grep of `CLAUDE.md` prints nothing). Commit `test(herdrify-hc): add the isolated live core and terminal lifecycle tests`.

### Task 2 — Test both streams live

**Files:**
- Modify `crates/herdrify-hc/tests/live.rs`.

**Change:** No source change. Live tests for `events.subscribe` and `terminal.attach`.

**Proof:**
- `live_events_subscribe` — subscribe, run a terminal, and receive its `created` event. Then `close()`, and `next()` ends with `None` after the core's empty CLOSE (W14). A call afterwards still succeeds.
- `live_terminal_attach` — the sequence of probe B4 (`evidence/probe-framed-output.txt:294-385`):
  1. run `sh` and attach at 80×24
  2. `input(b"echo framed-ok\r")` and read `Output` until it contains `framed-ok`
  3. `resize(100, 30)`
  4. `close()`, then read `Closed(Detached)` (W15)
  5. confirm `terminal_get` still succeeds
  6. attach again, `input(b"exit\r")`, and read `Closed(Ended { reason: Exited, exit_code: Some(0), .. })` (W15, `:381-385`)

**Done when:** `cargo test -p herdrify-hc --test live` passes and the Definition of Done is green. Commit `test(herdrify-hc): test event and attach streams live`.

### Task 3 — Test the remaining methods and the typed errors live

**Files:**
- Modify `crates/herdrify-hc/tests/live.rs`.

**Change:** No source change. The rest of the spec §9.5 table.

**Proof:**
- `live_machine_exec` — `MachineExec::new(ExecCommand::Argv(["echo","hi"]))` returns `stdout == b"hi\n"` and code 0, as recorded in `evidence/probe-exec-output.txt`.
- `live_files` — `file_write` to a path inside the test directory, `file_read` returns that text, and `file_edit` replaces it.
- `live_docs` — `doc_get` of an unwritten name is version 0. `doc_set` at version 0 returns version 1, and `doc_set` of `null` at version 1 succeeds.
- `live_machine_list` and `live_rpc_discover` — succeed.
- `live_machine_typed_errors` — `machine_add(Via { name: m1, command: "false" })` returns `Rpc` with kind `Unreachable`. `machine_connect`, `machine_rename` and `machine_rm` on `nope` return `Rpc` with kind `MachineUnknown` (`evidence/probe-methods-output.txt:283-306`).
- `live_server_restart` — returns `stopped: true`, and the connection then ends with `PeerClosed` (W22).
- `live_server_stop` — returns `stopped: true`, and the connection then ends with `PeerClosed` (W22).

**Done when:** `cargo test -p herdrify-hc --test live` passes and the Definition of Done is green. Commit `test(herdrify-hc): test the remaining methods and typed errors live`.

### Task 4 — Add the live drift check

**Files:**
- Modify `crates/herdrify-hc/tests/live.rs`.

**Change:** No source change. `live_schema_matches_the_fixture` compares two documents with the fixture `tests/fixtures/hc-0.1.21.openrpc.json`, each as JSON:
- the output of `hc schema`, run with the isolated environment
- `rpc_discover()`

Each must be equal to the fixture (A8). On a difference, the failure message names every method and schema whose JSON differs (spec §9.5).

**Proof:**
- `live_schema_matches_the_fixture` — passes against hc 0.1.21.

**Done when:** `cargo test -p herdrify-hc --test live` passes and the Definition of Done is green. Commit `test(herdrify-hc): add the live schema drift check`.

---

## Risks

- **Riskiest step:** a guard that fails to stop its core leaves a stray isolated core running. Its session name and directory are unique, so it never touches the default session. `Drop` runs even when an assertion fails.
- **Live tests run inside `cargo make dod` on any machine with `hc`.** Each test is short: `timeout`s are 1 s, and every terminal a test starts is killed or ends by itself.
- **Windows.** CI has no `hc`, so the live tests skip there. The Windows directory rule (`temp_dir()`) is not exercised.

## Proof

- With `hc` on `PATH`: `cargo test -p herdrify-hc --test live` passes.
- `grep -n '"hc"' crates/herdrify-hc/tests/*.rs crates/herdrify-hc/tests/support/*.rs` matches only `tests/support/live_core.rs`, so every `hc` spawn goes through the isolated environment. The default session is never addressed, not even to read it.
- Without `hc`: the same tests pass by skipping.
- The Definition of Done is green.
