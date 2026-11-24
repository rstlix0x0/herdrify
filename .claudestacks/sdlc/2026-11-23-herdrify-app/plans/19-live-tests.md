---
status: done
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

## Review findings

Reviewer (one pass, 2026-11-24): plan-compliant with amendments, blocking set empty. Gate re-run once: `dod-app.sh` → `exit=0`, no warning/error lines; `live` `5 passed` in 3.69 s with herdr-app and hc on `PATH`, so the window scenario ran. `cargo clippy -p herdrify-app --all-targets --all-features --target x86_64-pc-windows-msvc -- -D warnings` clean. After the run `pgrep -fl herdr-app` printed nothing; no new `/tmp/hfa*`. Isolation audit: every herdr-app and hc spawn (launch, status poll, `app()`, `hc()`, leftover and teardown `hc server stop`, teardown `app.quit`, the drift test) goes through `Isolation::command` or `Isolation::app_builder`; removals precede sets; nothing sends `window.close` or a `RawReply` command; `shutdown` runs on every path and `Drop` covers panics.

- open 🟡 `tests/support/live_window.rs:143,156`: no offline test checks that `Isolation::command` and `app_builder` apply the env edits; removing the loop would pass every offline test and send the live run (teardown `app.quit` included) to the person's own window. A unix test running `sh` through both, printing `$HC_SESSION`/`$HERDR_APP_HOME`, would guard it.
- open 🟡 `tests/support/live_window.rs:285`: the owner-check file `/tmp/hfa<pid>-owner` is shared by every isolation in the process and opened `create_new`; two tests meeting a leftover at once, or a stale file after a crash, fail spuriously.
- open 🟡 `tests/live.rs:140,182`: `isolation_env_is_complete` and `isolation_removes_inherited_variables` return `()` (house rule: tests return `Result`).
- open 🟡 `tests/live.rs:259`, `tests/support/live_window.rs:365,370,389`: libtest captures `println!` on passing tests, so the drift `warning:` line and every `skipped:` line never reach the gate log; "run, not skipped" cannot be read from it.
- open 🔵 `tests/live.rs:272`: the park retry is never exercised.
- open 🔵 `tests/live.rs:336`: the fixed 500 ms sleep before `keep` is timing-dependent; a bounded retry on `no running terminal` would be sturdier.
- open 🔵 `tests/live.rs:374-377`: a `shutdown` error is dropped when the scenario also failed.
- open 🔵 `tests/live.rs:5-6`: the `//!` implies the drift test skips without a window; it does not.
- open 🔵 `tests/support/live_window.rs:57,98`: `EnvEdit` uses `None` for "remove"; `Isolation::new(pid, n)` takes two swappable `u32`s.
- open 🔵 `tests/support/live_window.rs:213`: "elsewhere the temporary directory is already private" is unverified on Windows.
- open 🔵 `tests/support/live_window.rs:463-486`: a failed `hc server stop` or kill is only `eprintln!`ed.
- open ❓ Task 3 step 3: "tab.new shell" is sent as `new_tab().argv(SLEEP)`, never in its default-shell form.
- open 🔵 (plan drift, recorded below): the 500 ms wait, `isolation_removes_inherited_variables` and `Scratch`, the drift test's home layout, `Child::kill` for `kill -KILL`.

## Deviations

- 2026-11-24: `cargo make dod` is replaced by the scoped gate (workspace fmt, clippy and doc; herdrify-app tests and doctests, live tests included); `cargo make dod` fails only in herdrify-hc's `tests/live.rs` `live_schema_matches_the_fixture` against the installed hc 0.1.22, outside this chain.
- 2026-11-24 (step 7): `hc run --bg --json --no-parent` followed at once by `terminal.keep` is refused on herdr-app 0.1.17: `Refused { invocation: "do terminal.keep", message: "no running terminal \"t6\"" }`. The window learns of the core's terminal a moment later, so the scenario waits 500 ms before `keep`, as `evidence/probe-gaps/src/main.rs:112` did. Built that way.
- 2026-11-24: `herdr-app commands` writes its assets into `HERDR_APP_HOME`, so the drift test's home is a `Scratch` directory with `x/{state,config,run}` (removed on drop), not an empty one.
- 2026-11-24: the harness adds `Scratch` (0700 home, owner and symlink refusal, isolated `hc server stop` on a leftover) and the offline test `isolation_removes_inherited_variables` (synthetic inherited `HC_*` names), which guards the remove-then-set order.
- 2026-11-24: `Drop`'s `kill -KILL <pid>` is `Child::kill` (SIGKILL on unix); `shutdown` runs the same blocking teardown through `spawn_blocking` and marks the guard done.
- 2026-11-24: leftover `/tmp/hfa48416-0` and `/tmp/hfa48416-1` (2026-11-23) predate this run and were left untouched.

## Fix pass (2026-11-24, on the author's word "fix all of open findings")

- fixed 🟡 live_window.rs:143,156: `Isolation::apply` / `apply_to_builder`; offline tests `a_command_runs_in_the_isolated_environment` (sh through `apply` and `Isolation::command`, over a parent env holding `HC_SESSION`, `HERDR_APP_HOME`, `HC_GUARD`) and `the_sdk_entrypoint_runs_in_the_isolated_environment` (script through `HerdrApp::builder().program(..)`); no herdr-app starts. Mutation 1 (both loops skipped) → `left: "mine|/home/me/.config/herdr-app|leak"` vs `right: "hfa4242-5|/tmp/hfa4242-5|unset"`; mutation 2 (Remove sets `x`) → both red; restored.
- fixed 🟡 :285: owner file is `/tmp/<session>-owner` (includes `<n>`); a stale one is removed first.
- fixed 🟡 live.rs:140,182: both return `Result` and use `?`; `#[cfg(unix)]`, as they assert `/tmp` paths.
- fixed 🟡 live.rs:259 etc.: `live_window::notice` writes straight to `io::stderr().lock()`, which libtest does not capture (probe: `println!`/`eprintln!` hidden on pass, a direct write shown); real output under `PATH=/usr/bin:/bin`: `skipped: herdr-app is not on PATH` printed beside `test isolated_window_scenario ... ok`.
- fixed 🔵 :272/:336: `retry_refused(attempts, pause, transient, attempt)` serves park (one retry, 500 ms) and keep (`no running terminal`, 50 × 100 ms; the fixed sleep is gone); offline tests `a_refusal_that_passes_is_tried_again`, `a_refusal_that_stays_ends_after_the_attempts`, `any_other_refusal_is_not_tried_again`; mutation red, restored.
- fixed 🔵 :374-377 (both errors reported), :5-6 (what skips when), live_window.rs:57,98 (`EnvEdit` enum, `IsolationId`), :213 (owner check is unix-only, said so), :463-486 (`shutdown` returns the kill / `hc server stop` / home-removal failure; `Drop` prints).
- fixed ❓ step 3: the scenario sends `tabs().new_tab()` in its default-shell form and asserts `TabMade` with a terminal.
- `cargo test -p herdrify-app --test live` → `10 passed` (four runs), drift and window tests ran.
- Gate after the pass (all five fixes together): `dod-app.sh` → `exit=0`, no warning/error lines; 253 unit, catalog 5, client_fake 8, files_roundtrip 3, live 10 (ran, not skipped), 447 + 29 doctests; `cargo clippy -p herdrify-app --all-targets --all-features --target x86_64-pc-windows-msvc -- -D warnings` clean; `cargo deny check` → `advisories ok, bans ok, licenses ok, sources ok`; the seven §2 greps print nothing; `pgrep -fl herdr-app` prints nothing.

## Deviations (fix pass)

- 2026-11-24: the drift notice reads `hfa-live: herdr-app <v> differs from the proven <p>: …`, not `warning: …`: CI fails on any log line matching `^ *(warning|error)`, and a newer herdr-app must pass with a notice.
- 2026-11-24: the 500 ms wait before `keep` (step 7) is replaced by the bounded retry above.
