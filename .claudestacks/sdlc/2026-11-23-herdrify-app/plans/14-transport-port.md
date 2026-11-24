---
status: done
created: 2026-11-23
depends-on: [08]
built-on: aab842c3c7d3
---

# Transport Port Implementation Plan

**Goal:** Define the public `Transport` port that runs one herdr-app invocation.

**Architecture:** The `port` layer depends on `protocol` only (spec §2). `transport.rs` is a pure
trait file (exemption #3, cited in its `//!`), `error.rs` holds `TransportError`, `port/mod.rs` is
export-only, and `lib.rs` gains `pub mod port;`. The shared rules of plan 02's Architecture apply
verbatim. The crate-private spawn-error mapping is added by plan 16, its first user.

**Tech Stack:** Rust 1.94, thiserror 2.0.21.

**Content authority:** spec §6; §10 (`TransportError` rows).

---

## Files that change

```
crates/herdrify-app/src/lib.rs                 — modify: pub mod port
crates/herdrify-app/src/port/mod.rs            — create: export-only table of contents of the port
crates/herdrify-app/src/port/transport.rs      — create: trait Transport
crates/herdrify-app/src/port/error.rs          — create: TransportError
```

## Order of work

### Task 1 — Add Transport and TransportError

**Files:**
- Modify `crates/herdrify-app/src/lib.rs`.
- Create `crates/herdrify-app/src/port/mod.rs`, `transport.rs` and `error.rs`.

**Change:**
- `pub trait Transport: Send + Sync + 'static { fn run(&self, invocation: &Invocation) -> impl
  Future<Output = Result<RunOutput, TransportError>> + Send; }`, with a doc comment showing a
  minimal implementor.
- `TransportError`, public:
  - `NotFound { program: String }`, Display ``cannot start `<program>`: it was not found``
  - `Io { program: String, source: std::io::Error }`, Display ``cannot start `<program>`:
    <source>`` with `#[source]`

**Proof:**
- `transport_error_display` — both variants display their exact texts.
- `transport_error_keeps_its_source` — `Io`'s `source()` is the given `io::Error`.
- `transport_error_is_send_sync_static` — a compile-time bound check.
- A doctest on `Transport` implementing it for a test type that returns a fixed `RunOutput`.

**Done when:** `cargo test -p herdrify-app port` passes and the Definition of Done is green. Commit
`feat(herdrify-app): add the Transport port`.

---

## Risks

- None specific.

## Proof

- `cargo make dod` is green.

## Review findings

Reviewer verdict: plan-compliant, no drift; blocking: none. Scoped gate re-run by the reviewer: `dod-app.sh` → `exit=0`, no warning/error lines; 154 unit, 3 integration, 166 doctests passed. `cargo test -p herdrify-app port` → 3 passed.

- open 🟡 `port/transport.rs:60` — no reversion guard: dropping the `Send + Sync + 'static` supertraits or the `+ Send` on the returned future still passes the gate; a compile-time bound check on `Transport` (like `transport_error_is_send_sync_static`) would catch it.
- open 🔵 `port/error.rs:41` — `Io` displays `{source}` and also returns it from `source()`, so chain printers show the io text twice. Matches the plan's Display text and herdrify-hc's `TransportError::Spawn`.
- open 🔵 `port/error.rs:85` — `transport_error_keeps_its_source` checks only `kind()`; it could also check the source's text.
- open 🔵 `port/transport.rs:1` — the `//!` block opens with the exemption line instead of the summary sentence.
- open 🔵 `port/transport.rs:4` — the `//!` block has no "why it exists" paragraph of its own.

## Deviations

- 2026-11-24 — `cargo make dod` was replaced by the scoped gate (fmt, workspace clippy and doc with `-D warnings`, herdrify-app tests and doctests). The full gate fails only in herdrify-hc `tests/live.rs` `live_schema_matches_the_fixture`, against the installed `hc 0.1.22`, which this plan does not touch.
- 2026-11-24 — Built in parallel with plan 11 (disjoint files: `lib.rs` + `src/port/` here, `src/protocol/` there). The coder's gate failed twice mid-run on the other plan's in-progress files and went green on retry.

## Fix pass (2026-11-24, on the author's word "fix all findings")

- fixed: a second doctest on `Transport` checks, through generic helpers, that every implementor is `Send + Sync + 'static` and that `run`'s future is `Send`. Removing `Send + Sync` from the trait fails the build ("future cannot be sent between threads safely"); restored.
- fixed: `transport_error_keeps_its_source` also asserts the source's text `permission denied`.
- fixed: `transport.rs`'s `//!` opens with the summary, then the exemption line, and has its own "why it exists" paragraph.
- declined (spec-fixed): `Io` keeps Display ``cannot start `<program>`: <source>`` with `#[source]`; spec §10 sets the text, and it matches herdrify-hc's `TransportError::Spawn`.
- `cargo test -p herdrify-app port` → 7 passed. Gate after the pass: `dod-app.sh` → `exit=0`, no warning/error lines.
