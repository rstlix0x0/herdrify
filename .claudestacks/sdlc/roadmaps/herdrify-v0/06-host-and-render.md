---
status: draft
created: 2026-11-22
---

# Milestone: Host and render

## Goal

`herdrify-host` runs inside Herdr and draws component UI trees with its widgets: list, table, text, badge, button and sparkline. Each component declares the `ui = 1` render contract. `herdrify-core` holds the Lua API description, and `herdrify-lua` implements it, with a test that fails when the two differ. The `herdrify.hc` module enforces the `hc` grant. `herdrify check` and `herdrify render` work.

## Non-goals

Re-rendering on events. Author scaffolding and test tooling.
