---
status: approved
created: 2026-11-23
depends-on: [08, 09, 10, 12, 14]
built-on: aab842c3c7d3
---

# Driver Client Implementation Plan

**Goal:** Give callers `Client<T: Transport>`, which runs typed commands and returns typed replies
or a typed `Error`.

**Architecture:** The `driver` application layer depends on `port`, `protocol` and `files`, never
on `adapter` or `herdr_app` (spec §2). `driver/mod.rs` is export-only, and `lib.rs` gains
`pub mod driver;`. Behaviour:
- `Client<T>` holds `Arc<T>`, so it is `Clone` for every `T`.
- Unit tests use an in-file scripted transport under `#[cfg(test)]` with `#[tokio::test]`; tokio
  is a dev-dependency only here.

The shared rules of plan 02's Architecture apply verbatim; doctests needing herdr-app are
`no_run`.

**Tech Stack:** Rust 1.94, serde_json 1.0.154, thiserror 2.0.21; tokio 1.54.1 in tests only.

**Content authority:** spec §7; §10 (`driver::Error` rows and the `(no output)` rule); §11 (the
driver rows).

---

## Files that change

```
crates/herdrify-app/src/lib.rs                — modify: pub mod driver
crates/herdrify-app/src/driver/mod.rs         — create: export-only table of contents of the driver
crates/herdrify-app/src/driver/error.rs       — create: Error (and, in Task 2, its crate-private from_failure)
crates/herdrify-app/src/driver/client.rs      — create: Client<T>
```

## Order of work

### Task 1 — Add Error

**Files:**
- Modify `crates/herdrify-app/src/lib.rs`.
- Create `crates/herdrify-app/src/driver/mod.rs` and `driver/error.rs`.

**Change:** `Error`, public, with every variant of spec §10's `driver::Error` table and its exact
Display:
- `Transport` and `InvalidVersion` are transparent.
- Only `Unexpected` substitutes `(no output)` for an empty stderr.

No crate-private helper is added in this task.

**Proof:**
- `error_display_per_variant` — each variant's Display equals its §10 text with the spec examples,
  e.g. `herdr-app do tab.new refused: tab.new focus is true or false`.
- `unexpected_shows_no_output_for_empty_stderr` — `Unexpected{Code(3), ""}` displays
  `herdr-app exited with code 3: (no output)`, and `Unexpected{Signal, ""}` displays
  `herdr-app was ended by a signal: (no output)`.
- `cli_shows_an_empty_message_as_is` — `Cli{message:""}` displays
  `herdr-app did not accept the invocation: `.
- `error_is_send_sync_static` — a compile-time bound check.

**Done when:** `cargo test -p herdrify-app driver::error` passes and the Definition of Done is
green. Commit `feat(herdrify-app): add the driver Error`.

### Task 2 — Add Client

**Files:**
- Create `crates/herdrify-app/src/driver/client.rs`.
- Modify `crates/herdrify-app/src/driver/error.rs` — crate-private `Error::from_failure(label,
  Failure)`, first used by `Client`.
- Modify `crates/herdrify-app/src/driver/mod.rs`.

**Change:**
- `Client<T: Transport>`:
  - `new(transport)`, wrapping it in `Arc`
  - `version`, `check_version` (refusing below `AppVersion::MINIMUM` with `UnsupportedVersion {
    found, minimum }`)
  - `commands`, `ls`, `status`
  - `run<C: Command>`
  - `raw(Invocation)`
- Each follows spec §7's flow.
- `run` refuses a command that does not serialize to an object with `InvalidArgument { argument:
  "command", detail: "a command must serialize to a JSON object" }`.
- Decode failures become `Decode { invocation: label, detail }`.
- Every public async method returns a `Send` future.

**Proof** (an in-file `ScriptedTransport` returns given `RunOutput`s and records argv):
- `version_parses_stdout` — `herdr-app 0.1.17\n` gives `0.1.17`.
- `check_version_refuses_old` — `herdr-app 0.1.15` gives `UnsupportedVersion`, displaying
  `herdr-app 0.1.15 is older than 0.1.16, the oldest version this crate supports`.
- `commands_decodes_the_catalog` — a two-entry `commands` stdout gives two `CommandInfo`s.
- `ls_and_status_decode` — the recorded empty-window `ls` (`{"spaces":[],"recent":[]}`) and the
  fresh-window `status` decode.
- `run_sends_the_command_and_decodes` — `run(TabNext)` records argv `["do","tab.next","{}"]` and
  returns `Ack`.
- `run_refuses_a_non_object_command` — a test-only `Command` whose `Serialize` writes the number
  `7` gives `InvalidArgument`, displaying `invalid command: a command must serialize to a JSON
  object`, and records no argv.
- `run_maps_refusals` — exit 1 `{"error":"x"}` gives `Refused { invocation: "do tab.next",
  message: "x" }`.
- `ls_decode_failure_is_decode` — exit 0 `{"recent":[]}` gives `Decode`, displaying
  ``cannot read the output of herdr-app ls: missing field `spaces` at line 1 column 13``.
- `raw_returns_unnamed_fields` — a `status` reply with an extra field is readable through `raw`.
- `raw_version_is_decode` — `raw(Invocation::version())` gives `Decode`.
- `client_is_clone_and_send` — `Client<ScriptedTransport>` clones (sharing the `Arc`), and `run`'s
  future is `Send` (compile-time check).
- `no_run` doctests on `Client::new`, `run` and `raw`.

**Done when:** `cargo test -p herdrify-app driver` passes and the Definition of Done is green.
Commit `feat(herdrify-app): add Client`.

---

## Risks

- None beyond the scripted-transport fidelity; plan 18 repeats the paths through the public API.

## Proof

- `cargo make dod` is green, and the §2 grep `crate::(herdr_app|adapter)` in `src/driver` prints
  nothing.
