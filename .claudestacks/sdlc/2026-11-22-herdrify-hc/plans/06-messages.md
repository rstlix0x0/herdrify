---
status: approved
created: 2026-11-22
depends-on: [03, 05]
built-on: ca0f990f7018
---

# Messages Implementation Plan

**Goal:** Decode every JSON-RPC message herdr-core sends on channel 0 into typed requests, responses, errors, hello results and notifications.

**Architecture:**
- The last four domain files (spec §2): `rpc_error.rs`, `envelope.rs`, `hello.rs` and `notification.rs`. Pure, with no tokio.
- The raw notification of the envelope is the `Incoming::Notification { method, params }` variant, not a separate type. The one `Notification` type is the typed enum in `notification.rs`, so no two types share a name (`modularity.md` Rule 2).
- Classification uses `serde_json::Value` for `result` and `params`. Typed decoding happens afterwards, because `RawValue` inside an untagged enum never matches (spec §4.4, C7f).
- Each file has the four-question `//!` block and colocated unit tests. Tests return `Result`.

**Tech Stack:** Rust 1.94.1, serde 1.0.229, serde_json 1.0.154, thiserror 2.0.21.

**Content authority:** spec §3 (W12, W13, W19, W20), §4.4, §4.5, §4.6, §4.10, §8.1 (`RpcError` row), §8.2 (rows owned by `protocol::hello`, `protocol::notification`, `protocol::rpc_error`).

---

## Files that change

```
crates/herdrify-hc/src/protocol/mod.rs           — modify: mod and re-export lines
crates/herdrify-hc/src/protocol/rpc_error.rs     — create: RpcError, ErrorKind
crates/herdrify-hc/src/protocol/envelope.rs      — create: Request, RequestId, Incoming, classification
crates/herdrify-hc/src/protocol/hello.rs         — create: HelloParams, HelloResult, Generation, CoreVersion, the hello Method impl
crates/herdrify-hc/src/protocol/notification.rs  — create: Notification, Event, EventKind, Seq
```

## Order of work

### Task 1 — Add RpcError and ErrorKind

**Files:**
- Create `crates/herdrify-hc/src/protocol/rpc_error.rs`.
- Modify `crates/herdrify-hc/src/protocol/mod.rs`.

**Change:**
- `RpcError { code: i64, kind: ErrorKind, message: String, hint: Option<String>, data: Value }`, decoded from the JSON-RPC `error` object `{code, message, data:{kind, hint}}` (W20). `data` keeps the whole `data` object. `Display` is `{message} [{kind}]` (spec §8.1).
- `ErrorKind` has one variant per kind in spec §4.10: the 26 schema kinds plus `unknown_method` and `parse_error`, and `Other(String)` for any other word (spec §8.2). `as_str` returns the wire word, and `Display` prints it.

**Proof:**
- `every_error_kind_decodes` — each of the 28 named words decodes to its variant, and `as_str` gives it back.
- `unknown_error_kind_is_other` — `"brand_new"` decodes to `Other("brand_new")`.
- `rpc_error_decodes_a_live_reply` — the `terminal.get` error of `evidence/probe-plain-output.txt:247` decodes with code -32000, kind `not_found` and its message, and displays `no terminal "t999" [not_found]`.
- `rpc_error_keeps_the_hint` — the `machine_unknown` error of `evidence/probe-methods-output.txt:300` keeps hint `no machines yet: hc machine add <ssh-target>`.

**Done when:** `cargo test -p herdrify-hc rpc_error` passes and the Definition of Done is green (`cargo make dod` exits 0; the log grep of `CLAUDE.md` prints nothing). Commit `feat(herdrify-hc): add RpcError and ErrorKind`.

### Task 2 — Add the JSON-RPC envelope

**Files:**
- Create `crates/herdrify-hc/src/protocol/envelope.rs`.
- Modify `crates/herdrify-hc/src/protocol/mod.rs`.

**Change:**
- `RequestId(u64)`.
- `Request { id, method: &str, params: Value }` serializes `{"jsonrpc":"2.0","id":…,"method":…,"params":{…}}`. The client always sends an object `params` (spec §4.4).
- `Incoming::classify(&[u8]) -> Result<Incoming, ProtocolError>`:
  - `Incoming::Response { id: Option<RequestId>, outcome: Result<Value, RpcError> }`. `id` is `None` for `id:null` (W9); a hello reply with `id: None` is `UnexpectedHelloReply` displaying `a response with id null` (spec §8.1).
  - `Incoming::Notification { method: String, params: Value }`.
  - Valid JSON that is neither is `ProtocolError::NotJsonRpc`, and so is a response whose `id` is neither a JSON integer fitting `u64` nor `null` (spec §4.4). Invalid JSON is `ProtocolError::InvalidJson`.

**Proof:**
- `request_serializes_with_object_params` — id 7, `terminal.get`, `{"terminal":"t7"}` serializes to that exact object.
- `classifies_a_result_response` — `{"id":1,"jsonrpc":"2.0","result":{"x":1}}` is a response with id 1 and `Ok`.
- `classifies_an_error_response_with_null_id` — the `-32700` reply of `evidence/probe-framed-output.txt:422-424` is a response with `id: None` and kind `parse_error`.
- `classifies_a_notification` — a `heartbeat` notification line is `Incoming::Notification` with method `heartbeat`.
- `refuses_json_that_is_not_json_rpc` — `[1,2]` is `NotJsonRpc`, and `{` is `InvalidJson`.
- `odd_response_ids_are_not_json_rpc` — responses with `"id":"a"`, `"id":-1` and `"id":1.5` are `NotJsonRpc`.

**Done when:** `cargo test -p herdrify-hc envelope` passes and the Definition of Done is green. Commit `feat(herdrify-hc): add the JSON-RPC envelope`.

### Task 3 — Add hello and CoreVersion

**Files:**
- Create `crates/herdrify-hc/src/protocol/hello.rs`.
- Modify `crates/herdrify-hc/src/protocol/mod.rs`.

**Change:**
- `HelloParams` has all five schema params as optional setters: `calls_you`, `core`, `features`, `framed`, `generation`. It implements `Method` with `NAME = "hello"` and `Output = HelloResult` (spec §4.5, §4.8).
- `HelloResult` has every schema property: required `core`, `features`, `framed`, `generation`, `methods` and `version`; optional `keeps_running_after_logout`, `login_session` and `logout_linger_user`. Field types come from the fixture's `HelloResult` schema, using plan 04's `CoreInfo` and `LoginSession`. It implements `Serialize` and `Deserialize`.
- `Generation(u32)` with `Generation::ONE`.
- `CoreVersion` parses the SemVer 2.0.0 grammar of spec §4.5, orders by semver precedence (build metadata ignored, a pre-release below its release), and has `CoreVersion::MINIMUM` = 0.1.21. `CoreVersion::parse` returns `Err` for an unparsable string. `Display` prints the original text.

**Proof:**
- `version_table_accepted` — `0.1.21`, `0.1.22`, `0.2.0`, `1.0.0` and `0.1.21+abc` parse and are `>= MINIMUM`.
- `version_table_below_floor` — `0.1.20`, `0.0.9` and `0.1.21-rc.1` parse and are `< MINIMUM`.
- `version_table_invalid` — `v0.1.21`, `0.1`, `""`, `0.1.x`, `0.1.21.4`, `0.01.21`, `0.1.21-`, `0.1.21+`, ` 0.1.21` and `18446744073709551616.0.0` fail to parse.
- `hello_result_decodes_the_live_reply` — the hello reply of `evidence/probe-plain-output.txt:119` decodes with version `0.1.21` and generation 1.
- `hello_params_serialize_the_client_request` — `HelloParams::new().generation(Generation::ONE).framed()` serializes `{"generation":1,"framed":true}`.

**Done when:** `cargo test -p herdrify-hc hello` passes and the Definition of Done is green. Commit `feat(herdrify-hc): add hello and CoreVersion`.

### Task 4 — Add notifications and events

**Files:**
- Create `crates/herdrify-hc/src/protocol/notification.rs`.
- Modify `crates/herdrify-hc/src/protocol/mod.rs`.

**Change:**
- `Notification::from_parts(method: &str, params: Value) -> Result<Notification, ProtocolError>` gives:
  - `Event(Event)` for `event`
  - `Heartbeat { seq }` for `heartbeat`
  - `Missed { seq, terminals: Vec<Terminal> }` for `missed`
  - `Other { method, params }` for any other method (spec §4.6)
- `Event { seq: Seq, terminal: Option<TerminalAddress>, machine: Option<MachineName>, kind: EventKind }`.
- `EventKind` has one variant per W13 type, with the `data` shapes of `evidence/upstream-v0.1.21-schema-README.md:129-140` and `:198`:
  - `Created(Terminal)`, `Restarted(Terminal)`
  - `Lost` and `Finished` with `reason: EndReason`, `exit_code`, `signal` (an integer, as in the terminal record) and `finished_ms`
  - `Keep { keep }`, `Removed`
  - `Status { status: Option<Status>, records, reported }`
  - `Turn { turn, state: StatusWord }`, `Title { title }`
  - `Labels { labels, record: Option<Terminal> }`, `Size { cols, rows }`
  - `Foreground(Option<Foreground>)`, `Doc { name, version }`
- Any other `type` is `EventKind::Unknown { kind: String, data: Value }`.
- A known `type` whose `data` does not match is `ProtocolError::InvalidEvent { kind, source }`.
- `heartbeat` or `missed` params that do not decode, and an `event` whose envelope does not decode (`seq` missing or not a `u64`, `type` missing or not a string, `terminal` not a string, `machine` not a string or `""`), are `ProtocolError::InvalidNotification { method, source }` (spec §6.3, §8.1).
- `Seq(u64)`.

**Proof:**
- `decodes_observed_events` — the `status`, `turn` and `title` event `data` objects recorded at `evidence/probe-methods-output.txt:388-391`, `:423-427` and `:449`, each wrapped in an `event` notification with a `seq` and its `type`, decode to their variants.
- `decodes_the_doc_event` — the README example `{"seq":43,"type":"doc","data":{"name":"herdr-app","version":2}}` decodes to `Doc`.
- `status_event_with_null_status` — a `status` event with `"status":null` decodes with `status: None`.
- `unknown_event_type_is_kept` — `type: "brand_new"` decodes to `Unknown` with its data.
- `mismatched_event_data_is_invalid_event` — a `title` event with `data: 5` is `InvalidEvent` with kind `title`, displaying `cannot decode a title event: …`.
- `heartbeat_missed_and_other` — `heartbeat {seq:3}`, `missed {seq:9, terminals:[]}` and method `brand.new` decode to `Heartbeat`, `Missed` and `Other`.
- `malformed_notifications_are_invalid_notification` — `heartbeat {}`, `missed {seq:"x"}`, an `event` without `type`, and an `event` with `"machine":""` each give `InvalidNotification` with that method, displaying `cannot decode a heartbeat notification: …` for the first.

**Done when:** `cargo test -p herdrify-hc notification` passes and the Definition of Done is green. Commit `feat(herdrify-hc): add notifications and events`.

---

## Risks

- **Event data shapes come from the README, not the schema.** Notifications are outside the OpenRPC document (spec §1). The `Unknown` and `InvalidEvent` paths keep a changed shape from ending a stream (spec §6.3).
- **`lost` and `finished` share a shape.** They are two variants with the same fields, so the event type stays visible to the caller.

## Proof

- `cargo test -p herdrify-hc` passes.
- `grep -rn 'tokio\|crate::port\|crate::connection\|crate::adapter' crates/herdrify-hc/src/protocol` prints nothing.
- The Definition of Done is green.
