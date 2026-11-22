---
status: draft
created: 2026-11-22
---

# Milestone: Install lifecycle

## Goal

`herdrify add`, `remove` (including `--purge`) and `gc` work end to end, from git sources and `dev` links:
- the lock and its `[generated]` table are kept
- files are regenerated on every change
- edited files and sections are refused
- core owns the grants (`fs`, `env`, `proc`, `hc`); the ceiling refuses requests over it, and approval is all-or-nothing
- the file-type allowlist and the keys rules are enforced
- bundles work as ADR 0006 describes

## Non-goals

Drawing components in Herdr. Live updates.
