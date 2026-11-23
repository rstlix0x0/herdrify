---
status: approved
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
