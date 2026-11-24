---
status: done
created: 2026-11-23
milestone: herdrify-v0/03
---

# Intent: herdrify-app SDK

## Problem

No Rust client for herdr-app exists. herdr-app 0.1.16 is driven only through its CLI: `herdr-app
ls`, `do` and `status` (`.claudestacks/sdlc/adrs/evidence/herdr-app-0.1.16-help.txt:5,6,9`).
Its 75 commands are untyped everywhere they are published:
- `herdr-app commands` gives each arg only `name` and `required`
  (`.claudestacks/sdlc/adrs/evidence/probe-facts-output.txt:42-44`)
- `herdr.d.ts` types the call as `run(name: string, args?: Record<string, unknown>)`
  (`.claudestacks/sdlc/adrs/evidence/herdr-app-0.1.16-dts-excerpts.md`)

The layout, theme and keys files that herdrify must generate (ADR 0003) are documented only in
prose in AGENTS.md (`.claudestacks/sdlc/adrs/evidence/herdr-app-0.1.16-agents-excerpts.md`).

A Rust program that wants to drive the window or produce those files must hand-write process
handling, output parsing, every command's arguments and every file shape.

This blocks two groups:
- **Outside Rust engineers building on Herdr.** ADR 0002 promises them a standalone, typed,
  fakeable client.
- **herdrify itself.** `herdrify-herdr` backs core's `Window` port by wrapping this SDK (ADR 0001).
  Milestone 04 composes frames and themes into a layout, so it needs these file models.

Why now: it is milestone `herdrify-v0/03`, the next step after `herdrify-hc`. It is the second
crate herdrify publishes, since SDKs publish first (ADR 0001).

## Affected systems

- `crates/herdrify-app`, which is a stub today: a crate-level doc comment, no dependencies,
  version 0.0.0, `publish = false`.
- The workspace root: `Cargo.toml` (`[workspace.dependencies]`) and `Cargo.lock`, for the crate's
  dependencies.
- `deny.toml`, if a new dependency's licence or source is not yet allowed.
- `CLAUDE.md` and `docs/herdr-and-airsl.md`, where the SDK's state and herdr-app facts are recorded.
- herdr-app 0.1.17, the outside party the SDK is proven and tested against. The installed
  herdr-app updated itself from 0.1.16 to 0.1.17 on 2026-11-23, during this chain's design, and
  0.1.17 adds two commands (`pane.rename`, `pane.resize`). herdr-app updates itself, so the
  installed version can move again.

## Desired outcome

**Five subcommands are covered.** `herdrify-app` drives herdr-app through `--version`, `commands`,
`ls`, `status` and `do <command>`:
- `do` has typed arguments for every command herdr-app 0.1.17 lists
- `ls`, `status` and `commands` have typed results
- herdr-app's refusals become typed errors

**The ADR 0002 shape.**
- a public async `Transport` trait that runs one `herdr-app` argv and returns its stdout, stderr
  and exit code
- `Client<T: Transport>`
- a version check: `herdr-app --version` below 0.1.16 is a typed error, and a newer version is
  accepted and reported

**Pure file models.** These are typed values for Herdr's three file shapes:
- `layouts/<name>.json`
- `themes/<name>.json`
- `keys.json`

Each parses from and serializes to Herdr's JSON, and round-trip tests cover them. They do no file
I/O: choosing where to write is the caller's job.

**Untyped access.** Callers can reach what the SDK does not type yet: they can run any command
with raw JSON args, and read fields that a typed result does not name.

**Drift surfaces.** A test compares the SDK's command names and `required` flags with the
installed `herdr-app commands`:
- when the installed version is the proven one (0.1.17), it fails on any difference
- when the installed version is newer, it passes and prints the full difference as a warning

**Tested.**
- against a fake `Transport`, with no herdr-app needed
- against a live herdr-app:
  - the live tests launch a throwaway window with its own isolated `HERDR_APP_HOME`, and drive
    `ls`, `status` and `do` there
  - they never touch the person's window or home
  - they run where herdr-app is installed and a window can be launched, and are skipped elsewhere
  - the drift test needs only `herdr-app` on `PATH`

**Ready to publish.** `cargo publish --dry-run -p herdrify-app` passes at version 0.1.0.

## Constraints

- **The ADRs are binding.** From ADR 0002:
  - async only, on tokio
  - no Cargo features
  - every public async method returns a `Send` future when its transport is `Send`
  - no dependency on any herdrify crate, and only Herdr's vocabulary (no components, no grants)
  - command argument types are written by hand, since no published source types them

  From ADR 0003: the layout, theme and keys models must be able to express what herdrify
  generates. The models themselves enforce none of herdrify's rules.
- **The gate.** The workspace Definition-of-Done gate (`cargo make dod`) passes with zero warnings
  on ubuntu, macOS and Windows CI, and `cargo deny check` passes.
- **Workspace rules.** Edition 2024, rust-version 1.94, and workspace-inherited metadata and lints,
  per `claudestacks-guideline-rust:rust-guidelines`. The module layout follows the author's rules:
  - no `<concept>_<qualifier>.rs` files
  - each error lives with its context
  - no `module_inception`
- **Every claim is proven.** Each claim about herdr-app's behaviour or a crate's behaviour is
  proven with a Rust probe against herdr-app 0.1.17 and the exact crate versions used, and each
  live probe output records the herdr-app version it ran on. Facts no
  published source states are probed live, not assumed:
  - output formats
  - refusal shapes and exit codes
  - behaviour with no window
  - whether an isolated second window can run
- **The person's Herdr is untouched.** No probe or test writes into `~/.config/herdr-app` or acts
  on the person's running window. A probe that launches an isolated window is shown to the author
  first.

## Non-goals

- The `shot`, `check`, `feedback` and `--skill` subcommands, and opening the window from the SDK's
  public API.
- herdr-app's window socket (`herdr-app.sock`). ADR 0002 fixes the transport as the CLI's argv.
- Reading or writing files in a Herdr home. That belongs to `herdrify-herdr`.
- herdrify's ports, adapters, allowlist or key rules. ADR 0003's rules are enforced by herdrify,
  not by the SDK.
- Publishing to crates.io or reserving the name. Only the dry run is in scope.
- Release automation, changelogs, or versions of any other crate.
- Proving compatibility with herdr-app versions other than 0.1.17. 0.1.16 stays the accepted
  minimum (ADR 0002) without being proven; it lacks `pane.rename` and `pane.resize`.
