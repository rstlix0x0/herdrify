---
status: approved
created: 2026-11-24
depends-on: [01]
built-on: 2db7af9df077
---

# Put Protocol Types Implementation Plan

**Goal:** Add the request and reply types of the put in pieces to the sans-IO `protocol` layer.

**Architecture:** Two new concept files group everything about sending a file beside a terminal
(spec §2.1, §2.2):
- `protocol/methods/terminal/put.rs` holds the five requests. `TerminalPut` moves here from
  `interaction.rs`.
- `protocol/types/terminal/put.rs` holds their replies. `PutResult` moves here from
  `results.rs`.

Public paths stay the same, because every `mod.rs` re-exports flat (mod-rs-export-only).

**Tech Stack:** Rust 1.94, serde 1.0.229, serde_json 1.0.154, base64 0.23.1 (through the existing
`protocol::base64` serializer).

**Content authority:** spec §3.2, §3.3.

---

## Files that change

```
crates/herdrify-hc/src/protocol/methods/terminal/put.rs          — create: TerminalPut (moved), TerminalPutBegin, TerminalPutChunk, TerminalPutFinish, TerminalPutAbort
crates/herdrify-hc/src/protocol/methods/terminal/interaction.rs  — modify: remove TerminalPut and its test
crates/herdrify-hc/src/protocol/methods/terminal/mod.rs          — modify: mod put; re-exports; //! role count
crates/herdrify-hc/src/protocol/methods/mod.rs                   — modify: re-export the four new requests
crates/herdrify-hc/src/protocol/types/terminal/put.rs            — create: PutResult (moved), PutBeginResult, PutPieceResult, EmptyResult
crates/herdrify-hc/src/protocol/types/terminal/results.rs        — modify: remove PutResult and its test
crates/herdrify-hc/src/protocol/types/terminal/mod.rs            — modify: mod put; re-exports; //! role count
crates/herdrify-hc/src/protocol/types/mod.rs                     — modify: re-export the three new replies
crates/herdrify-hc/src/protocol/mod.rs                           — modify: flat re-exports of the new requests and replies
```

## Order of work

### Task 1 — Move TerminalPut and PutResult into put.rs files

**Files:**
- Create `crates/herdrify-hc/src/protocol/methods/terminal/put.rs`, with the `//!` four-question
  block. Move in `TerminalPut`, its `Method` impl and its test `terminal_put_encodes_data`.
- Modify `crates/herdrify-hc/src/protocol/methods/terminal/interaction.rs`: remove them, and
  update its `//!` Responsibilities.
- Modify `crates/herdrify-hc/src/protocol/methods/terminal/mod.rs`: `mod put;`, and re-export
  `TerminalPut` from `put`. Its `//!` says "in two roles" (`:1-5`): make that three, adding
  sending a file.
- Create `crates/herdrify-hc/src/protocol/types/terminal/put.rs`, with the `//!` block. Move in
  `PutResult` and its decoding test. Widen its doc to "The reply to `terminal.put` and
  `terminal.put_finish`".
- Modify `crates/herdrify-hc/src/protocol/types/terminal/results.rs`: remove `PutResult`, and
  update its `//!` list.
- Modify `crates/herdrify-hc/src/protocol/types/terminal/mod.rs`: `mod put;`, and re-export
  `PutResult` from `put`. Its `//!` says "in four roles" (`:1-5`): make that five, adding the
  put replies.

**Change:** A move only. No behaviour or public path changes.

**Proof:**
- `terminal_put_encodes_data` (moved) and the moved `PutResult` decoding test still pass.
- `tests/schema_coverage.rs` and `tests/client_fake.rs` compile unchanged, which shows the public
  paths held.

**Done when:** `cargo test -p herdrify-hc --all-targets` passes, except the known
`live_schema_matches_the_fixture` failure on hc 0.1.22 (spec P1, fixed in plan 05 Task 2). The
Definition of Done is green with that same one exception. Commit `refactor(herdrify-hc): group
the put types in put.rs files`.

### Task 2 — Add the four requests

**Files:**
- Modify `crates/herdrify-hc/src/protocol/methods/terminal/put.rs`.
- Modify `crates/herdrify-hc/src/protocol/methods/terminal/mod.rs`,
  `crates/herdrify-hc/src/protocol/methods/mod.rs` and `crates/herdrify-hc/src/protocol/mod.rs`:
  re-export `TerminalPutBegin`, `TerminalPutChunk`, `TerminalPutFinish` and `TerminalPutAbort`.

**Change:** Four structs with private fields, each deriving `Debug`, `Clone`, `PartialEq`, `Eq`
and `Serialize` as `TerminalPut` does (`interaction.rs:389`), and each with a `Method` impl:

| Type | `NAME` | Constructor | Params | `Output` |
|---|---|---|---|---|
| `TerminalPutBegin` | `terminal.put_begin` | `new(terminal: TerminalRef, name: FileName, bytes: u64, transfer: TransferId)` | `terminal`, `name`, `bytes`, `transfer` | `PutBeginResult` |
| `TerminalPutChunk` | `terminal.put_chunk` | `new(transfer: TransferId, offset: u64, data: Vec<u8>)` | `transfer`, `offset`, `data` | `PutPieceResult` |
| `TerminalPutFinish` | `terminal.put_finish` | `new(transfer: TransferId)` | `transfer` | `PutResult` |
| `TerminalPutAbort` | `terminal.put_abort` | `new(transfer: TransferId)` | `transfer` | `EmptyResult` |

- `data` uses `crate::protocol::base64::serialize`, as `TerminalPut` does.
- `TerminalPutBegin`'s doc says the terminal must be on this core, and points to `TerminalPut`
  for a terminal on another machine (spec P13).
- `# Examples` on each.

**Proof:**
- `put_begin_serializes_its_four_params`: the JSON has exactly `bytes`, `name`, `terminal` and
  `transfer`, with the given values, and `NAME` is `terminal.put_begin`.
- `put_chunk_encodes_data_as_base64`: `data` `b"hello"` is sent as `"aGVsbG8="`, with `offset`
  and `transfer`, and `NAME` is `terminal.put_chunk`.
- `put_finish_and_abort_send_only_the_transfer`: each serializes to `{"transfer": …}`, with
  `NAME` `terminal.put_finish` and `terminal.put_abort`.

**Done when:** `cargo test -p herdrify-hc --lib protocol::methods::terminal` passes. The
Definition of Done is green except the known `live_schema_matches_the_fixture` failure (Task 1).
Commit `feat(herdrify-hc): add the put-in-pieces requests`.

### Task 3 — Add the three replies

**Files:**
- Modify `crates/herdrify-hc/src/protocol/types/terminal/put.rs`.
- Modify `crates/herdrify-hc/src/protocol/types/terminal/mod.rs`,
  `crates/herdrify-hc/src/protocol/types/mod.rs` and `crates/herdrify-hc/src/protocol/mod.rs`:
  re-export `PutBeginResult`, `PutPieceResult` and `EmptyResult`.

**Change:**
- `PutBeginResult { pub transfer: TransferId }`.
- `PutPieceResult { pub bytes: u64 }`, the bytes received so far.
- `EmptyResult {}`, the schema `Empty`. Its doc says only `terminal.put_abort` returns it today.
- All three derive `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize` and `Deserialize`, with
  `# Examples`.

**Proof:**
- `put_begin_result_decodes_and_validates_the_id`: `{"transfer":"0123456789abcdef"}` decodes,
  and `{"transfer":"x"}` fails to decode.
- `put_piece_result_decodes_bytes`: `{"bytes":5}` decodes to 5.
- `empty_result_reads_and_writes_an_empty_object`: `{}` and `{"later":1}` decode, `null` does
  not, and serializing gives `{}`.

**Done when:** `cargo test -p herdrify-hc --lib protocol::types::terminal` passes. The
Definition of Done is green except the known `live_schema_matches_the_fixture` failure (Task 1).
Commit `feat(herdrify-hc): add the put-in-pieces replies`.

---

## Risks

- The move in Task 1 touches six files. A missed re-export shows up as a compile error in
  `tests/`. That is caught at once, and it is why Task 1 runs the whole `--all-targets` suite.
- `tests/schema_coverage.rs` asserts that every fixture schema was checked. Against the 0.1.21
  fixture it has no new schemas yet, so nothing changes there until plan 05.

## Proof

- `cargo test -p herdrify-hc --all-targets` and `cargo test -p herdrify-hc --doc` pass.
- `cargo clippy -p herdrify-hc --all-targets --all-features -- -D warnings` is clean.
