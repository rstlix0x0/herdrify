---
status: approved
created: 2026-11-22
sealed: 4923cd09a46e
---

# ADR 0002: Standalone async Herdr SDKs

## Context

herdrify talks to two Herdr programs. The author wants both clients to be useful to any Rust
engineer building on Herdr, not only to herdrify, and wants a single API style to keep them simple.

**herdr-core** speaks JSON-RPC 2.0 over `hc connect`. Its `hc schema` output
(`evidence/probe-facts-output.txt:3-7,40`) shows:

- an OpenRPC 1.3.2 document for herdr-core 0.1.21
- 32 methods and 36 component schemas
- "one message per line … (or framed after `hello`)"
- "Protocol generation 1"
- "Errors carry a stable `data.kind`"

More facts from the same output and the help text:

- `hello` takes `generation` and `framed` parameters (`evidence/probe-facts-output.txt:8`).
- `hc --help` documents the remote form `ssh box hc connect` (`evidence/hc-0.1.21-help.txt:41`).
- Two of the methods hold a long-lived connection:
  - `events.subscribe` streams every fact change (`evidence/probe-facts-output.txt:33`)
  - `terminal.attach` opens a raw stream (`evidence/probe-facts-output.txt:21`)

**herdr-app 0.1.16** is driven through its CLI: `herdr-app ls`, `do` and `status`
(`evidence/herdr-app-0.1.16-help.txt:5,6,9`). Its command arguments are untyped in every place it
publishes them:

- `herdr-app commands` lists 75 commands. Each has only `name`, `about` and `args`, and each arg has
  only `name` and `required` (`evidence/probe-facts-output.txt:42-44`).
- The generated `herdr.d.ts` types the call as
  `run(name: string, args?: Record<string, unknown>)`. Its `commands()` returns the same untyped
  shape (`evidence/herdr-app-0.1.16-dts-excerpts.md`, `herdr.d.ts:91,282`).

herdrify must also generate Herdr's layout, theme and keys files (ADR 0003). Their formats are
documented in AGENTS.md:
- layouts: `layouts/<name>.json`, block content `{ "mod", "props" }` (AGENTS.md:78,224)
- keys: `keys.json` (AGENTS.md:79,523-534)
- themes: `themes/<name>.json` (AGENTS.md:333-338)

All AGENTS.md lines are in `evidence/herdr-app-0.1.16-agents-excerpts.md`.

The workspace's quality gate runs every `cargo` command with `--all-features`
(`claudestacks-guideline-rust/0.1.5/skills/rust-guidelines/SKILL.md:16-22`). That flag "equals the
default build" only while no crate declares features (same file, `:39-41`).

## Options considered

1. **Standalone, async only (tokio), with a sans-IO protocol core and a public `Transport`.**
   - Pros:
     - one API surface
     - streams fit naturally
     - anyone can fake the transport
   - Cons:
     - every user needs a tokio runtime, herdrify included
     - callers with sync code must build a runtime
2. **Standalone, with both a blocking and an async client, async behind a Cargo feature.**
   - Pros: fits both kinds of caller.
   - Cons: two client surfaces to test, and a default build without the feature that the
     `--all-features` gate never compiles on its own.
3. **Standalone, blocking only.**
   - Pros: the simplest option, and it matches airsl being sync (ADR 0001).
   - Cons: each stream needs its own thread, and async tools must wrap every call in
     `spawn_blocking`.
4. **SDKs inside herdrify.** The SDKs depend on the domain crate `herdrify-core` (ADR 0001) and
   implement its ports directly.
   - Pros: no mapping layer.
   - Cons: outside users inherit herdrify's domain types, and the SDKs cannot evolve apart from
     herdrify.

## Decision

Option 1. The two crates are `herdrify-hc` (herdr-core) and `herdrify-app` (herdr-app).

**Shared rules**

- **Standalone.** Neither crate depends on a herdrify crate, and both use only Herdr's vocabulary:
  no components and no grants.
- **No features.** Neither crate declares Cargo features.
- **Send everywhere.** Every public async method returns a `Send` future when its transport is
  `Send`.
- **Version checks.**
  - `herdrify-hc` sends `hello` with protocol generation 1 and returns a typed error when the core
    refuses it.
  - `herdrify-app` reads `herdr-app --version` (`evidence/herdr-app-0.1.16-help.txt:13`). It
    returns a typed error for any version below 0.1.16. It accepts newer versions and reports the
    version it saw.
- **Type generation is not decided here.** Whether types are generated from `hc schema` and
  `herdr.d.ts` or written by hand is settled in the `herdrify-hc` design stage.

**`herdrify-hc`**

- A `protocol` module with no I/O:
  - request and result types for all 32 methods
  - the message codec
  - line and framed modes
  - the `hello` handshake
  - a typed error built from `data.kind`
- A public async `Transport` trait that moves one message at a time. It ships with two
  implementations:
  - stdio over a spawned `hc connect`
  - an argv-configured command, for example `["ssh", "box", "hc", "connect"]`
- `Client<T: Transport>`.

**`herdrify-app`**

- Pure models of the files herdrify writes into Herdr's home: layout, theme and keys.
- Arguments for every command, written against AGENTS.md's command list, since no published source
  types them.
- A public async `Transport` trait that runs one `herdr-app` argv and returns its stdout, stderr and
  exit code.
- `Client<T: Transport>`.

## Consequences

**Benefits**

- Outside engineers get typed, streaming, fakeable clients for both Herdr programs.
- `herdrify-herdr` is a thin mapping onto core's async, `Send` ports (ADR 0001).
- With no features declared, the `--all-features` gate compiles every configuration that exists.

**Costs**

- **tokio is mandatory.** Every SDK user needs a tokio runtime. It is also why herdrify runs the
  sync airsl engine through a `spawn_blocking` bridge (ADR 0001).
- **Command types can drift.** No published source types herdr-app's command arguments (see above).
  `herdrify-app`'s argument types are therefore written by hand, and they can drift without any
  machine check. Command names and `required` flags can still be checked against `herdr-app
  commands`.
- **Newer herdr-app versions are accepted, not proven.** A herdr-app release that changes a command
  surfaces as a runtime error from herdr-app, not as a version refusal.
- **A semver promise.** Both crates are promised to outside users and publish before every other
  herdrify crate.
- **Release coupling.** A new herdr-core protocol generation needs an SDK release.
