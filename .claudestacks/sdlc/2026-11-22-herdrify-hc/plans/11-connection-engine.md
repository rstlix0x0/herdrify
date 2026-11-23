---
status: superseded
created: 2026-11-22
depends-on: [06, 08]
built-on: ca0f990f7018
---

# Connection Engine Implementation Plan

**Goal:** Give `Client` a framed connection over any `Transport` that carries concurrent raw calls from connect to connection end.

**Architecture:**
- The `connection` layer, the application (spec §2). It depends on `port` and `protocol`, never on `adapter`, and uses tokio `rt` and `sync`.
- `Client<T: Transport>` is generic, with no `Box<dyn …>`. It is `Clone` through an internal `Arc` and `Send + Sync`, and has no constructor that names an adapter (spec §2, §6.1).
- After the hello exchange, `into_framed()` splits the transport:
  - the **writer** task owns the `FrameSink` and takes commands (a frame plus an optional oneshot acknowledgement) from an unbounded `mpsc`
  - the **router** task owns the `FrameSource` (spec §6.5)
- Shared state that the writer and router both reach:
  - the pending-call table (`pending.rs`)
  - the first-wins connection-end record (`connection_end.rs`)
- The router holds **no** writer sender. Otherwise the writer's channel would never close when the last `Client` drops (spec §6.5 Lifetime).
- `connection/memory_link.rs` is a `#[cfg(test)]` in-memory `Transport` (lines, then a `FrameSink`/`FrameSource` pair) for colocated unit tests, with no adapter (spec §9.1 "Client logic"). It is a file added inside the approved layer; the dependency rule is unchanged.
- **Dead code.** No crate-private item lands before its first non-test use, because `dead_code` warnings fail the gate. So Task 2 builds the writer, router, pending table and `call_raw` together.
- Integration tests use `tests/support/fake_transport.rs`, which implements the public `Transport` from outside the crate (spec §9.2). Test binaries include it with `#[path]`.
- The crate root re-exports `Client` and `Error` with `#[doc(inline)]`.

**Tech Stack:** Rust 1.94.1, tokio 1.54.1 (`rt`, `sync`; dev: `macros`, `rt-multi-thread`, `time`), serde_json 1.0.154, thiserror 2.0.21.

**Content authority:** spec §3 (W6, W9, W18, W19, W21, W22), §6.1, §6.2, §6.5 (writer, router, connection end, lifetime), §8.1 (`Error`, `ConnectionEnd` rows), §8.2 (rows owned by `client::client`, `client::router`, `client::writer` that concern calls), §9.1 (Writer, Client logic), §9.2 (`fake_transport.rs`), §9.4 (connect refusals, connection loss for calls).

---

## Files that change

```
crates/herdrify-hc/src/lib.rs                       — modify: pub mod connection; Client and Error re-exports
crates/herdrify-hc/src/connection/mod.rs                — create: export-only table of contents
crates/herdrify-hc/src/connection/client_error.rs       — create: Error (every variant of spec §8.1)
crates/herdrify-hc/src/connection/connection_end.rs     — create: ConnectionEnd, ConnectionEndKind, the first-wins end record
crates/herdrify-hc/src/connection/pending.rs            — create: request-id allocator and pending-call table
crates/herdrify-hc/src/connection/writer.rs             — create: the spawned writer task and its command sender
crates/herdrify-hc/src/connection/router.rs             — create: the spawned router task
crates/herdrify-hc/src/connection/client.rs             — create: Client<T>: connect, hello, call_raw, close_reason
crates/herdrify-hc/src/connection/memory_link.rs        — create: #[cfg(test)] in-memory FrameSink/FrameSource pair
crates/herdrify-hc/tests/support/fake_transport.rs  — create: scripted fake Transport and its peer handle
crates/herdrify-hc/tests/client_fake.rs             — create: connect refusals, raw calls, connection loss
```

## Order of work

### Task 1 — Add the client Error and ConnectionEnd

**Files:**
- Create `crates/herdrify-hc/src/connection/mod.rs`, `client_error.rs`, `connection_end.rs`.
- Modify `crates/herdrify-hc/src/lib.rs` — `pub mod connection;` and the `Error` re-export.

**Change:**
- `Error` has every variant of spec §8.1 with its exact `Display`:
  - `Transport(TransportError)`
  - `Rpc { method: MethodName, error: RpcError }`
  - `Decode { method: MethodName, source: serde_json::Error }`
  - `Protocol(ProtocolError)`
  - `UnsupportedGeneration { message }`
  - `UnsupportedVersion { found, minimum }`
  - `InvalidVersion { found }`
  - `PayloadTooLarge { len }`
  - `Closed`
  - `StreamClosed`
- `ConnectionEnd { kind: ConnectionEndKind, message: String }` is `Clone` and displays `{kind}: {message}`. `ConnectionEndKind::{PeerClosed, TransportFailed, ProtocolViolation}` displays `peer closed`, `transport failed` and `protocol violation`.
- All are `Send + Sync + 'static`, with `source()` chained.

**Proof:**
- `error_display_texts` — each variant displays its spec §8.1 text, including `terminal.get refused by herdr-core: no terminal "t999" [not_found]`, `herdr-core 0.1.20 is older than the supported minimum 0.1.21`, `herdr-core reported version "v1", which is not MAJOR.MINOR.PATCH`, `payload of 16777217 bytes exceeds the 16777216-byte frame limit`, `the connection to herdr-core is closed` and `the terminal.attach stream is closed`.
- `connection_end_display` — `ConnectionEnd { kind: PeerClosed, message: "x" }` displays `peer closed: x`.
- `errors_are_send_sync_static` — a compile-time bound check for `Error` and `ConnectionEnd`.

**Done when:** `cargo test -p herdrify-hc client_error` and `cargo test -p herdrify-hc connection_end` pass and the Definition of Done is green (`cargo make dod` exits 0; the log grep of `CLAUDE.md` prints nothing). Commit `feat(herdrify-hc): add the client Error and ConnectionEnd`.

### Task 2 — Connect and run raw calls

**Files:**
- Create `crates/herdrify-hc/src/connection/pending.rs`, `writer.rs`, `router.rs`, `client.rs`, `memory_link.rs`.
- Create `crates/herdrify-hc/tests/support/fake_transport.rs`, `crates/herdrify-hc/tests/client_fake.rs`.
- Modify `crates/herdrify-hc/src/connection/mod.rs`, `crates/herdrify-hc/src/lib.rs` — the `Client` re-export.

**Change:**
- `Client::connect(transport)` runs the eight steps of spec §6.1 in order. Each refusal is its listed `Error`, and the transport is dropped on any failure. Step 8 calls `into_framed()` and spawns the writer and the router.
- `Client::hello()` returns the stored `HelloResult`.
- `pending.rs` allocates `RequestId`s from 1 upward and keeps the table of oneshot senders. The handshake used id 0 (spec §4.4).
- `Client::call_raw(MethodName, Value) -> Result<Value, Error>` follows spec §6.2's sequence: allocate, register, serialize, hand the channel-0 DATA frame to the writer, await its acknowledgement, then await the reply. An `Err(RpcError)` reply is `Error::Rpc { method, error }`.
- Dropping a call future removes its pending entry, and a late reply is discarded (spec §6.2).
- The writer writes each command's frame and sends the result on its acknowledgement when it has one.
- The router classifies channel-0 DATA (spec §4.4):
  - a response goes to its pending entry; an unknown id or `id:null` is discarded
  - a channel-0 notification is discarded
  - a channel-0 payload that classifies as `InvalidJson` or `NotJsonRpc` is discarded, and the connection stays open (spec §6.5)
  - every frame on any other channel, and every CREDIT, FOCUS, FACTS, AUTOMATIC_INPUT or unknown kind, is discarded (spec §6.5; no stream exists yet)
- `fake_transport.rs` provides `fake_pair() -> (FakeTransport, FakePeer)`. `FakePeer` scripts the hello reply line, reads each frame the client sent, pushes frames back, and ends the stream. The two failure helpers (a scripted source `TransportError`, a sink that fails its next write) arrive in Task 3 with their first use, so no test binary carries an unused helper.

**Proof:**
- `connect_succeeds_on_a_valid_hello` (client_fake) — the client sends exactly `{"id":0,"jsonrpc":"2.0","method":"hello","params":{"framed":true,"generation":1}}` (key order free), and `hello().version` is `0.1.21`.
- `connect_refusals` (client_fake), one case each:
  - EOF → `Error::Closed`
  - non-JSON → `Protocol(InvalidJson)`
  - `[1,2]` → `Protocol(NotJsonRpc)`
  - a reply with id 4 → `Protocol(UnexpectedHelloReply)`
  - a reply with `id:null` → `Protocol(UnexpectedHelloReply)` displaying `… got a response with id null`
  - a notification → `Protocol(UnexpectedHelloReply)`
  - the W19 `unsupported_generation` error → `UnsupportedGeneration` with its message
  - another error reply → `Rpc` with method `hello`
  - an undecodable result → `Decode` with method `hello`
  - `framed:false` → `Protocol(HelloMismatch)`
  - `generation: 2` with `framed:true` → `Protocol(HelloMismatch)`
  - version `0.1.x` → `InvalidVersion`
  - version `0.1.20` → `UnsupportedVersion`
- `concurrent_calls_resolve_out_of_order` (client_fake) — two `call_raw`s; the peer answers id 2 before id 1, and each call gets its own result (W6).
- `late_reply_after_drop_is_discarded` (client_fake) — a call future is dropped; the peer's reply to it is discarded, and a later call still works.
- `null_id_error_and_channel_zero_notification_are_discarded` (client_fake) — after a `-32700` reply with `id:null`, a channel-0 notification, a non-JSON payload and a `[1,2]` payload, a following call still resolves and `close_reason()` is `None`.
- `writer_acknowledges_a_write` (writer, unit) — a command with an acknowledgement over `memory_link` returns `Ok` once the frame reaches the sink.
- `router_routes_a_response_to_its_pending_entry` (router, unit) — a response frame for a registered id completes its oneshot; an unknown id leaves the table unchanged.
- `ids_start_at_one_and_increase` (pending, unit) — the first two allocated ids are 1 and 2.
- `removed_entry_is_not_completed` (pending, unit) — after `remove(id)`, completing that id reports no entry, and the table is empty.
- `connect_over_memory_link_sends_hello_id_zero` (client, unit) — over `memory_link`, `connect` writes the hello line with id 0 and then speaks frames.
- `call_raw_writes_channel_zero_data` (client, unit) — over `memory_link`, a `call_raw` writes one DATA frame on channel 0 holding the request with id 1.

**Done when:** `cargo test -p herdrify-hc connection` and `cargo test -p herdrify-hc --test client_fake` pass, and the Definition of Done is green. Commit `feat(herdrify-hc): connect a Client and run raw calls`.

### Task 3 — End the connection cleanly

**Files:**
- Modify `crates/herdrify-hc/src/connection/connection_end.rs` — the first-wins end record.
- Modify `crates/herdrify-hc/src/connection/writer.rs`, `router.rs`, `client.rs`, `pending.rs`.
- Modify `crates/herdrify-hc/tests/support/fake_transport.rs` — the two failure helpers.
- Modify `crates/herdrify-hc/tests/client_fake.rs`.

**Change:**
- The first of these four events ends the connection, mapped to its `ConnectionEndKind` by spec §6.5's table:
  - the source returns `None`
  - the source returns `TransportError::Io`
  - the source returns `TransportError::Frame`
  - the sink fails a write
- At the end:
  - the call whose write failed gets `Error::Transport(TransportError::Io(..))` and its pending entry is removed
  - every other pending call gets `Error::Closed`
  - later calls return `Error::Closed` without writing
  - the router stops
- `Client::close_reason() -> Option<ConnectionEnd>` is `None` while open, then the recorded end, whose `message` is the causing error's `Display`.
- **Payload check.** Before anything is sent, a call on an ended connection returns `Error::Closed`. Otherwise a payload over 16777216 bytes returns `Error::PayloadTooLarge { len }`, in that order (spec §6.2).
- **Lifetime.** When the last `Client` clone drops, the writer's channel closes and the writer calls `FrameSink::close`. The router reads `None` and ends (spec §6.5).
- `server.stop`-style endings need nothing extra: the reply is routed, then EOF ends the connection with `PeerClosed` (W22).

**Proof:**
- `eof_fails_pending_calls_with_closed` (client_fake) — with a call pending, the peer ends the stream; the call gets `Closed`, and `close_reason()` displays exactly `peer closed: herdr-core closed the connection` (spec §6.5).
- `failed_write_fails_that_call_with_transport` (client_fake) — the sink fails the next write; that call gets `Transport(Io)`, another pending call gets `Closed`, and `close_reason()` is `TransportFailed`.
- `oversized_frame_is_a_protocol_violation` (client_fake) — the source returns `Frame(TooLarge)`; pending calls get `Closed`, and `close_reason()` is `ProtocolViolation` with message `frame payload of 16777217 bytes exceeds the 16777216-byte limit`.
- `calls_after_the_end_return_closed_without_writing` (client_fake) — after EOF, `call_raw` returns `Closed`, and the peer records no new frame.
- `oversized_call_is_refused_before_sending` (client_fake) — a `call_raw` whose params serialize past 16777216 bytes returns `PayloadTooLarge`, the peer records no frame, and a following small call succeeds.
- `closed_wins_over_payload_too_large` (client_fake) — on an ended connection, the oversized call returns `Closed`.
- `last_client_drop_closes_the_sink` (client_fake) — after dropping every clone, the peer observes the sink closed.
- `writer_failure_ends_the_connection` (writer, unit) — over `memory_link` with a failing sink, the acknowledgement is an error and the end record holds `TransportFailed`.
- `closed_is_checked_before_payload_size` (client, unit) — over `memory_link` after the end, a call with an oversized payload returns `Closed`; on an open link it returns `PayloadTooLarge`, and the link records no frame in either case.

**Done when:** `cargo test -p herdrify-hc connection` and `cargo test -p herdrify-hc --test client_fake` pass, and the Definition of Done is green. Commit `feat(herdrify-hc): end the client connection cleanly`.

---

## Risks

- **Riskiest step:** Task 2. Concurrency ordering between the writer, the router and the pending table is new code. The fake peer's out-of-order test is the main guard.
- **Leaked writer sender.** If any long-lived task keeps a writer sender, the sink never closes. `last_client_drop_closes_the_sink` catches it.
- **Tests that hang.** Integration tests that wait on the peer use `tokio::time::timeout`, so a bug fails the test instead of hanging the gate.

## Proof

- The four spec §2 grep checks print nothing.
- `cargo test -p herdrify-hc` passes.
- The Definition of Done is green.
