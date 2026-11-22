---
status: done
created: 2026-11-22
depends-on: [02]
built-on: ca0f990f7018
---

# Wire Codec Implementation Plan

**Goal:** Implement herdr-core's sans-IO wire codec, which converts between bytes and frames, lines or NDJSON lines.

**Architecture:** `protocol` layer files, pure and sans-IO (spec §2): bytes in, values out. No tokio. Each file has the four-question `//!` block and colocated unit tests. Every `pub` item is re-exported from `protocol/mod.rs`. The workspace lints deny `unwrap_used` and `panic`, test code included, so tests return `Result`.

**Tech Stack:** Rust 1.94.1, serde_json 1.0.154, thiserror 2.0.21.

**Content authority:** spec §3 (W4, W5, W7, W10), §4.1, §4.2, §4.3, §8.1 (`FrameError`, `ProtocolError` rows).

---

## Files that change

```
crates/herdrify-hc/src/protocol/mod.rs             — modify: mod and re-export lines for the new files
crates/herdrify-hc/src/protocol/frame_error.rs     — create: FrameError
crates/herdrify-hc/src/protocol/frame.rs           — create: Frame, FrameKind, Channel, Focus; header encode; control payloads
crates/herdrify-hc/src/protocol/frame_decoder.rs   — create: FrameDecoder (bytes in any chunking → frames)
crates/herdrify-hc/src/protocol/line.rs            — create: Line, LineDecoder and its hand-off to a FrameDecoder
crates/herdrify-hc/src/protocol/protocol_error.rs  — create: ProtocolError
crates/herdrify-hc/src/protocol/ndjson.rs          — create: NdjsonDecoder (stream DATA bytes → complete lines)
```

## Order of work

### Task 1 — Add frames and their encoding

**Files:**
- Create `crates/herdrify-hc/src/protocol/frame_error.rs` — `FrameError::{TooLarge { len }, BadControlPayload { kind, expected, got }}`.
- Create `crates/herdrify-hc/src/protocol/frame.rs` — the frame types.
- Modify `crates/herdrify-hc/src/protocol/mod.rs` — `mod` and re-export lines.

**Change:**
- `Frame { channel: Channel, kind: FrameKind, payload: Vec<u8> }`. `Channel(u32)` with `Channel::RPC` = 0. `FrameKind` is `Data` 0, `Credit` 1, `Close` 2, `Resize` 3, `Focus` 4, `Facts` 5, `AutomaticInput` 6, and `Unknown(u8)` for any other byte (W5, W9). `Focus::{Gained, Lost}`.
- `Frame::encode` writes `[channel u32 LE][kind u8][len u32 LE][payload]`. A payload over 16777216 bytes returns `FrameError::TooLarge { len }` before anything is written (spec §4.1).
- Typed constructors `Frame::credit(channel, u32)`, `Frame::resize(channel, u16, u16)`, `Frame::focus(channel, Focus)`, and readers for the same three, per the spec §4.1 table (4, 4 and 1 bytes; focus `1` gained, `0` lost). A reader given the wrong payload length returns `FrameError::BadControlPayload { kind, expected, got }`.
- `Display` texts exactly as spec §8.1: `frame payload of {len} bytes exceeds the 16777216-byte limit` and `{kind} frame payload must be {expected} bytes, got {got}`, where `kind` prints as the upper-case kind name (`CREDIT frame payload must be 4 bytes, got 3`).

**Proof:**
- `encode_writes_the_nine_byte_header` — a DATA frame on channel 0 with payload `{}` encodes to `00 00 00 00 | 00 | 02 00 00 00 | 7b 7d`.
- `kind_bytes_map_both_ways` — bytes 0..=6 map to the seven named kinds and back; byte 9 is `Unknown(9)` and encodes as 9.
- `encode_accepts_16777216_and_refuses_16777217` — a 16777216-byte payload encodes; 16777217 is `TooLarge { len: 16777217 }`.
- `control_payloads_round_trip` — credit 262144, resize 80×24 and both focus values read back what was built.
- `control_reader_refuses_wrong_length` — a 3-byte CREDIT payload is `BadControlPayload`, displaying `CREDIT frame payload must be 4 bytes, got 3`.
- `frame_error_display_too_large` — `TooLarge { len: 16777217 }` displays `frame payload of 16777217 bytes exceeds the 16777216-byte limit`.

**Done when:** `cargo test -p herdrify-hc frame` passes and the Definition of Done is green (`cargo make dod` exits 0; the log grep of `CLAUDE.md` prints nothing). Commit `feat(herdrify-hc): add frames and their encoding`.

### Task 2 — Add the frame decoder

**Files:**
- Create `crates/herdrify-hc/src/protocol/frame_decoder.rs` — `FrameDecoder`.
- Modify `crates/herdrify-hc/src/protocol/mod.rs` — `mod` and re-export lines.

**Change:** `FrameDecoder` takes bytes in any chunking (`push(&[u8])`) and yields complete frames (`next_frame() -> Result<Option<Frame>, FrameError>`). A header whose `len` exceeds 16777216 returns `FrameError::TooLarge`, and every later call returns that error again: the decoder is unusable afterwards (spec §4.1). `FrameDecoder::with_bytes(Vec<u8>)` starts from bytes already read, for the line hand-off of Task 3. `has_partial()` reports whether bytes of an unfinished frame are buffered, which the adapter's end-of-stream rule reads (spec §7.1).

**Proof:**
- `decodes_a_frame_split_across_every_byte` — one frame pushed one byte at a time yields exactly that frame, once.
- `decodes_several_frames_from_one_push` — two frames in one push yield both, in order.
- `oversized_header_poisons_the_decoder` — a header with `len` 16777217 yields `TooLarge`, and a following valid frame still yields `TooLarge`.
- `accepts_a_16777216_byte_payload` — a header with `len` 16777216 and its payload yields the frame.
- `reports_partial_bytes` — after pushing 5 header bytes, `has_partial()` is true; after a whole frame is taken, it is false.

**Done when:** `cargo test -p herdrify-hc frame_decoder` passes and the Definition of Done is green. Commit `feat(herdrify-hc): add the frame decoder`.

### Task 3 — Add lines and the line-to-frame hand-off

**Files:**
- Create `crates/herdrify-hc/src/protocol/line.rs` — `Line`, `LineDecoder`.
- Modify `crates/herdrify-hc/src/protocol/mod.rs` — `mod` and re-export lines.

**Change:** `Line` is one JSON text without its terminating `\n`, with `as_str` and `into_string` (W1). `LineDecoder` takes bytes in any chunking and yields complete lines, splitting on `\n` only, with no length cap (spec §7.1). `into_frame_decoder(self) -> FrameDecoder` hands every unconsumed byte to a `FrameDecoder`, so frames that arrive in the same read as the hello reply are kept (W4, spec §4.2). `has_partial()` reports whether an unfinished line is buffered.

**Proof:**
- `yields_lines_across_chunks` — `{"a":1}\n{"b"` then `:2}\n` yields two lines, without the `\n`.
- `hands_leftover_bytes_to_the_frame_decoder` — a hello reply line followed in the same push by one encoded frame yields the line, and `into_frame_decoder()` then yields that frame.
- `reports_a_partial_line` — after pushing `{"a"`, `has_partial()` is true.

**Done when:** `cargo test -p herdrify-hc line` passes and the Definition of Done is green. Commit `feat(herdrify-hc): add lines and the line-to-frame hand-off`.

### Task 4 — Add ProtocolError and the NDJSON decoder

**Files:**
- Create `crates/herdrify-hc/src/protocol/protocol_error.rs` — `ProtocolError`.
- Create `crates/herdrify-hc/src/protocol/ndjson.rs` — `NdjsonDecoder`.
- Modify `crates/herdrify-hc/src/protocol/mod.rs` — `mod` and re-export lines.

**Change:**
- `ProtocolError` has the variants of spec §8.1: `InvalidJson { source: serde_json::Error }`, `NotJsonRpc`, `HelloMismatch { framed: bool, generation: u32 }`, `EventLineTooLong`, `UnexpectedHelloReply { got }` where `got` is an enum displaying `a response with id {id}`, `a response with id null` or `a notification`, `MissingChannel { method: MethodName }`, `InvalidEvent { kind: String, source: serde_json::Error }`, `InvalidNotification { method: String, source: serde_json::Error }`, `ResponseOnStream`, `Frame(FrameError)` (transparent). Each `Display` is the exact spec §8.1 text. `source()` chains through `thiserror`.
- `NdjsonDecoder` turns a stream channel's DATA bytes into complete lines (W10): it carries a partial line across frames, yields several lines from one frame, and returns `ProtocolError::EventLineTooLong` once a pending partial line exceeds 16777216 bytes (spec §4.3).

**Proof:**
- `protocol_error_display_texts` — each variant displays its spec §8.1 text, including `expected the hello reply (id 0), got a response with id 3`, `expected the hello reply (id 0), got a response with id null`, `expected the hello reply (id 0), got a notification`, `a JSON-RPC response arrived on an event stream` and `the events.subscribe reply has no channel on a framed connection`.
- `ndjson_joins_a_line_split_across_frames` — a 20000-byte line plus `\n`, pushed as its first 16384 bytes and then the rest (W10 splits lines over 16384 bytes), yields that one line.
- `ndjson_yields_packed_lines` — three lines in one push yield three lines, in order.
- `ndjson_refuses_a_partial_line_over_16_mib` — 16777217 bytes with no `\n` returns `EventLineTooLong`; 16777216 bytes with no `\n` do not.

**Done when:** `cargo test -p herdrify-hc ndjson` and `cargo test -p herdrify-hc protocol_error` pass and the Definition of Done is green. Commit `feat(herdrify-hc): add ProtocolError and the NDJSON decoder`.

---

## Risks

- **The 16 MiB tests allocate 16 MiB buffers.** They are unit tests on in-memory vectors. They stay fast, but they must not be repeated in loops.
- **Riskiest step:** Task 3's hand-off. A byte lost between the hello line and the first frame breaks every framed connection. `tests/adapter_stream.rs` (plan 09) checks it again end to end.

## Proof

- `cargo test -p herdrify-hc` passes.
- `grep -rn 'tokio' crates/herdrify-hc/src/protocol` prints nothing.
- The Definition of Done is green.

## Review findings

Final batch check (2026-11-23): `cargo make dod` exit 0, zero warning lines; herdrify-hc 181 unit + 47 doc + 6 schema_coverage tests pass; `cargo make deny` advisories/bans/licenses/sources ok.

- Reviewer: blocking none; 3🟡 10🔵 code, 2🔵 plan, open:
- declined 🟡 frame.rs:217 `credit`/`resize`/`read_resize` take plain u32/u16 (cols/rows swappable): the author kept the spec §4.1 signature (2026-11-23).
- open 🟡 frame_decoder.rs:120 the drain branch of `compact` has no test (mutation deleting `self.consumed = 0` survives).
- open 🟡 line.rs:39 public infallible `Line::new` can hold a `\n`.
- open 🔵 frame.rs:93 Facts/AutomaticInput docs imprecise; frame.rs:283 any non-zero FOCUS byte reads Gained; frame_decoder.rs:52 `refused` state redundant; frame_decoder.rs:72 push keeps buffering after poison; frame_error.rs:35 and protocol_error.rs:83 hard-code 16777216; line.rs:59 lossy doc should say "each invalid sequence"; line.rs:117 lossy decoding hides bad UTF-8; line.rs:126 has_line/partial_len could be pub(super); protocol_error.rs:33 UnexpectedReply derives Error; protocol_error.rs:213 test name; additions not in the plan (Line::new, Display for Channel, #[from] Frame, has_line, pub MAX_PAYLOAD_LEN).

## Deviations

- 2026-11-23 — Commits skipped: execute commits only when instructed; every task's changes are left uncommitted in the working tree.
- 2026-11-23 — One coder ran each plan's tasks in order (tasks share `protocol/mod.rs`, so no two ran at once); the coder read the plan directly instead of a task-briefer brief.
- 2026-11-23 — Spec silent, decided: `MAX_PAYLOAD_LEN` is `pub` (a `pub(crate)` item in a private module fails clippy `redundant_pub_crate`); hello-reply enum named `UnexpectedReply { Response { id }, NullIdResponse, Notification }`; control readers check payload length only; `LineDecoder` decodes with `from_utf8_lossy`; `LineDecoder::has_line`/`partial_len` and `NdjsonDecoder::has_line` added for plan 13; `FrameKind::Unknown(n)` displays `UNKNOWN(n)`; `NdjsonDecoder::push` returns `Result` and raises `EventLineTooLong`.
- 2026-11-23 — After the build, the files this plan created were moved into the layout of the redesigned spec (§2, redesign #3): a concept and its error share one file, `frame/`, `jsonrpc/` and `methods/terminal/` split by role, and `types/` is grouped by domain. Paths named in the tasks above are the pre-move paths. Behaviour and public paths are unchanged. `cargo make dod` exits 0 with zero warnings (herdrify_hc: 212 unit, 7 schema_coverage, 49 doctests).
