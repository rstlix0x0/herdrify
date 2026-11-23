---
status: approved
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
