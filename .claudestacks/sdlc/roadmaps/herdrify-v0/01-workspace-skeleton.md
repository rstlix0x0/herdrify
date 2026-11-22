---
status: done
created: 2026-11-22
---

# Milestone: Workspace skeleton

## Goal

A Cargo workspace exists with the nine crates of ADR 0001 under `crates/`: eight libraries and the `herdrify-cli` binary. The root `Cargo.toml` centralises `[workspace.package]`, `[workspace.dependencies]` and `[workspace.lints]`, and every member inherits them. The Definition-of-Done gate passes on a clean checkout, and CLAUDE.md lists the real build, lint and test commands.

## Non-goals

Any behaviour beyond what the gate needs to compile and pass.
