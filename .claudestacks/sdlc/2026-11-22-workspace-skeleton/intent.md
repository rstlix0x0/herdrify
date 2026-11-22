---
status: approved
created: 2026-11-22
milestone: herdrify-v0/01
---

# Intent: Workspace skeleton

## Problem

herdrify has an approved architecture (ADRs 0001–0007) and a roadmap, but no code at all. There
is no Cargo workspace, no crates, no build and no quality gate.

Every later milestone, starting with the `herdrify-hc` SDK, needs the same groundwork:
- the nine crates of ADR 0001 must exist with their layering in place
- shared metadata, dependencies and lints must be defined in one location
- the Definition-of-Done gate must guard every change from the first commit

Without that groundwork, each milestone would set up its own crates ad hoc. The layering rule
(normal dependencies point only downward), the lint policy and the toolchain floor would drift
before any behaviour is written.

Why now: this is milestone `herdrify-v0/01`, and every other milestone depends on it.

## Affected systems

- The repository root, which gains a workspace manifest, a lockfile and a toolchain pin.
- `crates/`, a new directory holding the nine members: `herdrify-core`, `herdrify-hc`,
  `herdrify-app`, `herdrify-lua`, `herdrify-herdr`, `herdrify-store`, `herdrify-git`,
  `herdrify-testkit` and `herdrify-cli`.
- CI configuration for the repository's GitHub remote.
- `CLAUDE.md`, whose "Status" section must list the real build, lint and test commands.

## Desired outcome

**The workspace builds.** A clean checkout builds a Cargo workspace whose nine members match ADR
0001's crate list and layers.

**The structure follows ADR 0001.** Members inherit shared package metadata, dependencies and
lints from the root. The normal dependency edges between members follow ADR 0001: downward only,
and the SDKs depend on no herdrify crate.

**The gate passes everywhere.** The Definition-of-Done gate of
`claudestacks-guideline-rust:rust-guidelines` passes with zero warnings:
- locally, on a clean checkout
- in CI, on every push and pull request

**Manifests are publish-ready.** Each member has a description, license, repository and readme.

**CLAUDE.md is current.** It documents the real commands.

## Constraints

- **Rust and toolchain.** Edition 2024, with a `rust-version` of at least 1.94, which is airsl
  0.1.4's floor (`.claudestacks/sdlc/adrs/evidence/airsl-0.1.4-excerpts.md`). The workspace
  follows the rust-guidelines workspace rules: members live under `crates/`, the root has no
  `[package]`, and `[workspace.*]` is centralised.
- **The ADRs are binding.**
  - Members, layers and dependency directions come from ADR 0001.
  - The SDKs declare no Cargo features (ADR 0002).
  - `herdrify-core` has no tokio, airsl or SDK among its normal dependencies (ADR 0001).
- **Licensing.** Apache-2.0, as in the existing `LICENSE`.
- **Every claim is proven.** Each claim about tool or crate behaviour is proven with a probe
  against the exact versions used (global rules).

## Non-goals

- Any herdrify behaviour, ports, SDK clients or Lua integration. Those belong to milestones 02–09.
- Publishing or reserving crate names on crates.io.
- Release automation, version bumping or changelogs.
- The JavaScript host, components or anything inside Herdr's home.
