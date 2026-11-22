---
status: draft
created: 2026-11-22
---

# Milestone: herdrify-hc SDK

## Goal

`herdrify-hc` is an async client for herdr-core, as described in ADR 0002:
- it covers all 32 methods of protocol generation 1
- its protocol module is sans-IO
- it exposes a public `Transport` trait, with stdio and argv-command implementations
- it performs the `hello` handshake and returns typed errors from `data.kind`

It is tested against a fake `Transport` and a live `hc`, and is ready to publish.

## Non-goals

herdrify ports or adapters. Choosing a type-generation approach beyond what this milestone's design settles.
