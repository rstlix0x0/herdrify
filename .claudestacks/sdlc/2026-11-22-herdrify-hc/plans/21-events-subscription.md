---
status: done
created: 2026-11-23
depends-on: [19]
built-on: cba93d6df53e
disproves: cba93d6df53e
---

# Events Subscription Implementation Plan

**Goal:** Deliver herdr-core's event stream through `Client::events_subscribe` as a credit-managed, cancel-safe `Subscription`.

**Architecture:**
- **Stream handle.** `connection/stream.rs` holds the crate-private stream handle of spec §6.5. It is shared by `Subscription` (here) and `AttachStream` (plan 22), so there is one canonical type (`modularity.md` Rule 2). It holds:
  - the channel
  - an unbounded receiver of routed stream frames
  - the shared closed-by-core flag
  - a writer sender
  - the locally-closed state (Task 2)

  Its `Drop` applies the CLOSE rule (Task 2).
- **Route opening.** A stream-opening call registers its pending entry together with a route request. When the router routes that call's successful reply, it:
  1. reads `channel`
  2. opens the route
  3. builds the stream handle
  4. sends the reply plus the handle on the oneshot

  All four happen before it reads the next frame (spec §6.5).
- **Closed-by-core flag.** The router sets the flag before it forwards the core's CLOSE, then forgets the channel.
- **Writer sender for the handle.** The caller's route request carries a clone of the writer sender, and the router moves it into the handle it builds. The router itself never holds a writer sender (plan 19, spec §6.5 Lifetime).
- **Synchronous commands.** The writer gains two commands sent with `UnboundedSender::send` and no acknowledgement: `send_credit(channel, n)`, used by `next()` (Task 1), and `send_close(channel)`, used by `Drop` (Task 2).
- Each new crate-private item lands in the same task as its first non-test use, so `dead_code` stays silent.
- `Subscription` lives in `connection/subscription.rs` and is re-exported from `connection`.
- **Conventions for every new file** (spec §2): it opens with the four-question `//!` block; tests return `Result`, because the workspace lints deny `unwrap`, `expect` and `panic`; every `mod.rs` is export-only and cites unit-test exemption #1 in its `//!`; every public item is re-exported flat from its layer module, or `unreachable_pub` fails the gate.

**Tech Stack:** Rust 1.94.1, tokio 1.54.1 (`sync`), serde_json 1.0.154.

**Content authority:** spec §3 (W10–W14), §4.3, §4.6, §4.9 (`call_raw` on a stream method), §6.3, §6.5 (closed-by-core, the CLOSE rule, opening a stream channel, lifetime), §8.2 (rows owned by `connection::subscription`, and `connection::router` rows for streams), §9.1 (Client logic, Stream lifecycle, Writer drop-time CLOSE), §9.4 (events, events failure paths, `Subscription::close()`, `call_raw` with `events.subscribe`, lifetime, connection loss with streams, `Send`).

---

## Files that change

```
crates/herdrify-hc/src/connection/mod.rs           — modify: mod stream, mod subscription; Subscription re-export
crates/herdrify-hc/src/connection/stream.rs — create: the crate-private stream handle and its CLOSE-rule Drop
crates/herdrify-hc/src/connection/subscription.rs  — create: Subscription
crates/herdrify-hc/src/connection/router.rs        — modify: stream routes, route opening on reply, closed-by-core flag
crates/herdrify-hc/src/connection/pending.rs       — modify: pending entries that carry a route request
crates/herdrify-hc/src/connection/writer.rs        — modify: synchronous CLOSE and CREDIT commands
crates/herdrify-hc/src/connection/client.rs        — modify: events_subscribe; call_raw doc comment
crates/herdrify-hc/tests/client_fake.rs        — modify: subscription tests
```

## Order of work

### Task 1 — Open a subscription and deliver its events

**Files:**
- Create `crates/herdrify-hc/src/connection/stream.rs`, `crates/herdrify-hc/src/connection/subscription.rs`.
- Modify `crates/herdrify-hc/src/connection/router.rs`, `pending.rs`, `writer.rs`, `client.rs`, `connection/mod.rs`.
- Modify `crates/herdrify-hc/tests/client_fake.rs`.

**Change:**
- `Client::events_subscribe(EventsSubscribe) -> Result<Subscription, Error>` registers the call with a route request. A reply without `channel` is `Error::Protocol(MissingChannel)` with method `events.subscribe`. A `channel` that is not a `u32` is `Error::Decode`. Neither opens anything (spec §6.5).
- `Subscription` exposes `seq()`, `terminals()` and `channel()` from the reply (spec §6.3).
- The router opens the route and builds the handle before reading the next frame. It forwards DATA, RESIZE and CLOSE on an open channel to the handle. On CLOSE it sets the closed-by-core flag first, then forwards it, then forgets the channel.
- `Subscription::next(&mut self) -> Option<Result<Notification, Error>>`, per spec §6.3:
  - **Taking frames.** It takes a DATA frame only when no complete line is buffered. On taking N > 0 bytes it enqueues CREDIT N at once through `send_credit`, then appends them to its `NdjsonDecoder`. A zero-length frame is skipped, with no CREDIT. RESIZE frames are discarded.
  - **Decoding.** Each line is classified (spec §4.4) and decoded with `Notification::from_parts`. Every bad line yields one `Err` and the stream continues (spec §6.3): not JSON → `Protocol(InvalidJson)`; valid JSON that is not JSON-RPC → `Protocol(NotJsonRpc)`; a response → `Protocol(ResponseOnStream)`; undecodable `heartbeat`/`missed` params or an `event` whose envelope does not decode (no valid `seq` or `type`, `terminal` not a string, `machine` not a string or `""`) → `Protocol(InvalidNotification)`; a mismatched known event → `Protocol(InvalidEvent)`.
  - **Stale events.** After `Missed { seq }`, events with `seq ≤` that value are dropped before delivery.
  - **Endings.** Any CLOSE from the core ends the stream with `None`. `EventLineTooLong` yields that `Err`, then `None`. A connection end (the route sender dropped with no CLOSE) yields `Err(Error::Closed)`, then `None`.
  - **Cancel safety.** Its only await is the receive from the handle. Taking a frame, enqueuing its CREDIT and buffering happen with no await between them.

**Proof:**
- `route_opens_before_the_next_frame` (router, unit) — over `memory_link`, the reply and a DATA frame for its channel arrive back to back; the handle receives that frame.
- `credit_follows_frames` (subscription, unit) — the spec §6.3 example: one 101-byte frame holding a stale event line (39 bytes plus `\n`), a deliverable line (59 bytes plus `\n`) and 1 byte of the next line. The first `next()` enqueues CREDIT 101, drops the stale event and returns the notification.
- `a_line_longer_than_the_window_completes` (subscription, unit) — a 300000-byte event line split over 19 DATA frames of at most 16384 bytes is delivered, with one CREDIT per frame.
- `zero_length_frame_sends_no_credit` (subscription, unit) — an empty DATA frame produces no CREDIT command.
- `cancelled_next_loses_nothing` (subscription, unit) — a `next()` is cancelled while it waits for a frame (no complete line buffered). The frame then arrives, and the following `next()` returns its notification with exactly one CREDIT for it.
- `events_split_and_packed_lines` (client_fake) — a line split across two frames and three lines packed in one frame are all delivered, in order, and the peer receives CREDIT frames summing to the DATA bytes sent.
- `events_subscribe_request_and_reply` (client_fake) — the peer records the request `{"jsonrpc":"2.0","id":<n>,"method":"events.subscribe","params":{}}` (as sent at `evidence/probe-methods-output.txt:331`), and answers with the recorded reply of `evidence/probe-methods-output.txt:332` (its id replaced by `<n>`); the `Subscription` exposes its `SubscribeResult` fields, `seq` 12 and channel 1 (spec §9.4: every method's request asserted and a recorded reply decoded).
- `non_u32_channel_is_decode_error` (client_fake) — subscribe replies whose `channel` is `"x"`, `-1` and `4294967296` each return `Error::Decode` with method `events.subscribe`; nothing is opened, and a following `call_raw` succeeds (spec §6.5, §8.2).
- `first_frame_after_the_reply_is_delivered` (client_fake) — the peer sends the subscribe reply and the first DATA frame back to back; the first `next()` yields that event.
- `missed_drops_stale_events` (client_fake) — after a `missed` notification with params `{"seq":10,"terminals":[]}`, an event with seq 9 is dropped and seq 11 is delivered.
- `bad_lines_do_not_end_the_stream` (client_fake) — in one stream, in order: a non-JSON line (`InvalidJson`), `[1,2]` (`NotJsonRpc`), a response line `{"id":9,"jsonrpc":"2.0","result":{}}` (`ResponseOnStream`), `heartbeat {}` (`InvalidNotification` with method `heartbeat`), a `title` event with `data: 5` (`InvalidEvent`), then a valid event, which is still delivered (spec §6.3).
- `over_long_line_ends_the_stream` (client_fake) — 16777217 bytes with no newline yield `EventLineTooLong`, then `None`.
- `reply_without_channel_is_missing_channel` (client_fake) — returns `Protocol(MissingChannel)` displaying `the events.subscribe reply has no channel on a framed connection`.
- `core_close_ends_the_stream` (client_fake) — an empty CLOSE, and in a second case a non-empty CLOSE, both end the stream with `None`.
- `resize_on_the_event_channel_is_discarded` (client_fake) — a RESIZE between two events is not delivered.

**Done when:** `cargo test -p herdrify-hc connection` and `cargo test -p herdrify-hc --test client_fake` pass, and the Definition of Done is green (`cargo make dod` exits 0; the log grep of `CLAUDE.md` prints nothing). Commit `feat(herdrify-hc): open event subscriptions and deliver their events`.

### Task 2 — Close subscriptions under the CLOSE rule

**Files:**
- Modify `crates/herdrify-hc/src/connection/stream.rs`, `subscription.rs`, `router.rs`, `writer.rs`, `client.rs`.
- Modify `crates/herdrify-hc/tests/client_fake.rs`.

**Change:**
- The writer gains `send_close(channel)`.
- `Subscription::close(&mut self) -> Result<(), Error>` is checked in spec §6.3's order:
  1. connection ended → `Err(Error::Closed)`, nothing sent
  2. stream ended (closed-by-core flag set, or `close()` already ran) → `Ok(())`, nothing sent
  3. otherwise send CLOSE through the writer, await the acknowledgement, and return `Ok(())`
- After `close()`, `next()` keeps yielding until the core's CLOSE.
- A stream-opening call's pending entry stays registered when its future is dropped before the reply. The reply still opens the route, the oneshot send fails, and the handle is dropped (spec §6.2).
- The stream handle's `Drop` sends CLOSE with `send_close` only when neither `close()` ran nor the closed-by-core flag is set. On an ended connection the send fails and the drop does nothing more (spec §6.5).
- **Orphaned handles.** A handle that never becomes a `Subscription` is dropped and so sends CLOSE:
  - the reply decodes with an error (`Error::Decode` to the caller)
  - the `events_subscribe` future is dropped after the reply was routed

  Frames routed to it before then are dropped with it (spec §6.5).

**Proof:**
- `drop_time_close_on_open_and_ended_connections` (writer, unit) — `send_close` on an open link writes a CLOSE frame; on an ended link it returns without writing or panicking.
- `close_twice_sends_one_close` (client_fake) — two `close()` calls, then drop; the peer records exactly one CLOSE, and the second call returns `Ok(())`.
- `close_after_core_close_sends_nothing` (client_fake) — after the core's CLOSE is routed, `close()` returns `Ok(())`, and dropping sends no CLOSE.
- `queued_core_close_suppresses_drop_close` (client_fake) — the peer sends CLOSE on the channel and then the reply to a `call_raw` the test sent after subscribing. The router handles frames in order, so once that call resolves, the CLOSE was routed. The test then drops the subscription without calling `next()`, and no CLOSE is sent.
- `close_after_connection_end_is_closed` (client_fake) — after EOF, `close()` returns `Err(Error::Closed)`, on the first call and on a second.
- `next_after_close_yields_until_core_close` (client_fake) — after `close()`, an event the peer already sent is still delivered, then the peer's empty CLOSE gives `None`.
- `decode_failure_closes_the_orphaned_route` (client_fake) — a reply with a valid `channel` whose `terminals` fails to decode returns `Error::Decode`, and the peer then records a CLOSE on that channel.
- `dropped_subscribe_future_closes_the_route` (client_fake) — the `events_subscribe` future is dropped after the peer sent the reply; the peer records a CLOSE on that channel.
- `subscribe_future_dropped_before_reply_still_closes` (client_fake) — the `events_subscribe` future is dropped before the peer replies. The peer then sends the reply with a channel, and records a CLOSE on that channel (spec §6.2, §6.5).
- `handle_drop_follows_the_close_rule` (stream, unit) — over `memory_link`, dropping an open handle sends one CLOSE; dropping after `close()` ran, or with the closed-by-core flag set, sends none.

**Done when:** `cargo test -p herdrify-hc connection` and `cargo test -p herdrify-hc --test client_fake` pass, and the Definition of Done is green. Commit `feat(herdrify-hc): close event subscriptions under the CLOSE rule`.

### Task 3 — Cover subscription lifetime, loss, raw calls and Send

**Files:**
- Modify `crates/herdrify-hc/src/connection/client.rs` — the `call_raw` doc comment.
- Modify `crates/herdrify-hc/tests/client_fake.rs`.

**Change:**
- `call_raw`'s doc comment states the spec §4.9 facts:
  - with `events.subscribe` or `terminal.attach` the reply is returned but no stream opens, and the channel's frames are discarded
  - the core stops at its credit window, and a stalled stream never blocks other channels
  - the caller cannot close the channel, which stays open for the connection's lifetime
  - a raw attach makes the connection an active viewer that sets the terminal's size
  - use `events_subscribe` and `terminal_attach` for streams
- No other source change.

**Proof:**
- `connection_stays_open_while_a_subscription_lives` (client_fake) — after dropping every `Client` clone with a subscription alive, the sink is not closed; after dropping the subscription too, the peer observes the sink closed.
- `subscription_sees_connection_loss` (client_fake) — one case per cause (EOF, a failed write, a source `Frame(TooLarge)`). The subscription yields `Err(Error::Closed)` then `None`, and `close_reason()` has the matching kind.
- `call_raw_on_events_subscribe_opens_no_stream` (client_fake) — `call_raw("events.subscribe", {})` returns the reply value; a DATA frame on its channel is discarded, and a following call still succeeds.
- `subscription_and_its_futures_are_send` (client_fake) — `assert_send` on `Subscription`, and on the futures of `events_subscribe`, `next` and `close`.

**Done when:** `cargo test -p herdrify-hc --test client_fake` passes and the Definition of Done is green. Commit `test(herdrify-hc): cover subscription lifetime, loss and raw stream calls`.

---

## Risks

- **Riskiest step:** Task 1's route opening. The reply and the first stream frame can arrive in one read. The unit test and `first_frame_after_the_reply_is_delivered` both guard it.
- **Credit counting.** Credit sent per frame, not per line, is what keeps lines of 256 KiB or more from deadlocking (spec §6.3). `a_line_longer_than_the_window_completes` pins it.
- **Frames on a closed channel.** Input, CREDIT or CLOSE already on the wire when the core closes may race with its CLOSE. The SDK does not depend on the core's reaction (spec §6.5).

## Proof

- `cargo test -p herdrify-hc` passes.
- The four spec §2 grep checks print nothing.
- The Definition of Done is green.

## Review findings

- 🟡 `router.rs:111` a reply naming channel 0 or a channel in use opened a broken route (reviewer probe: a channel-0 subscription never yields and its drop sends CLOSE on the RPC channel; reuse overwrote the first route) — fixed: refused as `Error::Decode` ("channel 0 carries calls, not streams" / "channel N is already open for another stream"), nothing opened or sent; tests `a_reply_naming_channel_zero_opens_nothing`, `a_reply_naming_a_channel_in_use_leaves_the_first_route_alone`, `channel_zero_is_refused_and_opens_nothing`, `a_channel_in_use_is_refused_and_the_first_subscription_keeps_working`, `a_channel_whose_handle_is_gone_can_be_opened_again` (all failed before the fix).
- 🟡 `router.rs:71` end-then-drop-routes order unguarded — fixed: `the_end_is_recorded_before_the_routes_drop` (multi-thread; order swapped: "the route ended before the end was fully handled").
- 🟡 `subscription.rs:128` cancel safety unguarded — fixed: `cancelling_next_with_a_frame_queued_loses_nothing` (with `yield_now().await` inserted: `left: []`, `right: [10]`).
- 🔵 `client.rs:228` `# Streams` names `Client::terminal_attach` before it exists — accepted: the plan asks for it; plain code text, and plan 22 adds the method.
- 🔵 `client.rs:256` `opening: bool` — fixed: `Option<Opening>`.
- 🔵 `client.rs:544` "closed again" on the non-u32 case — fixed.
- 🔵 `client.rs:563` wrong "router is gone" comment — fixed: the arm returns the unusable-channel `Decode`.
- 🔵 `router.rs:131` long doc line — fixed.
- 🔵 `router.rs:141` dead-route removal unguarded — fixed: `a_route_whose_handle_is_gone_is_removed`.
- 🔵 `subscription.rs:40` two bools for a three-state lifecycle — fixed: `enum Phase`.
- 🔵 `subscription.rs:164` silent saturation — fixed: comment says it is unreachable.
- 🔵 `writer.rs:87` wrong lifetime comment — fixed.
- 🔵 `credit_follows_frames` byte sizes — see Deviations.
- 🔵 `close()` returns `Err(Closed)` when the connection ends before the acknowledgement — see Deviations.
- After fixes: `connection` 51 passed; `--test client_fake` 76 passed ×3; `cargo make dod` exit 0; log grep prints nothing.

## Deviations

- 2026-11-23 — Spec §6.3's credit example (a 39-byte stale event line plus `\n`, a 59-byte line, CREDIT 101) cannot be built: the shortest event line whose envelope decodes is 48 bytes (`printf '%s' '{"method":"event","params":{"seq":9,"type":"x"}}' | wc -c` → `48`). `credit_follows_frames` keeps the shape (stale line, delivered line, 1 byte of the next) with 54- and 76-byte lines and asserts CREDIT equals the frame length. `disproves` set on the spec's fingerprint.
- 2026-11-23 — `Subscription::close()` returns `Err(Closed)` if the connection ends before the CLOSE acknowledgement (a stalled write never acknowledges). This needed `EndRecord::wait_ended` (a `Notify`) in `connection/error.rs`, outside the file list, and an `end` field on the opening handle.
- 2026-11-23 — `Subscription::seq()` returns `Seq`, `channel()` returns `Channel`, `terminals()` returns `&[Terminal]`; `Pending::complete` became `take`, with the router building the answer.
- 2026-11-23 — A stream reply naming channel 0 or a channel in use is refused as `Error::Decode`; no new `Error` variant.
- 2026-11-23 — Not committed: the instruction to execute did not ask for commits; the change is left staged.
