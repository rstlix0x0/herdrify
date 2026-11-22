---
status: draft
created: 2026-11-22
---

# Milestone: Core and composition

## Goal

`herdrify-core` holds the domain types, the six ports and the compose use case.
`herdrify-store` manages `$HERDRIFY_HOME` as ADR 0005 lays it out. `herdrify-herdr` writes the files that frame and theme components generate into a temporary Herdr home: a layout that uses only `herdrify-host` and allowlisted Herdr mods with empty props, themes, and owned `keys.json` sections. Use cases are tested against `herdrify-testkit` fakes.

## Non-goals

Installing from git. Grant approval. Rendering component UI.
