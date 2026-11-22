---
status: draft
created: 2026-11-22
---

# Milestone: Author tooling

## Goal

Component and bundle authors have the full SDK of ADR 0007:
- `herdrify new`
- `herdrify test`, which runs on the real runtime with `herdrify-testkit` fakes and the `herdrify.test` seeding module
- `herdrify dev`, with live reload in Herdr
- `herdrify sdk emit`, which writes Lua editor stubs and JSON Schemas

Authors also have documentation.

## Non-goals

A library facade for extending herdrify from Rust.
