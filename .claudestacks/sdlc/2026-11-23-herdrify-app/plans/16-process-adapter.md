---
status: done
created: 2026-11-23
depends-on: [14, 15]
built-on: aab842c3c7d3
---

# Process Adapter Implementation Plan

**Goal:** Implement `Transport` by spawning the herdr-app program.

**Architecture:** The `adapter` layer depends on `port` and `protocol` only, and is the only code
that touches tokio's process API (spec §2). `adapter/mod.rs` is export-only, and `lib.rs` gains
`pub mod adapter;`. The shared rules of plan 02's Architecture apply verbatim. This plan depends
on plan 15 only to order the edits of `lib.rs`.

Spec §12.1 sketches the env test as `.program("sh")` running `sh -c 'printf …'`. No `Invocation`
can carry `-c`, so the unix tests run a written script instead, as below.

**Tech Stack:** Rust 1.94, tokio 1.54.1 (`process`).

**Content authority:** spec §9; §6 (the spawn-error mapping); §11 (the process rows).

---

## Files that change

```
crates/herdrify-app/src/lib.rs                  — modify: pub mod adapter
crates/herdrify-app/src/port/error.rs           — modify: crate-private TransportError::from_spawn (first used here)
crates/herdrify-app/src/adapter/mod.rs          — create: export-only table of contents of the adapters
crates/herdrify-app/src/adapter/process.rs      — create: ProcessTransport, ProcessTransportBuilder
```

## Order of work

### Task 1 — Add ProcessTransport

**Files:**
- Modify `crates/herdrify-app/src/lib.rs`.
- Modify `crates/herdrify-app/src/port/error.rs` — `pub(crate) fn from_spawn(program:
  &OsStr, io::Error) -> TransportError`, mapping `ErrorKind::NotFound` to `NotFound` and anything
  else to `Io`, with a unit test there.
- Create `crates/herdrify-app/src/adapter/mod.rs` and `adapter/process.rs`.

**Change:**
- `ProcessTransport { program: OsString, home: Option<PathBuf>, env: Vec<EnvChange> }`, where
  `EnvChange` is a private `Set` or `Remove`. It is `Clone` and `Debug`.
- `ProcessTransport::herdr_app()`, with program `"herdr-app"`.
- `ProcessTransport::builder()` → `ProcessTransportBuilder` with `.program`, `.home`, `.env`,
  `.env_remove` and `.build()`.
- `impl Transport` follows spec §9 steps 1–4: `from_spawn` maps spawn errors, and `status.code()`
  maps to `Exit::Code` or `Exit::Signal`.
- The type's docs say the child inherits the environment, that dropping the future kills the
  child, and that there is no timeout.

**Proof:**
- `from_spawn_maps_not_found` (in `port/error.rs`) — `NotFound` maps to `NotFound`;
  `PermissionDenied` maps to `Io` and keeps its source.
- `missing_program_is_not_found` — `.program("herdrify-app-no-such-program")` running
  `Invocation::version()` gives `TransportError::NotFound`, displaying
  ``cannot start `herdrify-app-no-such-program`: it was not found``.
- `process_transport_on_unix` (`#[cfg(unix)]`), **one** test function so that no other test of
  this file forks while its scripts are written:
  1. It first writes three `#!/bin/sh` scripts (mode 0755) under
     `std::env::temp_dir()/herdrify-app-adapter-<pid>/`, each closed after writing.
  2. It then runs, in sequence:
     - **argv and output:** a script printing `$1` on stdout and `err` on stderr, run with
       `Invocation::ls()`, gives stdout `ls`, stderr `err` and `Exit::Code(0)`.
     - **home and env order:** a script printing `$HERDR_APP_HOME|$A`, run with
       `.home("/tmp/h")`, `.env("A","1")` then `.env_remove("A")`, gives stdout `/tmp/h|`.
     - **signal:** a script running `kill -TERM $$` gives `Exit::Signal`.
  3. It removes the directory.
- A `no_run` doctest on `ProcessTransport::builder`.

**Done when:** `cargo test -p herdrify-app adapter` and `cargo test -p herdrify-app port` pass, and
the Definition of Done is green on the unix and Windows CI legs (the script test is unix-only).
Commit `feat(herdrify-app): add ProcessTransport`.

---

## Risks

- Windows runs only the `NotFound` test.
- Writing all scripts before any spawn, in one test function, avoids `ETXTBSY` ("text file busy")
  from a concurrent fork.

## Proof

- `cargo make dod` is green, and the §2 grep `crate::(driver|herdr_app)` in `src/adapter` prints
  nothing.

## Review findings

Reviewer (one pass, 2026-11-24): plan-compliant, blocking set empty. Gate re-run: `dod-app.sh` → `exit=0`, no warning/error lines; 198 unit, 5 + 3 integration, 263 + 29 doctests passed. `cargo clippy -p herdrify-app --all-targets --target x86_64-pc-windows-msvc -- -D warnings` green (type check only). `grep -rnE 'crate::(driver|herdr_app)' crates/herdrify-app/src/adapter` prints nothing.

- open 🟡 `adapter/process.rs:47`: "dropping the future kills the child" rests on `kill_on_drop(true)` and no test fails if it is removed; a unix test with `sleep 1; touch <marker>` under a 200 ms timeout would guard it.
- open 🟡 `adapter/process.rs:141`: `output()` errors after the child started (pipe read, wait) also go through `from_spawn` and display "cannot start"; split `spawn()` from `wait_with_output()` or widen the docs.
- open 🟡 `adapter/process.rs:256`: `missing_program_is_not_found` forks concurrently with `process_transport_on_unix`, so the ETXTBSY guard of writing scripts before any spawn does not cover it; fold the unix check into the script test.
- open 🔵 `adapter/process.rs:18`: "A missing program is run as it is" is unclear.
- open 🔵 `adapter/process.rs:58`: doctests assert on derived `Debug` text.
- open 🔵 `adapter/process.rs:60`: derived `Debug` prints env values; a hand-written `Debug` could print keys only.
- open 🔵 `adapter/process.rs:123`: null stdin and piped stdout/stderr restate `output()`'s defaults.
- open 🔵 `adapter/process.rs:154`: "a later env change wins over the home" has no test.
- open 🔵 `adapter/process.rs:327`: a failed assert leaves `herdrify-app-adapter-<pid>` behind.
- open 🔵 `port/error.rs:57`: no test of a non-UTF-8 program name shown lossily.
- open 🔵 (plan Risks): the ETXTBSY risk line misses the concurrent fork above.

## Deviations

- 2026-11-24: `cargo make dod` is replaced by the scoped gate (workspace fmt, clippy and doc; herdrify-app tests and doctests), because `cargo make dod` fails only in herdrify-hc's `tests/live.rs` `live_schema_matches_the_fixture` against the installed hc 0.1.22, outside this chain.

## Fix pass (2026-11-24, on the author's word "fix all of open findings")

- fixed 🟡 :47: `process_transport_on_unix` runs `sleep 1; touch marker` under a 200 ms timeout and asserts no marker after 1.5 s. Mutation `kill_on_drop(false)` → "the child kept running after its future was dropped"; restored.
- fixed 🟡 :141: `spawn()` maps through `from_spawn`; `wait_with_output()` through new crate-private `TransportError::from_wait` (always `Io`, test `from_wait_is_io_even_for_not_found`). `Io`'s Display stays spec-fixed; the docs of `TransportError`, `Io`, `ProcessTransport`, `Transport::run`, `driver::Error::Transport`, the `Client` methods and `HerdrApp`/`HerdrAppBuilder::connect` now say a run that could not be started or failed while it ran.
- fixed 🟡 :256: on unix the missing-program check runs inside `process_transport_on_unix` after every script is closed; the standalone test is `#[cfg(not(unix))]`. This also covers the plan Risks line.
- fixed 🔵 :60/:58: hand-written `Debug` on `EnvChange` prints keys only (`debug_shows_env_keys_but_never_values`, red first); doctests assert behaviour, not Debug text.
- fixed 🔵 :154: new env-over-home case (`.home("/tmp/h").env("HERDR_APP_HOME","/tmp/later")` → `/tmp/later|1`); mutation (home after env) red, restored.
- fixed 🔵 :327: `ScratchDir` guard removes a stale dir on create and its dir on drop.
- fixed 🔵 port/error.rs:57: unix test `non_unicode_program_is_shown_lossily` (`b"herdr\xffapp"`); mutation red, restored.
- fixed 🔵 :18: wording. justified 🔵 :123: with `spawn()` the null stdin and piped stdout/stderr are now required, and a comment says so.
- Gate after the pass (all five fixes together): `dod-app.sh` → `exit=0`, no warning/error lines; 253 unit, catalog 5, client_fake 8, files_roundtrip 3, live 10 (ran, not skipped), 447 + 29 doctests; `cargo clippy -p herdrify-app --all-targets --all-features --target x86_64-pc-windows-msvc -- -D warnings` clean; `cargo deny check` → `advisories ok, bans ok, licenses ok, sources ok`; the seven §2 greps print nothing; `pgrep -fl herdr-app` prints nothing.
