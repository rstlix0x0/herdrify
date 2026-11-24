---
status: done
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

## Review findings

Reviewer verdict: compliant with amendments; blocking: none. Scoped gate re-run by the reviewer: `dod-app.sh` → `exit=0`, no warning/error lines; 190 unit, 3 + 3 integration, 253 doctests passed. `cargo test -p herdrify-app driver` → 19 passed. `grep -rnE 'crate::(herdr_app|adapter)' crates/herdrify-app/src/driver` → prints nothing (rc 1).

- open 🟡 `driver/client.rs:363` — two of `run`'s three `InvalidArgument` branches are untested: a serialization failure (L363) and a refused `NAME` (empty, L369).
- open 🔵 `driver/error.rs:26` — the doc says `message` has the leading `herdr-app: ` and trailing whitespace removed; that holds for `NoWindow`/`Cli` only. `Refused` keeps the JSON `error` text unchanged (reviewer probe: `Err(Refused { message: "herdr-app: x \n" })`).
- open 🔵 `driver/client.rs:33` — "Each runs its own process" depends on the transport; say each call is one transport run.
- open 🔵 `driver/client.rs:140` — "a newer version than `PROVEN` is accepted" has no test (e.g. 0.2.0).
- open 🔵 `driver/client.rs:323` — `run`'s `# Errors` names `InvalidArgument` only for the not-an-object case.
- open 🔵 `driver/client.rs:532` — `version_parses_stdout` compares with `AppVersion::PROVEN` instead of the parsed 0.1.17 literal.
- open 🔵 `driver/client.rs:701` — only `run`'s future has an in-tree `Send` check; the other six methods' futures are `Send` today (reviewer probe, Rust 1.94.1) but unguarded.
- open 🔵 `driver/client.rs:443`, `driver/error.rs:140` — test modules re-import names `use super::*` already provides.
- open 🔵 `driver/error.rs:219` — `unwrap_or(AppVersion::MINIMUM)` inside a test that does not return `Result`.
- open 🔵 Task 2 drift — `run`'s signature adds `+ Send` (see Deviations).

## Deviations

- 2026-11-24 — `cargo make dod` was replaced by the scoped gate (fmt, workspace clippy and doc with `-D warnings`, herdrify-app tests and doctests). The full gate fails only in herdrify-hc `tests/live.rs` `live_schema_matches_the_fixture`, against the installed `hc 0.1.22`, which this plan does not touch.
- 2026-11-24 — Tasks 1–2 ran as one coder in order (Task 2 edits Task 1's `error.rs`), in parallel with plan 13 (disjoint files: `src/` here, `tests/` there).
- 2026-11-24 — Task 2: `run<C: Command>` is built as `run<C: Command + Send>`. `Command`'s only supertrait is `Serialize`, and the async body keeps `command` alive across the await, so without the bound the Architecture's "every public async method returns a `Send` future" fails for a non-`Send` command.
- 2026-11-24 — Where the plan was silent: `InvalidArgument.argument` is a `String` (like the other `Error` fields); a serialization failure or an empty `NAME` in `run` also gives `InvalidArgument { argument: "command", … }`. tokio was already a dev-dependency, so `Cargo.toml` is unchanged.

## Fix pass (2026-11-24, on the author's word "fix all findings")

- fixed: new tests `run_refuses_a_command_that_cannot_serialize` and `run_refuses_an_empty_command_name`; both give `InvalidArgument { argument: "command", … }` and record no argv.
- fixed (drift resolved): `run` is now `fn run<C: Command>(&self, command: C) -> impl Future<Output = Result<C::Output, Error>> + Send + '_`. It builds the invocation synchronously (serialize and name check) before the async part, so the `C: Send` bound recorded in Deviations is gone and the signature is the plan's. Red first: the generic Send check failed with E0277 "`C` cannot be sent" before the change.
- fixed: the `Error` doc says `herdr-app: ` and trailing whitespace are stripped for `NoWindow` and `Cli` only; `Refused` keeps the JSON `error` text unchanged.
- fixed: the `Client` doc says each call is one run of the transport.
- fixed: new test `check_version_accepts_a_newer_version_and_returns_it` (`herdr-app 0.2.0`).
- fixed: `run`'s `# Errors` names every `InvalidArgument` case (cannot be written, not an object, empty name).
- fixed: `version_parses_stdout` compares with the parsed `herdr-app 0.1.17`, not `AppVersion::PROVEN`.
- fixed: `assert_calls_are_send<T: Transport, C: Command>` checks the futures of all seven methods.
- fixed: test modules no longer re-import what `use super::*` gives.
- fixed: `error_display_per_variant` returns `Result` and uses `?`.
- `cargo test -p herdrify-app driver` → 22 passed; `grep -rnE 'crate::(herdr_app|adapter)' crates/herdrify-app/src/driver` prints nothing. Gate after the pass: `dod-app.sh` → `exit=0`, no warning/error lines; 195 unit, 5 + 3 integration, 254 + 29 doctests passed.
