---
status: approved
created: 2026-11-22
---

# Roadmap: herdrify v0

## Vision

Ship a usable 0.x herdrify, with two products:

- **Standalone async Rust SDKs** for herdr-core and herdr-app that any engineer can build on.
- **A sandboxed framework** where people add, remove and share Lua components and bundles that
  together make up a Herdr window.

The framework is safe by construction: there is one trust level, and herdrify is the only writer of
the files it creates in Herdr's home. All of its logic is testable with fakes. These decisions are
recorded in ADRs 0001–0007.

## Non-goals

- **No library facade.** There is no `herdrify` crate for extending herdrify itself from Rust
  (deferred in ADR 0001).
- **No trusted tier.** Components and bundles carry no third-party JavaScript, assets or raw Herdr
  files (ADR 0003).
- **No 1.0 API stability.** Every crate stays 0.x.
- **No package registry.** Sources are git repositories, plus local `dev` links. There is no index
  site.
