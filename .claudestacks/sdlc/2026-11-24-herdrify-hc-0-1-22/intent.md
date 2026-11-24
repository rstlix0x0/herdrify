---
status: approved
created: 2026-11-24
---

# Intent: herdrify-hc support for hc 0.1.22

## Problem

`herdrify-hc` 0.1.0 covers herdr-core 0.1.21: all 32 methods of its OpenRPC document. The
installed `hc` is now 0.1.22 (`evidence/gate-failure.txt:2`), and its document has 36 methods
(`evidence/method-counts.txt:2-3`). The gate is red: `cargo make dod` fails in
`live_schema_matches_the_fixture` (`crates/herdrify-hc/tests/live.rs:626`), which names 12
methods, 3 schemas and the rest of the document as different
(`evidence/gate-failure.txt:12-13`).

What 0.1.22 adds over 0.1.21 (`evidence/schema-0.1.21-to-0.1.22.diff`, a diff of the sorted
documents). Nothing is removed.
- Four methods: `terminal.put_begin`, `terminal.put_chunk`, `terminal.put_finish`,
  `terminal.put_abort` (lines 184, 272, 349, 392). The document describes them as "Between
  cores": a put sent in pieces.
- Three schemas: `Empty`, `PutBeginResult`, `PutPieceResult` (lines 7, 18, 30).
- An optional `docs` param on `events.subscribe` (line 434).
- New error entries on existing methods, and `internal` on `doc.set` moving from code -32000 to
  -32603.
- The document's `info.version` (lines 46-47) and some description text.

So the SDK no longer covers every method of the core it is run against, and no caller can use
what 0.1.22 adds.

Why now: the gate is red on the author's machine, and every later milestone depends on
`herdrify-hc`.

## Affected systems

- `crates/herdrify-hc`: its methods and types, the fixtures, the offline coverage test
  (`tests/schema_coverage.rs`), the live tests, README and CHANGELOG.
- `CLAUDE.md`, `docs/herdr-and-airsl.md` and `docs/concepts.md`, which say hc has 32 methods.
- ADR 0004, whose closed hc method table counts 32 methods ("18 of 32 never exposed"). The
  four new methods are not in it.
- herdr-core 0.1.22 and 0.1.21 (`hc`), the outside parties the SDK is tested against.

## Desired outcome

- **Covers 0.1.22.** All 36 methods, including the four `terminal.put_*` methods and the `docs`
  param of `events.subscribe`, can be reached through the SDK's typed API.
- **0.1.21 still supported.** The supported minimum stays 0.1.21. Every call 0.1.21 has keeps
  working against it. A call that only 0.1.22 has gets a clear refusal on a 0.1.21 core.
- **Drift surfaces.** The offline coverage test and the live drift test check the SDK against
  the 0.1.22 document.
- **Green.** `cargo make dod` passes with hc 0.1.22 installed. `cargo deny check` and
  `cargo publish --dry-run -p herdrify-hc` pass.
- **Recorded.** The CHANGELOG has an entry for a new version. The docs state 36 methods, and
  ADR 0004's table accounts for the four new methods.

## Constraints

- **The ADRs are binding** (0002 for the SDK's shape). Changing ADR 0004's table needs a new
  ADR that supersedes it, written with `/claudestacks-sdlc:adr`. The body of an approved ADR is
  never edited.
- **The gate.** `cargo make dod` passes with zero warnings on ubuntu, macOS and Windows CI.
- **Every claim is proven** with a Rust probe against hc 0.1.22. A claim about 0.1.21 behaviour
  is proven against 0.1.21, or the spec says why it cannot be.
- **Live probes are isolated:** their own `HC_SESSION` and a short scratch directory, with no
  inherited `HC_*` variables. They never touch the person's default session.
- **Workspace rules:** `claudestacks-guideline-rust:rust-guidelines`, edition 2024, rust-version
  1.94.

## Non-goals

- `herdrify-app`, and herdrify's ports, adapters or `hc` grant filtering. The SDK still carries
  only Herdr's vocabulary.
- Publishing to crates.io. Only the dry run is in scope.
- Raising the supported minimum above 0.1.21.
- Proving any herdr-core version other than 0.1.21 and 0.1.22.
