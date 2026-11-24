---
status: done
created: 2026-11-24
depends-on: [02, 03]
built-on: 2db7af9df077
---

# Put Calls Implementation Plan

**Goal:** Make the put in pieces reachable through `Client` and `HerdrCore`.

**Architecture:**
- `Client` gains one typed method per request, sent through the existing generic `call`.
- `HerdrCore` gains one-call builders:
  - `TerminalHandle::put_begin`
  - `TerminalOps::put_chunk`, `put_finish` and `put_abort`, which name no terminal
- The multi-call procedure `TerminalHandle::put_in_pieces` lives in its own file,
  `herdr_core/put.rs` (spec §2.2).
- `depends-on: [03]` only orders the work, because both plans edit `client.rs` and
  `tests/client_fake.rs`.

**Tech Stack:** Rust 1.94, tokio 1.54.1, serde_json 1.0.154.

**Content authority:** spec §4.1, §5.1, §5.2, §6, §7, §8 (client_fake).

---

## Files that change

```
crates/herdrify-hc/src/connection/client.rs     — modify: terminal_put_begin, terminal_put_chunk, terminal_put_finish, terminal_put_abort
crates/herdrify-hc/src/herdr_core/terminals.rs  — modify: TerminalHandle::put_begin and pub(super) ops(); TerminalOps::put_chunk, put_finish, put_abort; invoke_by! entries
crates/herdrify-hc/src/herdr_core/put.rs        — create: TerminalHandle::put_in_pieces, PIECE_BYTES, piece_ranges, refused_by_core
crates/herdrify-hc/src/herdr_core/mod.rs        — modify: mod put; //! TerminalOps row
crates/herdrify-hc/tests/client_fake.rs         — modify: the put calls, put_in_pieces, and the Send checks
```

## Order of work

### Task 1 — Add the four Client methods

**Files:**
- Modify `crates/herdrify-hc/src/connection/client.rs`.
- Modify `crates/herdrify-hc/tests/client_fake.rs`.

**Change:**
- `terminal_put_begin(TerminalPutBegin) -> Result<PutBeginResult, Error>`,
  `terminal_put_chunk(TerminalPutChunk) -> Result<PutPieceResult, Error>`,
  `terminal_put_finish(TerminalPutFinish) -> Result<PutResult, Error>` and
  `terminal_put_abort(TerminalPutAbort) -> Result<EmptyResult, Error>`.
- Each calls `self.call::<M>(params)`, as `terminal_put` does.
- Each has `# Errors`: those of `call_raw`, plus `Error::Decode`. `terminal_put_begin`'s doc says
  the terminal must be on this core.
- Each has `# Examples` (`no_run`, since it needs a running core).

**Proof** (`tests/client_fake.rs`):
- `put_calls_send_their_method_and_params`: for each of the four, the written request has the
  method name and params, and the scripted reply decodes into the right type.
- `put_refusal_keeps_the_core_kind`: an `unknown_method` reply to `terminal.put_begin` gives
  `Error::Rpc` with `ErrorKind::UnknownMethod`.
- `client_and_its_futures_are_send` (existing): extended with the four new futures.

**Done when:** `cargo test -p herdrify-hc --test client_fake` passes. The Definition of Done is
green except `live_schema_matches_the_fixture`, the one known failure on hc 0.1.22 (spec P1),
which plan 05 Task 2 fixes. Commit `feat(herdrify-hc): add the put-in-pieces calls to Client`.

### Task 2 — Add the HerdrCore builders

**Files:**
- Modify `crates/herdrify-hc/src/herdr_core/terminals.rs`.
- Modify `crates/herdrify-hc/src/herdr_core/mod.rs` (the `//!` table only).

**Change:**
- `TerminalHandle::put_begin(name: impl Into<String>, bytes: u64, transfer: TransferId) ->
  Call<T, TerminalPutBegin>`, built on this handle's reference. `name` becomes a
  `FileName::new` unchecked.
- `TerminalOps::put_chunk(transfer: TransferId, offset: u64, data: impl Into<Vec<u8>>) ->
  Call<T, TerminalPutChunk>`, `put_finish(transfer) -> Call<T, TerminalPutFinish>` and
  `put_abort(transfer) -> Call<T, TerminalPutAbort>`.
- `invoke_by!` entries tie each request to its `Client` method.
- Docs:
  - `put_begin` says the terminal must be on this core. A handle for `m/t7` is refused by the
    core with `machine_unknown`; use `put` for such a terminal.
  - each has `# Examples` (`no_run`, since it needs a running core).
- `TerminalHandle` gains `pub(super) fn ops(&self) -> TerminalOps<T>`, built with
  `TerminalOps::new(self.client.clone())`. `put.rs` uses it to reach `put_chunk`, `put_finish`
  and `put_abort`. Being `pub(super)` keeps it inside `herdr_core`, so `unreachable_pub` does not
  fire.
- `herdr_core/mod.rs`'s `//!` table lists what `TerminalOps` covers (`:15`). Add `put_chunk`,
  `put_finish` and `put_abort` there.

**Proof** (colocated in `terminals.rs`, over `herdr_core/fake.rs`):
- `put_begin_names_this_terminal`: `handle.put_begin("a.txt", 3, id)` sends
  `{"terminal":<handle ref>,"name":"a.txt","bytes":3,"transfer":<id>}`.
- `put_chunk_finish_abort_send_the_transfer`: each sends its method with the given transfer
  (and `offset` and base64 `data` for the chunk).
- `handle_ops_share_the_handle_client`: a `put_abort` through `handle.ops()` is sent on the
  handle's connection.

**Done when:** `cargo test -p herdrify-hc --lib herdr_core::terminals` passes. The Definition of
Done is green except the known `live_schema_matches_the_fixture` failure (Task 1). Commit
`feat(herdrify-hc): add the put-in-pieces builders to HerdrCore`.

### Task 3 — Add put_in_pieces

**Files:**
- Create `crates/herdrify-hc/src/herdr_core/put.rs`, with the `//!` four-question block.
- Modify `crates/herdrify-hc/src/herdr_core/mod.rs`: `mod put;`. It exports nothing new; the
  method is on `TerminalHandle`. It reaches `TerminalOps` through `ops()` from Task 2.
- Modify `crates/herdrify-hc/tests/client_fake.rs`.

**Change:**
- `const PIECE_BYTES: usize = 1_048_576`. Its comment says this is the largest piece
  herdr-core 0.1.22 accepts: "at most 1 MB per piece" refuses 1048577.
- `impl<T: Transport> TerminalHandle<T> { pub async fn put_in_pieces(&self, name: impl
  Into<String>, data: impl Into<Vec<u8>>) -> Result<PutResult, HerdrCoreError> }`. It follows
  spec §5.2:
  1. `id = TransferId::random()`.
  2. `put_begin(name, len, id)`. If the core refused it (`HerdrCoreError::Call` holding
     `Error::Rpc`), return that error with no abort. On any other error, `put_abort(id)`,
     ignoring its result, then return the first error.
  3. For each `PIECE_BYTES` slice, in order, at `offset` = bytes sent so far: `put_chunk`. On
     any error, abort as above and return the chunk's error. An empty `data` sends no piece.
  4. `put_finish(id)`. On any error, abort as above and return the error.
  5. Return the `PutResult`.
- Two private helpers, each unit-tested in `put.rs`:
  - `fn piece_ranges(len: usize) -> impl Iterator<Item = Range<usize>>`: the slices, in order,
    of at most `PIECE_BYTES` each
  - `fn refused_by_core(error: &HerdrCoreError) -> bool`: true only for `HerdrCoreError::Call`
    holding `Error::Rpc`
- The doc has `# Errors`: any `HerdrCoreError` from the calls, the first one, after the abort
  rules. It also covers:
  - the 8 MiB limit, enforced by the core
  - this core's terminals only, with `put` for other machines
  - the abort rules
  - a dropped future sends no abort, and the core drops the put when the connection or terminal
    goes
  - `# Examples` (`no_run`)

**Proof:**

Colocated in `put.rs` (unit-test-mandate):
- `piece_ranges_split_at_one_mebibyte`: lengths 0, 1, 1048576 and 1048577 give `[]`, `[0..1]`,
  `[0..1048576]` and `[0..1048576, 1048576..1048577]`.
- `refused_by_core_tells_rpc_from_other_errors`: true for `Call(Rpc)`, false for `Call(Decode)`,
  `Call(Closed)` and `Empty`.

In `tests/client_fake.rs`, over `HerdrCore::over(FakeTransport)`:
- `put_in_pieces_splits_at_one_mebibyte`: for 0, 1, 1048576 and 1048577 bytes, the requests are
  begin with `bytes` = the length, then 0, 1, 1 and 2 pieces at offsets 0 and 1048576, then
  finish. The `PutResult` of the finish is returned.
- `put_in_pieces_aborts_after_a_refused_piece`: a `finished` reply to the first chunk is
  followed by `put_abort` with the begin's id, and the returned error is that `finished`
  refusal.
- `put_in_pieces_aborts_after_a_refused_finish`: a refused finish is followed by `put_abort`.
- `put_in_pieces_sends_no_abort_when_begin_is_refused`: a `machine_unknown` begin reply returns
  that error, and `nothing_sent()` holds after it.
- `put_in_pieces_aborts_when_begin_does_not_decode`: a begin reply `{"transfer":"x"}` gives
  `Error::Decode`, and a `put_abort` follows.
- `put_in_pieces_ignores_a_failed_abort`: a refused abort leaves the chunk's error as the one
  returned.
- `client_and_its_futures_are_send` (existing): extended with the `put_in_pieces` future.

**Done when:** `cargo test -p herdrify-hc --test client_fake` and
`cargo test -p herdrify-hc --lib herdr_core::put` pass. The Definition of Done is green except
the known `live_schema_matches_the_fixture` failure (Task 1). Commit
`feat(herdrify-hc): add TerminalHandle::put_in_pieces`.

---

## Risks

- **Fake scripting is the riskiest step.** `FakePeer::request()` returns the next written
  request, and `reply(id, …)` answers it. A test that replies out of order hangs instead of
  failing. Bound each test with `tokio::time::timeout`, as the existing tests do.
- **The begin's id.** `put_in_pieces` makes a random id, so the tests read it from the written
  begin request before checking the abort, instead of predicting it.
- **The Rpc check.** Telling a core refusal of begin from other errors needs a match through
  `HerdrCoreError::Call(Box<Error>)`. That is `refused_by_core`, tested in Task 3.

## Proof

- `cargo test -p herdrify-hc --all-targets` and `cargo test -p herdrify-hc --doc` pass.
- `cargo clippy -p herdrify-hc --all-targets --all-features -- -D warnings` is clean.

## Review findings

Reviewer verdict: SPEC COMPLIANT, 0🔴 1🟡 5🔵, nothing blocking. DoD re-run green (`-p herdrify-hc`: fmt, clippy -D warnings, doc -D warnings, lib 356, doc 96, client_fake 106, schema_coverage 7).

- 🟡 `tests/client_fake.rs:1109-1111` the `TRANSFER_FINISHED` fixture was invented ("the transfer is already finished") — fixed: it now carries the reply herdr-core 0.1.22 sent in probe E3, `{"code":-32000,"data":{"hint":"nothing there to read it","kind":"finished"},"message":"t3 has finished"}`, described as the refusal of a piece put to a terminal that ended mid-put. `cargo test -p herdrify-hc --test client_fake` → `test result: ok. 106 passed`.
- 🔵 `tests/client_fake.rs:1008` `put_calls_send_their_method_and_params` connected with a 0.1.21 hello — fixed: `connected_at("0.1.22")`. Same command, 106 passed.
- 🔵 `herdr_core/put.rs:53-58` the abort list left out a begin that fails without a core refusal — fixed. `cargo test -p herdrify-hc --doc` → `test result: ok. 96 passed`.
- 🔵 `client.rs:496,523,549,571` `# Errors` before `# Examples` — fixed crate-wide: 11 items reordered to the M-CANONICAL-DOCS order (Examples, Errors, Panics, Safety) in `client.rs` (5), `transfer.rs` (4), `herdr_core/put.rs` (1), `identifiers.rs` (1); sections moved whole, no wording changed. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` clean.
- 🔵 `client.rs:507` the only `.expect` in a `client.rs` doc example — fixed: the example uses `?`. `cargo test -p herdrify-hc --doc` → 98 passed.
- 🔵 `herdr_core/put.rs:56-57` "the core drops the put when the connection goes" rested on the core's hint text — fixed: the doc states the observed terminal case and attributes the connection case to the core's own refusal text, quoted verbatim ("it was aborted, or its terminal or connection went").
- Noted by the fix coder: `put_in_pieces_aborts_after_a_refused_finish` used a piece refusal as a stand-in — fixed: new `FINISH_TOO_EARLY` fixture, the reply 0.1.22 recorded for a finish before all bytes (`{"code":-32602,"data":{"hint":"see `hc schema`","kind":"invalid_params"},"message":"got 0 of 3 bytes"}`), and the test asserts kind `InvalidParams`. `put_in_pieces_ignores_a_failed_abort` keeps an arbitrary abort refusal, commented as such: 0.1.22 answers `{}` to every abort, so no recorded abort refusal exists.

## Gate

With hc 0.1.22 on `PATH`, after all four plans:
- `(set -o pipefail; cargo make dod 2>&1 | tee target/dod.log)` → exit 105, with exactly one failure: `live_schema_matches_the_fixture` (`crates/herdrify-hc/tests/live.rs:626`), the known failure plan 05 Task 2 fixes. The log has no `warning`/`error` line except cargo's `error: test failed, to rerun pass -p herdrify-hc --test live`.
- `cargo make dod` stops at the first failing test binary, so the rest ran as `cargo test --workspace --all-targets --all-features --no-fail-fast` plus `--doc`: every other target ok (herdrify-hc lib 356, client_fake 106, live 15 of 16, live_herdr_core 5, schema_coverage 7, adapter_stream 4; herdrify-app lib 253, live 10; doc tests 447 + 29 + 96), `error: 1 target failed` (the drift test).
- After the last fix: `cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` clean.
- Leftovers: `/tmp/hfa48416-0`, `/tmp/hfa48416-1` and `/tmp/hfy.4K8V` date from 2026-11-23 (before this run); no `--hc-launchd` core of an `hfy` session runs.

## Deviations

- 2026-11-24: Task 2 added `#[cfg_attr(not(test), expect(dead_code, ..))]` on `TerminalHandle::ops()` (no non-test caller until Task 3); Task 3 removed it. Clippy is clean with no unfulfilled expectation.
- 2026-11-24: `refused_by_core` is a named wrapper over the existing `HerdrCoreError::refusal().is_some()`, which is true exactly for `Call(Error::Rpc)`.
- 2026-11-24: The gate was run with `--no-fail-fast` after `cargo make dod` stopped at the known failure, so every target's result is recorded.
- 2026-11-24: Not committed (instruction: do not commit).
