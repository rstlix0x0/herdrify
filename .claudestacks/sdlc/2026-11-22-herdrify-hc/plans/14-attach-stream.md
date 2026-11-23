---
status: superseded
created: 2026-11-22
depends-on: [13]
built-on: ca0f990f7018
---

# Attach Stream Implementation Plan

**Goal:** Drive a terminal's raw byte stream through `Client::terminal_attach` as an `AttachStream`.

**Architecture:**
- `connection/attach.rs` holds `AttachStream`, built on plan 13's stream handle. The router's route opening, closed-by-core flag and CLOSE-rule `Drop` are reused unchanged, so nothing is duplicated (`modularity.md` Rule 2).
- `AttachStream` is re-exported from `connection`.
- The `automatic_input` decision reads the stored `HelloResult.features` and the attach reply (W17).
- Control payloads are built and read with plan 03's `Frame::resize`/`Frame::focus` constructors and readers.

**Tech Stack:** Rust 1.94.1, tokio 1.54.1 (`sync`), serde_json 1.0.154.

**Content authority:** spec §3 (W15, W16, W17), §4.1, §6.4, §6.5 (the CLOSE rule, opening a stream channel), §8.1 (`StreamClosed`, `PayloadTooLarge`), §8.2 (rows owned by `client::attach`), §9.1 (Stream lifecycle), §9.4 (attach; connection loss with streams; `Send`).

---

## Files that change

```
crates/herdrify-hc/src/connection/mod.rs     — modify: mod attach; AttachStream, AttachEvent, CloseReason re-exports
crates/herdrify-hc/src/connection/attach.rs  — create: AttachStream, AttachEvent, CloseReason
crates/herdrify-hc/src/connection/client.rs  — modify: terminal_attach
crates/herdrify-hc/tests/client_fake.rs  — modify: attach tests
```

## Order of work

### Task 1 — Open an attach stream and read it

**Files:**
- Create `crates/herdrify-hc/src/connection/attach.rs`.
- Modify `crates/herdrify-hc/src/connection/client.rs`, `crates/herdrify-hc/src/connection/mod.rs`.
- Modify `crates/herdrify-hc/tests/client_fake.rs`.

**Change:**
- `Client::terminal_attach(TerminalAttach) -> Result<AttachStream, Error>` opens its route the same way `events_subscribe` does. A reply without `channel` is `Protocol(MissingChannel)` with method `terminal.attach`. A `channel` that is not a `u32` is `Error::Decode`.
- `AttachStream` exposes `terminal()`, `channel()`, `automatic_input()` and `resize_notifications()` from the reply (spec §6.4).
- `next(&mut self) -> Option<Result<AttachEvent, Error>>`:
  - `Output(Vec<u8>)` for DATA. On taking N > 0 bytes it enqueues CREDIT N at once (synchronous, no acknowledgement), then returns them. A zero-length DATA frame is skipped, with no `Output` and no CREDIT.
  - `Resize { cols: u16, rows: u16 }` for an incoming RESIZE, in stream order. A RESIZE whose payload is not 4 bytes yields `Err(Protocol(Frame(BadControlPayload)))`, and the stream continues.
  - `Closed(CloseReason)` and then `None` when the core's CLOSE arrives:
    - `Detached` for an empty CLOSE
    - `Ended { reason: EndReason, exit_code: Option<i32>, signal: Option<i32>, finished_ms: Option<u64> }` for a JSON object CLOSE that decodes as that shape: a required string `reason`, and the other three optional, null or absent reading as `None` (spec §6.4)
    - a payload that fails that shape (for example a string `signal`) is `Unrecognized`, never an error
    - `Unrecognized { payload }` for any other non-empty CLOSE
  - A connection end yields `Err(Error::Closed)`, then `None`.
  - It is cancel-safe: its only await is the receive.

**Proof:**
- `attach_reply_fields` (client_fake) — the attach reply of `evidence/probe-framed-output.txt:303-304` gives the stream's `terminal`, `channel`, `automatic_input` and `resize_notifications`.
- `output_is_credited` (client_fake) — a 339-byte DATA frame yields `Output` of those bytes, and the peer receives CREDIT 339 on that channel.
- `zero_length_data_is_skipped` (client_fake) — an empty DATA frame then a 3-byte frame yield one `Output` of 3 bytes and one CREDIT 3.
- `resize_events_and_a_malformed_resize` (client_fake) — RESIZE 80×24 yields `Resize { cols: 80, rows: 24 }`; a 3-byte RESIZE yields `BadControlPayload`, and a following `Output` is still delivered.
- `close_reasons` (client_fake) — an empty CLOSE yields `Closed(Detached)`, the W15 payload `{"exit_code":0,"finished_ms":…,"reason":"exited","signal":null}` yields `Closed(Ended { reason: Exited, exit_code: Some(0), … })`, and `garbage` yields `Closed(Unrecognized)`. Each is followed by `None`.
- `ended_accepts_the_readme_shape` (attach, unit) — `{"reason":"exited","exit_code":0,"signal":null}` (README:109-110, no `finished_ms`) is `Ended` with `finished_ms: None`; `{"reason":"killed","exit_code":null,"signal":1}` is `Ended` with `signal: Some(1)`; `{"exit_code":0}` (no `reason`) and `{"reason":"exited","signal":"TERM"}` are `Unrecognized`.
- `attach_reply_without_channel_is_missing_channel` (client_fake) — returns `MissingChannel`, displaying `the terminal.attach reply has no channel on a framed connection`.

**Done when:** `cargo test -p herdrify-hc --test client_fake` passes and the Definition of Done is green (`cargo make dod` exits 0; the log grep of `CLAUDE.md` prints nothing). Commit `feat(herdrify-hc): open attach streams and read their output`.

### Task 2 — Write to an attach stream

**Files:**
- Modify `crates/herdrify-hc/src/connection/attach.rs`.
- Modify `crates/herdrify-hc/tests/client_fake.rs`.

**Change:** Six writes, each `&mut self -> Result<(), Error>` and acknowledged by the writer (spec §6.4):
- `input(&[u8])` sends DATA
- `automatic_input(&[u8])` sends kind 6 when `hello.features` contains `automatic_input` and the attach reply says `automatic_input: true`, otherwise DATA (W17)
- `resize(u16, u16)` sends RESIZE
- `focus(Focus)` sends FOCUS
- `facts(DisplayFacts)` sends FACTS with the JSON of the facts
- `close()` sends CLOSE under the CLOSE rule

Refusals are checked in spec §6.4's order, before anything is sent:
1. connection ended → `Error::Closed`
2. stream ended (closed-by-core flag set, or `close()` already ran) → `Error::StreamClosed`; `close()` instead returns `Ok(())` and sends nothing
3. a payload over 16777216 bytes (`input`, `automatic_input`, `facts`) → `Error::PayloadTooLarge`

After `close()`, `next()` keeps yielding until the core's CLOSE arrives as `Closed(Detached)`.

**Proof:**
- `writes_send_their_frames` (client_fake) — `input(b"ls\n")`, `resize(100, 30)`, `focus(Gained)` and `facts(..)` reach the peer as DATA, RESIZE (4 bytes), FOCUS (`1`) and FACTS (JSON) on the stream's channel.
- `automatic_input_needs_both_sides` (client_fake) — with `automatic_input` in the hello features and in the reply, the peer receives kind 6. With either missing, it receives DATA.
- `write_after_core_close_is_stream_closed` (client_fake) — after the core's CLOSE is routed, `input` returns `StreamClosed` (displaying `the terminal.attach stream is closed`), nothing is sent, and `close()` returns `Ok(())` and sends nothing.
- `write_after_connection_end_is_closed` (client_fake) — after EOF, `input` returns `Closed`, even after the stream also ended.
- `oversized_input_is_refused` (client_fake) — `input` of 16777217 bytes returns `PayloadTooLarge`, nothing is sent, and a following small `input` succeeds.
- `close_then_detached` (client_fake) — `close()` sends one CLOSE, a second `close()` sends nothing, and the peer's empty CLOSE yields `Closed(Detached)`.
- `drop_follows_the_close_rule` (client_fake) — dropping an open stream sends one CLOSE. In a second case the peer sends CLOSE and then the reply to a `call_raw` the test sent after attaching; once that call resolves the CLOSE was routed, and dropping the unread stream sends none.
- `write_refusal_order` (attach, unit) — over `memory_link`: with both the connection ended and the stream ended, `input` gives `Closed`; with only the stream ended, `StreamClosed`; with neither and a 16777217-byte payload, `PayloadTooLarge`. No case writes a frame.

**Done when:** `cargo test -p herdrify-hc --test client_fake` passes and the Definition of Done is green. Commit `feat(herdrify-hc): write to attach streams`.

### Task 3 — Cover attach connection loss and Send

**Files:**
- Modify `crates/herdrify-hc/tests/client_fake.rs`.

**Change:** No source change.

**Proof:**
- `attach_sees_connection_loss` (client_fake) — one case per cause (EOF, a failed write, a source `Frame(TooLarge)`). The stream yields `Err(Error::Closed)` then `None`, and `close_reason()` has the matching kind.
- `attach_stream_and_its_futures_are_send` (client_fake) — `assert_send` on `AttachStream`, and on the futures of `terminal_attach`, `next`, `input`, `automatic_input`, `resize`, `focus`, `facts` and `close`.

**Done when:** `cargo test -p herdrify-hc --test client_fake` passes and the Definition of Done is green. Commit `test(herdrify-hc): cover attach connection loss and Send`.

---

## Risks

- **The `Ended` close shape.** Only one live CLOSE payload was recorded (W15). Any other non-empty payload is `Unrecognized`, so a new shape never ends in an error.
- **Riskiest step:** Task 2's refusal order. The tests fix each pair of conditions.

## Proof

- `cargo test -p herdrify-hc` passes.
- The four spec §2 grep checks print nothing.
- The Definition of Done is green.
