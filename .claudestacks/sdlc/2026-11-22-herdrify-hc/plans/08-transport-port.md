---
status: done
created: 2026-11-22
depends-on: [03]
built-on: ca0f990f7018
---

# Transport Port Implementation Plan

**Goal:** Define the public transport port that sits between the client and every adapter or fake.

**Architecture:**
- The `port` layer (spec §2). It depends on `protocol` only and names no tokio type.
- `port/transport.rs` holds the three traits exactly as spec §5 writes them. It is a pure trait file, exempt from colocated tests (`unit-test-mandate.md` exemption #3), and its `//!` block cites the exemption.
- `port/mod.rs` is export-only.
- The crate root re-exports `Transport`, `FrameSink`, `FrameSource` and `TransportError` with `#[doc(inline)]` (spec §2).

**Tech Stack:** Rust 1.94.1, thiserror 2.0.21.

**Content authority:** spec §5, §8.1 (`TransportError` rows).

---

## Files that change

```
crates/herdrify-hc/src/lib.rs                     — modify: pub mod port; crate-root re-exports of the four port items
crates/herdrify-hc/src/port/mod.rs                — create: export-only table of contents
crates/herdrify-hc/src/port/transport_error.rs    — create: TransportError
crates/herdrify-hc/src/port/transport.rs          — create: Transport, FrameSink, FrameSource
```

## Order of work

### Task 1 — Add TransportError

**Files:**
- Create `crates/herdrify-hc/src/port/mod.rs`, `crates/herdrify-hc/src/port/transport_error.rs`.
- Modify `crates/herdrify-hc/src/lib.rs` — `pub mod port;` and the `TransportError` re-export.

**Change:** `TransportError` has three variants (spec §8.1):
- `Spawn { program: String, source: io::Error }`, displaying `cannot start {program}: {source}`
- `Io(io::Error)`, displaying `I/O error: {source}`
- `Frame(FrameError)`, transparent

`source()` chains through `thiserror`. It is `Send + Sync + 'static`.

**Proof:**
- `spawn_display` — `Spawn { program: "hc", source: io::Error::from(ErrorKind::NotFound) }` displays starting with `cannot start hc: `.
- `io_display` — `Io(io::Error::from(ErrorKind::BrokenPipe))` displays starting with `I/O error: `.
- `frame_is_transparent` — `Frame(FrameError::TooLarge { len: 16777217 })` displays exactly `frame payload of 16777217 bytes exceeds the 16777216-byte limit`.
- `transport_error_is_send_sync_static` — a compile-time bound check.

**Done when:** `cargo test -p herdrify-hc transport_error` passes and the Definition of Done is green (`cargo make dod` exits 0; the log grep of `CLAUDE.md` prints nothing). Commit `feat(herdrify-hc): add TransportError`.

### Task 2 — Add the Transport, FrameSink and FrameSource traits

**Files:**
- Create `crates/herdrify-hc/src/port/transport.rs`.
- Modify `crates/herdrify-hc/src/port/mod.rs` and `crates/herdrify-hc/src/lib.rs` — re-exports of the three traits.

**Change:** The three traits as written in spec §5:
- `Transport: Send + 'static` with associated `Sink: FrameSink` and `Source: FrameSource`, `send_line`, `recv_line` and `into_framed(self)`
- `FrameSink: Send + 'static` with `send` and `close`
- `FrameSource: Send + 'static` with `recv`

Every method returns `impl Future<Output = …> + Send`. The doc comments state the type state: lines until `into_framed` consumes the transport, then only frames, and `Ok(None)` means the peer closed (spec §5).

**Proof:** no colocated test, because the file is a pure trait file (exemption #3, cited in its `//!`). Plan 09's `StreamTransport` and plan 11's outside-the-crate fake transport implement it, which proves the traits are implementable and that their futures are `Send` (spec §5, `evidence/probe-trait-output.txt:1-6`). This task's check is that `cargo doc -p herdrify-hc` builds the traits with no rustdoc warning.

**Done when:** `RUSTDOCFLAGS="-D warnings" cargo doc -p herdrify-hc --no-deps` passes and the Definition of Done is green. Commit `feat(herdrify-hc): add the transport port traits`.

---

## Risks

- **The port's shape is fixed for 0.1.0 callers.** Changing it later is a breaking change. The spec's probe proved the RPITIT shape on 1.94.1 (§5).

## Proof

- `grep -rn 'crate::connection\|crate::adapter\|tokio' crates/herdrify-hc/src/port` prints nothing (spec §2 enforcement).
- The Definition of Done is green.

## Review findings

Final batch check (2026-11-23): `cargo make dod` exit 0, zero warning lines; herdrify-hc 181 unit + 47 doc + 6 schema_coverage tests pass; `cargo make deny` advisories/bans/licenses/sources ok.

- Reviewer (with plan 07): blocking none; 5🔵 + 1❓ on port files:
- open 🔵 transport.rs:22 redundant `use std::future::Future`; transport_error.rs:37,45 Spawn/Io show `{source}` and return it from `source()` (as the plan specifies); transport_error.rs:49 `#[from] FrameError` untested; lib.rs trait re-exports unguarded until plans 09/11.
- open ❓ transport_error.rs:35 `TransportError` is not `#[non_exhaustive]`.
- `cargo test -p herdrify-hc transport_error` → 4 passed; port grep for connection/adapter/tokio prints nothing.

## Deviations

- 2026-11-23 — Commits skipped: execute commits only when instructed; every task's changes are left uncommitted in the working tree.
- 2026-11-23 — One coder ran each plan's tasks in order (tasks share `protocol/mod.rs`, so no two ran at once); the coder read the plan directly instead of a task-briefer brief.
- 2026-11-23 — `TransportError::Io` is `Io(#[source] io::Error)` with Display `I/O error: {0}`; `Frame` is `#[error(transparent)]` + `#[from]`.
- 2026-11-23 — The `into_framed` doc states that bytes buffered but not yet returned by `recv_line` carry into the source (the adapter contract plan 09 relies on).
- 2026-11-23 — After the build, the files this plan created were moved into the layout of the redesigned spec (§2, redesign #3): a concept and its error share one file, `frame/`, `jsonrpc/` and `methods/terminal/` split by role, and `types/` is grouped by domain. Paths named in the tasks above are the pre-move paths. Behaviour and public paths are unchanged. `cargo make dod` exits 0 with zero warnings (herdrify_hc: 212 unit, 7 schema_coverage, 49 doctests).
