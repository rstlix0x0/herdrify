---
status: approved
created: 2026-11-22
milestone: herdrify-v0/02
---

# Intent: herdrify-hc SDK

## Problem

No Rust client for herdr-core exists. herdr-core speaks JSON-RPC 2.0 over `hc connect`
(`.claudestacks/sdlc/adrs/evidence/hc-0.1.21-help.txt:41`), described by an OpenRPC 1.3.2
document with 32 methods and 36 component schemas
(`.claudestacks/sdlc/adrs/evidence/probe-facts-output.txt:3-7,40`). A Rust program that wants to
drive it must hand-write the wire protocol, the `hello` handshake, line and framed modes, and two
long-lived streams (`events.subscribe`, `terminal.attach`).

This blocks two groups:
- **Outside Rust engineers building on Herdr.** ADR 0002 promises them a standalone, typed,
  fakeable client.
- **herdrify itself.** `herdrify-herdr` backs core's `Terminals` port and the `hc` grant
  (ADRs 0001, 0004) by wrapping this SDK. Nothing downstream can be built until it exists.

Why now: it is milestone `herdrify-v0/02`, the first step of the roadmap after the workspace
skeleton, and the first crate herdrify publishes (ADR 0001, publish order).

## Affected systems

- `crates/herdrify-hc`, which is a stub today: a crate-level doc comment and no dependencies.
- The workspace root: `Cargo.toml` (`[workspace.dependencies]`) and `Cargo.lock`, for the
  crate's new dependencies.
- `deny.toml`, if a new dependency's licence or source is not yet allowed.
- `CLAUDE.md` and `docs/herdr-and-airsl.md`, where the SDK's state and herdr-core facts are
  recorded.
- herdr-core 0.1.21 (`hc`), the outside party the SDK is tested against.

## Desired outcome

**A complete client.** `herdrify-hc` drives herdr-core through every one of the 32 methods of
protocol generation 1, including both streams:
- `events.subscribe`, the notification stream
- `terminal.attach`, the raw stream on a framed connection

**The ADR 0002 shape.**
- a sans-IO protocol module
- a public async `Transport` trait, shipped with a stdio `hc connect` implementation and an
  argv-configured command implementation (e.g. `["ssh", "box", "hc", "connect"]`)
- `Client<T: Transport>`
- the `hello` handshake with protocol generation 1
- typed errors built from `data.kind`

**Versions.** It works with any herdr-core that speaks protocol generation 1 at version 0.1.21
or newer. It is proven against 0.1.21. Newer versions are accepted, not proven.

**Untyped access.** Callers can reach what the SDK does not type yet: call any method with raw
JSON params, and read fields a typed result does not name.

**Drift surfaces.** A test compares the SDK's methods and types with the installed `hc schema`
and fails on any difference.

**Tested.**
- against a fake `Transport`, with no `hc` needed
- against a live `hc`: live tests, including the drift test, run where `hc` is installed and are
  skipped elsewhere

**Ready to publish.** `cargo publish --dry-run -p herdrify-hc` passes at version 0.1.0.

## Constraints

- **The ADRs are binding.** From ADR 0002:
  - async only, on tokio
  - no Cargo features
  - every public async method returns a `Send` future when its transport is `Send`
  - no dependency on any herdrify crate, and only Herdr's vocabulary (no components, no grants)

  The type-generation choice, generated from `hc schema` or hand-written, is left to this
  chain's design (ADR 0002).
- **The gate.** The workspace Definition-of-Done gate (`cargo make dod`) passes with zero
  warnings on ubuntu, macOS and Windows CI, and `cargo deny check` passes.
- **Workspace rules.** Edition 2024, rust-version 1.94, workspace-inherited metadata and lints,
  per `claudestacks-guideline-rust:rust-guidelines`.
- **Every claim is proven.** Each claim about herdr-core's behaviour or a crate's behaviour is
  proven with a Rust probe against hc 0.1.21 and the exact crate versions used. Facts the
  schema does not define (the framed wire format, notification shapes, how requests and
  notifications interleave) are probed live, not assumed.

## Non-goals

- herdr-app or the `herdrify-app` SDK (milestone 03).
- herdrify's ports, adapters or `hc` grant filtering. ADR 0004's method table is enforced by
  `herdrify-herdr` and `herdrify-lua`, not by the SDK.
- Publishing to crates.io or reserving the name. Only the dry run is in scope.
- Release automation, changelogs, or versions of any other crate.
- Proving compatibility with herdr-core versions other than 0.1.21.
