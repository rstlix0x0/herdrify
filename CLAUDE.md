# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Status

The Cargo workspace exists: nine crates, no features yet. `herdrify-hc` is implemented: the async
SDK for herdr-core, at 0.1.0 (typed client for all 32 methods, `ProcessTransport` over `hc connect`,
event and attach streams). The other crates are still stubs. The architecture is decided:

- **Approved ADRs** in `.claudestacks/sdlc/adrs/` (0001–0007). They are sealed and are the
  authority. A changed decision is a new superseding ADR, written with `/claudestacks-sdlc:adr`.
  Never edit the body of an approved ADR.
- **Evidence** in `.claudestacks/sdlc/adrs/evidence/`: raw command output and Rust probes the ADRs
  cite.
- **Explanations** in `docs/`: `architecture.md`, `concepts.md` (one word per concept) and
  `herdr-and-airsl.md` (external facts with versions).

## Commands

```sh
rustup toolchain install                       # installs the toolchain rust-toolchain.toml pins
cargo install --locked cargo-make@0.37.24 cargo-deny@0.20.2   # once
cargo make dod                                 # the Definition-of-Done gate: fmt-check, clippy, doc, test, test-doc
(set -o pipefail; mkdir -p target && cargo make dod 2>&1 | tee target/dod.log)   # the gate, keeping its log
sed $'s/\x1b\\[[0-9;]*m//g' target/dod.log | grep -E '^ *(warning|error)'          # prints nothing when the gate had zero warnings
cargo make fmt                                 # rewrite formatting
cargo make deny                                # supply chain: advisories, licences, bans, sources
cargo run -q -p herdrify-cli                   # prints the command's name and version
```

`cargo make dod` also runs the `herdrify-hc` live tests (`crates/herdrify-hc/tests/live.rs`). They
run when `hc` is on `PATH` and skip otherwise. Each starts a throwaway isolated core (its own
session and scratch directory, no inherited `HC_*` variables) and never touches the default session.

CI runs `cargo make dod` on ubuntu, macos and windows, failing if its log has any warning or error
line, and `cargo deny check` on ubuntu, for every push to `main` and every pull request.

## What herdrify is

Rust tools and libraries for [Herdr](https://herdr.dev):

- standalone async SDKs for herdr-core and herdr-app
- a framework of sandboxed Lua components run under [airsl](https://github.com/airsstack/airsl),
  shared on their own or as bundles

## Architecture (ADR 0001)

```text
 L2  herdrify-cli       composition root
 L1  herdrify-lua · herdrify-herdr · herdrify-store · herdrify-git · herdrify-testkit
 L0  herdrify-core      domain, ports, use cases (no I/O, no tokio, no airsl)
     herdrify-hc · herdrify-app   standalone async SDKs (no herdrify deps)
```

**Dependencies**
- Normal dependencies point only downward. Dev-dependencies are exempt.

**Ports**
- There are six: `Runtime`, `Window`, `Terminals`, `Store`, `Source` and `Approver`.
- They are async traits used through generics, never `Box<dyn …>`.
- Methods return `impl Future + Send`, and implementors are `Send + Sync + 'static`.

**Tests**
- Use-case tests live in `herdrify-core/tests/` and use `herdrify-testkit` fakes. A `#[cfg(test)]`
  unit test in core cannot use them (E0277, see evidence).

**The sync/async bridge**
- airsl is sync. Every eval runs in `spawn_blocking`.
- A host module reaches an async port through a captured `Handle::block_on`. That call panics on an
  async worker thread.

## Invariants

- **Sandbox only (0003).**
  - A component or bundle folder contains only `*.lua`, `*.toml`, `*.md` and `LICENSE*`.
  - The only JavaScript is herdrify's own `herdrify-host`.
  - Generated layouts name `herdrify-host` or allowlisted Herdr mods, always with empty props.
- **Sole writer (0003).**
  - herdrify generates every file and section it creates in Herdr's home (`HERDR_APP_HOME`, else
    `~/.config/herdr-app`).
  - Keys go only into `layout:herdrify-<frame>` and `mod:herdrify-host`, and bind only to herdrify
    actions. `send:` and `argv` are refused.
- **Grants (0004).**
  - Core owns the kinds `fs`, `env`, `proc` and `hc`. The `config.toml` ceiling caps them, and a
    request over it refuses the plan.
  - `hc` and `herdr-app` are banned as `proc` entries. `proc` entries are bare names.
  - The hc method table is closed: 18 of 32 methods are never exposed.
- **Home (0005).**
  - `$HERDRIFY_HOME` holds `config.toml` (the only record of intent), `herdrify.lock`, `cache/`,
    `components/<name>@<commit>/`, `dev/` and `state/`.
  - Generated files are regenerated on every add and remove.
- **Bundles (0006).**
  - All-or-nothing install, with at most one frame and one theme.
  - Identity is git URL plus path, with one revision per name. Clashes are refused, never renamed.
- **Theme colors.** Components use theme color names, never fixed colors.

## Working rules

- Rust work follows the `claudestacks-guideline-rust:rust-guidelines` Definition of Done and its
  references.
- Prove behaviour claims with a probe against the exact crate version, in Rust. Save the output
  under `.claudestacks/sdlc/adrs/evidence/` or the chain that needs it.
- Use the terms in `docs/concepts.md`.

## Roadmap order

1. `herdrify-hc`
2. `herdrify-app`
3. core + store, frames and themes composed into a layout
4. add/remove/lock, grants, bundles
5. host + widgets + Lua API + `ui = 1`, `check`, `render`
6. components
7. events and live updates
8. author tooling (`new`, `test`, `dev`, `sdk emit`) and docs
