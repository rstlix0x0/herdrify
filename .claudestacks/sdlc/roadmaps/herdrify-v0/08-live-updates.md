---
status: draft
created: 2026-11-22
---

# Milestone: Live updates

## Goal

Components re-render when Herdr state changes. The changes are driven by herdr-core `events.subscribe` through the `Terminals` port, with `doc` events filtered out before they reach Lua, so components no longer poll.

## Non-goals

New widgets. New grant kinds.
