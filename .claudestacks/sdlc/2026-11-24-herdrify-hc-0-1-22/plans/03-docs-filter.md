---
status: done
created: 2026-11-24
built-on: 2db7af9df077
---

# Docs Filter Implementation Plan

**Goal:** Let callers narrow `events.subscribe` to documents, refusing the filter on cores that
would ignore it.

**Architecture:**
- `EventsSubscribe` gains a `docs` field (protocol layer).
- `Client::events_subscribe` checks the core's version before sending (connection layer),
  against a crate-private `DOCS_MINIMUM` beside `CoreVersion::MINIMUM`.
- A refusal is the new `Error::NeedsNewerCore`.
- `HerdrCore`'s `Call<T, EventsSubscribe>` gains a `docs` option beside `children_of`, `labels`
  and `terminal`.

Dependency direction is unchanged (spec §2.3).

**Tech Stack:** Rust 1.94, serde 1.0.229, thiserror (workspace), tokio 1.54.1 (dev).

**Content authority:** spec §3.4, §4.2, §4.3, §5.1 (the `docs` option), §6, §7.

---

## Files that change

```
crates/herdrify-hc/src/protocol/methods/events.rs   — modify: EventsSubscribe::docs, a crate-private docs_set()
crates/herdrify-hc/src/protocol/hello.rs            — modify: pub(crate) CoreVersion::DOCS_MINIMUM (0.1.22)
crates/herdrify-hc/src/connection/error.rs          — modify: Error::NeedsNewerCore
crates/herdrify-hc/src/connection/client.rs         — modify: the docs guard in events_subscribe
crates/herdrify-hc/src/herdr_core/entry.rs          — modify: Call<T, EventsSubscribe>::docs
crates/herdrify-hc/src/herdr_core/fake.rs           — modify: connected_at(version), a fake connection at a chosen version
crates/herdrify-hc/tests/client_fake.rs             — modify: the guard tests
```

## Order of work

### Task 1 — Add EventsSubscribe::docs

**Files:**
- Modify `crates/herdrify-hc/src/protocol/methods/events.rs`.

**Change:**
- A private field `docs: Option<Vec<DocName>>`, skipped when `None`.
- `pub fn docs(self, names: impl IntoIterator<Item = DocName>) -> Self`:
  - an empty iterator sets `None`, clearing earlier names
  - otherwise it replaces any earlier names, keeping the given order and duplicates
- Its doc states the 0.1.22 behaviour from spec §3.4 (line and framed connections, spec P16):
  - a trailing `*` is a prefix, and `*` alone matches every document
  - `docs` alone gives only matching doc events, no terminal events, and an empty snapshot
  - with `terminal`, `labels` or `children_of`, both kinds of event are delivered
  - it needs herdr-core 0.1.22 (`Client::events_subscribe` refuses it on older cores)
- `# Examples` on `docs`: names sent, and an empty iterator sending none.
- `pub(crate) fn docs_set(&self) -> bool`.
- Update the `//!` block.

**Proof:**
- `docs_are_sent_in_order_with_duplicates`: `docs([a, b, a])` serializes `"docs":["a","b","a"]`.
- `docs_with_an_empty_iterator_sends_nothing`: `docs([])` serializes `{}`, and `docs_set()` is
  false.
- `a_second_docs_call_replaces_the_first`: `docs([a]).docs([b])` sends `["b"]`, and
  `docs([a]).docs([])` sends no `docs`.
- `events_subscribe_serializes_every_filter` (the existing test): extended with `docs`.

**Done when:** `cargo test -p herdrify-hc --lib protocol::methods::events` passes. The
Definition of Done is green except `live_schema_matches_the_fixture`, the one known failure on hc
0.1.22 (spec P1), which plan 05 Task 2 fixes. Commit `feat(herdrify-hc): add the docs filter to
EventsSubscribe`.

### Task 2 — Add Error::NeedsNewerCore and DOCS_MINIMUM

**Files:**
- Modify `crates/herdrify-hc/src/connection/error.rs`.
- Modify `crates/herdrify-hc/src/protocol/hello.rs`.

**Change:**
- `Error::NeedsNewerCore { option: &'static str, needs: CoreVersion, found: CoreVersion }`. Its
  message is `{option} needs herdr-core {needs} or newer; this core is {found}`. Each field is
  documented.
- `UnsupportedVersion` and its doc are unchanged: only `connect` returns it (spec §2.4, §4.3).
- In `hello.rs`, `pub(crate) const DOCS_MINIMUM: Self` (0.1.22) inside `impl CoreVersion`, built
  as `MINIMUM` is. Its comment says why: the core version that first has `events.subscribe`
  `docs`.

**Proof:**
- `needs_newer_core_message_is_exact`: with option `"events.subscribe docs"`, needs 0.1.22 and
  found 0.1.21, the message is
  `events.subscribe docs needs herdr-core 0.1.22 or newer; this core is 0.1.21`.
- `docs_minimum_is_0_1_22`: `CoreVersion::DOCS_MINIMUM == CoreVersion::parse("0.1.22")`. It is
  greater than `MINIMUM`, and greater than `parse("0.1.22-rc.1")`.

**Done when:** `cargo test -p herdrify-hc --lib -- connection::error protocol::hello` passes. The
Definition of Done is green except the known `live_schema_matches_the_fixture` failure (Task 1).
Commit `feat(herdrify-hc): add Error::NeedsNewerCore`.

### Task 3 — Guard events_subscribe

**Files:**
- Modify `crates/herdrify-hc/src/connection/client.rs`.
- Modify `crates/herdrify-hc/tests/client_fake.rs`.

**Change:**
- `connect` already parses the hello version into a `CoreVersion` (`found`,
  `src/connection/client.rs:174`). The client keeps that parsed value beside `hello`, so the
  guard never parses again and cannot fail to.
- `Client::events_subscribe` starts with the guard, before the closed-connection check and
  before anything is sent: when `params.docs_set()` and the kept version is older than
  `DOCS_MINIMUM`, it returns `Error::NeedsNewerCore { option: "events.subscribe docs", needs:
  DOCS_MINIMUM, found: <kept version> }`.
- Add `# Errors` for the new case on `events_subscribe`.

**Proof** (in `tests/client_fake.rs`, using `hello_result(version, …)`):
- `docs_on_0_1_21_is_refused_before_sending`: on a 0.1.21 hello, `docs` set gives
  `NeedsNewerCore { option: "events.subscribe docs", needs: 0.1.22, found: 0.1.21 }`, and
  `nothing_sent()` holds.
- `docs_on_a_0_1_22_prerelease_is_refused`: the same on `0.1.22-rc.1`.
- `docs_on_0_1_22_is_sent`: on a 0.1.22 hello the request is written with `"docs"`.
- `subscribe_without_docs_on_0_1_21_is_sent`: no `docs` on a 0.1.21 hello sends as before.
- `docs_guard_runs_before_the_closed_check`: on a 0.1.21 hello with the connection ended,
  `docs` set gives `NeedsNewerCore`, not `Closed`.

**Done when:** `cargo test -p herdrify-hc --test client_fake` passes. The Definition of Done is
green except the known `live_schema_matches_the_fixture` failure (Task 1). Commit
`feat(herdrify-hc): refuse the docs filter on cores older than 0.1.22`.

### Task 4 — Add the docs option to HerdrCore's events call

**Files:**
- Modify `crates/herdrify-hc/src/herdr_core/entry.rs`.
- Modify `crates/herdrify-hc/src/herdr_core/fake.rs`. Add a `connected_at(version: &str)`
  helper beside `connected()`, which answers `hello` with that version. Leave `connected()` and
  its 0.1.21 default unchanged, because `entry.rs:370-371` asserts it.

**Change:**
- `pub fn docs<S: Into<String>>(self, names: impl IntoIterator<Item = S>) -> Self` on
  `Call<T, EventsSubscribe>`, beside `children_of`, `labels` and `terminal`.
- Each name becomes a `DocName::new`. The first empty name is kept as the call's error, through
  `try_with`, and reported when the call is awaited as `HerdrCoreError::Empty`.
- Its doc repeats the 0.1.22 requirement, with `# Examples` (`no_run`, as `HerdrCore`'s
  examples are).

**Proof** (colocated in `entry.rs`, over `herdr_core/fake.rs`):
- `events_docs_option_sends_the_names`: `core.events().docs(["a", "b*"])` sends
  `"docs":["a","b*"]` on a 0.1.22 hello.
- `events_docs_option_refuses_an_empty_name`: `docs(["a", ""])` gives `HerdrCoreError::Empty`
  reading `doc name must not be empty`, and nothing is sent.

**Done when:** `cargo test -p herdrify-hc --lib herdr_core` passes. The Definition of Done is
green except the known `live_schema_matches_the_fixture` failure (Task 1). Commit
`feat(herdrify-hc): add the docs option to HerdrCore::events`.

---

## Risks

- **The fake hello's version.**
  - `src/herdr_core/fake.rs:19` answers `hello` with 0.1.21, which Task 4 handles with
    `connected_at`.
  - In `tests/client_fake.rs`, `connected()` (`:75-77`) is also 0.1.21. Task 3's tests pick
    their version through the existing `hello_result(version, …)` (`:55`) instead.
- **The parsed version is kept.** The client keeps the version `connect` parsed, rather than
  re-parsing `hello.version` (spec §4.2). Same behaviour, and the guard cannot fail to parse.
- **`Error` gains a variant**, which breaks exhaustive matches downstream. In-crate matches on
  `Error` fail to compile until they handle it, and that shows at once. The version bump is
  plan 07's.

## Proof

- `cargo test -p herdrify-hc --all-targets` and `cargo test -p herdrify-hc --doc` pass.
- `cargo clippy -p herdrify-hc --all-targets --all-features -- -D warnings` is clean.

## Review findings

Reviewer verdict: SPEC COMPLIANT, 0🔴 1🟡 8🔵, nothing blocking. DoD re-run green (`-p herdrify-hc`: fmt, clippy -D warnings, doc -D warnings, lib 351, doc 87, client_fake 98, schema_coverage 7).

- 🟡 `herdr_core/entry.rs:234` the `docs` doc said doc events come on top of terminal events; with `docs` alone the core sends only doc events and an empty snapshot — fixed: reworded to match `events.rs`, and the refusal named as `HerdrCoreError::Call` holding `Error::NeedsNewerCore`. `cargo test -p herdrify-hc --doc` → `test result: ok. 95 passed`; `RUSTDOCFLAGS="-D warnings" cargo doc -p herdrify-hc --all-features --no-deps` clean.
- 🔵 `client.rs:1-17` `//!` Responsibilities missed the docs guard — fixed.
- 🔵 `client.rs:645` `events_subscribe` summary only named terminals — fixed.
- 🔵 `client.rs:653` "checked before the connection's end" was ambiguous — fixed: "returned even when the connection has ended, ahead of `Error::Closed`".
- 🔵 `error.rs:82-85` `option` doc said "as the caller named it" — fixed: the option the client refused, as the client names it.
- 🔵 `herdr_core/fake.rs:3-5` uneven `//!` wrap — fixed.
- 🔵 `client.rs:660` the guard was proven only in `tests/client_fake.rs` — fixed: the decision is a private `docs_refusal(&CoreVersion, &EventsSubscribe) -> Option<Error>`, called first in `events_subscribe` as before, with four colocated tests (0.1.21 refused, 0.1.22-rc.1 refused, 0.1.22 sent, no docs on 0.1.21 sent). `cargo test -p herdrify-hc --lib connection` → 64 passed; client_fake 106 passed.
- 🔵 `error.rs:322-335` the exact-message test repeated the message-table row — fixed: the table row is removed; `needs_newer_core_message_is_exact` stays.
- 🔵 `option: &'static str` is open text for a closed set — fixed within the plan's shape: the field type stays (later plans match `option: "events.subscribe docs"`), and the value is named once as `pub const Error::DOCS_OPTION`, used by the client; the field doc says its values are the `Error::*_OPTION` constants. Test `docs_option_names_the_events_subscribe_docs_option`.

## Deviations

- 2026-11-24: Task 2 added `#[cfg_attr(not(test), expect(dead_code, ..))]` on `DOCS_MINIMUM` because nothing outside tests used it until Task 3; Task 3 removed it. Clippy is clean with no unfulfilled expectation.
- 2026-11-24: Task 3 also gave `tests/client_fake.rs` a `connected_at(version)` helper (`connected()` delegates to it at 0.1.21), to script 0.1.22 and pre-release hellos.
- 2026-11-24: Run interleaved with plans 01, 02 and 04 in shared batches (disjoint files per task).
- 2026-11-24: Not committed (instruction: do not commit).
