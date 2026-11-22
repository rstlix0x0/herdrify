---
status: approved
created: 2026-11-22
depends-on: [04]
built-on: ca0f990f7018
---

# Method Params Implementation Plan

**Goal:** Give each of the 31 non-hello methods a send-only `Params` type that can express exactly the requests the core accepts.

**Architecture:**
- `protocol/method.rs` holds `trait Method { const NAME: &'static str; type Params: Serialize; type Output: DeserializeOwned; }` (spec §4.8).
- `protocol/methods/` holds one file per group (spec §2 tree): `server`, `terminal`, `templates`, `machine`, `file`, `doc`, `events`, `rpc`.
- Two shared helpers are small files added inside the same module; the dependency rule is unchanged:
  - `methods/argv.rs` holds `Argv`, which `terminal.run`, `machine.exec` and `ProcessTransport` (plan 10) share
  - `methods/seconds.rs` is the serde `with` module for timeouts
- `hello`'s Params and `Method` impl live in `protocol/hello.rs` (plan 06).
- **Params are send-only.** They implement `Serialize`, not `Deserialize` (spec §4.8).
- **Constructors and setters.** Required params are `new()` arguments. Optional params are setters taking `self` by value and returning `Self`. Optional booleans that default to false are argument-less setters (`.keep()`, `.rm()`), so no `bool` parameter exists (`strong-types.md`, spec §4.8).
- **Serialization.** Unset optional fields are omitted from the JSON. Field names are the schema's param names, read from `.claudestacks/sdlc/2026-11-22-herdrify-hc/evidence/probe-schema-params-output.txt` and the fixture `evidence/upstream-v0.1.21-schema-hc.openrpc.json` (`methods[].params`).
- Each file has the four-question `//!` block and colocated unit tests. Tests return `Result`.

**Tech Stack:** Rust 1.94.1, serde 1.0.229, serde_json 1.0.154.

**Content authority:** spec §4.8 (mapping rules, refused combinations, allowed empties, the 32-method table), §8.2 (rows owned by `protocol::methods`).

---

## Files that change

```
crates/herdrify-hc/src/protocol/mod.rs               — modify: mod method, mod methods, re-exports
crates/herdrify-hc/src/protocol/method.rs            — create: trait Method
crates/herdrify-hc/src/protocol/methods/mod.rs       — create: export-only table of contents
crates/herdrify-hc/src/protocol/methods/argv.rs      — create: Argv (non-empty program and arguments)
crates/herdrify-hc/src/protocol/methods/seconds.rs   — create: serde with-module: Duration as f64 seconds
crates/herdrify-hc/src/protocol/methods/server.rs    — create: ServerStatus, ServerStop, ServerRestart
crates/herdrify-hc/src/protocol/methods/templates.rs — create: TemplatesList
crates/herdrify-hc/src/protocol/methods/rpc.rs       — create: RpcDiscover
crates/herdrify-hc/src/protocol/methods/events.rs    — create: EventsSubscribe
crates/herdrify-hc/src/protocol/methods/terminal.rs  — create: the 15 terminal Params plus Command, Keep, LabelChange, LabelChanges, WaitTarget
crates/herdrify-hc/src/protocol/methods/machine.rs   — create: MachineExec, ExecCommand, MachineAdd, MachineTarget, MachineList, MachineConnect, MachineRename, MachineRm
crates/herdrify-hc/src/protocol/methods/file.rs      — create: FileRead, FileWrite, FileEdit
crates/herdrify-hc/src/protocol/methods/doc.rs       — create: DocGet, DocSet
```

## Order of work

Tasks 2 to 5 share one structure. Each Params type gets two tests that assert its exact serialized JSON object:
- `new()` only gives exactly the schema's required names, plus the chosen variant's params where a command, target or wait enum stands in for optional params
- every setter applied gives exactly the schema's full param list

Each `Method` impl's `NAME` is the wire method name, and its `Output` is the spec §4.8 table's output type.

### Task 1 — Add the Method trait, Argv and timeouts in seconds

**Files:**
- Create `crates/herdrify-hc/src/protocol/method.rs`, `methods/mod.rs`, `methods/argv.rs`, `methods/seconds.rs`.
- Modify `crates/herdrify-hc/src/protocol/mod.rs` — `mod method; mod methods;` and re-exports.

**Change:**
- `Method` as above. `method.rs` is a pure trait file, exempt from colocated tests (`unit-test-mandate.md` exemption #3), and its `//!` cites the exemption.
- `Argv` wraps a non-empty `Vec<String>`. `Argv::new` with an empty list is `Err(EmptyValueError { what: "argv" })`, which displays `argv must not be empty` (spec §4.8, §8.1). `program()` returns the first element and `args()` the rest. It serializes as a JSON array.
- `seconds` serializes `Duration` as an f64 count of seconds (`Duration::from_millis(1500)` → `1.5`) (spec §4.8). It is crate-private.

**Proof:**
- `argv_refuses_an_empty_list` — `Argv::new(vec![])` displays `argv must not be empty`.
- `argv_splits_program_and_args` — `["ssh","box","hc","connect"]` gives program `ssh` and three args, and serializes as that array.
- `seconds_serializes_fractional_seconds` — 1500 ms serializes as `1.5`, and 180 s as `180.0`.

**Done when:** `cargo test -p herdrify-hc argv` and `cargo test -p herdrify-hc seconds` pass and the Definition of Done is green (`cargo make dod` exits 0; the log grep of `CLAUDE.md` prints nothing). Commit `feat(herdrify-hc): add the Method trait, Argv and timeouts in seconds`.

### Task 2 — Add the server, templates, rpc and events Params

**Files:**
- Create `crates/herdrify-hc/src/protocol/methods/server.rs`, `templates.rs`, `rpc.rs`, `events.rs`.
- Modify `crates/herdrify-hc/src/protocol/methods/mod.rs`.

**Change:**
- `ServerStatus` (output `StatusResult`), `ServerStop` (`StopResult`), `ServerRestart` with the optional `environment` as `Environment` (`StopResult`), and `TemplatesList` (`TemplatesResult`).
- `RpcDiscover`, whose output is `serde_json::Value`.
- `EventsSubscribe` with the optional `children_of`, `labels` and `terminal` (output `SubscribeResult`).
- All required-arg-free `new()`s.

**Proof:**
- `server_and_templates_params_serialize_as_empty_objects` — `ServerStatus::new()`, `ServerStop::new()`, `TemplatesList::new()` and `RpcDiscover::new()` serialize as `{}`.
- `server_restart_serializes_environment` — with the setter, it serializes `{"environment":{...}}` with the given map.
- `events_subscribe_serializes_every_filter` — with every setter, the keys are exactly `children_of`, `labels`, `terminal`.
- `method_names_match_the_wire` — the six `NAME`s are `server.status`, `server.stop`, `server.restart`, `templates.list`, `rpc.discover`, `events.subscribe`.

**Done when:** `cargo test -p herdrify-hc methods` passes and the Definition of Done is green. Commit `feat(herdrify-hc): add the server, templates, rpc and events params`.

### Task 3 — Add the terminal lifecycle Params

**Files:**
- Create `crates/herdrify-hc/src/protocol/methods/terminal.rs`.
- Modify `crates/herdrify-hc/src/protocol/methods/mod.rs`.

**Change:**
- `TerminalRun::new(Command)` with `Command::{Shell, Argv(Argv), Line(String)}`. `Shell` sends neither `argv` nor `line`, so the core starts its shell. `Line` may be empty. "Both" cannot be expressed (spec §4.8, E2–E4). Setters cover every other schema param: `caller`, `cwd`, `die_with_parent`, `display`, `env`, `keep`, `labels`, `name`, `no_notify`, `no_parent`, `on`, `outlive_parent`, `parent`, `rm`, `size`, `timeout`, `wait`. Each optional boolean is an argument-less setter. `size` is `InitialSize`, `display` is `DisplayFacts`, `timeout` is `Duration`.
- `TerminalList` (optional `all`, `labels`).
- `TerminalGet`, `TerminalKill` and `TerminalRm`, each `new(TerminalRef)` with an optional `caller`.
- `TerminalKeep::new(TerminalRef, Keep)`, where `Keep::{On, Off}` serializes as the required boolean.
- `TerminalLabel::new(TerminalRef, LabelChanges)`. `LabelChanges` maps `LabelKey` to `LabelChange::{Set(LabelValue), Remove}`, and `Remove` serializes as `""` (spec §4.8).
- `TerminalRead::new(TerminalRef)` with optional `caller` and `lines`.
- Outputs follow the spec §4.8 table.

**Proof:**
- `terminal_run_shell_sends_no_command` — `TerminalRun::new(Command::Shell)` serializes as `{}`.
- `terminal_run_argv_and_line` — `Command::Argv(["sleep","5"])` serializes `{"argv":["sleep","5"]}`; `Command::Line("")` serializes `{"line":""}`.
- `terminal_run_every_setter` — with every setter, the keys equal the schema's 19 `terminal.run` params minus `line` for `Command::Argv`, minus `argv` for `Command::Line`, and minus both for `Command::Shell` (spec §9.3 item 2).
- `terminal_keep_serializes_the_boolean` — `Keep::On` gives `"keep":true`, `Keep::Off` gives `"keep":false`.
- `label_remove_serializes_as_empty_string` — `{a: Set("x"), b: Remove}` serializes `"labels":{"a":"x","b":""}`.
- `terminal_lifecycle_required_only` — `TerminalGet`, `TerminalKill`, `TerminalRm` and `TerminalRead` with `new()` only serialize exactly `{"terminal":"t7"}`.

**Done when:** `cargo test -p herdrify-hc methods::terminal` passes and the Definition of Done is green. Commit `feat(herdrify-hc): add the terminal lifecycle params`.

### Task 4 — Add the terminal interaction Params

**Files:**
- Modify `crates/herdrify-hc/src/protocol/methods/terminal.rs`.
- Modify `crates/herdrify-hc/src/protocol/methods/mod.rs`.

**Change:**
- `TerminalAttach::new(TerminalRef, u16 cols, u16 rows)`. 0 is allowed (spec §4.8, R2). Optional: `caller`, `display`, `history`, `observe`, `resize_notifications`, `resumed`. Output `AttachResult`.
- `TerminalSend::new(TerminalRef, String text)`. Empty text is allowed (E17). Optional: `caller`, `force`, `if` as `IdleOrWorking`, `timeout`, `wait`.
- `TerminalWait::new(WaitTarget)` with `WaitTarget::{Terminal(TerminalRef), Children(Children), ChildrenOf(TerminalRef, Children)}`, so a target is required (E23). Optional: `caller`, `output`, `timeout`. `ChildrenOf` sends both `terminal` and `children`.
- `TerminalKeys::new(TerminalRef, Vec<KeyName>)`. An empty list is allowed (E18). Optional `caller`.
- `TerminalInterrupt::new(TerminalRef)` with an optional `timeout`.
- `TerminalPut::new(TerminalRef, FileName, Vec<u8>)`, with `data` base64-encoded through `base64_bytes`.

**Proof:**
- `terminal_attach_required_only` — `TerminalAttach::new(t7, 0, 0)` serializes exactly `{"cols":0,"rows":0,"terminal":"t7"}` (key order free).
- `terminal_send_allows_empty_text` — `TerminalSend::new(t7, "")` serializes `{"terminal":"t7","text":""}`; with every setter, `if` appears under the key `if`.
- `terminal_wait_targets` — `Terminal(t7)` gives `{"terminal":"t7"}`, `Children(Any)` gives `{"children":"any"}`, and `ChildrenOf(t7, All)` gives `{"terminal":"t7","children":"all"}`.
- `terminal_keys_allows_an_empty_list` — `TerminalKeys::new(t7, vec![])` serializes `"keys":[]`.
- `terminal_put_encodes_data` — data `x\n` serializes as `"data":"eAo="`.
- `interaction_every_setter` — `TerminalAttach`, `TerminalSend`, `TerminalWait`, `TerminalInterrupt` with every setter have exactly their schema's param names.

**Done when:** `cargo test -p herdrify-hc methods::terminal` passes and the Definition of Done is green. Commit `feat(herdrify-hc): add the terminal interaction params`.

### Task 5 — Add the machine, file and doc Params

**Files:**
- Create `crates/herdrify-hc/src/protocol/methods/machine.rs`, `file.rs`, `doc.rs`.
- Modify `crates/herdrify-hc/src/protocol/methods/mod.rs`.

**Change:**
- `MachineExec::new(ExecCommand::{Argv(Argv), Line(String)})`: exactly one of the two (E6, E7). Optional: `cwd`, `env`, `input` as bytes via `base64_bytes`, `on`, `timeout`.
- `MachineAdd::new(MachineTarget::{Ssh { target: String, name: Option<MachineName> }, Via { name: MachineName, command: String }})`. `Ssh` sends `target` and, when set, `name`; `Via` sends `name` and `via` (spec §4.8). `MachineAdd` has no `name` setter. Its setters are `session` as `SessionName` and the argument-less `temporary`.
- `MachineList::new()`, `MachineConnect::new(MachineName)`, `MachineRename::new(MachineName, MachineName new_name)` and `MachineRm::new(MachineName)`.
- `FileRead::new(RemotePath)` with optional `cwd`, `limit`, `offset`, `on`. `FileWrite::new(RemotePath, Vec<u8>)` with optional `cwd`, `on`. `FileEdit::new(RemotePath, Vec<Edit>)` with optional `cwd`, `on`.
- `DocGet::new(DocName)` with optional `on`. `DocSet::new(DocName, serde_json::Value, DocVersion)` with optional `on`. `Value::Null` is allowed and deletes (E24).
- Outputs follow the spec §4.8 table.

**Proof:**
- `machine_exec_commands` — `Argv(["true"])` gives `{"argv":["true"]}`; `Line("echo hi")` gives `{"line":"echo hi"}`.
- `machine_add_targets` — `Ssh { target: "box", name: None }` gives `{"target":"box"}`; `Ssh` with `name: Some(m2)` adds `"name":"m2"`; `Via { name: m1, command: "false" }` gives `{"name":"m1","via":"false"}`.
- `file_write_encodes_data` — data `x\n` gives `"data":"eAo="`.
- `doc_set_allows_null` — `DocSet::new(n, Value::Null, DocVersion(1))` serializes `{"name":"n","value":null,"version":1}`.
- `machine_file_doc_every_setter` — each of the 11 Params with every setter has exactly its schema's param names, except the two enum-backed ones, which follow spec §9.3 item 2 per variant: `MachineExec` with `Argv` lacks only `line` and with `Line` lacks only `argv`; `MachineAdd` with `Ssh` (name set) lacks only `via` and with `Via` lacks only `target`.

**Done when:** `cargo test -p herdrify-hc methods` passes and the Definition of Done is green. Commit `feat(herdrify-hc): add the machine, file and doc params`.

---

## Risks

- **`MachineAdd` wire keys.** The `Ssh`/`Via` serialization above follows the schema's `machine.add` params (`evidence/probe-schema-params-output.txt:100-104`) and the observed calls: `{"name":"m1","via":"false"}` (`evidence/probe-methods-output.txt:302`) and `{"target":"hfy-invalid.invalid"}` (`:310`).
- **A param missed by hand.** Plan 07's coverage check 2 compares every Params with the fixture.
- **Riskiest step:** Task 3, the largest Params type (`TerminalRun`, 19 schema params).

## Proof

- `cargo test -p herdrify-hc` passes.
- `grep -rn 'tokio' crates/herdrify-hc/src/protocol` prints nothing.
- The Definition of Done is green.
