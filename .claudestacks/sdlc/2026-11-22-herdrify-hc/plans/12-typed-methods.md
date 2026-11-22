---
status: approved
created: 2026-11-22
depends-on: [11]
built-on: ca0f990f7018
---

# Typed Methods Implementation Plan

**Goal:** Expose the 29 request-reply methods of herdr-core as typed inherent `Client` methods.

**Architecture:**
- `connection/client.rs` gains one crate-private generic path, `call::<M: Method>(params)`. It serializes `M::Params`, runs the raw call path of plan 11, and decodes the result into `M::Output` with `serde_json::from_value`. A decode failure is `Error::Decode { method, source }` (spec §6.2).
- Each typed method is an inherent `async fn(&self, params) -> Result<Output, Error>` (M-ESSENTIAL-FN-INHERENT, spec §4.8). Its name comes from the spec §4.8 table.
- `hello` is not a call: `Client::hello()` already returns the stored result. `events_subscribe` and `terminal_attach` are plans 13 and 14.
- 29 + `hello` + the two streams = 32.
- Tests reuse plan 11's `tests/support/fake_transport.rs`. Each canned reply is a verbatim reply payload for that method copied from `.claudestacks/sdlc/2026-11-22-herdrify-hc/evidence/probe-methods-output.txt`, `probe-plain-output.txt`, `probe-framed-output.txt`, `probe-edges-output.txt` or `probe-review-output.txt` into the test source. No test reads a path outside the crate.

**Tech Stack:** Rust 1.94.1, tokio 1.54.1, serde_json 1.0.154.

**Content authority:** spec §4.8 (the 32-method table, "What each method exposes"), §6.2 (Results), §9.4 (every method through the fake; `Send` assertions).

---

## Files that change

```
crates/herdrify-hc/src/connection/client.rs  — modify: the generic typed call path and 29 typed methods
crates/herdrify-hc/tests/client_fake.rs  — modify: one request/reply test per method; Send assertions for Client
```

## Order of work

Tasks 1 to 3 share one structure. Each method gets one `client_fake` test named `<method>_request_and_reply`, which:
- asserts the exact request JSON the peer received: method name and params, key order free
- answers with the canned probe reply
- asserts that the decoded output carries the reply's values

### Task 1 — Add the typed call path and the server, template and rpc methods

**Files:**
- Modify `crates/herdrify-hc/src/connection/client.rs`.
- Modify `crates/herdrify-hc/tests/client_fake.rs`.

**Change:** The crate-private `call::<M>` path, plus `server_status`, `server_stop`, `server_restart`, `templates_list` and `rpc_discover`. `rpc_discover` returns `serde_json::Value`.

**Proof:**
- `server_status_request_and_reply`, `server_stop_request_and_reply`, `server_restart_request_and_reply`, `templates_list_request_and_reply` and `rpc_discover_request_and_reply` — as above.
- `server_stop_reply_then_peer_closed` — after the `{"stopped":true}` reply the peer ends the stream; `server_stop` returns `StopResult { stopped: true }`, and `close_reason()` becomes `PeerClosed` (W22).
- `undecodable_result_is_decode_error` — `server_status` answered with `{"pid":-1}` returns `Error::Decode` with method `server.status`.
- `rpc_error_reply_is_rpc_error` — `server_restart` answered with an `invalid_params` error returns `Error::Rpc` with method `server.restart` and kind `InvalidParams`.

**Done when:** `cargo test -p herdrify-hc --test client_fake` passes and the Definition of Done is green (`cargo make dod` exits 0; the log grep of `CLAUDE.md` prints nothing). Commit `feat(herdrify-hc): add typed server, template and rpc methods`.

### Task 2 — Add the typed terminal methods

**Files:**
- Modify `crates/herdrify-hc/src/connection/client.rs`.
- Modify `crates/herdrify-hc/tests/client_fake.rs`.

**Change:** `terminal_run`, `terminal_list`, `terminal_get`, `terminal_kill`, `terminal_rm`, `terminal_keep`, `terminal_label`, `terminal_read`, `terminal_send`, `terminal_wait`, `terminal_keys`, `terminal_interrupt` and `terminal_put`, with the outputs of the spec §4.8 table.

**Proof:** one `<method>_request_and_reply` test for each of the 13 methods. In addition:
- `terminal_put_sends_base64` — the peer receives `"data":"eAo="` for bytes `x\n`.
- `terminal_get_not_found` — answered with the `not_found` error of `evidence/probe-plain-output.txt:247`, it returns `Error::Rpc` displaying `terminal.get refused by herdr-core: no terminal "t999" [not_found]`.

**Done when:** `cargo test -p herdrify-hc --test client_fake` passes and the Definition of Done is green. Commit `feat(herdrify-hc): add typed terminal methods`.

### Task 3 — Add the typed machine, file and doc methods

**Files:**
- Modify `crates/herdrify-hc/src/connection/client.rs`.
- Modify `crates/herdrify-hc/tests/client_fake.rs`.

**Change:** `machine_exec`, `file_read`, `file_write`, `file_edit`, `doc_get`, `doc_set`, `machine_add`, `machine_list`, `machine_connect`, `machine_rename` and `machine_rm`.

**Proof:** one `<method>_request_and_reply` test for each of the 11 methods. `machine_add`, `machine_connect`, `machine_rename` and `machine_rm` use their recorded error replies (`unreachable`, `machine_unknown`; `evidence/probe-methods-output.txt:283-306`) and assert `Error::Rpc` with that kind. In addition:
- `machine_exec_decodes_base64_output` — the recorded success `{"code":0,"ms":6,"signal":null,"stderr":"","stdout":"aGkK","timed_out":false,"truncated":false}` (`evidence/probe-exec-output.txt`) gives `stdout == b"hi\n"` and `code == Some(0)`.
- `invalid_base64_in_a_reply_is_decode_error` — `machine_exec` answered with `stdout: "@@"` returns `Error::Decode` with method `machine.exec`.

**Done when:** `cargo test -p herdrify-hc --test client_fake` passes and the Definition of Done is green. Commit `feat(herdrify-hc): add typed machine, file and doc methods`.

### Task 4 — Assert that Client and its futures are Send

**Files:**
- Modify `crates/herdrify-hc/tests/client_fake.rs`.

**Change:** No source change. A compile-time helper `fn assert_send<T: Send>(_: &T)` is applied, with the fake transport, to:
- `Client<FakeTransport>` itself
- the future of `Client::connect`, `call_raw` and each of the 29 typed methods

The futures are built, never awaited (spec §9.4). A non-`Send` future fails the build, and so fails the gate.

**Proof:**
- `client_and_its_futures_are_send` — compiles, and runs as a test that does nothing at run time.

**Done when:** `cargo test -p herdrify-hc --test client_fake` passes and the Definition of Done is green. Commit `test(herdrify-hc): assert Client and its futures are Send`.

---

## Risks

- **Recorded replies.** Every canned reply comes from a probe output, never written by hand. The four machine methods use their recorded error replies, and `machine_exec` uses the success recorded in `evidence/probe-exec-output.txt`.
- **Riskiest step:** Task 2's 13 methods. The structure is mechanical, but `TerminalRun`'s options are the largest request.

## Proof

- `cargo test -p herdrify-hc --test client_fake` passes.
- The four spec §2 grep checks print nothing.
- The Definition of Done is green.
