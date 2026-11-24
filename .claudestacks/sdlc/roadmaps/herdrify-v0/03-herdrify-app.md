---
status: done
created: 2026-11-22
---

# Milestone: herdrify-app SDK

## Goal

`herdrify-app` is an async client for herdr-app, as described in ADR 0002:
- it runs herdr-app commands through a public `Transport`
- it models the layout, theme and keys files
- it checks that herdr-app is version 0.1.16 or newer

It is tested against a fake `Transport` and a live herdr-app, and is ready to publish.

## Non-goals

Writing into a person's real Herdr home. Any herdrify concept.
