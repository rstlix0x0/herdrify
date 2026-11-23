---
status: approved
created: 2026-11-23
depends-on: [08]
built-on: cba93d6df53e
---

# Stream Adapter Implementation Plan

**Goal:** Implement the transport port over any async byte pair with `StreamTransport`.

**Architecture:**
- The `adapter` layer (spec §2). It depends on `port` and `protocol`, never on `connection`, and uses tokio `io-util`.
- `StreamTransport<R: AsyncRead, W: AsyncWrite>` splits into `StreamSink<W>` and `StreamSource<R>` (spec §7.1). Its `AsyncRead`/`AsyncWrite` bounds are the only tokio types in the public API (spec §2, M-DONT-LEAK-TYPES).
- The crate root re-exports `StreamTransport` with `#[doc(inline)]`.
- `adapter/mod.rs` is export-only.
- The integration test uses `tokio::io::duplex`.
- **Conventions for every new file** (spec §2): it opens with the four-question `//!` block; tests return `Result`, because the workspace lints deny `unwrap`, `expect` and `panic`; every `mod.rs` is export-only and cites unit-test exemption #1 in its `//!`; every public item is re-exported flat from its layer module, or `unreachable_pub` fails the gate.

**Tech Stack:** Rust 1.94.1, tokio 1.54.1 (`io-util`; dev: `macros`, `rt-multi-thread`).

**Content authority:** spec §3 (W4, W7), §4.2, §7.1, §8.2 (rows owned by `adapter::stream`), §9.4 (`adapter_stream.rs`).

---

## Files that change

```
crates/herdrify-hc/src/lib.rs              — modify: pub mod adapter; StreamTransport re-export
crates/herdrify-hc/src/adapter/mod.rs      — create: export-only; re-exports StreamTransport, StreamSink, StreamSource
crates/herdrify-hc/src/adapter/stream.rs   — create: StreamTransport, StreamSink, StreamSource
crates/herdrify-hc/tests/adapter_stream.rs — create: StreamTransport over tokio::io::duplex
```

## Order of work

### Task 1 — Add StreamTransport

**Files:**
- Create `crates/herdrify-hc/src/adapter/mod.rs`, `crates/herdrify-hc/src/adapter/stream.rs`.
- Modify `crates/herdrify-hc/src/lib.rs`.

**Change:**
- `StreamTransport::new(reader, writer)` with `R: AsyncRead + Unpin + Send + 'static` and `W: AsyncWrite + Unpin + Send + 'static`.
- `send_line` writes the line, then `\n`, then flushes. `recv_line` reads through `LineDecoder`, with no length cap. The doc comment says the SDK trusts the peer the caller chose and is not hardened against a hostile peer in line mode (spec §7.1).
- `into_framed` moves the `LineDecoder`'s leftover bytes into `StreamSource`'s `FrameDecoder` (spec §4.2).
- `StreamSink::send` encodes and writes one frame, then flushes. `StreamSink::close` shuts the writer down.
- `StreamSource::recv` reads through `FrameDecoder`. A decoder `TooLarge` is `TransportError::Frame`, and a read error is `TransportError::Io`.
- **End of stream mid-message.** End of stream inside a line or a frame discards the partial bytes and returns `Ok(None)`, the same as a clean close (spec §7.1).

**Proof** (colocated unit tests over in-memory readers):
- `send_line_appends_newline` — sending `{}` writes `{}\n`.
- `recv_line_returns_none_on_clean_eof` — an empty reader gives `Ok(None)`.
- `recv_line_discards_a_partial_line_at_eof` — the reader `{"id":0` with no newline gives `Ok(None)`.
- `recv_discards_a_partial_frame_at_eof` — after `into_framed`, 5 header bytes then end of stream give `Ok(None)`.
- `recv_reports_an_oversized_header` — a header with `len` 16777217 gives `Err(TransportError::Frame(FrameError::TooLarge { len: 16777217 }))`.
- `sink_encodes_frames` — sending a CREDIT frame for channel 1 writes the 9-byte header and a 4-byte payload.

**Done when:** `cargo test -p herdrify-hc adapter::stream` passes and the Definition of Done is green (`cargo make dod` exits 0; the log grep of `CLAUDE.md` prints nothing). Commit `feat(herdrify-hc): add StreamTransport`.

### Task 2 — Test StreamTransport over a duplex pipe

**Files:**
- Create `crates/herdrify-hc/tests/adapter_stream.rs`.

**Change:** No source change. The test plays the core's side of a `tokio::io::duplex` pair, using only the public API.

**Proof:**
- `hello_line_then_frames_in_one_write` — the peer writes the hello reply line and two encoded DATA frames in one write. `recv_line` yields the line, and after `into_framed` the source yields both frames, in order (W4).
- `eof_inside_a_line_is_peer_closed` — the peer writes half a line and drops its end; `recv_line` gives `Ok(None)`.
- `eof_inside_a_frame_is_peer_closed` — after the switch, the peer writes half a frame and drops its end; `recv` gives `Ok(None)`.
- `sink_close_reaches_the_peer_as_eof` — after `close()`, the peer's read returns 0 bytes.

**Done when:** `cargo test -p herdrify-hc --test adapter_stream` passes and the Definition of Done is green. Commit `test(herdrify-hc): test StreamTransport over a duplex pipe`.

---

## Risks

- **Flushing.** A missing flush would leave a request in a buffer and hang a call. Every send flushes.
- **Riskiest step:** the hand-off in `into_framed`. Task 2's first test covers it end to end.

## Proof

- `grep -rn 'crate::connection' crates/herdrify-hc/src/adapter` prints nothing.
- The Definition of Done is green.
