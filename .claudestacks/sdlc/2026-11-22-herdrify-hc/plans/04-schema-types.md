---
status: done
created: 2026-11-22
depends-on: [02]
built-on: ca0f990f7018
---

# Schema Types Implementation Plan

**Goal:** Write by hand a Rust type for each of the 35 component schemas other than `HelloResult`, with exactly the schema's properties.

**Architecture:**
- `protocol/types/` in the domain layer, one file per type, named after the type in snake_case (`mod-rs-export-only.md`: implementation lives in sibling files named after the item). `types/mod.rs` is export-only.
- `HelloResult`, the 36th schema, lives in `protocol/hello.rs` (spec §2 tree) and is plan 06's.
- `protocol/base64_bytes.rs` is a serde `with` module shared by these types and plan 05's Params. It is a small file added inside the domain layer; the dependency rule of spec §2 is unchanged.
- Every type implements `Serialize` and `Deserialize`, so a reply value round-trips (spec §4.9). Unknown reply fields are ignored, which is serde's default.
- Optional and nullable properties are `Option<T>`, and null and absent both read as `None`. `None` serializes as an absent property, never `null` (spec §4.9).
- Field types follow the spec §4.8 mapping table and the §4.7 newtypes. The source of every property name, requirement and type is the fixture `.claudestacks/sdlc/2026-11-22-herdrify-hc/evidence/upstream-v0.1.21-schema-hc.openrpc.json` (`components.schemas`), summarized by the `S` lines of `evidence/probe-schema-table-output.txt`.
- The schema's `Display` is named `DisplayFacts`, and its doc comment names the schema type. `Terminal` and `RunResult` stay two types, each saying so in its doc comment (spec §4.9).
- Each file has the four-question `//!` block and colocated unit tests. Tests return `Result`, since the lints deny `unwrap_used` and `panic`.

**Tech Stack:** Rust 1.94.1, serde 1.0.229, serde_json 1.0.154, base64 0.23.1.

**Content authority:** spec §1 (the compatibility promise), §4.7, §4.8 (mapping rules, open enums), §4.9, §8.2 (rows owned by `protocol::types`).

---

## Files that change

```
crates/herdrify-hc/src/protocol/mod.rs                    — modify: mod types, mod base64_bytes, re-exports
crates/herdrify-hc/src/protocol/base64_bytes.rs           — create: serde with-module: Vec<u8> as standard padded base64
crates/herdrify-hc/src/protocol/types/mod.rs              — create: export-only table of contents
crates/herdrify-hc/src/protocol/types/environment.rs      — create: Environment (name → value map; names may be empty)
crates/herdrify-hc/src/protocol/types/labels.rs           — create: Labels (LabelKey → LabelValue map)
crates/herdrify-hc/src/protocol/types/children.rs         — create: Children (closed: any, all)
crates/herdrify-hc/src/protocol/types/idle_or_working.rs  — create: IdleOrWorking (closed: idle, working)
crates/herdrify-hc/src/protocol/types/terminal_state.rs   — create: TerminalState (open)
crates/herdrify-hc/src/protocol/types/end_reason.rs       — create: EndReason (open)
crates/herdrify-hc/src/protocol/types/wait_reason.rs      — create: WaitReason (open)
crates/herdrify-hc/src/protocol/types/status_word.rs      — create: StatusWord (open)
crates/herdrify-hc/src/protocol/types/status.rs           — create: Status
crates/herdrify-hc/src/protocol/types/foreground.rs       — create: Foreground
crates/herdrify-hc/src/protocol/types/first_message.rs    — create: FirstMessage
crates/herdrify-hc/src/protocol/types/display_facts.rs    — create: DisplayFacts (schema Display)
crates/herdrify-hc/src/protocol/types/initial_size.rs     — create: InitialSize
crates/herdrify-hc/src/protocol/types/terminal.rs         — create: Terminal
crates/herdrify-hc/src/protocol/types/run_result.rs       — create: RunResult
crates/herdrify-hc/src/protocol/types/core_info.rs        — create: CoreInfo
crates/herdrify-hc/src/protocol/types/login_session.rs    — create: LoginSession
crates/herdrify-hc/src/protocol/types/holder_info.rs      — create: HolderInfo
crates/herdrify-hc/src/protocol/types/status_result.rs    — create: StatusResult
crates/herdrify-hc/src/protocol/types/stop_result.rs      — create: StopResult
crates/herdrify-hc/src/protocol/types/machine.rs          — create: Machine
crates/herdrify-hc/src/protocol/types/machine_view.rs     — create: MachineView
crates/herdrify-hc/src/protocol/types/machine_list_result.rs — create: MachineListResult
crates/herdrify-hc/src/protocol/types/template_info.rs    — create: TemplateInfo
crates/herdrify-hc/src/protocol/types/templates_result.rs — create: TemplatesResult
crates/herdrify-hc/src/protocol/types/list_result.rs      — create: ListResult
crates/herdrify-hc/src/protocol/types/remove_result.rs    — create: RemoveResult
crates/herdrify-hc/src/protocol/types/read_result.rs      — create: ReadResult
crates/herdrify-hc/src/protocol/types/send_result.rs      — create: SendResult
crates/herdrify-hc/src/protocol/types/wait_result.rs      — create: WaitResult
crates/herdrify-hc/src/protocol/types/keys_result.rs      — create: KeysResult
crates/herdrify-hc/src/protocol/types/interrupt_result.rs — create: InterruptResult
crates/herdrify-hc/src/protocol/types/put_result.rs       — create: PutResult
crates/herdrify-hc/src/protocol/types/attach_result.rs    — create: AttachResult
crates/herdrify-hc/src/protocol/types/subscribe_result.rs — create: SubscribeResult
crates/herdrify-hc/src/protocol/types/edit.rs             — create: Edit
crates/herdrify-hc/src/protocol/types/file_read_result.rs — create: FileReadResult
crates/herdrify-hc/src/protocol/types/file_result.rs      — create: FileResult
crates/herdrify-hc/src/protocol/types/doc_get_result.rs   — create: DocGetResult
crates/herdrify-hc/src/protocol/types/doc_set_result.rs   — create: DocSetResult
crates/herdrify-hc/src/protocol/types/exec_result.rs      — create: ExecResult
```

## Order of work

Tasks 3 to 6 share one structure: each type is a struct with exactly the fixture's properties for that schema, mapped by spec §4.8, and each gets the same two kinds of test. One is a round trip of a fully populated JSON value. The other reads a value with every optional property absent, and then with it `null`, into `None`.

### Task 1 — Add base64 bytes and the two map types

**Files:**
- Create `crates/herdrify-hc/src/protocol/base64_bytes.rs` — the serde `with` module.
- Create `crates/herdrify-hc/src/protocol/types/mod.rs`, `environment.rs`, `labels.rs`.
- Modify `crates/herdrify-hc/src/protocol/mod.rs` — `mod base64_bytes; mod types;` and re-exports.

**Change:**
- `base64_bytes` serializes `Vec<u8>` as a string in the standard padded alphabet and deserializes it back. Invalid base64 is a serde error, so a reply holding it becomes `Error::Decode` for that method (spec §4.8, §8.2). It is crate-private, used with `#[serde(with = …)]`, and has no public surface: no base64 type is exposed (spec §2, M-DONT-LEAK-TYPES).
- `Environment` is a transparent map of `String` names to `String` values. A name may be empty, because the core accepts `env:{"":"x"}` (spec §4.7, §4.8).
- `Labels` is a transparent map of `LabelKey` to `LabelValue`.

**Proof:**
- `base64_bytes_encodes_with_padding` — bytes `x\n` serialize as `"eAo="` (`evidence/probe-base64-output.txt:36-41`).
- `base64_bytes_refuses_invalid_input` — `"not base64!"` fails to deserialize.
- `environment_accepts_an_empty_name` — `{"":"x"}` round-trips.
- `labels_refuse_an_empty_value` — `{"k":""}` fails to deserialize into `Labels`; `{"k":"v"}` round-trips.

**Done when:** `cargo test -p herdrify-hc base64_bytes`, `cargo test -p herdrify-hc environment` and `cargo test -p herdrify-hc labels` pass and the Definition of Done is green (`cargo make dod` exits 0; the log grep of `CLAUDE.md` prints nothing). Commit `feat(herdrify-hc): add base64 bytes and the environment and label maps`.

### Task 2 — Add the closed and open word enums

**Files:**
- Create `crates/herdrify-hc/src/protocol/types/children.rs`, `idle_or_working.rs`, `terminal_state.rs`, `end_reason.rs`, `wait_reason.rs`, `status_word.rs`.
- Modify `crates/herdrify-hc/src/protocol/types/mod.rs` — `mod` and re-export lines.

**Change:**
- Closed: `Children::{Any, All}` and `IdleOrWorking::{Idle, Working}`, the schema's own `enum`s (spec §1). An unknown word fails to deserialize.
- Open, each with an `Other(String)` variant that serializes back to the exact word it read (spec §4.8 "Open enums"):
  - `TerminalState`: `live`, `finished`
  - `EndReason`: `exited`, `killed`, `server_stop`, `host_shutdown`, `holder_lost`, `stopped_by_restart`, `signaled`, `spawn_pending`
  - `WaitReason`: `exit`, `timeout`, `message`
  - `StatusWord`: `idle`, `working`, `done`, `blocked`, `error`
- Each enum has `as_str` returning the wire word.

**Proof:**
- `closed_enums_round_trip_and_refuse_unknown` — `"any"`, `"all"`, `"idle"`, `"working"` round-trip; `"some"` fails for `Children`.
- `open_enums_round_trip_every_known_word` — every known word above round-trips into its named variant.
- `open_enums_keep_unknown_words` — `"paused"` reads as `Other("paused")` for each open enum and serializes back as `"paused"`.

**Done when:** `cargo test -p herdrify-hc types` passes and the Definition of Done is green. Commit `feat(herdrify-hc): add the schema word enums`.

### Task 3 — Add the terminal record and its parts

**Files:**
- Create `crates/herdrify-hc/src/protocol/types/status.rs`, `foreground.rs`, `first_message.rs`, `display_facts.rs`, `initial_size.rs`, `terminal.rs`, `run_result.rs`.
- Modify `crates/herdrify-hc/src/protocol/types/mod.rs`.

**Change:** `Status`, `Foreground`, `FirstMessage`, `DisplayFacts`, `InitialSize`, `Terminal`, `RunResult`, each with exactly its fixture properties. The narrowings of spec §4.8 apply here:
- `InitialSize` `cols`/`rows` are `NonZeroU16`, so 0 fails to deserialize (E22).
- `Status.progress` is `Option<u8>`, and `Status.state` is `StatusWord`.
- `DisplayFacts.cell` is `[u32; 2]`.
- `Terminal.state` and `RunResult.state` are `TerminalState`. `Terminal.reason` is `EndReason`.
- `address` is `TerminalAddress`, `name` is `TerminalName`, and `labels` is `Labels`.

**Proof:**
- `terminal_round_trips_a_live_record` — a `terminal.get` reply record copied from `evidence/probe-methods-output.txt` deserializes into `Terminal` and serializes back JSON-equal, except that its `null` properties come back absent (spec §4.9).
- `optional_fields_read_absent_and_null_as_none` — for `Terminal` and `RunResult`, an optional property absent and set to `null` both read as `None`.
- `initial_size_refuses_zero` — `{"cols":0,"rows":24}` fails to deserialize.
- `display_facts_cell_has_two_items` — `cell: [8, 16]` round-trips; `cell: [8]` fails.
- `run_result_carries_output_and_wait` — a `RunResult` with `output` and `wait` set round-trips.

**Done when:** `cargo test -p herdrify-hc types` passes and the Definition of Done is green. Commit `feat(herdrify-hc): add the terminal record types`.

### Task 4 — Add the server, machine and template result types

**Files:**
- Create `crates/herdrify-hc/src/protocol/types/core_info.rs`, `login_session.rs`, `holder_info.rs`, `status_result.rs`, `stop_result.rs`, `machine.rs`, `machine_view.rs`, `machine_list_result.rs`, `template_info.rs`, `templates_result.rs`.
- Modify `crates/herdrify-hc/src/protocol/types/mod.rs`.

**Change:** `CoreInfo`, `LoginSession`, `HolderInfo`, `StatusResult`, `StopResult`, `Machine`, `MachineView`, `MachineListResult`, `TemplateInfo`, `TemplatesResult`, each with exactly its fixture properties. Record `name` fields stay `String`: spec §4.7 lists `MachineName` for params and the event's `machine`, not for these records (spec §4.8 "every other plain string").

**Proof:**
- `status_result_round_trips_a_live_reply` — a `server.status` reply copied from `evidence/probe-methods-output.txt` round-trips.
- `stop_result_reads_stopped_true` — `{"stopped":true}` reads (W22).
- `machine_types_round_trip` — fully populated `Machine`, `MachineView` and `MachineListResult` values round-trip.
- `optional_fields_read_absent_and_null_as_none` — for `StatusResult`, `Machine` and `MachineView`, an optional property absent and set to `null` both read as `None`.

**Done when:** `cargo test -p herdrify-hc types` passes and the Definition of Done is green. Commit `feat(herdrify-hc): add the server, machine and template result types`.

### Task 5 — Add the terminal operation result types

**Files:**
- Create `crates/herdrify-hc/src/protocol/types/list_result.rs`, `remove_result.rs`, `read_result.rs`, `send_result.rs`, `wait_result.rs`, `keys_result.rs`, `interrupt_result.rs`, `put_result.rs`, `attach_result.rs`, `subscribe_result.rs`.
- Modify `crates/herdrify-hc/src/protocol/types/mod.rs`.

**Change:** `ListResult`, `RemoveResult`, `ReadResult`, `SendResult`, `WaitResult`, `KeysResult`, `InterruptResult`, `PutResult`, `AttachResult`, `SubscribeResult`, each with exactly its fixture properties. `WaitResult.reason` and the `wait` object of `SendResult` use `WaitReason`. `WaitResult.children` is `Vec<WaitResult>`: the fixture defines it as `array` of `$ref WaitResult`, one result per child. `AttachResult.channel` is `u32`, and the nullable `SubscribeResult.channel` is `Option<u32>`. A string property that spec §4.7's table does not list stays `String` (spec §4.8 "every other plain string").

**Proof:**
- `list_result_round_trips_a_live_reply` — a `terminal.list` reply copied from `evidence/probe-plain-output.txt` round-trips.
- `wait_result_round_trips_a_live_reply` — a `terminal.wait` reply copied from `evidence/probe-plain-output.txt` round-trips, `null` properties coming back absent.
- `wait_result_children_nest` — a `WaitResult` whose `children` holds one `WaitResult` round-trips.
- `subscribe_result_channel_is_optional` — a `SubscribeResult` without `channel` reads `None`; with `channel: 1`, `Some(1)`.
- `attach_result_round_trips_a_live_reply` — the attach reply of `evidence/probe-framed-output.txt:303-304` round-trips.
- `optional_fields_read_absent_and_null_as_none` — for `SendResult`, `WaitResult` and `KeysResult`, an optional property absent and set to `null` both read as `None`.

**Done when:** `cargo test -p herdrify-hc types` passes and the Definition of Done is green. Commit `feat(herdrify-hc): add the terminal operation result types`.

### Task 6 — Add the file, doc and exec result types

**Files:**
- Create `crates/herdrify-hc/src/protocol/types/edit.rs`, `file_read_result.rs`, `file_result.rs`, `doc_get_result.rs`, `doc_set_result.rs`, `exec_result.rs`.
- Modify `crates/herdrify-hc/src/protocol/types/mod.rs`.

**Change:** `Edit`, `FileReadResult`, `FileResult`, `DocGetResult`, `DocSetResult`, `ExecResult`, each with exactly its fixture properties.
- `ExecResult.stdout` and `stderr` are `Vec<u8>` through `base64_bytes`.
- `DocGetResult.value` is `serde_json::Value`, and versions are `DocVersion`.
- `path` fields in these results stay `String`: spec §4.7 maps `RemotePath` to the `path` and `cwd` params only.

**Proof:**
- `exec_result_decodes_base64_output` — `stdout: "eAo="` reads as bytes `x\n` and serializes back as `"eAo="`.
- `exec_result_refuses_invalid_base64` — `stdout: "@@"` fails to deserialize.
- `doc_get_result_reads_an_unwritten_doc` — `{"value":null,"version":0}` reads value `Null` and version 0.
- `file_types_round_trip` — fully populated `Edit`, `FileReadResult` and `FileResult` values round-trip.

**Done when:** `cargo test -p herdrify-hc types` passes and the Definition of Done is green. Commit `feat(herdrify-hc): add the file, doc and exec result types`.

---

## Risks

- **A property missed or misnamed by hand.** Plan 07's offline coverage test fails on any dropped or renamed property of all 36 schemas. That test is the backstop for these six tasks.
- **Live replies carry `null`.** `None` serializes as absent (spec §4.9), so round trips of live replies compare with their `null` properties removed. Plan 07 compares fully populated instances.
- **Option not taken:** typify or `herdr-protocol` code generation, rejected by the spec (§4.9).

## Proof

- `cargo test -p herdrify-hc` passes.
- `grep -rn 'tokio' crates/herdrify-hc/src/protocol` prints nothing.
- `ls crates/herdrify-hc/src/protocol/types | grep -vc mod.rs` prints 41: 35 schema types plus the six helper types (`Environment`, `Labels` and the four open enums). `Children` and `IdleOrWorking` are among the 35.
- The Definition of Done is green.

## Review findings

Final batch check (2026-11-23): `cargo make dod` exit 0, zero warning lines; herdrify-hc 181 unit + 47 doc + 6 schema_coverage tests pass; `cargo make deny` advisories/bans/licenses/sources ok.

- Reviewer: blocking wait_reason.rs:3 — FIXED: the docs said `Other` holds a word "not in the documented list", but the fixture documents status/output/not_started/children/running, which read as `Other`. Docs at wait_reason.rs:3-4, 9, 20, 42 now say "not one of the named variants". Verified: `rustfmt --check` ok; `cargo make dod` exit 0 with zero warning lines.
- open 🟡 public `Other(String)` lets a caller build `Other("live")`, which reads back as `Live` (end_reason.rs:52, status_word.rs:46, terminal_state.rs:40, wait_reason.rs:42); the plan fixes this shape.
- open 🟡 status-word fields are `Option<String>`, not `StatusWord` (keys_result.rs:27, send_result.rs:33, wait_result.rs:48); the plan allows it.
- open 🔵 13 + ❓ 2: open-enum code repeated 4×, `without_nulls` test helper copied 5×, no test that unknown reply fields are ignored, doc wording, Default/Copy/IntoIterator gaps, #[non_exhaustive]/Display questions.
- open plan ❓: the plan names 3 WaitReason words; the fixture documents 8. 🔵 list_result live-reply test uses probe-methods evidence.

## Deviations

- 2026-11-23 — Commits skipped: execute commits only when instructed; every task's changes are left uncommitted in the working tree.
- 2026-11-23 — One coder ran each plan's tasks in order (tasks share `protocol/mod.rs`, so no two ran at once); the coder read the plan directly instead of a task-briefer brief.
- 2026-11-23 — WaitResult was built in Task 3, because RunResult.wait needs it to compile.
- 2026-11-23 — `base64_bytes` carried `#[cfg_attr(not(test), expect(dead_code))]` in Tasks 1–5 (removed in Task 6); its functions are `pub(super)` (clippy `redundant_pub_crate` / `unreachable_pub`).
- 2026-11-23 — The list_result live-reply test uses `probe-methods-output.txt:98`, because probe-plain's list replies hold no terminals.
- 2026-11-23 — Decided where the spec is silent: non-required props with a schema default are `Option<T>`; parent/session/path fields are `String`; `Terminal.name` is `Option<TerminalName>`; Labels/Environment are sorted BTreeMap newtypes; open enums have `as_str` + `From<String>`; closed enums are `Copy` with const `as_str`.
- 2026-11-23 — After the build, the files this plan created were moved into the layout of the redesigned spec (§2, redesign #3): a concept and its error share one file, `frame/`, `jsonrpc/` and `methods/terminal/` split by role, and `types/` is grouped by domain. Paths named in the tasks above are the pre-move paths. Behaviour and public paths are unchanged. `cargo make dod` exits 0 with zero warnings (herdrify_hc: 212 unit, 7 schema_coverage, 49 doctests).
