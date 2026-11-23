---
status: superseded
created: 2026-11-22
built-on: da60b634bb29
adrs: [0001, 0002]
---

# Spec: herdrify-hc SDK

This spec delivers `herdrify-hc` 0.1.0, a standalone async Rust client for herdr-core. It covers:
- every one of the 32 methods of protocol generation 1, including the `events.subscribe` and
  `terminal.attach` streams
- a sans-IO protocol module
- a public `Transport` port with two adapters, so any caller can drive or fake herdr-core
- tests against a fake transport and a throwaway live core, and a drift check against the
  installed `hc schema`

It adds no herdrify concept: no components, no grants.

All paths under `evidence/` are relative to this chain directory. `probe-*/` directories hold the
Rust probe sources, and `probe-*-output.txt` files hold their raw output. Labels such as `A5`,
`B3`, `C7`, `M5` and `E22` name the matching `== …` sections of `probe-plain`, `probe-framed`,
`probe-crates`, `probe-methods` and `probe-edges` respectively. Every live probe ran
against an isolated core (its own `HC_SESSION` and scratch XDG directories). The author's default
core was only read, for its pid.

## 1. Premises

**Versions.**
- Toolchain: rustc 1.94.1 / cargo 1.94.1, pinned by the repo's `rust-toolchain.toml`
  (`evidence/probe-trait-output.txt:2`).
- herdr-core: `hc` 0.1.21 (`evidence/probe-plain-output.txt:6-23`).
- Crates: tokio 1.54.1, serde 1.0.229, serde_json 1.0.154, thiserror 2.0.21, base64 0.23.1
  (`evidence/probe-deps-output.txt`, resolved-versions block).

**ADRs relied on.**
- ADR 0001: one crate, `herdrify-hc`, at layer L0, depending on no herdrify crate. Publishing goes
  SDKs first.
- ADR 0002: async only on tokio; no Cargo features; `Send` futures; a sans-IO protocol module; a
  public `Transport` moving one message at a time, with stdio and argv-command implementations;
  `Client<T: Transport>`; `hello` with generation 1; typed errors from `data.kind`. ADR 0002 left
  the type-generation choice to this design (§4.9).

**Upstream documentation.** herdr-core is open source (Apache-2.0). Its `schema/README.md` at tag
v0.1.21 documents what OpenRPC does not: the two connection modes, frames, attach streams and
events (`evidence/upstream-v0.1.21-schema-README.md`).
- **The schema matches the core.** `hc schema` output and `rpc.discover` are JSON-equal to each
  other and to upstream `schema/hc.openrpc.json` at v0.1.21 (`evidence/probe-plain-output.txt:522-525`).
- **The fixture.** That file is copied as `evidence/upstream-v0.1.21-schema-hc.openrpc.json`, and
  becomes the crate's test fixture (§9).

**The compatibility promise.** "The API only grows … a test fails on any change that would break a
client of it: a removed method, param or result field, a new required input, an input that
accepts less, or an answer that can carry a new kind of value or loses its type … A real break
needs a new protocol generation" (`evidence/upstream-v0.1.21-schema-README.md:14-19`). Two design
rules rest on it:
- the schema's own `enum`s (`Children`, `IdleOrWorking`) are closed Rust enums. A new value there
  would be a new kind of value.
- input newtypes never refuse a value the core accepts, because a newer core may accept more
  (§4.7)

A plain `string` field taking a new word is not a new kind of value, so it is not covered. Fields
with a documented open word list become open enums with an `Other(String)` variant (§4.8).
Notifications are not part of the OpenRPC document either, so event types stay open too (§4.6).

**Live tests are isolated.** hc's own guidance: "Never `hc server stop` or restart the user's core
unless asked … Tests need their own `HC_SESSION` and scratch XDG paths"
(`evidence/hc-0.1.21-skill.txt:340-341`). `XDG_STATE_HOME`, `XDG_CONFIG_HOME` and
`XDG_RUNTIME_DIR` move the state, config and socket directories
(`evidence/upstream-v0.1.21-docs-security.md:59-77`).

## 2. Architecture

The author approved this section on its own before the spec was written.

It applies clean architecture with ports and adapters inside the crate. The layers, from inside
out:
- `protocol` is the domain: pure and sans-IO
- `port` is the seam the application owns
- `client` is the application: the use cases
- `adapter` is the I/O

```
crates/herdrify-hc/
├─ Cargo.toml  README.md
├─ src/
│  ├─ lib.rs                    table of contents: //! docs, pub mod, pub use
│  ├─ protocol/                 DOMAIN (serde, serde_json, thiserror, base64; no tokio, no I/O)
│  │  ├─ mod.rs
│  │  ├─ frame.rs               Frame, FrameKind, Channel; header encode/decode; 16 MiB cap
│  │  ├─ frame_decoder.rs       bytes → Frames across partial reads
│  │  ├─ frame_error.rs         FrameError
│  │  ├─ line.rs                Line; LineDecoder (hands leftover bytes to the FrameDecoder)
│  │  ├─ ndjson.rs              stream bytes → complete JSON lines
│  │  ├─ envelope.rs            Request, Response, Notification, RequestId, Incoming
│  │  ├─ hello.rs               HelloParams, HelloResult, Generation, CoreVersion, floor check
│  │  ├─ rpc_error.rs           RpcError, ErrorKind
│  │  ├─ protocol_error.rs      ProtocolError
│  │  ├─ empty_value_error.rs   EmptyValueError
│  │  ├─ identifiers.rs         the identifier newtypes of §4.7
│  │  ├─ notification.rs        Notification, Event, EventKind
│  │  ├─ method.rs              trait Method
│  │  ├─ methods/               one file per group: server, terminal, templates, machine, file,
│  │  │                         doc, events, rpc: the 32 Params types and their Method impls
│  │  └─ types/                 the 36 component schemas, one file per concept
│  ├─ port/                     PORT (depends on protocol only)
│  │  ├─ mod.rs
│  │  ├─ transport.rs           Transport, FrameSink, FrameSource (pure trait definitions)
│  │  └─ transport_error.rs     TransportError
│  ├─ client/                   APPLICATION (depends on port + protocol; tokio rt + sync)
│  │  ├─ mod.rs
│  │  ├─ client.rs              Client<T>: connect, the 32 typed methods, call_raw
│  │  ├─ writer.rs              the spawned writer task: owns the FrameSink, takes write commands
│  │  ├─ router.rs              the spawned reader task: owns the FrameSource, routes frames
│  │  ├─ connection_end.rs      ConnectionEnd, ConnectionEndKind
│  │  ├─ pending.rs             request-id allocator and pending-call table
│  │  ├─ subscription.rs        Subscription (events use case)
│  │  ├─ attach.rs              AttachStream (attach use case)
│  │  └─ client_error.rs        Error
│  └─ adapter/                  ADAPTERS (depend on port + protocol; tokio io-util + process)
│     ├─ mod.rs
│     ├─ stream.rs              StreamTransport<R, W> and its two halves
│     └─ process.rs             ProcessTransport and its builder
└─ tests/
   ├─ fixtures/hc-0.1.21.openrpc.json
   ├─ support/                  fake_transport.rs, schema_instance.rs
   ├─ schema_coverage.rs
   ├─ client_fake.rs
   ├─ adapter_stream.rs
   └─ live.rs
```

**Why each module exists.**

| Module | Load-bearing reason |
|---|---|
| `protocol` | ADR 0002 requires a sans-IO protocol module. Keeping tokio out makes every codec rule testable as bytes in, values out (M-IMPL-IO) |
| `port` | ADR 0002 requires a public transport seam that callers can fake. The application owns it, and adapters implement it |
| `client` | The only owner of concurrency state: request ids, channels, credit, the spawned task |
| `adapter` | The only code that touches pipes and processes (M-MOCKABLE-SYSCALLS) |

**Dependency rule.** Every dependency points inward.

| Layer | May depend on | Must not depend on | External crates |
|---|---|---|---|
| `protocol` | nothing internal | `port`, `client`, `adapter` | serde, serde_json, thiserror, base64 |
| `port` | `protocol` | `client`, `adapter` | none |
| `client` | `port`, `protocol` | `adapter` | tokio (`rt`, `sync`) |
| `adapter` | `port`, `protocol` | `client` | tokio (`io-util`, `process`) |

The caller is the composition root. It builds an adapter and passes it to `Client::connect`.
`Client` has no constructor that names an adapter.

**Enforcement.** Modules in one crate are not separated by the compiler, so the reviewer runs
these checks, and each must print nothing:
```sh
grep -rn 'crate::adapter' crates/herdrify-hc/src/protocol crates/herdrify-hc/src/port crates/herdrify-hc/src/client
grep -rn 'crate::client'  crates/herdrify-hc/src/protocol crates/herdrify-hc/src/port crates/herdrify-hc/src/adapter
grep -rn 'crate::port'    crates/herdrify-hc/src/protocol
grep -rn 'tokio'          crates/herdrify-hc/src/protocol crates/herdrify-hc/src/port
```

**Guideline rules applied** (`claudestacks-guideline-rust` 0.1.5, `references/`):
- `mod-rs-export-only.md`:
  - `lib.rs` and every `mod.rs` hold only `//!` docs, `mod` and `pub use`
  - every module opens with the four-question `//!` block
  - the public surface is flattened with `#[doc(inline)]` re-exports
  - the crate root re-exports `Client`, `Error`, `Transport`, `FrameSink`, `FrameSource`,
    `TransportError`, `StreamTransport` and `ProcessTransport`
- `modularity.md`: one concept per file, and no `*Manager`, `*Util` or `*Helper`. `Error`,
  `TransportError`, `RpcError`, `ProtocolError` and `FrameError` carry different invariants, each
  documented in its own doc comment. One shared `EmptyValueError` replaces field-identical
  per-newtype error types (Rule 2).
- `static-dispatch.md`:
  - `Client<T: Transport>` is generic
  - no `Box<dyn …>` or `Arc<dyn …>`
  - `Client`'s `Arc` inner exists for cheap `Clone` (M-SERVICES-CLONE), not dispatch
- `strong-types.md`:
  - the identifier newtypes of §4.7
  - required params are constructor arguments
  - optional flags are argument-less setters, so there are no `bool` parameters (§4.8)
  - durations are `std::time::Duration`
- `unit-test-mandate.md`:
  - every logic-bearing file has a colocated `#[cfg(test)] mod tests`
  - `port/transport.rs` is a pure trait file (exemption #3)
  - `lib.rs` and every `mod.rs` are export-only (exemption #1)
  - each exemption is cited in its file's `//!`
- `doc-comment-discipline.md`: nothing under `crates/herdrify-hc/` names ADRs, specs, plans,
  milestones or chain paths.
- `workspace.md`: dependencies come from `[workspace.dependencies]`, and the member inherits
  metadata and lints.
- `microsoft-guidelines.md`:
  - M-TYPES-SEND: public types are `Send`
  - M-OOBE: the default `cargo test` needs no `hc` (§9.5)
  - M-ESSENTIAL-FN-INHERENT: the 32 methods are inherent on `Client`
  - M-DONT-LEAK-TYPES: no tokio type in the public API except the `AsyncRead`/`AsyncWrite`
    bounds of `StreamTransport`. `serde_json::Value` is part of the contract, for raw access
    (`call_raw`, `rpc_discover`) and opaque documents (`doc.set`). No base64 type is exposed.
- The Rust Definition of Done in `SKILL.md`: `cargo make dod` with zero warnings.

## 3. Wire facts the design rests on

| # | Fact | Evidence |
|---|---|---|
| W1 | A connection starts in plain mode: JSON-RPC 2.0, one compact message per line, `\n` only | README:28; `probe-plain-output.txt:583-590` |
| W2 | In plain mode, calls on one connection run one at a time: `terminal.list` waited ~3 s behind `terminal.wait` | `probe-plain-output.txt:405-409` |
| W3 | In plain mode, after `events.subscribe` the connection silently ignores further requests | `probe-plain-output.txt:444-448` |
| W4 | `hello {"generation":1,"framed":true}` gets its reply as a plain `\n`-terminated line. Frames start right after it, in the same byte stream | `probe-framed-output.txt:36-42` |
| W5 | A frame is `[channel u32 LE][kind u8][len u32 LE][payload]`, payload ≤ 16 MiB. Kinds: 0 DATA, 1 CREDIT, 2 CLOSE, 3 RESIZE, 4 FOCUS, 5 FACTS, 6 AUTOMATIC_INPUT | README:47-58; header bytes `probe-framed-output.txt:38-39` |
| W6 | Channel 0 carries JSON-RPC both ways, and calls on it run side by side: a later `terminal.list` was answered before an earlier `terminal.wait`. Channel 0 uses no CREDIT: 415110 bytes of replies (six concurrent `rpc.discover`) arrived with no CREDIT either way | README:60; `probe-framed-output.txt:118-122`; `probe-review-output.txt` (`== R1`) |
| W7 | Core channel-0 payloads carry no trailing `\n`. Client payloads are accepted with or without one | `probe-framed-output.txt:38-42` |
| W8 | A plain JSON line written after the framed switch makes the core close the connection | `probe-framed-output.txt:460-464` |
| W9 | The core silently drops an unknown frame kind and DATA on an unopened channel. Non-JSON DATA on channel 0 gets `-32700 parse_error` with `id:null` | `probe-framed-output.txt:434-437`, `:447-450`, `:422-424` |
| W10 | `events.subscribe` (framed) returns a `channel`. Its DATA is a newline-delimited JSON byte stream, not aligned to frames: lines over 16384 bytes are split, and several lines can share one frame | `probe-framed-output.txt:133-143`, `:232-286` |
| W11 | Stream credit is enforced. With no CREDIT sent, event delivery stopped at exactly 262144 bytes and resumed after CREDIT | README:103; `probe-framed-output.txt:255-258` |
| W12 | Notifications: `event {seq, type, terminal?, data}`, `heartbeat {seq}`, `missed {seq, terminals}`. After `missed`, drop events whose `seq` is not above it. `seq` restarts with each core process | README:117-121, 174-177 |
| W13 | Event types observed or documented: created, lost, restarted, keep, finished, removed, status, turn, title, labels, size, foreground, doc | README:129-140, 198; `probe-plain-output.txt:422-440`; `probe-methods-output.txt:388-391`, `:423-427`, `:449` |
| W14 | Sending CLOSE on an event channel gets an empty CLOSE back, and no more events on it. Channel 0 keeps working, and channel numbers are not reused | `probe-methods-output.txt:326-366` |
| W15 | `terminal.attach` returns `{automatic_input, channel, resize_notifications, terminal}`. Viewer CLOSE gets an empty CLOSE. On program exit the core sends CLOSE with `{"exit_code":0,"finished_ms":…,"reason":"exited","signal":null}` | `probe-framed-output.txt:303-304`, `:370-371`, `:384-385` |
| W16 | Attach in plain mode fails: `needs_framed`, "attach needs a framed connection" | `probe-framed-output.txt:397`, `:403` |
| W17 | Kind 6 may be sent only when `hello.features` contains `automatic_input` and the attach reply says `automatic_input: true`. Otherwise send DATA | README:78-81 |
| W18 | `hello` is optional: a request before it is answered. A second `hello` is accepted. On a framed connection, `framed:false` replies `false` but the connection stays framed | `probe-plain-output.txt:166-168`, `:214-219`; `probe-framed-output.txt:89-98` |
| W19 | `hello` with generation 2 or 0 fails: `-32000 unsupported_generation`, message "protocol generation 2 not supported", hint "this core speaks generation 1". The reply is a plain line | `probe-plain-output.txt:212`, `:225`; `probe-framed-output.txt:102-103` |
| W20 | Error replies are `{code, message, data:{kind, hint}}`: `-32000` with a stable kind, `-32602 invalid_params`, `-32601 unknown_method`, `-32700 parse_error` | README:36-40; `probe-plain-output.txt:247-295` |
| W21 | Closing `hc connect`'s stdin makes it exit 0, after it sends any in-flight reply | `probe-plain-output.txt:603-623` |
| W22 | `server.restart` and `server.stop` reply `{"stopped":true}`, then the connection ends and `hc connect` exits 0 | `probe-methods-output.txt:629-664` |
| W23 | `hc connect` starts a stopped isolated core by itself | `probe-plain-output.txt:109-122` |
| W24 | A socket path over 103 bytes is refused. A `/var/folders` `mktemp` directory produced 110 bytes, and `/tmp/…` works | `probe-plain-output.txt:26-101` |
| W25 | Over 20 live results, no key is missing from or extra to the schema | `probe-plain-output.txt:557-563` |

## 4. Protocol (domain)

### 4.1 Frames

- `Frame { channel: Channel, kind: FrameKind, payload: Vec<u8> }`.
- `FrameKind` is `Data`, `Credit`, `Close`, `Resize`, `Focus`, `Facts`, `AutomaticInput`, or
  `Unknown(u8)` for any other byte (W5, W9).
- `Channel(u32)`. `Channel::RPC` is channel 0.
- Encoding writes the 9-byte header and the payload. A payload over 16777216 bytes is refused
  before encoding with `FrameError::TooLarge` (W5).
- Typed constructors and readers for control payloads:

  | Kind | Payload | Size |
  |---|---|---|
  | `credit(channel, bytes: u32)` | u32 LE | 4 |
  | `resize(channel, cols, rows)` | u16 LE cols, u16 LE rows | 4 |
  | `focus(channel, Focus)` | `1` gained, `0` lost | 1 |

  Reading a control frame whose payload length differs is `FrameError::BadControlPayload`.
- `FrameDecoder` accepts bytes in any chunking and yields complete frames. A header whose `len`
  exceeds 16777216 is `FrameError::TooLarge`, and the decoder is unusable afterwards.

### 4.2 Lines

- `Line` is one JSON text without the terminating `\n`.
- `LineDecoder` yields complete lines from bytes in any chunking. After a line it can hand its
  unconsumed bytes to a `FrameDecoder`, so frames that arrive in the same read as the hello reply
  are kept (W4). This is new code: a unit test (§9.1) and `tests/adapter_stream.rs` (§9.4) check
  it.

### 4.3 NDJSON stream

`ndjson` turns a stream channel's DATA bytes into complete lines (W10):
- it carries a partial line across frames
- it yields several lines from one frame
- a pending partial line over 16777216 bytes is `ProtocolError::EventLineTooLong`

### 4.4 JSON-RPC envelope

- **Request** `{jsonrpc:"2.0", id, method, params}`. The client always sends an object `params`.
- **RequestId** is a `u64`, allocated from 1 upward per connection. The handshake uses 0.
- **Incoming** classification of a channel-0 payload:
  - `Response { id, Ok(Value) | Err(RpcError) }`
  - `Notification { method, params }`
  - a payload that is valid JSON but neither is `ProtocolError::NotJsonRpc`
  - invalid JSON is `ProtocolError::InvalidJson`
- Classification uses `serde_json::Value` for `result` and `params`. Typed decoding happens
  afterwards, because `RawValue` inside an untagged enum never matches
  (`evidence/probe-crates-output.txt:1448-1680`, C7f).

### 4.5 Hello and version

- `HelloParams` has all five schema params: `calls_you`, `core`, `features`, `framed`,
  `generation` (`evidence/probe-schema-params-output.txt`, first five lines).
- The client sends `{"generation":1,"framed":true}`.
- `HelloResult` has every schema property: required `core`, `features`, `framed`, `generation`,
  `methods`, `version`; optional `keeps_running_after_logout`, `login_session`,
  `logout_linger_user` (`evidence/probe-schema-table-output.txt`, `S HelloResult`).
- `Generation(u32)`, with `Generation::ONE`.
- **`CoreVersion`** follows the SemVer 2.0.0 grammar, with no surrounding whitespace:
  - `MAJOR.MINOR.PATCH`, each a decimal that fits `u64` with no leading zero (except `0` itself)
  - optionally `-<pre-release>` and/or `+<build>`, each a non-empty dot-separated list of
    non-empty `[0-9A-Za-z-]` identifiers
  - Ordering follows semver precedence: build metadata is ignored, and a pre-release sorts below
    its release.
  - The floor is `CoreVersion::MINIMUM` = 0.1.21.

| `version` | Outcome |
|---|---|
| `0.1.21`, `0.1.22`, `0.2.0`, `1.0.0`, `0.1.21+abc` | accepted |
| `0.1.20`, `0.0.9`, `0.1.21-rc.1` | `Error::UnsupportedVersion` |
| `v0.1.21`, `0.1`, `""`, `0.1.x`, `0.1.21.4`, `0.01.21`, `0.1.21-`, `0.1.21+`, ` 0.1.21`, `18446744073709551616.0.0` | `Error::InvalidVersion` |

The live core's hello reply carries `"version":"0.1.21"` (`evidence/probe-plain-output.txt:119`).

### 4.6 Notifications and events

- `Notification`:
  - `Event(Event)`
  - `Heartbeat { seq }`
  - `Missed { seq, terminals: Vec<Terminal> }`
  - `Other { method, params }` for an unknown method name
- `Event { seq: Seq, terminal: Option<TerminalAddress>, machine: Option<MachineName>, kind: EventKind }`.
- `EventKind` has one variant per W13 type, with the `data` shapes of README:129-140 and 198, as
  observed:
  - `status`: `{records, reported, status}`, where `status` may be null
  - `turn`: `{state, turn}`
  - `title`: `{title}`
  - (`probe-methods-output.txt:388-391`, `:423-427`, `:449`)
- Any other `type` is `EventKind::Unknown { kind: String, data: Value }`. Upstream adds event types
  without a new generation (`foreground` and `doc` exist outside the OpenRPC document, W13), so
  events stay open.
- `Seq(u64)`.

### 4.7 Identifiers

| Newtype | Used for (schema params and fields) |
|---|---|
| `TerminalRef` | `terminal`, `parent`, `children_of` inputs: `t7`, a name, or `machine/t7` |
| `TerminalAddress` | the `address` field of records, and event `terminal` |
| `TerminalName` | `terminal.run` `name`, the record `name` |
| `MachineName` | `on`, `machine`, `new_name`, `machine.add` `name`, event `machine` |
| `SessionName` | `machine.add` `session`, `ProcessTransport::session` |
| `DocName` | `doc.get` and `doc.set` `name` |
| `LabelKey`, `LabelValue` | label maps |
| `CallerId` | every `caller` param |
| `RemotePath` | `path` and `cwd` params, which refer to the core's machine, not the local one |
| `FileName` | `terminal.put` `name` |
| `KeyName` | entries of `terminal.keys` `keys` |
| `MethodName` | `Client::call_raw` |
| `DocVersion(u64)` | `doc.set` `version`, and versions in results |

- **Validation mirrors the core.** A newtype refuses the empty string exactly when the core
  refuses it, and refuses nothing else (§1, the compatibility promise).

  | Newtype | `""` | Core evidence |
  |---|---|---|
  | `TerminalRef` | refused | `invalid_params` "bad terminal address" (`probe-edges-output.txt:29`; `parent`, `children_of`: `probe-review-output.txt`, `== R3`) |
  | `TerminalName` | refused | `invalid_params` "bad name" (`probe-edges-output.txt:50`) |
  | `DocName` | refused | `invalid_params` "bad name" (`probe-edges-output.txt:35`) |
  | `LabelKey` | refused | `invalid_params` "bad label key" (`probe-edges-output.txt:44`) |
  | `MachineName` | refused | `machine_unknown` for `machine`, `on` (`probe-review-output.txt`, `== R3`) |
  | `SessionName` | refused | `invalid_params` "bad name" (`probe-review-output.txt`, `== R3`) |
  | `KeyName` | refused | `invalid_params` "unknown key" (`probe-review-output.txt`, `== R3`) |
  | `MethodName` | refused | `unknown_method` (`probe-review-output.txt`, `== R3`) |
  | `CallerId` | accepted | `terminal.get` with `caller:""` succeeds (`probe-review-output.txt`, `== R3`) |
  | `FileName` | accepted | `terminal.put` with `name:""` stores the file as `file` (`probe-review-output.txt`, `== R3`) |
  | `RemotePath` | accepted | `cwd:""` runs in the default directory (`probe-review-output.txt`, `== R3`). `path:""` resolves to that directory (`probe-edges-output.txt:62`) |
  | env names in `Environment` | accepted | `env:{"":"x"}` succeeds (`probe-review-output.txt`, `== R3`) |
  | `TerminalAddress` | accepted | output only: produced by the core, never refused when read |
  | `LabelValue` | refused | structural: `""` is how the core removes a label (`probe-methods-output.txt:102-109`), so removal is `LabelChange::Remove` instead |
  | `DocVersion(u64)` | n/a | integer |

- **Values the core refuses.** Non-empty values the core refuses, such as `a b` as a name and
  `a=b` as a label key (`probe-edges-output.txt:38`, `:47`), pass through. They surface as
  `Error::Rpc` with kind `invalid_params`. A whitespace-only terminal ref passes through, and the
  core answers `not_found` (`:32`).
- **Shape of the refusing newtypes.** They have `new` returning `Result<Self, EmptyValueError>`,
  `TryFrom<&str>`, `FromStr`, `as_str`, `Display` and `#[serde(transparent)]`. Deserialization goes
  through `new` (`strong-types.md`).
- **Shape of the accepting newtypes.** They have an infallible `new` and `From<String>`, plus the
  same accessors. They deviate from `strong-types.md`'s "fallible construction", because any
  check would refuse an input the core accepts. Each one's doc comment says the core decides.

### 4.8 Methods and parameters

- **The `Method` trait.** `trait Method { const NAME: &'static str; type Params: Serialize; type Output: DeserializeOwned; }`.
- **Params mirror the schema.** One `Params` type per method has exactly the schema's params
  (`evidence/probe-schema-params-output.txt`).
- **Constructor and setters.** Required params are `new()` arguments. Optional params are
  setters.
- **Field mapping rules.**

  | Schema | Rust |
  |---|---|
  | identifier strings (§4.7) | the newtype |
  | `integer<uint16>` with schema `minimum: 1` (`InitialSize` `cols`/`rows`; the core refuses 0, E22) | `NonZeroU16` |
  | `integer<uint16>` with `minimum: 0` (`terminal.attach` `cols`/`rows`, RESIZE). The core accepts an attach at 0×0 and ignores a RESIZE to 0×0 (`probe-review-output.txt`, `== R2`) | `u16` |
  | `integer<uint8>` (`Status.progress`, nullable) | `Option<u8>` |
  | array with `minItems` = `maxItems` = 2 (`Display.cell`, `uint32` items) | `[u32; 2]` |
  | `integer<uint32>` | `u32` |
  | `integer<uint64>`, `integer<uint>` | `u64` |
  | `integer<int32>` | `i32` |
  | `number<double>` timeouts (seconds) | `std::time::Duration`, serialized as f64 seconds |
  | `object{str:string}` env | `Environment` (a map with non-empty names) |
  | `object{str:string}` labels | `Labels`, a `LabelKey → LabelValue` map |
  | base64 strings (`terminal.put data`, `file.write data`, `machine.exec input`, `ExecResult stdout/stderr`) | `Vec<u8>` / `&[u8]`, base64-encoded and decoded by the SDK with the standard padded alphabet (`evidence/probe-base64-output.txt:36-41`) |
  | `any` (`doc.set value`) | `serde_json::Value` (null deletes, `probe-edges-output.txt:75-77`) |
  | the schema's `enum`s (`Children`, `IdleOrWorking`) | closed Rust enums (§1) |
  | plain strings with a documented open word list | the open enums below, each with `Other(String)` |
  | every other plain string | `String` |
  | optional boolean, default false | an argument-less setter that turns it on (`TerminalRun::new(cmd).keep().rm()`) |
  | required boolean (`terminal.keep keep`) | a two-variant enum: `Keep::{On, Off}` |

- **Open enums.** Each serializes back to the exact word it read, including `Other`:

  | Enum | Fields | Known variants | Evidence |
  |---|---|---|---|
  | `TerminalState` | `Terminal.state`, `RunResult.state` | `live`, `finished` | README:133; `probe-plain-output.txt:404`; `probe-methods-output.txt:170-177` |
  | `EndReason` | `Terminal.reason`, `lost`/`finished` event `reason`, the attach CLOSE `reason` | `exited`, `killed`, `server_stop`, `host_shutdown`, `holder_lost`, `stopped_by_restart`, `signaled`, `spawn_pending` | README:111, 153, 165, 171-173 |
  | `WaitReason` | `WaitResult.reason`, the `wait` object inside `RunResult`/`SendResult` | `exit`, `timeout`, `message` | `probe-plain-output.txt:407`; `probe-methods-output.txt:121-133`; fixture `upstream-v0.1.21-schema-hc.openrpc.json:2664` |
  | `StatusWord` | `Status.state`, the `turn` event `state` | `idle`, `working`, `done`, `blocked`, `error` | README:217-218; `probe-methods-output.txt:423-427`, `:449` |

- **Rules where the core refuses a combination.** These make the refused combination impossible
  to express:

  | Method | Rust shape | Evidence |
  |---|---|---|
  | `terminal.run` | `TerminalRun::new(Command)`, `Command::{Shell, Argv(Argv), Line(String)}`. `Shell` omits both, and the core starts its shell. `Argv` is non-empty | E2, E4 (`probe-edges-output.txt:9-17`); E3 (both refused, `:14`) |
  | `machine.exec` | `MachineExec::new(ExecCommand::{Argv(Argv), Line(String)})` | E6, E7 (`:23`, `:26`) |
  | `terminal.wait` | `TerminalWait::new(WaitTarget::{Terminal(TerminalRef), Children(Children), ChildrenOf(TerminalRef, Children)})` | E23 (`:74`); param descriptions (`probe-schema-params-output.txt`, `terminal.wait`) |
  | `terminal.label` | `TerminalLabel::new(terminal, LabelChanges)`, entries `LabelChange::{Set(LabelValue), Remove}`. `Remove` serializes as `""` | `probe-methods-output.txt:90-113` |
  | `machine.add` | `MachineAdd::new(MachineTarget::{Ssh(String), Via{name: MachineName, command: String}})`. `via` needs a name | `probe-methods-output.txt:300-312` |

- **Allowed empties.** `Argv::new` with an empty list is `EmptyValueError { what: "argv" }`.
  `Line` may be empty: the core runs `sh -c ""` (`probe-edges-output.txt:18-20`). Empty
  `terminal.send` `text` and empty `terminal.keys` `keys` are allowed (`:54-59`).

- **The 32 methods.** `Client` method, Params type, required constructor arguments, and output.
  Source: `evidence/probe-schema-table-output.txt`, `M` lines.

  | Wire method | `Client` method | Params | `new(…)` | Output |
  |---|---|---|---|---|
  | `hello` | sent by `connect`; `Client::hello()` returns the result | `HelloParams` | (all optional) | `HelloResult` |
  | `server.status` | `server_status` | `ServerStatus` | | `StatusResult` |
  | `server.stop` | `server_stop` | `ServerStop` | | `StopResult` |
  | `server.restart` | `server_restart` | `ServerRestart` | | `StopResult` |
  | `terminal.run` | `terminal_run` | `TerminalRun` | `Command` | `RunResult` |
  | `templates.list` | `templates_list` | `TemplatesList` | | `TemplatesResult` |
  | `terminal.list` | `terminal_list` | `TerminalList` | | `ListResult` |
  | `terminal.get` | `terminal_get` | `TerminalGet` | `TerminalRef` | `Terminal` |
  | `terminal.kill` | `terminal_kill` | `TerminalKill` | `TerminalRef` | `Terminal` |
  | `terminal.rm` | `terminal_rm` | `TerminalRm` | `TerminalRef` | `RemoveResult` |
  | `terminal.keep` | `terminal_keep` | `TerminalKeep` | `TerminalRef, Keep` | `Terminal` |
  | `terminal.label` | `terminal_label` | `TerminalLabel` | `TerminalRef, LabelChanges` | `Terminal` |
  | `terminal.read` | `terminal_read` | `TerminalRead` | `TerminalRef` | `ReadResult` |
  | `terminal.attach` | `terminal_attach` | `TerminalAttach` | `TerminalRef, u16 cols, u16 rows` | `AttachStream` (§6.4) |
  | `terminal.send` | `terminal_send` | `TerminalSend` | `TerminalRef, String text` | `SendResult` |
  | `terminal.wait` | `terminal_wait` | `TerminalWait` | `WaitTarget` | `WaitResult` |
  | `terminal.keys` | `terminal_keys` | `TerminalKeys` | `TerminalRef, Vec<KeyName>` | `KeysResult` |
  | `terminal.interrupt` | `terminal_interrupt` | `TerminalInterrupt` | `TerminalRef` | `InterruptResult` |
  | `terminal.put` | `terminal_put` | `TerminalPut` | `TerminalRef, FileName, Vec<u8>` | `PutResult` |
  | `machine.exec` | `machine_exec` | `MachineExec` | `ExecCommand` | `ExecResult` |
  | `file.read` | `file_read` | `FileRead` | `RemotePath` | `FileReadResult` |
  | `file.write` | `file_write` | `FileWrite` | `RemotePath, Vec<u8>` | `FileResult` |
  | `file.edit` | `file_edit` | `FileEdit` | `RemotePath, Vec<Edit>` | `FileResult` |
  | `doc.get` | `doc_get` | `DocGet` | `DocName` | `DocGetResult` |
  | `doc.set` | `doc_set` | `DocSet` | `DocName, Value, DocVersion` | `DocSetResult` |
  | `events.subscribe` | `events_subscribe` | `EventsSubscribe` | | `Subscription` (§6.3) |
  | `machine.add` | `machine_add` | `MachineAdd` | `MachineTarget` | `Machine` |
  | `machine.list` | `machine_list` | `MachineList` | | `MachineListResult` |
  | `machine.connect` | `machine_connect` | `MachineConnect` | `MachineName` | `MachineView` |
  | `machine.rename` | `machine_rename` | `MachineRename` | `MachineName, MachineName` | `Machine` |
  | `machine.rm` | `machine_rm` | `MachineRm` | `MachineName` | `RemoveResult` |
  | `rpc.discover` | `rpc_discover` | `RpcDiscover` | | `serde_json::Value` |

- **What each method exposes.** Each typed method is `async fn(&self, params) -> Result<Output, Error>`.
  `terminal_attach` returns the stream, which also exposes the `AttachResult` fields.
  `events_subscribe` returns the `Subscription`, which also exposes the `SubscribeResult` fields.

### 4.9 Types (the 36 component schemas)

- **Written by hand.** The author chose this. Generators were measured and rejected:
  - typify 0.10.1 cannot build on 1.94, because typespace needs rustc 1.97
    (`evidence/probe-crates-output.txt:372-399`)
  - typify 0.8.0 output fails the gate with 95 warnings (78 + 14 + 2 + 1), and generates no
    method params (`:752-760`, `:849-875`)
  - `herdr-protocol` 0.2.3 needs rustc 1.100 (`:917-1001`)
- **Fields.** Each type has exactly the schema's properties
  (`evidence/probe-schema-table-output.txt`, `S` lines), mapped by §4.8's rules:
  - required properties are plain fields
  - optional and nullable properties are `Option<T>`, where null and absent both read as `None`
    (`evidence/probe-crates-output.txt:1448-1680`, C7d)
- **Unknown fields.** Unknown fields in replies are ignored, which is serde's default
  (`probe-crates-output.txt`, C7a).
- **Names.** Each Rust type takes its schema name, with one exception: the schema's `Display`
  becomes `DisplayFacts`, so it never shadows `std::fmt::Display`. Its doc comment names the
  schema type. `Terminal` and `RunResult` stay two types, because `RunResult` adds `output` and
  `wait` (schema `S` lines). The doc comment of each says so (`modularity.md` Rule 2).
- **Untyped access.** `Client::call_raw(MethodName, Value) -> Result<Value, Error>` reaches any
  method, including ones newer than the SDK, and any field a typed result does not name. Typed
  structs carry no catch-all map.

### 4.10 Errors from the core

- `RpcError { code: i64, kind: ErrorKind, message: String, hint: Option<String>, data: Value }`.
- `ErrorKind` is one variant per stable kind, plus `Other(String)`:
  - the kinds in the schema's method `errors` lists (`evidence/probe-schema-table-output.txt`, `M`
    lines): `already_added`, `blocked`, `cancelled`, `config`, `conflict`, `edit_failed`,
    `finished`, `hc_missing`, `incompatible`, `internal`, `invalid_params`, `invalid_template`,
    `io`, `live`, `machine_unknown`, `name_taken`, `needs_framed`, `needs_target`,
    `needs_update`, `not_found`, `not_text`, `spawn_failed`, `status_mismatch`, `unavailable`,
    `unreachable`, `unsupported_generation`
  - the two observed live but not listed: `unknown_method` and `parse_error` (W20)
- `ErrorKind::as_str` returns the wire string, and `Display` prints it.

## 5. Port

```rust
pub trait Transport: Send + 'static {
    type Sink: FrameSink;
    type Source: FrameSource;
    fn send_line(&mut self, line: Line) -> impl Future<Output = Result<(), TransportError>> + Send;
    fn recv_line(&mut self) -> impl Future<Output = Result<Option<Line>, TransportError>> + Send;
    fn into_framed(self) -> (Self::Sink, Self::Source);
}
pub trait FrameSink: Send + 'static {
    fn send(&mut self, frame: Frame) -> impl Future<Output = Result<(), TransportError>> + Send;
    fn close(&mut self) -> impl Future<Output = Result<(), TransportError>> + Send;
}
pub trait FrameSource: Send + 'static {
    fn recv(&mut self) -> impl Future<Output = Result<Option<Frame>, TransportError>> + Send;
}
```

- **The type state.** A transport moves lines until `into_framed` consumes it. After that, only
  frames can be written, so W8 cannot happen. `Ok(None)` means the peer closed.
- **Proven.** The load-bearing part compiles on 1.94.1: a trait with associated sink and source
  types whose methods return `impl Future + Send` (RPITIT), with the halves moved into
  `tokio::spawn` (`evidence/probe-trait-output.txt:1-6`). The probe's error type was `io::Error`
  and its sink had no `close`. Neither affects that property.
- **Other modes.** Plain-mode callers can use `send_line`/`recv_line` for the whole connection.

## 6. Client (application)

### 6.1 Connect

`Client::connect(transport).await -> Result<Client<T>, Error>` runs these steps in order:
1. `send_line` the hello request (id 0, `{"generation":1,"framed":true}`).
2. `recv_line`. On `None`, return `Error::Closed`.
3. Classify the line (§4.4):
   - invalid JSON → `Error::Protocol(InvalidJson)`
   - a notification, or a response whose id is not 0 → `Error::Protocol(UnexpectedHelloReply)`
4. If the reply is an error with kind `unsupported_generation`, return
   `Error::UnsupportedGeneration { message }` (W19). Any other error reply is `Error::Rpc` with
   method `hello`.
5. Decode `HelloResult`. On failure, return `Error::Decode` with method `hello`.
6. If `framed != true` or `generation != 1`, return `Error::Protocol(HelloMismatch)`.
7. Parse `version`. An unparsable version is `Error::InvalidVersion`, and one below 0.1.21 is
   `Error::UnsupportedVersion` (§4.5).
8. `into_framed()`, spawn the writer and the router (§6.5), and return the `Client`.

On any failure the transport is dropped. `Client<T>` is `Clone` through an internal `Arc` and
`Send + Sync`. `Client::hello()` returns the `HelloResult`.

### 6.2 Calls

- **Sequence:**
  1. allocate an id
  2. register a oneshot in the pending table
  3. serialize the request
  4. a payload over 16777216 bytes is `Error::PayloadTooLarge`, returned before anything is sent
  5. hand the DATA frame for channel 0 to the writer (§6.5), and await its write acknowledgement
  6. await the oneshot
- **Write failure.** If the writer reports a failed write, the call returns
  `Error::Transport(TransportError::Io(..))` and its pending entry is removed. The connection
  ends (§6.5).
- **Concurrency.** Calls run concurrently (W6).
- **Results.** An `Ok(Value)` decodes into `M::Output` with `serde_json::from_value`. A decode
  failure is `Error::Decode`. An `Err(RpcError)` is `Error::Rpc { method, error }`.
- **Dropping.** Dropping a call future removes its pending entry. A reply that arrives later is
  discarded. There are no SDK timeouts: `terminal.wait` runs up to 180 s by default
  (`probe-schema-params-output.txt`, `terminal.wait timeout`).
- **`call_raw`** follows the same path, without typed decoding.

### 6.3 Events: `Subscription`

- **Fields.** `events_subscribe(params)` returns `Subscription { seq, terminals, channel }` from
  the reply (W10).
- **A missing channel.** A reply without `channel` (the schema makes it optional for plain mode)
  is `Error::Protocol(MissingChannel)` with method `events.subscribe`.
- **Delivery.** `Subscription::next().await -> Option<Result<Notification, Error>>`:
  1. the router forwards the channel's DATA bytes
  2. `ndjson` assembles lines (§4.3)
  3. each line decodes as a `Notification`
- **Bad items do not end the stream.**
  - A line that is not JSON yields `Err(Error::Protocol(InvalidJson))`.
  - An event whose `data` does not match its known `type` yields
    `Err(Error::Protocol(InvalidEvent { kind }))`.
  - In both cases the next line is read as usual.
- **Credit.** After the caller takes the notifications assembled from N bytes, the subscription
  sends CREDIT N through the writer, so delivery never stalls at 256 KiB (W11). Unconsumed data is
  bounded by the core's credit window.
- **Stale events.** After `Missed { seq }`, events with `seq ≤` that value are dropped before
  delivery (W12).
- **Closing.** `close().await`, or dropping the subscription, sends CLOSE through the writer
  (§6.5). The core's empty CLOSE ends it (W14).
- **Endings.**
  - Any CLOSE from the core, empty or not, ends the stream with `None`.
  - `EventLineTooLong` yields that `Err`, then `None`.
  - A connection end yields `Err(Error::Closed)`, then `None`.

### 6.4 Attach: `AttachStream`

- **Fields.** `terminal_attach(params)` returns
  `AttachStream { terminal, channel, automatic_input, resize_notifications }` (W15).
- **Events.** `next().await -> Option<Result<AttachEvent, Error>>`:
  - `Output(Vec<u8>)` for DATA
  - `Resize { cols: u16, rows: u16 }` for an incoming RESIZE, in stream order
  - `Closed(CloseReason)`, then `None`
- **A malformed RESIZE.** A RESIZE whose payload is not 4 bytes yields
  `Err(Error::Protocol(Frame(BadControlPayload)))`, and the stream continues.
- **`CloseReason`:**
  - `Detached` for an empty CLOSE
  - `Ended { reason: EndReason, exit_code, signal, finished_ms }` for a JSON CLOSE that decodes as
    that shape (W15)
  - `Unrecognized { payload: Vec<u8> }` for any other non-empty CLOSE
- **Writes.** Each write is acknowledged by the writer and returns `Result<(), Error>`:

  | Method | Sends |
  |---|---|
  | `input(&[u8])` | DATA |
  | `automatic_input(&[u8])` | kind 6 when `hello.features` contains `automatic_input` and the attach reply says `automatic_input: true`, otherwise DATA (W17) |
  | `resize(u16, u16)` | RESIZE |
  | `focus(Focus)` | FOCUS |
  | `facts(DisplayFacts)` | FACTS (JSON) |
  | `close()` | CLOSE |

- **Credit.** CREDIT is sent as `Output` bytes are taken (README:103-104).
- **Dropping.** Dropping the stream sends CLOSE through the writer.

### 6.5 Writer, router and lifecycle

- **The writer.** It is a `tokio::spawn`ed task that owns the `FrameSink`.
  - Everything that writes holds a clone of its unbounded `mpsc` sender: the client, every
    `Subscription` and every `AttachStream`.
  - A command is a frame plus an optional oneshot acknowledgement.
  - `Drop` impls send a CLOSE command with the synchronous `UnboundedSender::send`. If the
    connection has already ended, that send fails, and the drop does nothing more.
- **The router.** It is a `tokio::spawn`ed task that owns the `FrameSource`. For each frame:
  - channel 0 DATA → `Incoming`
    - a response goes to its pending entry
    - an unknown id, or `id:null`, is discarded (W9)
    - a notification on channel 0 is discarded
  - DATA, RESIZE or CLOSE on an open stream channel → that stream. After a CLOSE the channel is
    forgotten.
  - CREDIT, FOCUS, FACTS or AUTOMATIC_INPUT from the core → discarded. The core never sent CREDIT
    on any channel (W6, `probe-framed-output.txt:118-122`); the others are viewer → core only
    (README:55-58).
  - an unknown channel or an unknown kind → discarded (W9)
- **Connection end.** These events end the connection:

  | Event | `ConnectionEndKind` |
  |---|---|
  | the source returns `None` | `PeerClosed` |
  | the source returns `TransportError::Io` | `TransportFailed` |
  | the source returns `TransportError::Frame` (e.g. `TooLarge`) | `ProtocolViolation` |
  | the sink fails a write | `TransportFailed` |

  The first event wins, and then:
  - the call whose write failed gets `Error::Transport`
  - every other pending call gets `Error::Closed`
  - every stream gets `Err(Error::Closed)`, then `None`
  - later calls and writes return `Error::Closed` without writing
  - `Client::close_reason()` returns `Some(ConnectionEnd { kind, message })`, where `message` is
    the `Display` of the causing error and `ConnectionEnd` is `Clone`. It returns `None` while the
    connection is open.
- **Server stop.** `server_stop` and `server_restart` return their reply, and the connection then
  ends with `PeerClosed` (W22).
- **Lifetime.** The connection stays open while any `Client` clone, `Subscription` or
  `AttachStream` exists, because each holds a writer sender. When the last one drops:
  1. the writer's channel closes
  2. the writer closes the sink. For `ProcessTransport` that closes stdin, and `hc connect` exits
     0 (W21).
  3. the router reads EOF and ends

## 7. Adapters

### 7.1 `StreamTransport<R: AsyncRead, W: AsyncWrite>`

- **Lines.** `send_line` writes the line and `\n`. `recv_line` uses `LineDecoder`.
- **Frames.** `into_framed` moves the decoder's leftover bytes into the `FrameSource` half's
  `FrameDecoder` (§4.2). `StreamSink<W>` encodes frames, and `StreamSource<R>` decodes them.
- **Testing.** It works over any byte pair, including `tokio::io::duplex`.

### 7.2 `ProcessTransport`

- **Construction.** `ProcessTransport::builder(argv: Argv)` returns a `ProcessTransportBuilder`.
  `Argv` is §4.8's non-empty argument list: `argv[0]` is the program, so an empty command cannot
  be expressed (`Argv::new` refuses `[]` with `argv must not be empty`). The builder has these
  methods:
  - `env(key, value)`
  - `env_remove(key)`
  - `session(SessionName)`, which appends `--session <name>`. `hc connect --session <name>`
    reaches the same core as `HC_SESSION=<name>` (`probe-plain-output.txt:132-138`).
  - `spawn() -> Result<ProcessTransport, TransportError>`
- **Shortcuts.** Both return a `ProcessTransportBuilder`, so `.spawn()` follows:
  - `ProcessTransport::hc_connect()` is the builder for `["hc", "connect"]`, resolved on `PATH`
  - `ProcessTransport::from_argv(argv: Argv)` takes any command, such as
    `["ssh", "box", "hc", "connect"]`
- **The child process.** stdin and stdout are piped, stderr is inherited, and `kill_on_drop(true)`
  is set (`probe-crates-output.txt:1338-1446`, C6b).
- **Environment.** The child inherits the caller's environment. `HC_*` variables are not stripped,
  so the caller decides (§8 edge cases).
- **Halves.** `StreamTransport` over the child's stdout and stdin carries the protocol. The child
  handle lives in the sink half, so closing the sink closes stdin (W21).
- **Errors.** A program that cannot start is
  `TransportError::Spawn { program, source }`, with source kind `NotFound`
  (`probe-crates-output.txt`, C6a).

## 8. Errors and edge cases

### 8.1 Errors (exact `Display`)

| Type | Variant | Display |
|---|---|---|
| `Error` (client) | `Transport(TransportError)` | `transport failed: {source}` |
| | `Rpc { method: MethodName, error }` | `{method} refused by herdr-core: {message} [{kind}]`, e.g. `terminal.get refused by herdr-core: no terminal "t999" [not_found]` (message: `probe-plain-output.txt:247`) |
| | `Decode { method: MethodName, source }` | `cannot decode the {method} result: {source}`, e.g. ``cannot decode the terminal.get result: invalid value: integer `-1`, expected u32`` (the `serde_json::from_value` text: `probe-review-output.txt`, `== R4`) |
| | `Protocol(ProtocolError)` | `herdr-core protocol violation: {source}` |
| | `UnsupportedGeneration { message }` | `herdr-core refused protocol generation 1: {message}` |
| | `UnsupportedVersion { found, minimum }` | `herdr-core {found} is older than the supported minimum {minimum}`, e.g. `herdr-core 0.1.20 is older than the supported minimum 0.1.21` |
| | `InvalidVersion { found }` | `herdr-core reported version "{found}", which is not MAJOR.MINOR.PATCH` |
| | `PayloadTooLarge { len }` | `payload of {len} bytes exceeds the 16777216-byte frame limit` |
| | `Closed` | `the connection to herdr-core is closed` |
| `TransportError` | `Spawn { program, source }` | `cannot start {program}: {source}`, e.g. `cannot start hc: No such file or directory (os error 2)` (C6a) |
| | `Io(io::Error)` | `I/O error: {source}`, e.g. `I/O error: Broken pipe (os error 32)` (C6d) |
| | `Frame(FrameError)` | transparent |
| `FrameError` | `TooLarge { len }` | `frame payload of {len} bytes exceeds the 16777216-byte limit` |
| | `BadControlPayload { kind, expected, got }` | `{kind} frame payload must be {expected} bytes, got {got}`, e.g. `CREDIT frame payload must be 4 bytes, got 3` |
| `ProtocolError` | `InvalidJson { source }` | `invalid JSON from herdr-core: {source}` |
| | `NotJsonRpc` | `message is not a JSON-RPC response or notification` |
| | `HelloMismatch { framed, generation }` | `hello reply has framed={framed} and generation={generation}, expected framed=true and generation=1` |
| | `EventLineTooLong` | `event line exceeds 16777216 bytes without a newline` |
| | `UnexpectedHelloReply { got }` | `expected the hello reply (id 0), got {got}`, where `got` is `a response with id {id}` or `a notification` |
| | `MissingChannel { method }` | `the {method} reply has no channel on a framed connection` |
| | `InvalidEvent { kind, source }` | `cannot decode a {kind} event: {source}` |
| | `Frame(FrameError)` | transparent |
| `RpcError` | (struct) | `{message} [{kind}]` |
| `EmptyValueError` | `{ what: &'static str }` | `{what} must not be empty`, e.g. `terminal reference must not be empty`, `argv must not be empty` |
| `ConnectionEnd` | `{ kind, message }` (`Clone`) | `{kind}: {message}`, where `kind` is `peer closed`, `transport failed` or `protocol violation` |

- **The `what` words.** `EmptyValueError` uses exactly these, one per refusing newtype (§4.7):
  `terminal reference`, `terminal name`, `doc name`, `label key`, `label value`, `machine name`,
  `session name`, `key name`, `method name`, `argv`.
- **Where `FrameError` appears.**
  - Decoding the byte stream happens in the adapter, so `TooLarge` reaches the client as
    `TransportError::Frame` and ends the connection.
  - Interpreting a control payload happens in the client, so `BadControlPayload` is
    `ProtocolError::Frame` on the stream it belongs to.
- **Error traits.** Every error implements `std::error::Error` through `thiserror`, with `source()`
  chained. All are `Send + Sync + 'static`.

### 8.2 Edge cases

| Input or event | Rule | Owner |
|---|---|---|
| Reply for an unknown id (a late reply after a dropped call) | discarded | `client::router` |
| Error with `id:null` (`-32700`) | discarded (W9) | `client::router` |
| A notification on channel 0 in framed mode | discarded | `client::router` |
| Frame on an unknown or closed channel, or an unknown kind | discarded (W9, W14) | `client::router` |
| CREDIT, FOCUS, FACTS or AUTOMATIC_INPUT from the core | discarded (§6.5) | `client::router` |
| Incoming `len` > 16 MiB | `TransportError::Frame(TooLarge)` ends the connection with `ProtocolViolation` (§6.5) | `protocol::frame_decoder`, `adapter::stream`, `client::router` |
| A sink write fails (e.g. `Broken pipe`, `probe-framed-output.txt:464`) | that caller gets `Error::Transport`; the connection ends with `TransportFailed` (§6.5) | `client::writer` |
| Outgoing payload > 16 MiB (a large `file.write`) | `Error::PayloadTooLarge` before sending. The connection is unaffected | `client::client` |
| Incoming RESIZE with a payload that is not 4 bytes | `Err(Protocol(Frame(BadControlPayload)))` on that attach stream; it continues | `protocol::frame`, `client::attach` |
| Event line split across frames, or several lines in one frame | reassembled (W10) | `protocol::ndjson` |
| Event line that is not JSON | `Err(Protocol(InvalidJson))`; the stream continues | `client::subscription` |
| Known event `type` whose `data` does not match | `Err(Protocol(InvalidEvent))`; the stream continues | `protocol::notification`, `client::subscription` |
| Event line over 16 MiB with no `\n` | that subscription yields `EventLineTooLong`, then ends | `protocol::ndjson`, `client::subscription` |
| `events.subscribe` reply without `channel` | `Error::Protocol(MissingChannel)` | `client::client` |
| `missed` | delivered. Later events with `seq ≤ missed.seq` are dropped (W12) | `client::subscription` |
| Unknown event `type` | `EventKind::Unknown` | `protocol::notification` |
| Unknown notification method | `Notification::Other` | `protocol::notification` |
| Unknown error kind | `ErrorKind::Other(String)` | `protocol::rpc_error` |
| Unknown word in an open-enum field (§4.8) | `Other(String)`, serialized back unchanged | `protocol::types` |
| Unknown value in a closed schema enum | `Error::Decode` for that method | `protocol::types` |
| Unknown field in a reply | ignored | `protocol::types` |
| Core CLOSE on an attach channel: empty / JSON `Ended` shape / anything else | `Detached` / `Ended` (W15) / `Unrecognized { payload }` | `client::attach` |
| Core CLOSE on an event channel, empty or not | the subscription ends | `client::subscription` |
| Hello reply: EOF / not JSON / wrong id or a notification | `Error::Closed` / `Protocol(InvalidJson)` / `Protocol(UnexpectedHelloReply)` | `client::client` |
| Version table of §4.5 | as listed | `protocol::hello` |
| `terminal.run` command | `Command::{Shell, Argv, Line}`. "Both" cannot be expressed (E3) | `protocol::methods::terminal` |
| `Argv::new([])` | `argv must not be empty`. `Command::Shell` is the explicit form of "no command" (E4) | `protocol::methods` |
| `machine.exec` command | exactly one of argv or line (E6, E7) | `protocol::methods::machine` |
| `terminal.wait` target | required (E23) | `protocol::methods::terminal` |
| `InitialSize` of 0 | impossible: `NonZeroU16` (E22) | `protocol::types` |
| Attach or RESIZE at 0 | allowed: `u16`. The core accepts the attach and ignores the RESIZE (`probe-review-output.txt`, `== R2`) | `protocol::methods::terminal`, `client::attach` |
| Label set to `""` | impossible: `LabelValue` is non-empty. Removal is `LabelChange::Remove` | `protocol::methods::terminal` |
| Empty identifier string | refused exactly where the core refuses it, per §4.7's table | `protocol::identifiers` |
| Non-empty identifier the core refuses (`a b`, `a=b`) | passed through. It returns `Error::Rpc` with `invalid_params` | `protocol::identifiers` |
| Whitespace-only terminal ref | passed through. The core answers `not_found` (`probe-edges-output.txt:32`) | `protocol::identifiers` |
| Empty `text` in `terminal.send`, empty `keys` | allowed (E17, E18) | `protocol::methods::terminal` |
| `doc.set` with value `null` | allowed: it deletes (E24) | `protocol::methods::doc` |
| Base64 bytes from the core that are not valid base64 | `Error::Decode` for that method | `protocol::types` |
| `ProcessTransport` started inside a Herdr terminal | inherits `HC_*` (`HC_SCOPE`, `HC_TERMINAL_ID`, `HC_TERMINAL_SOCKET`), as any child would. Callers use `env_remove` to opt out. The live tests remove every `HC_*` | `adapter::process`, `tests/live.rs` |
| Dropping a `Subscription` or `AttachStream` | sends CLOSE through the writer; a no-op on an ended connection | `client::subscription`, `client::attach`, `client::writer` |
| Last `Client` clone dropped while a stream lives | the connection stays open until the stream drops too (§6.5) | `client::writer` |
| Last holder of the writer dropped | sink closed, so stdin EOF and `hc connect` exits 0. `kill_on_drop` is the backstop | `client::writer`, `adapter::process` |

## 9. Testing

### 9.1 Colocated unit tests

Every logic-bearing file under `src/` has a `#[cfg(test)] mod tests` (`unit-test-mandate.md`):

| Area | What the unit tests cover |
|---|---|
| Codec | frame encode/decode, partial reads, the 16 MiB boundary (16777216 accepted, 16777217 refused), each control payload size |
| Hand-off | line → frame hand-off with leftover bytes |
| NDJSON | split, packed and over-long lines |
| Envelope | classification |
| Version | the floor, with every row of §4.5 |
| Serde | a round-trip for every type and Params, error decode for every `ErrorKind` |
| Identifiers | for every refusing newtype, `""` refused and non-empty accepted; for every accepting newtype, `""` accepted (§4.7) |
| Open enums | every known word, and an unknown word round-tripping through `Other` |
| Writer | an acknowledged write, a failed write ending the connection, and a drop-time CLOSE on an open and an ended connection |
| Base64 | encode and decode |
| Client logic | router, pending table, credit accounting and stale-seq dropping, driven by in-memory frames through a test-only `FrameSource`/`FrameSink`, with no adapter |
| `ProcessTransport` | argv and env assembly, and missing program → `Spawn`/`NotFound` |

### 9.2 `tests/support/`

- **`fake_transport.rs`** implements the public `Transport` from outside the crate. That also
  proves outside callers can. It replays scripted lines and frames, and records what the client
  sent.
- **`schema_instance.rs`** builds a fully populated JSON instance of any fixture schema, by these
  rules:

  | Schema | Instance value |
  |---|---|
  | object | every property set |
  | `$ref` | resolved |
  | `anyOf` with null, or `type: [T, "null"]` | the non-null branch |
  | `enum` | its first value |
  | string | `"x"` |
  | integer | its `minimum` if that is ≥ 1, else `1` |
  | number | `1.0` |
  | boolean | `true` |
  | array | `minItems` items if set, else one item |
  | map (`additionalProperties`) | one entry, key `"k"` |
  | `any` | `"x"` |

  One override table applies where §4.8 narrows a schema string to bytes:
  `ExecResult.stdout` and `ExecResult.stderr` get `"eAo="`, which is valid base64
  (`probe-base64-output.txt:37-38`). Every value these rules produce is accepted by §4.7's
  newtypes and §4.8's types: `"x"` is non-empty and an open-enum `Other`, integers ≥ 1 satisfy
  `NonZeroU16`, and `Display.cell` gets two items.

### 9.3 `tests/schema_coverage.rs` (offline, always runs)

It runs against `tests/fixtures/hc-0.1.21.openrpc.json`, a copy of
`evidence/upstream-v0.1.21-schema-hc.openrpc.json`:
1. The SDK's 32 `Method::NAME`s equal the fixture's method names.
2. Every `Params` serialized with every optional set has exactly the fixture's param names. With
   only `new()` arguments it has exactly the required names, except where §4.8 makes one of
   several optional params mandatory (`Command`, `ExecCommand`, `WaitTarget`, `MachineTarget`).
   There it has the required names plus the chosen variant's params.
3. For each of the 36 schemas, a `schema_instance` deserialized into the SDK type and serialized
   back equals the instance. A dropped or renamed property fails.

### 9.4 `tests/client_fake.rs` and `tests/adapter_stream.rs`

**Through the fake transport:**
- every one of the 32 methods: the request JSON is asserted, and a canned reply is decoded.
  Replies are copied from the probe outputs.
- events: split and packed lines, credit sent, `missed` dropping, close
- events failure paths: a non-JSON line, mismatched `data`, an over-long line, a reply without
  `channel`, CLOSE empty and non-empty
- attach: output, resize (including a malformed one), all three close reasons, `automatic_input`
  with and without negotiation
- every `connect` refusal of §6.1, including each bad-hello-reply case
- connection loss with calls and streams pending: by EOF, by a failed write, and by a frame
  over 16 MiB, with `close_reason()` checked for each
- lifetime: the connection stays open after the last `Client` clone drops while a stream lives,
  and the sink closes when the stream drops too

**`Send` assertions (ADR 0002).** A compile-time helper `fn assert_send<T: Send>(_: &T)` is
applied, with the fake transport, to:
- the future of every public async method on `Client`, `Subscription` and `AttachStream`
- `Client`, `Subscription` and `AttachStream` themselves

A non-`Send` future fails the build, and so fails the gate.

**Through `StreamTransport` over `tokio::io::duplex`:** the hello line followed in the same write
by frames.

### 9.5 `tests/live.rs`

- **Skip rule.** Each test first spawns `hc --version`. If it cannot spawn, the test returns
  early. Captured test output does not reach the log, so the CI warning grep stays clean
  (`evidence/probe-crates-output.txt:1277-1336`).
- **Isolated core per test.**
  - session `hfy-<pid>-<n>`
  - a fresh short directory: `/tmp/hfy<random>` on Unix (W24), `std::env::temp_dir()` on Windows
  - `XDG_STATE_HOME`, `XDG_CONFIG_HOME` and `XDG_RUNTIME_DIR` set under it
  - every `HC_*` variable removed
  - `hc connect` starts the core (W23)
  - a guard's `Drop` runs `hc server stop` with the same environment, then removes the directory
  - no test addresses the default session
- **Coverage: all 32 methods.**

  | Result | Methods |
  |---|---|
  | success (28) | `hello`, `server.status`, `server.restart`, `server.stop`, `terminal.run`, `templates.list`, `terminal.list`, `terminal.get`, `terminal.kill`, `terminal.rm`, `terminal.keep`, `terminal.label`, `terminal.read`, `terminal.attach`, `terminal.send`, `terminal.wait`, `terminal.keys`, `terminal.interrupt` (short `timeout`), `terminal.put`, `machine.exec`, `file.read`, `file.write`, `file.edit`, `doc.get`, `doc.set`, `events.subscribe`, `machine.list`, `rpc.discover` |
  | typed error (4) | `machine.add` with `MachineTarget::Via { name: m1, command: "false" }` → `unreachable` (18 ms, `probe-methods-output.txt:300-306`); `machine.connect`, `machine.rename` and `machine.rm` on `nope` → `machine_unknown` (`:283-300`) |

- **Drift.** The installed `hc schema` output and `rpc_discover()` are each JSON-equal to the
  fixture (A8). A difference fails, with the differing method and schema names in the message.

## 10. Dependencies and supply chain

**Root `Cargo.toml` `[workspace.dependencies]`.**
- Added:
  ```toml
  tokio      = { version = "1.54.1" }
  serde      = { version = "1.0.229", features = ["derive"] }
  serde_json = "1.0.154"
  thiserror  = "2.0.21"
  base64     = "0.23.1"
  ```
- The existing `herdrify-hc` entry changes to `version = "0.1.0"`. Leaving it at `0.0.0` makes
  every cargo command fail with ``failed to select a version for the requirement `herdrify-hc =
  "^0.0.0"` `` (`evidence/probe-crates-output.txt:1061-1086`).

**`crates/herdrify-hc/Cargo.toml`.**
- `version = "0.1.0"`.
- Dependencies, `{ workspace = true }`: tokio with features `rt`, `sync`, `io-util`, `process`;
  serde; serde_json; thiserror; base64.
- `[dev-dependencies]`: tokio with `macros`, `rt-multi-thread`, `time`.

**`deny.toml`.** `allow = ["Apache-2.0", "MIT", "Unicode-3.0"]`.
- The workspace-skeleton spec rules that "the milestone that adds the first such dependency
  extends `allow` in the same change"
  (`.claudestacks/sdlc/2026-11-22-workspace-skeleton/spec.md:491-492`).
- Under the current Apache-only list, tokio, tokio-macros, mio, bytes, memchr and zmij (MIT) and
  unicode-ident (Unicode-3.0) are rejected (`probe-crates-output.txt:83-215`, including `:115`
  and `:210`).

**Proven together.** This exact set builds on 1.94.1, passes `cargo deny check` with the new list
(`advisories ok, bans ok, licenses ok, sources ok`), and has no duplicate versions
(`evidence/probe-deps-output.txt`).

**Publish.** `cargo publish --dry-run -p herdrify-hc` passes on a clean commit with both versions
at 0.1.0 (`probe-crates-output.txt:1133-1167`). An uncommitted tree needs `--allow-dirty`
(`:1130`).

## 11. Documentation changes

- `crates/herdrify-hc/README.md`: what the crate is, plus a `no_run` example. The example is
  `ProcessTransport::hc_connect().spawn()?`, then `Client::connect`, then `terminal_run`.
- `CLAUDE.md`, "Status": `herdrify-hc` is implemented. Commands: live tests run when `hc` is on
  `PATH` and skip otherwise. They use throwaway isolated cores and never touch the default
  session.
- `docs/herdr-and-airsl.md`, "herdr-core": the W1–W25 facts that matter to callers, each citing
  its evidence file. That covers plain and framed modes, plain mode serializing calls, credit,
  CLOSE reasons, isolation and the socket-length limit.

## 12. Acceptance

- `cargo make dod` exits 0 with zero warnings in both of these environments:
  - with `hc` 0.1.21 on `PATH`, where the live tests run
  - without `hc`, where they skip
- `cargo deny check` exits 0.
- `cargo publish --dry-run -p herdrify-hc` passes on a clean commit.
- The four §2 grep checks print nothing.
- `grep -rnE 'ADR [0-9]|\.claudestacks|milestone' crates/herdrify-hc` prints nothing.

## 13. Non-goals

- herdr-app and `herdrify-app` (milestone 03).
- herdrify ports, adapters, and the `hc` grant filtering of ADR 0004. That table is enforced
  downstream by `herdrify-herdr` and `herdrify-lua`, so this SDK exposes all 32 methods.
- A direct Unix-socket or named-pipe transport. Callers can build one on `StreamTransport`.
- Reconnecting, retries, SDK timeouts, and re-applying events to a local snapshot (callers
  apply README:129-140 themselves).
- Typed access to the OSC 7501 status protocol beyond the event payloads.
- Publishing to crates.io, release automation, changelogs, and versions of other crates.
- Proving compatibility with herdr-core versions other than 0.1.21.
