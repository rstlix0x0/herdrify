---
status: done
created: 2026-11-23
depends-on: [05, 17]
built-on: cba93d6df53e
---

# Process Adapter Implementation Plan

**Goal:** Implement the transport port over a spawned command such as `hc connect`, with `ProcessTransport`.

**Architecture:**
- An `adapter` layer file (spec §2), using tokio `process`.
- `ProcessTransport` wraps a `StreamTransport` over the child's stdout (reader) and stdin (writer).
- Its sink half also owns the child handle, so closing the sink closes stdin and `hc connect` exits 0 (W21, spec §7.2). Its source half is `ProcessSource`, a crate type that wraps `StreamSource<ChildStdout>` privately, as `ProcessSink` wraps `StreamSink<ChildStdin>`, so no tokio type reaches the public API (spec §2, M-DONT-LEAK-TYPES).
- **Closing stdin means dropping it.** In tokio 1.54.1, `ChildStdin::poll_shutdown` returns `Ok(())` and does nothing (`tokio-1.54.1/src/process/unix/mod.rs:294-296`): a child whose stdin was only shut down does not see EOF, and one whose stdin was dropped exits 0 (`evidence/probe-childstdin-output.txt`, from `evidence/probe-childstdin/`). So `ProcessSink` holds its `StreamSink<ChildStdin>` as an `Option`, and `close` flushes it and then drops it.
- The builder is a plain struct, with no type state, because `Argv` already makes the one required value a constructor argument (`strong-types.md`).
- The crate root re-exports `ProcessTransport` with `#[doc(inline)]`.
- **Conventions for every new file** (spec §2): it opens with the four-question `//!` block; tests return `Result`, because the workspace lints deny `unwrap`, `expect` and `panic`; every `mod.rs` is export-only and cites unit-test exemption #1 in its `//!`; every public item is re-exported flat from its layer module, or `unreachable_pub` fails the gate.

**Tech Stack:** Rust 1.94.1, tokio 1.54.1 (`process`, `io-util`).

**Content authority:** spec §3 (W21, W23), §7.2, §8.1 (`TransportError::Spawn` row), §8.2 (rows owned by `adapter::process`), §9.1 (`ProcessTransport` row).

---

## Files that change

```
crates/herdrify-hc/src/adapter/mod.rs      — modify: mod process; re-exports ProcessTransport, ProcessTransportBuilder, ProcessSink, ProcessSource
crates/herdrify-hc/src/adapter/process.rs  — create: ProcessTransport, ProcessTransportBuilder, ProcessSink, ProcessSource
crates/herdrify-hc/src/lib.rs              — modify: ProcessTransport re-export
```

## Order of work

### Task 1 — Add ProcessTransport and its builder

**Files:**
- Create `crates/herdrify-hc/src/adapter/process.rs`.
- Modify `crates/herdrify-hc/src/adapter/mod.rs` and `crates/herdrify-hc/src/lib.rs`.

**Change:**
- `ProcessTransport::builder(Argv) -> ProcessTransportBuilder`. `ProcessTransport::hc_connect()` is the builder for `["hc", "connect"]`, resolved on `PATH`. `ProcessTransport::from_argv(Argv)` takes any command.
- Builder methods:
  - `env(key, value)`
  - `env_remove(key)`
  - `session(SessionName)`, which appends `--session <name>` to the arguments
  - `spawn() -> Result<ProcessTransport, TransportError>`
- The child has piped stdin and stdout, inherited stderr, and `kill_on_drop(true)` (C6b). It inherits the caller's environment, and `HC_*` variables are not stripped (spec §7.2).
- A program that cannot start is `TransportError::Spawn { program, source }`, where `program` is `argv[0]` and `source` is the operating system's spawn error, unchanged (spec §7.2).
- `ProcessTransport` implements `Transport` by delegating to its inner `StreamTransport`. `into_framed` returns a `ProcessSink`, which owns the `StreamSink<ChildStdin>` (in an `Option`) and the `Child`, and a `ProcessSource`, which owns the `StreamSource<ChildStdout>` and delegates `recv` to it. `ProcessSink::close` flushes and then drops the `ChildStdin`, so the child reads EOF.
- Doc comments say:
  - callers inside a Herdr terminal inherit `HC_SCOPE`, `HC_TERMINAL_ID` and `HC_TERMINAL_SOCKET`, and use `env_remove` to opt out (spec §8.2)
  - `kill_on_drop` is the backstop when the sink was never closed

**Proof** (colocated unit tests):
- `builder_assembles_argv` — `from_argv(["ssh","box","hc","connect"])` builds a command whose program is `ssh` and whose args are `box hc connect` (read through `tokio::process::Command::as_std` `get_program`/`get_args`).
- `session_appends_the_flag` — `hc_connect().session(s1)` gives args `connect --session s1`.
- `env_and_env_remove_are_recorded` — `env("A","1")` and `env_remove("HC_SCOPE")` appear in `get_envs` as `("A", Some("1"))` and `("HC_SCOPE", None)`.
- `missing_program_is_spawn_error` — `from_argv(["hfy-no-such-program-zz"]).spawn()` is `Spawn` with that program and a `Display` starting with `cannot start hfy-no-such-program-zz: `. The error kind is not asserted, because only macOS was probed (spec §7.2).
- `close_gives_the_child_eof` (`#[cfg(unix)]`) — `from_argv(["cat"]).spawn()`, then `into_framed`; after `ProcessSink::close`, `ProcessSource::recv` returns `Ok(None)` within 5 s, because `cat` read EOF and exited.

**Done when:** `cargo test -p herdrify-hc adapter::process` passes and the Definition of Done is green (`cargo make dod` exits 0; the log grep of `CLAUDE.md` prints nothing). Commit `feat(herdrify-hc): add ProcessTransport`.

---

## Risks

- **Spawning a real `hc` is not tested here.** Plan 23's live tests cover it, against an isolated core.

## Proof

- `grep -rn 'crate::connection' crates/herdrify-hc/src/adapter` prints nothing.
- The Definition of Done is green.

## Review findings

- 🟡 (blocking) `process.rs:72,78,145` `builder`/`from_argv`/`session` took references, not the plan's values — fixed: the builder stores `Argv`, `Option<SessionName>` and the env edits and assembles the `Command` at spawn; no lint suppression. `hc_connect` uses a new `pub(crate) const fn Argv::from_parts` (test `argv_from_parts_matches_new`), so it has no panic path.
- 🟡 `adapter/mod.rs:8` Responsibilities list missed the process types — fixed.
- 🟡 `process.rs:87-93,201-209` line and frame I/O untested — fixed: `lines_and_frames_round_trip_through_cat`.
- 🟡 `process.rs:152-158` `spawn` panics outside a runtime, undocumented — fixed: `# Panics` section.
- 🟡 `process.rs:169` reverting `kill_on_drop` failed nothing — fixed: `dropping_the_sink_kills_a_child_that_ignores_its_input` (spawns `sleep 30`; with `kill_on_drop(false)`: `Error: Elapsed(())`, 7 passed, 1 failed).
- 🔵 `kill_on_drop` doc overclaimed — fixed: "the child it started, though not any process that child started in turn".
- 🔵 unreachable "was not piped" branches wrapped a made-up error as `Spawn` — fixed: one let-else returning `TransportError::Io`.
- 🔵 send after close and double close untested — fixed: `send_after_close_is_a_broken_pipe_and_close_twice_is_a_no_op`.
- 🔵 `env_and_env_remove_are_recorded` returned `()` — fixed: returns `Result`.
- 🔵 double destructure in `missing_program_is_spawn_error` — fixed. The fix round added a `NotFound` kind assertion; removed again, since the plan says the kind is not asserted.
- After fixes: `cargo make dod` EXIT=0; 231 lib tests, 4 adapter_stream, 7 schema_coverage, 51 doctests passed; log grep prints nothing.

## Deviations

- 2026-11-23 — `src/protocol/methods/argv.rs` gained `pub(crate) const fn Argv::from_parts`, outside the plan's file list, so `hc_connect()` builds `["hc","connect"]` without a panic path.
- 2026-11-23 — Not committed: the instruction to execute did not ask for commits; the change is left staged.
