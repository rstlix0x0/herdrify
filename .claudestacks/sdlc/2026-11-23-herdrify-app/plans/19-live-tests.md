---
status: approved
created: 2026-11-23
depends-on: [06, 13, 17]
built-on: aab842c3c7d3
---

# Live Tests Implementation Plan

**Goal:** Prove the SDK live against the installed herdr-app, isolated from the person's own.

**Architecture:** `tests/support/live_window.rs` is the isolated harness, included by `#[path]`
from `tests/live.rs`, which holds the drift test and one window scenario (spec §12.6). The harness
never runs a command against the person's window or default core. It sets:
- `HERDR_APP_HOME=/tmp/hfa<pid>-<n>`
- `HC_SESSION=hfa<pid>-<n>`
- `XDG_STATE_HOME`, `XDG_CONFIG_HOME` and `XDG_RUNTIME_DIR` under `/tmp/hfa<pid>-<n>/x/`
- every inherited `HC_*` variable and `HERDR_APP_HOME` removed

The window scenario's `layout.set` reads `tests/fixtures/layouts/main.json`, from plan 06.

**Tech Stack:** Rust 1.94, tokio 1.54.1 (dev).

**Content authority:** spec §12.6; §3 F7, F15, F16.

---

## Files that change

```
crates/herdrify-app/tests/support/live_window.rs   — create: LiveWindow harness (isolation, launch, skip rules, teardown, Drop guard)
crates/herdrify-app/tests/live.rs                  — create: the isolation test, the drift test and the window scenario
```

## Order of work

### Task 1 — Add the isolated window harness and its test file

**Files:**
- Create `crates/herdrify-app/tests/support/live_window.rs`.
- Create `crates/herdrify-app/tests/live.rs` — `//!` block, `#[path = "support/live_window.rs"]
  mod live_window;`, and the isolation test.

**Change:**
- `LiveWindow::isolation(pid, n)` gives the home path, session name and env changes above.
- `LiveWindow::launch().await -> Result<Option<LiveWindow>, Box<dyn std::error::Error>>` (a test
  helper, outside the crate's `Box<dyn` rule):
  1. remove a leftover home and create the XDG dirs
  2. spawn `herdr-app` with stdout and stderr null
  3. poll `status` every 300 ms

  It returns `Ok(None)` and prints `skipped: <reason>` when `herdr-app` or `hc` is not on `PATH`,
  the target is not unix, or the child exits before answering. It returns an error after 45 s if
  the child is alive but silent.
- `LiveWindow::app()` gives a `HerdrApp` built through `HerdrApp::builder()` with the same
  isolation.
- `LiveWindow::hc(args)` runs `hc` with the same isolation.
- `async fn shutdown(self)` runs spec §12.6's teardown steps 1–5 in order:
  1. `app.quit`, then wait up to 20 s
  2. `kill -TERM`, then wait up to 10 s
  3. `Child::kill`
  4. `hc server stop`
  5. remove the home
- `Drop`, which cannot await, runs the same steps with blocking `std::process::Command` and
  `std::thread::sleep` polling:
  - `herdr-app do app.quit {}` (isolated env)
  - `kill -TERM <pid>`
  - `kill -KILL <pid>`
  - isolated `hc server stop`
  - `std::fs::remove_dir_all`

  This is hc's `tests/support/live_core.rs` approach. `shutdown` marks the guard done, so `Drop`
  does nothing after it.

**Proof:**
- `isolation_env_is_complete` (offline) — for pid 4242 and n 0, the env list sets
  `HERDR_APP_HOME=/tmp/hfa4242-0`, `HC_SESSION=hfa4242-0` and the three XDG variables under
  `/tmp/hfa4242-0/x/`, and removes every `HC_*` variable and `HERDR_APP_HOME` present in the
  test's environment.

**Done when:** `cargo test -p herdrify-app --test live isolation_env_is_complete` passes and the
Definition of Done is green. Commit `test(herdrify-app): add the isolated window harness`.

### Task 2 — Add the drift test

**Files:**
- Modify `crates/herdrify-app/tests/live.rs`.

**Change:** `installed_commands_match_known_commands` runs with no window and `HERDR_APP_HOME` an
empty scratch directory:
1. It skips when `herdr-app` is absent.
2. It fails when `--version` is below `MINIMUM`.
3. It compares `commands` with `KNOWN_COMMANDS` through a `drift(&[CommandInfo],
   &[KnownCommand]) -> Drift { added, removed, changed }` helper:
   - installed version == `PROVEN`: fails on a non-empty drift, listing it
   - otherwise: prints `warning: herdr-app <version> differs from the proven 0.1.17: <drift>` and
     passes

**Proof:**
- `installed_commands_match_known_commands` — passes on the author's 0.1.17 install.
- `drift_reports_each_kind` (offline) — given a known table of `a(x)`, `b(y required)` and an
  installed list of `b(y optional)`, `c()`, the drift is added `[c]`, removed `[a]` and changed
  `[b: y required true → false]`.

**Done when:** `cargo test -p herdrify-app --test live` passes and the Definition of Done is green.
Commit `test(herdrify-app): add the command drift test`.

### Task 3 — Add the window scenario

**Files:**
- Modify `crates/herdrify-app/tests/live.rs`.

**Change:** `isolated_window_scenario`, a single test function and so a single window:
- runs spec §12.6's ten scenario steps in order through `HerdrApp`, `Client::raw` and the
  harness's `hc`
- asserts each output type and the `Refused` message, which starts `tab.new: unknown argument(s):
  bogus`
- retries a park refused with `This terminal already has a pane change in progress.` once, after
  500 ms
- always calls `shutdown`
- never sends `window.close` or any `RawReply` command

**Proof:**
- `isolated_window_scenario` — every step's assertion holds on the author's Mac with herdr-app
  0.1.17, and it skips elsewhere.

**Done when:** `cargo test -p herdrify-app --test live` passes on the author's Mac and skips on CI,
and the Definition of Done is green. Commit `test(herdrify-app): drive an isolated herdr-app
window`.

---

## Risks

- This is the riskiest plan: it depends on the GUI and on timing (keep and park), and the "pane
  change in progress" refusal is handled by one retry.
- On the author's Mac, `cargo make dod` opens a window for a few seconds (the author's choice).
- A crashed run leaves `/tmp/hfa<pid>-<n>`, which the next run removes.

## Proof

- `cargo make dod` is green on the author's Mac with both live tests run, not skipped, and on CI
  with both skipped.
