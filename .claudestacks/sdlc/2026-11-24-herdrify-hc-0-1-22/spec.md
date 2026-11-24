---
status: approved
created: 2026-11-24
built-on: 6bb03f9533ee
adrs: [0002, 0008]
---

# Spec: herdrify-hc support for hc 0.1.22

`herdrify-hc` 0.1.0 covers herdr-core 0.1.21's 32 methods. herdr-core 0.1.22 adds four methods
for a put sent in pieces, and a `docs` filter on `events.subscribe`. This spec does five things:
- brings the SDK to all 36 methods
- keeps 0.1.21 supported
- refuses, with an error that names it, the one addition 0.1.21 silently ignores
- moves the drift checks to the 0.1.22 document
- releases the crate as 0.2.0, because `Error` gains a variant

Evidence paths are relative to this chain's directory, unless they start with `crates/`,
`docs/`, `src/` or `tests/`. Those last two are under `crates/herdrify-hc/`; the first two are
worktree-relative. Every probe was built with rustc 1.94.1, the toolchain `rust-toolchain.toml`
pins (each output's first line records the command). Every probe ran against isolated cores:
- their own `HC_SESSION` and `/tmp` directory
- no inherited `HC_*` variables

The probe sources are `evidence/probe-022/` (run plain, `extra` and `remote`) and
`evidence/probe-sdk/`.

## 1. Premises

| # | Fact | Proof |
|---|---|---|
| P1 | The installed `hc` is 0.1.22, and the gate fails only in `live_schema_matches_the_fixture`. | `evidence/gate-failure.txt:2,12-13`; `evidence/live-tests-0.1.22.txt` (20 of 21 live tests pass) |
| P2 | 0.1.22 has 36 methods and 39 component schemas; 0.1.21 has 32 and 36. Both are OpenRPC 1.3.2. | `evidence/method-counts.txt:2-3`; `evidence/schema-counts.txt:2-5` |
| P3 | Nothing is removed between the two documents. Changes: four methods, the schemas `Empty`, `PutBeginResult` and `PutPieceResult`, the `docs` param, new error entries, `doc.set`'s `internal` code moving from -32000 to -32603, and description text. | `evidence/schema-0.1.21-to-0.1.22.diff` (lines 7, 18, 30, 184, 272, 349, 392, 434 for the additions) |
| P4 | 0.1.22's `hello` lists all 36 methods and adds the feature `put_chunks`. 0.1.21's lists 32 and has no `put_chunks`. | `evidence/probe-022-output.txt:4-5`; `evidence/probe-021-output.txt:4-5` |
| P5 | The SDK's connection is framed. The put methods need a framed connection: on a line connection 0.1.22 refuses with `needs_framed`, "a put in pieces needs a framed connection". | `src/connection/client.rs:123`; `evidence/probe-022-output.txt:163-165` |
| P6 | The whole put works through the SDK's framed `Client::call_raw`. `put_begin` returns `{"transfer": id}`. Each `put_chunk` returns `{"bytes": <received so far>}`. `put_finish` returns `{"path", "bytes"}`, and `put_abort` returns `{}`. | `evidence/probe-022-output.txt:14-29,34-35` |
| P7 | An 8 MiB put in eight 1 MiB pieces lands whole. `wc -c` reads 8388608, and the sha256 on the core's machine equals the sha256 of the bytes sent. | `evidence/probe-022-output.txt:141-161` |
| P8 | On 0.1.21 every put method is refused with code -32601, kind `unknown_method`, and the message `unknown method "terminal.put_begin"` (likewise for the others). | `evidence/probe-021-output.txt:14-35` |
| P9 | 0.1.21 accepts `docs` on `events.subscribe` and ignores it. `{"docs":["probe-doc"]}` delivered every doc event and the terminal events. With `docs` and `terminal`, it delivered no doc events at all. | `evidence/probe-021-output.txt:161-175,225-236` |
| P10 | Every herdrify-hc live test passes against 0.1.21 at HEAD (21 of 21), including the drift test against the 0.1.21 fixture. | `evidence/live-tests-0.1.21.txt`. Binary provenance: `evidence/hc-0.1.21-binary.txt` (its sha256 matches the release manifest) |
| P11 | The SDK already decodes every error kind 0.1.22 uses. A `-32603` `internal` reply decodes as `ErrorKind::Internal` with `code` -32603. | `src/protocol/jsonrpc/error.rs:80-108`; `evidence/probe-022-output.txt:8` |
| P12 | The SDK refuses a request longer than 16777216 bytes before sending it. A 1 MiB piece as a request, with the longest transfer id, is 1398287 bytes. | `src/connection/client.rs:274`; `evidence/probe-sdk-output.txt:12` |
| P13 | The put in pieces works only for this core's own terminals. With a second core added as machine `box`, `put_begin` to `box/t1` is refused: `machine_unknown`, `unknown machine "box"`, hint "this core only runs local terminals; use an address like t7". `terminal.put` to `box/t1` succeeds. An unknown machine gets the same refusal. | `evidence/probe-022-remote-output.txt:19-22`; `evidence/probe-022-extra-output.txt:14-19` |
| P15 | On 0.1.22, `rpc.discover` returns the same document as `hc schema` (`serde_json::Value` equality, 36 methods each), so the drift test's two assertions both hold on 0.1.22. | `evidence/probe-022-framed-output.txt:14-15` |
| P16 | On a **framed** connection (the SDK's), 0.1.22's `docs` filter behaves as on a line connection. `docs` alone gives an empty terminal snapshot and only the matching doc's event on the stream channel. `docs` with `terminal` gives that terminal's snapshot, the matching doc event and the terminal's events. | `evidence/probe-022-framed-output.txt:18-28,29-40` (raw `hc connect` with `framed: true`, using the SDK's `Frame` and `FrameDecoder`) |
| P14 | A put to an ended terminal is refused with `finished`, `t2 has finished`, as `terminal.put` is. When the terminal is killed mid-put, the next piece gets `finished` and the core drops the put: `put_finish` then gets `not_found`, and `put_abort` answers `{}`. | `evidence/probe-022-extra-output.txt:26-29,34-45` |

## 2. Architecture

The author approved the module tree on 2026-11-24, before this spec was written. After the spec
review, three points changed, and they are marked ★ below:
- `TransferId` gets its own file instead of going into `identifiers.rs` (§2.2)
- `Error` gains a variant (§4.3)
- the version is 0.2.0 (§9)

### 2.1 Module tree (changes only)

```text
crates/herdrify-hc/src/
├── protocol/
│   ├── transfer.rs  ★           NEW: TransferId and InvalidTransferId
│   ├── hello.rs                 + a crate-private constant beside CoreVersion::MINIMUM:
│   │                              DOCS_MINIMUM, the version 0.1.22
│   ├── mod.rs                   + mod transfer; flat re-exports of TransferId, InvalidTransferId,
│   │                              the four request types and the three new result types
│   ├── methods/
│   │   ├── events.rs            + EventsSubscribe::docs, and a crate-private reader of it
│   │   ├── mod.rs               re-exports the new request types
│   │   └── terminal/
│   │       ├── mod.rs           + mod put; re-exports
│   │       ├── put.rs           NEW: TerminalPut (moved from interaction.rs), TerminalPutBegin,
│   │       │                         TerminalPutChunk, TerminalPutFinish, TerminalPutAbort
│   │       └── interaction.rs   − TerminalPut
│   └── types/
│       ├── mod.rs               re-exports the new result types
│       └── terminal/
│           ├── mod.rs           + mod put; re-exports
│           ├── put.rs           NEW: PutResult (moved from results.rs), PutBeginResult,
│           │                         PutPieceResult, EmptyResult
│           └── results.rs       − PutResult
├── connection/
│   ├── client.rs                + terminal_put_begin, terminal_put_chunk, terminal_put_finish,
│   │                              terminal_put_abort; the docs guard in events_subscribe
│   └── error.rs  ★              + Error::NeedsNewerCore
├── herdr_core/
│   ├── terminals.rs             + TerminalHandle::put_begin; TerminalOps::put_chunk, put_finish,
│   │                              put_abort
│   ├── put.rs                   NEW: TerminalHandle::put_in_pieces
│   ├── entry.rs                 + Call<T, EventsSubscribe>::docs
│   └── mod.rs                   + mod put
└── prelude.rs                   + TransferId
```

### 2.2 Why each new file exists

- **`protocol/transfer.rs`**: one concept, the id of a put in pieces, with its error inside.
  - It does not go in `identifiers.rs`. That file's documented rule is that its identifiers
    refuse only the empty string, return `EmptyValueError`, and leave every other check to the
    core (`src/protocol/identifiers.rs:10-20`).
  - `TransferId` validates fully and has its own error, so it gets its own file, and
    `identifiers.rs` stays true to its doc.
- **`methods/terminal/put.rs`**: one concept, sending a file beside a terminal. It holds the five
  request types for it: the whole put, and the four steps of a put in pieces.
  `interaction.rs` (592 lines) keeps the requests that talk to a running program.
- **`types/terminal/put.rs`**: the replies to those five requests, grouped the same way.
  `results.rs` (675 lines) keeps the rest.
  - `EmptyResult` maps the generic schema `Empty`, which only `put_abort` returns today. It moves
    to a shared home if another method comes to return it.
- **`herdr_core/put.rs`**: `put_in_pieces` is a procedure of several calls, with its own failure
  handling. That is a different responsibility from the one-call builders in `terminals.rs`
  (962 lines). It adds an `impl TerminalHandle` block in the same crate.

### 2.3 Dependency direction

Unchanged: `herdr_core → connection → port, protocol`. `protocol` depends on nothing else in the
crate. No crate dependency is added. `serde`, `serde_json`, `tokio`, `base64` and `thiserror` are
already there (`crates/herdrify-hc/Cargo.toml:15-20`).

### 2.4 Rust guideline rules applied

These come from `claudestacks-guideline-rust:rust-guidelines` and its references.

| Rule | How it applies |
|---|---|
| strong-types: newtype domain values, validate at construction | `TransferId` is validated by `new`, `TryFrom` and `Deserialize`, and has its own error struct. Byte counts and offsets stay `u64`, as `PutResult.bytes` already is (`src/protocol/types/terminal/results.rs:163`). |
| modularity: one responsibility per unit, no duplicate types | Four new concept files (§2.2). `TerminalPut` and `PutResult` move; nothing is duplicated. |
| mod-rs-export-only | `mod.rs` files only declare modules and `#[doc(inline)] pub use`. Moved items keep their public paths, because `protocol` re-exports flat (`src/protocol/mod.rs:45-80`). |
| static-dispatch | Every new method is generic over `T: Transport`. No `Box<dyn …>`. |
| ADR 0002: futures are `Send` when the transport is | The four new `Client` futures join `client_and_its_futures_are_send` (`tests/client_fake.rs:1244-1250`). `put_in_pieces` gets the same compile-time check. |
| M-ERRORS-CANONICAL-STRUCTS (microsoft-guidelines) | `InvalidTransferId` is a struct holding the refused value. `NeedsNewerCore` carries named fields. |
| M-TYPES-SEND, M-PUBLIC-DEBUG (microsoft-guidelines) | Every new public type is `Debug`, and `Send + Sync + 'static` where its siblings are. `InvalidTransferId` gets the `*_is_send_sync_static` test every error type has (`src/protocol/identifiers.rs:431`). |
| M-DOCUMENTED-MAGIC (microsoft-guidelines) | The piece size 1048576 is a named constant, with a comment giving its origin (the core's limit, P7 and §5.2). |
| unit-test-mandate | Every new or changed `src` file keeps or gains a colocated `#[cfg(test)] mod tests`. |
| doc-comment-discipline | `# Examples` on every new public item. No planning words in source. The doc of `PutResult` is widened where its meaning grows (§3.3). `UnsupportedVersion` keeps its meaning and doc (§4.3). |
| workspace.md: versions and publishing | The crate and the root `[workspace.dependencies]` entry both move to 0.2.0 (§9), and `cargo publish --dry-run` runs on the committed tree. |
| strict-quality | The workspace gate passes with zero warnings (§10). |

### 2.5 ADR 0002 and the method count

ADR 0002's *Context* records the document in force when it was written: "an OpenRPC 1.3.2
document for herdr-core 0.1.21", "32 methods and 36 component schemas"
(`.claudestacks/sdlc/adrs/0002-standalone-async-herdr-sdks.md:15-18`). Its *Decision* asks for
"request and result types for all 32 methods" (`:100`). That count is the size of that document,
not a cap. The rule is that the SDK types every method. Covering all 36 methods of 0.1.22 keeps
the rule, and every other 0002 rule (async on tokio, no features, generics over `Transport`,
`Send` futures) holds as before. No ADR change is needed for 0002.

### 2.6 How it is tested

| Layer | Where | What |
|---|---|---|
| Unit | colocated tests | the `TransferId` rules (§3.1), param serialization, result decoding, the `NeedsNewerCore` message |
| Client over a fake transport | `tests/client_fake.rs` | the four put calls, the docs guard on 0.1.21 and 0.1.22 hellos, `put_in_pieces` (piece split, abort rules) |
| Coverage against the document | `tests/schema_coverage.rs` | method names, param names, schemas and reply types, against the 0.1.22 document |
| Live | `tests/live.rs` | the drift test against 0.1.22, plus the live tests in §8 |

## 3. Protocol layer

### 3.1 `TransferId` (`protocol/transfer.rs`)

The name a put in pieces is known by, chosen by the sender.

- **Rule.** Valid when it has 16 to 64 characters and every one is an ASCII hex digit (`0-9`,
  `a-f`, `A-F`). Nothing is trimmed, and case is kept as given. This matches the core's rule:
  - refused with "transfer must be 16 to 64 hex digits": 15 and 65 digits, `zzzz…` and the empty
    string
  - accepted and echoed as given: 16 digits, 64 digits and uppercase hex
  - (`evidence/probe-022-output.txt:40-66`)
- **Constructors.** `TransferId::new(value: impl Into<String>) -> Result<TransferId,
  InvalidTransferId>`, `TryFrom<&str>` and `FromStr` (as sibling identifiers have,
  `src/protocol/identifiers.rs:70-85`), and `TryFrom<String>` for the validating
  `Deserialize`. All of them apply the rule.
- **`InvalidTransferId`** is a struct holding the refused value. Its message is
  `transfer id {value:?} must be 16 to 64 hex digits`. For example, `TransferId::new("xyz")`
  reads `transfer id "xyz" must be 16 to 64 hex digits`.
- **`TransferId::random() -> TransferId`**: 32 lowercase hex digits, made from two
  `std::collections::hash_map::RandomState` hashes. Fresh states give different values on rustc
  1.94.1 (`evidence/probe-sdk-output.txt:7`). It adds no dependency.
  - It is not a secret. The core uses it only to tell open puts apart.
  - The core refuses a second open put with the same id: "put … is taken"
    (`evidence/probe-022-output.txt:88-89`).
- **Traits.**
  - `as_str()`, and `Display` (the value as given).
  - `Serialize` as a plain string.
  - A **validating** `Deserialize` (through `TryFrom<String>`), so a reply carrying a bad id fails
    to decode.
  - `Debug`, `Clone`, `PartialEq`, `Eq` and `Hash`.

Checked examples. The plan's unit tests must assert each one.

| Input | Result |
|---|---|
| `"0123456789abcdef"` (16) | ok |
| `"a"` × 64 | ok |
| `"ABCDEF0123456789"` | ok, kept uppercase |
| `"0123456789abcde"` (15) | `InvalidTransferId` |
| `"b"` × 65 | `InvalidTransferId` |
| `"zzzzzzzzzzzzzzzz"` | `InvalidTransferId` |
| `""` | `InvalidTransferId` |
| `" 0123456789abcdef"` (a leading space) | `InvalidTransferId` |
| `"0123456789abcdeé"` (16 characters, the last one not ASCII) | `InvalidTransferId` |

### 3.2 The four requests

Each request is a `Method` with the `NAME` below, serialized by name with exactly the listed
params. The params come from the 0.1.22 schema (`evidence/hc-0.1.22-schema.json`), and are also
shown at `evidence/schema-0.1.21-to-0.1.22.diff:184-271,272-348,349-391,392-430`.

| Type | `NAME` | Constructor | Params on the wire | `Output` |
|---|---|---|---|---|
| `TerminalPutBegin` | `terminal.put_begin` | `new(terminal: TerminalRef, name: FileName, bytes: u64, transfer: TransferId)` | `terminal`, `name`, `bytes`, `transfer` | `PutBeginResult` |
| `TerminalPutChunk` | `terminal.put_chunk` | `new(transfer: TransferId, offset: u64, data: Vec<u8>)` | `transfer`, `offset`, `data` (base64) | `PutPieceResult` |
| `TerminalPutFinish` | `terminal.put_finish` | `new(transfer: TransferId)` | `transfer` | `PutResult` |
| `TerminalPutAbort` | `terminal.put_abort` | `new(transfer: TransferId)` | `transfer` | `EmptyResult` |

- `data` uses the base64 serializer that `TerminalPut` already uses
  (`src/protocol/methods/terminal/interaction.rs:393`). The core accepts it
  (`evidence/probe-022-output.txt:16-17`).
- `TerminalPutBegin`'s doc says the terminal must be on this core (P13).

### 3.3 The replies

| Type | Schema | Fields |
|---|---|---|
| `PutBeginResult` | `PutBeginResult` | `transfer: TransferId` |
| `PutPieceResult` | `PutPieceResult` | `bytes: u64`, the bytes received so far |
| `PutResult` | `PutResult` (moved) | `path: String`, `bytes: u64`. Its doc widens from "The reply to `terminal.put`" (`src/protocol/types/terminal/results.rs:155`) to "The reply to `terminal.put` and `terminal.put_finish`". |
| `EmptyResult` | `Empty` | none |

`EmptyResult` is `struct EmptyResult {}` with serde derive. Probed with serde 1.0.229 and
serde_json 1.0.154 (`evidence/probe-sdk-output.txt:8-10`):
- it reads `{}` and writes `{}`
- it reads `{"later":1}`, ignoring fields added later
- it refuses `null`

### 3.4 `EventsSubscribe::docs`

- **The setter.** `docs(names: impl IntoIterator<Item = DocName>) -> Self` sets the documents to
  narrow to. `DocName` already refuses the empty string (`src/protocol/identifiers.rs:115-119`).
- **Calling it again replaces** the earlier names, like the sibling setters
  (`src/protocol/methods/events.rs:44-61`).
- **An empty iterator leaves `docs` unset.** It also clears names set by an earlier call, and
  nothing is sent for `docs`. On 0.1.22, `[]` and an absent `docs` behave the same: every
  document and terminal event is delivered (`evidence/probe-022-output.txt:196-210,212-226`).
- **Names are sent in the given order**, duplicates included. 0.1.22 delivers a matching event
  once (`evidence/probe-022-output.txt:269-280`).
- **Prefixes.** A name ending in `*` is a prefix, and `*` alone matches every document. This is
  the core's rule (`evidence/schema-0.1.21-to-0.1.22.diff:433`), shown at
  `evidence/probe-022-output.txt:183-194,242-255`. The SDK passes names through unchanged.
- **What 0.1.22 does with `docs`.** The setter documents this. It was probed on line
  connections and, for the first two cases, on a framed connection too (P16):
  - `docs` alone: only matching doc events, no terminal events, and an empty terminal snapshot
    (`evidence/probe-022-output.txt:170-181`)
  - `docs` with `terminal`, `labels` or `children_of`: matching doc events, plus the events and
    snapshot of the terminals the other filter selects
    (`evidence/probe-022-output.txt:228-240`; `evidence/probe-022-extra-output.txt:61-72,75-90`)
  - the live test in §8 checks the same through the SDK's own framed client
- **A crate-private accessor** lets the client tell whether `docs` is set (§4.2).

## 4. Client

### 4.1 The four calls

`Client::terminal_put_begin`, `terminal_put_chunk`, `terminal_put_finish` and
`terminal_put_abort` each send their request through the existing generic `call`
(`src/connection/client.rs:304-309`), as `terminal_put` does (`:481-483`).
- Their errors are those of `call_raw`, plus `Error::Decode`.
- There is no client-side version check, because 0.1.21 refuses them itself (P8).

### 4.2 The docs guard

`Client::events_subscribe` checks this first, before the closed-connection check and before
anything is sent:

```text
if params has docs set
   and the core's version (parsed once by connect) < DOCS_MINIMUM (0.1.22)
then return Err(Error::NeedsNewerCore {
         option: "events.subscribe docs",
         needs:  DOCS_MINIMUM,
         found:  <the hello version>,
     })
```

- Nothing is sent, and the connection stays usable.
- `connect` already parses the version and refuses one that does not parse
  (`src/connection/client.rs:174-181`). The client keeps that parsed value, so the guard never
  parses again.
- The comparison is `CoreVersion`'s (`evidence/probe-sdk-output.txt:4-6`):
  - `0.1.22-rc.1` is older than 0.1.22, so it is refused
  - `0.1.22+abc` is not older, so it is allowed
- Without `docs`, `events_subscribe` is unchanged on every version.
- Why the guard exists: 0.1.21 silently ignores `docs` (P9).

### 4.3 `Error::NeedsNewerCore`

A new variant of `Error` (`src/connection/error.rs:41-90`):

```text
NeedsNewerCore {
    option: &'static str,   // what was asked for, e.g. "events.subscribe docs"
    needs:  CoreVersion,    // the oldest core that supports it
    found:  CoreVersion,    // the core's version
}
```

- **Message:** `{option} needs herdr-core {needs} or newer; this core is {found}`. With 0.1.21 it
  reads `events.subscribe docs needs herdr-core 0.1.22 or newer; this core is 0.1.21`.
- `CoreVersion`'s `Display` prints the text it was parsed from (`src/protocol/hello.rs:195-196`).
- Through `HerdrCore`, the error is wrapped in `HerdrCoreError::Call`, which shows the inner
  error's message unchanged. The same wrapping shows `UnsupportedVersion`'s text verbatim
  (`evidence/probe-sdk-output.txt:2-3`).
- `Error` is not `#[non_exhaustive]` (`src/connection/error.rs:41-42`), so the new variant is a
  breaking change. That is why the release is 0.2.0 (§9).
- `UnsupportedVersion` is unchanged. It still means only that `connect` met a core older than
  0.1.21.

## 5. HerdrCore layer

### 5.1 One-call builders

| Method | Returns | Notes |
|---|---|---|
| `TerminalHandle::put_begin(name: impl Into<String>, bytes: u64, transfer: TransferId)` | `Call<T, TerminalPutBegin>` | This handle's terminal. The params are in the same order as `TerminalPutBegin::new`. `name` becomes a `FileName` unchecked, as in `put` (`src/herdr_core/terminals.rs:208-215`). |
| `TerminalOps::put_chunk(transfer: TransferId, offset: u64, data: impl Into<Vec<u8>>)` | `Call<T, TerminalPutChunk>` | |
| `TerminalOps::put_finish(transfer: TransferId)` | `Call<T, TerminalPutFinish>` | |
| `TerminalOps::put_abort(transfer: TransferId)` | `Call<T, TerminalPutAbort>` | |

- The last three name no terminal, so they belong to `TerminalOps` (`core.terminals()`), not to
  a handle.
- A `TransferId` is taken already built, so these builders cannot fail before sending, and
  `HerdrCoreError` gains no variant.
- **A handle for another machine's terminal** (`m/t7`) is sent as given. The core refuses with
  `machine_unknown` (P13). `put_begin` and `put_in_pieces` say so in their docs, and point to
  `put` for such a terminal.

`Call<T, EventsSubscribe>::docs<S: Into<String>>(names: impl IntoIterator<Item = S>)` sits beside
`children_of`, `labels` and `terminal` (`src/herdr_core/entry.rs:229-247`).
- Each name becomes a `DocName`.
- An empty name is kept as the call's error and reported when the call is awaited, as
  `HerdrCoreError::Empty` with the message `doc name must not be empty`
  (`evidence/probe-sdk-output.txt:11`). Nothing is sent.
- Calling it again replaces the earlier names, as in §3.4.

### 5.2 `TerminalHandle::put_in_pieces`

```text
pub async fn put_in_pieces(&self, name: impl Into<String>, data: impl Into<Vec<u8>>)
    -> Result<PutResult, HerdrCoreError>

id = TransferId::random()
put_begin(name, len(data), id)
    Err(Error::Rpc)        → return it. The core refused, so nothing is open; no abort.
    any other Err          → put_abort(id), ignoring its result; return the first error.
                             The begin may have reached the core.
for each piece of PIECE_BYTES = 1048576 bytes, in order, at offset = bytes sent so far:
    put_chunk(id, offset, piece)
        any Err            → put_abort(id), ignoring its result; return the chunk's error
put_finish(id)
    any Err                → put_abort(id), ignoring its result; return the finish's error
return the PutResult
```

- **Piece size.** 1048576 bytes, the largest piece the core accepts. 1048577 is refused with
  "at most 1 MB per piece" (`evidence/probe-022-output.txt:128-133`). A request of that size
  fits a frame (P12).
- **Empty data**: begin with 0, no pieces, then finish. The core returns `{"bytes":0,"path":…}`
  (`evidence/probe-022-output.txt:82-85`).
- **Over 8 MiB** is not checked by the SDK. The core refuses it at begin with `invalid_params`,
  "at most 8 MB per put": it refuses 8388609 and more, and accepts 8388608
  (`evidence/probe-022-output.txt:68-75`).
- **Unknown or ended terminal**: refused at begin, with `not_found` `no terminal "t999"` or with
  `finished` `t2 has finished` (`evidence/probe-022-output.txt:102-103`;
  `evidence/probe-022-extra-output.txt:26-27`).
- **Another machine's terminal**: refused at begin with `machine_unknown` (P13).
- **0.1.21**: refused at begin with `unknown_method` (P8).
- **Terminal ended mid-put.** The next piece gets `finished` and is returned after the abort.
  The core has already dropped the put (P14).
- **The abort's own failure is ignored.** The first error is the one returned.
  - `put_abort` of an id that is unknown, finished or already aborted answers `{}`
    (`evidence/probe-022-output.txt:34-35,96-97`).
  - The core also removes an open put when its connection or terminal goes
    (`evidence/schema-0.1.21-to-0.1.22.diff:368`, and P14).
- **Dropped future.** If the caller drops the future mid-put, no abort is sent. The open put
  stays until the connection or terminal goes (same citation). The method's docs say so.
- **Name handling** is the core's, as for `terminal.put` (`evidence/probe-022-output.txt:104-127`):
  - `""` lands as `file`
  - `a/b.txt` lands as `b.txt`
  - a name already in use gets a suffix, such as `a-2.txt`
- Other calls on the same connection can run between pieces, because the client is concurrent
  (`src/connection/client.rs:1`).
- The returned future is `Send` when `T` is (§2.4).

## 6. Errors

Messages from the core are quoted from the probes. The SDK reports them as `Error::Rpc`
(`src/connection/error.rs:47-54`), carrying the core's `kind`, and `HerdrCore` wraps that in
`HerdrCoreError::Call`. A kind the schema lists but no probe triggered (`unavailable`, and
`internal` on the put methods) gets no special handling: it reaches the caller as `Error::Rpc`
with that kind (P11).

| Where | When | What the caller sees |
|---|---|---|
| `TransferId::new`, `TryFrom`, `FromStr` | the §3.1 rule is broken | `InvalidTransferId`: `transfer id "<value>" must be 16 to 64 hex digits` |
| `Client::events_subscribe` | `docs` set, core older than 0.1.22 | `Error::NeedsNewerCore`: `events.subscribe docs needs herdr-core 0.1.22 or newer; this core is <found>`. Nothing is sent. |
| `Call<_, EventsSubscribe>::docs` | an empty name | `HerdrCoreError::Empty`: `doc name must not be empty`. Nothing is sent. |
| any put call on 0.1.21 | always | `unknown_method`: `unknown method "terminal.put_begin"` (likewise for the others) |
| `put_begin` | over 8 MiB | `invalid_params`: `at most 8 MB per put` |
| `put_begin` | the id is already open | `invalid_params`: `put <id> is taken` |
| `put_begin` | unknown terminal | `not_found`: `no terminal "<t>"` |
| `put_begin` | ended terminal | `finished`: `<t> has finished` |
| `put_begin` | another machine's terminal, or an unknown machine | `machine_unknown`: `unknown machine "<m>"` |
| `put_chunk` | wrong offset | `invalid_params`: `the next piece starts at <n>` |
| `put_chunk` | piece over 1 MiB | `invalid_params`: `at most 1 MB per piece` |
| `put_chunk` | past the size begun with | `invalid_params`: `more than the <n> bytes begun with` |
| `put_chunk` | the terminal ended mid-put | `finished`: `<t> has finished` |
| `put_chunk`, `put_finish` | unknown, finished, aborted or dropped id | `not_found`: `no put <id> here` |
| `put_finish` | not all bytes sent | `invalid_params`: `got <n> of <m> bytes` |
| `put_in_pieces` | any of the above | the first error, after the abort rules of §5.2 |
| a reply that does not decode | for example a `transfer` that breaks §3.1 | `Error::Decode` |

Where each core message is quoted, in table order:

| Row | Citation |
|---|---|
| over 8 MiB | `evidence/probe-022-output.txt:72-73` |
| id already open | `evidence/probe-022-output.txt:88-89` |
| unknown terminal | `evidence/probe-022-output.txt:102-103` |
| ended terminal | `evidence/probe-022-extra-output.txt:26-27` |
| another machine | `evidence/probe-022-remote-output.txt:21-22` |
| wrong offset | `evidence/probe-022-output.txt:18-21` |
| piece over 1 MiB | `evidence/probe-022-output.txt:130-131` |
| past the size | `evidence/probe-022-output.txt:24-25` |
| ended mid-put | `evidence/probe-022-extra-output.txt:40-41` |
| unknown or gone id | `evidence/probe-022-output.txt:30-33,94-101`; `evidence/probe-022-extra-output.txt:42-43` |
| not all bytes | `evidence/probe-022-output.txt:90-91` |

Nothing is retried. Nothing is logged: the SDK has no logging today, and this adds none.

## 7. Edge cases (settled)

| Case | Outcome | Proof |
|---|---|---|
| `docs` with an empty iterator | unset: nothing is sent and no version check runs | §3.4 |
| `docs` called twice | the second call replaces the first | §3.4 |
| `docs([])` after `docs(["a"])` | cleared | §3.4 |
| `docs` with duplicate names | sent as given; delivered once on 0.1.22 | `evidence/probe-022-output.txt:269-280` |
| `docs` with `terminal` on 0.1.21 | refused by the guard. 0.1.21 would drop every doc event (P9). | §4.2 |
| `docs` on `0.1.22-rc.1` | refused by the guard | `evidence/probe-sdk-output.txt:6` |
| `docs` set on an ended connection with a 0.1.21 hello | `NeedsNewerCore`; the guard runs first | §4.2 |
| `TransferId` in uppercase | kept as given; the core echoes it | `evidence/probe-022-output.txt:60-61` |
| The same id reused after finish | allowed by the core | `evidence/probe-022-output.txt:36-37` |
| An empty piece | the core accepts it, with no change to `bytes`; `put_in_pieces` never sends one | `evidence/probe-022-output.txt:136-137` |
| A piece with bad base64 | cannot come from the SDK, which does the encoding; the core refuses it with "data is not base64" | `evidence/probe-022-output.txt:134-135` |
| `put_in_pieces` with exactly 1 MiB | one piece | §5.2 |
| `put_in_pieces` with 1 MiB + 1 | two pieces, of 1048576 and 1 bytes | §5.2 |
| A handle for `m/t7` | the core's `machine_unknown` is returned; the docs point to `put` | P13, §5.1 |
| Terminal killed between pieces | `finished`, then an abort the core answers `{}` | P14 |

## 8. Tests

The plan names the tests. Each behaviour below needs at least one.

- **Unit (colocated):**
  - every row of the §3.1 table, through `new`, `TryFrom` and `Deserialize`
  - `random()` gives 32 lowercase hex digits and passes `new`
  - `InvalidTransferId` is `Send + Sync + 'static`
  - the four requests serialize with exact param names and base64 `data`
  - the four replies decode, and `EmptyResult` reads `{}` and writes `{}`
  - `EventsSubscribe::docs`: an empty iterator sends no `docs`, names are sent in order, a second
    call replaces the first, and `docs([])` clears
  - the `NeedsNewerCore` message, exactly as in §4.3
- **`tests/client_fake.rs`:**
  - each put call sends its method name and params, and decodes its reply
  - a 0.1.21 hello with `docs` set gives `NeedsNewerCore { option: "events.subscribe docs",
    needs: 0.1.22, found: 0.1.21 }`, and no request is written
  - a `0.1.22-rc.1` hello is refused the same way; a 0.1.22 hello sends the request
  - `put_in_pieces` with 0, 1, 1048576 and 1048577 bytes sends begin, the right pieces at the
    right offsets, and finish
  - a refused piece is followed by `put_abort` with the same id, and the piece's error is
    returned
  - a refused finish is followed by `put_abort`
  - a begin refused by the core (`Error::Rpc`) sends no abort
  - a begin whose reply does not decode is followed by `put_abort`
  - the four new `Client` futures and the `put_in_pieces` future are `Send`, added to
    `client_and_its_futures_are_send`
- **`tests/schema_coverage.rs`**, against `tests/fixtures/hc-0.1.22.openrpc.json`:
  - the method names equal the document's 36
  - the four new requests send exactly their schema params
  - `EventsSubscribe` sends `docs`
  - `Empty`, `PutBeginResult` and `PutPieceResult` round-trip through `EmptyResult`,
    `PutBeginResult` and `PutPieceResult`
  - each new `Method::Output` is its schema's reply type
  - **The generator needs a valid id.** The instance generator writes `"x"` for every string
    (`tests/support/schema_instance.rs:4`), which `TransferId` refuses. It gains a
    `(schema, property) → value` override table beside `BYTE_PROPERTIES` (`:12-13`). The one
    entry is `("PutBeginResult", "transfer") → "0123456789abcdef"`.
- **`tests/live.rs`.** These run when `hc` is on `PATH` and skip otherwise. They use the existing
  isolated `LiveCore`, and choose what to assert from the core's `hello`:
  - `live_schema_matches_the_fixture` reads the 0.1.22 fixture. On any other version it fails and
    names the differences. That is its job as the drift test.
  - a live `put_in_pieces` of more than 1 MiB, checked with `machine.exec ["wc", "-c", path]`.
    It skips with a printed notice when `hello.methods` lacks `terminal.put_begin`.
  - live put refusals: a wrong offset, and a finish before all bytes. Same skip rule.
  - a live docs subscription through the framed client. On 0.1.22 or newer, it hears a matching
    doc and not another. On an older core, it asserts the `NeedsNewerCore` refusal instead.
- **The fixture.** `tests/fixtures/hc-0.1.22.openrpc.json` is `hc schema` from 0.1.22, byte for
  byte (`evidence/hc-0.1.22-schema.json`). `tests/fixtures/hc-0.1.21.openrpc.json` is removed; it
  remains in git history.
- **Acceptance on 0.1.21.** Once the change is built, run the live suite once against hc 0.1.21,
  using the release first on `PATH` as in `evidence/live-tests-0.1.21.txt:1`. Fetch it and check
  its sha256 against the manifest, as in `evidence/hc-0.1.21-binary.txt`, if it is no longer in
  the scratchpad. Expected:
  - every test passes except `live_schema_matches_the_fixture`, which fails naming the 0.1.21
    differences
  - the put tests skip
  - the docs test asserts the refusal
  - If running that binary is not permitted at execute, record it in the plan's `## Deviations`.

## 9. Docs and release

| File | Change |
|---|---|
| `crates/herdrify-hc/Cargo.toml:4` | `version = "0.2.0"`. For 0.x, Cargo treats a minor bump as breaking, and `Error` gains a variant (§4.3). |
| `Cargo.toml:14` (workspace root) | `herdrify-hc = { path = "crates/herdrify-hc", version = "0.2.0" }`. `Cargo.lock` follows. |
| `crates/herdrify-hc/CHANGELOG.md` | A `[0.2.0]` entry dated the day of the release commit. Added: the four put calls, `TransferId`, `put_in_pieces`, `docs`. Changed, breaking: `Error::NeedsNewerCore`; `docs` is refused on cores older than 0.1.22. Changed: tested against hc 0.1.22. |
| `crates/herdrify-hc/README.md:8,18` | 36 methods; tested against hc 0.1.22, with 0.1.21 still supported |
| `crates/herdrify-hc/src/lib.rs:19` | "tested against hc 0.1.22" |
| `crates/herdrify-hc/examples/README.md:28` | "written against hc 0.1.22" |
| `crates/herdrify-hc/examples/connect/README.md:37` | the sample output reads `connected to hc 0.1.22, 36 methods` |
| `CLAUDE.md:8` | "all 36 methods" |
| `CLAUDE.md:14` | "(0001–0008)" |
| `CLAUDE.md:91,95` | ADR 0008; "22 of 36 methods are never exposed" |
| `docs/concepts.md:24-27` | ADR 0008 in place of 0004; "22 of hc's 36 methods" |
| `docs/architecture.md:26,29,110` | ADR 0008 in place of 0004 |
| `docs/herdr-and-airsl.md:10,19,27,97` | hc 0.1.22; 36 methods, 39 schemas; ADR 0008. New rows for the put in pieces and the `docs` filter, citing this chain's evidence. |

## 10. The gate

- `cargo make dod` passes with zero warnings with hc 0.1.22 installed, and its log has no
  `warning` or `error` line.
- `cargo deny check` passes.
- `cargo publish --dry-run -p herdrify-hc` passes on the committed tree.

## 11. Non-goals

- Raising the supported minimum above 0.1.21. Connecting to 0.1.21 is unchanged.
- A client-side guard for the put methods. 0.1.21 refuses them itself (P8).
- Falling back from `put_in_pieces` to `terminal.put` for another machine's terminal. The
  caller uses `put` (§5.1).
- Running a 0.1.21 core in the automated suite or in CI. The acceptance run in §8 is a one-off
  check.
- Checking the 8 MiB put limit in the SDK. The core refuses it (§5.2).
- Publishing to crates.io. Only the dry run is in scope.
- herdrify's grant enforcement and `herdrify-app`. ADR 0008 is enforced by `herdrify-herdr` and
  `herdrify-lua`, not by the SDK.
