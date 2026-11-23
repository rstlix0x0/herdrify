---
status: superseded
created: 2026-11-22
depends-on: [05, 09]
built-on: ca0f990f7018
---

# Process Adapter Implementation Plan

**Goal:** Implement the transport port over a spawned command such as `hc connect`, with `ProcessTransport`.

**Architecture:**
- An `adapter` layer file (spec §2), using tokio `process`.
- `ProcessTransport` wraps a `StreamTransport` over the child's stdout (reader) and stdin (writer).
- Its sink half also owns the child handle, so closing the sink closes stdin and `hc connect` exits 0 (W21, spec §7.2). Its source half is `StreamSource<ChildStdout>`.
- The builder is a plain struct, with no type state, because `Argv` already makes the one required value a constructor argument (`strong-types.md`).
- The crate root re-exports `ProcessTransport` with `#[doc(inline)]`.

**Tech Stack:** Rust 1.94.1, tokio 1.54.1 (`process`, `io-util`).

**Content authority:** spec §3 (W21, W23), §7.2, §8.1 (`TransportError::Spawn` row), §8.2 (rows owned by `adapter::process`), §9.1 (`ProcessTransport` row).

---

## Files that change

```
crates/herdrify-hc/src/adapter/mod.rs      — modify: mod process; re-exports
crates/herdrify-hc/src/adapter/process.rs  — create: ProcessTransport, ProcessTransportBuilder, ProcessSink
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
- `ProcessTransport` implements `Transport` by delegating to its inner `StreamTransport`. `into_framed` returns a `ProcessSink`, which owns the `StreamSink<ChildStdin>` and the `Child`, and a `StreamSource<ChildStdout>`. `ProcessSink::close` closes stdin.
- Doc comments say:
  - callers inside a Herdr terminal inherit `HC_SCOPE`, `HC_TERMINAL_ID` and `HC_TERMINAL_SOCKET`, and use `env_remove` to opt out (spec §8.2)
  - `kill_on_drop` is the backstop when the sink was never closed

**Proof** (colocated unit tests):
- `builder_assembles_argv` — `from_argv(["ssh","box","hc","connect"])` builds a command whose program is `ssh` and whose args are `box hc connect` (read through `tokio::process::Command::as_std` `get_program`/`get_args`).
- `session_appends_the_flag` — `hc_connect().session(s1)` gives args `connect --session s1`.
- `env_and_env_remove_are_recorded` — `env("A","1")` and `env_remove("HC_SCOPE")` appear in `get_envs` as `("A", Some("1"))` and `("HC_SCOPE", None)`.
- `missing_program_is_spawn_error` — `from_argv(["hfy-no-such-program-zz"]).spawn()` is `Spawn` with that program and a `Display` starting with `cannot start hfy-no-such-program-zz: `. The error kind is not asserted, because only macOS was probed (spec §7.2).

**Done when:** `cargo test -p herdrify-hc adapter::process` passes and the Definition of Done is green (`cargo make dod` exits 0; the log grep of `CLAUDE.md` prints nothing). Commit `feat(herdrify-hc): add ProcessTransport`.

---

## Risks

- **Spawning a real `hc` is not tested here.** Plan 15's live tests cover it, against an isolated core.

## Proof

- `grep -rn 'crate::connection' crates/herdrify-hc/src/adapter` prints nothing.
- The Definition of Done is green.
