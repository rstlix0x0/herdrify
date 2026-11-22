---
status: approved
created: 2026-11-22
depends-on: [01]
built-on: 38eeb1508131
---

# Supply Chain Policy Implementation Plan

**Goal:** Add the `deny.toml` supply-chain policy under which `cargo deny check` passes on the workspace.

**Architecture:** A single `deny.toml` at the repository root. It covers advisories (no ignores), licences (Apache-2.0 only, confidence 0.8, private crates not ignored), bans (multiple versions and wildcards denied), and sources (crates.io only). It is not part of the Definition-of-Done gate. Plan 03 runs it as its own `deny` task, and plan 04 runs it as its own CI job (spec §5, §6).

**Tech Stack:** cargo-deny 0.20.2, against the plan 01 workspace.

**Content authority:** spec §6, and §9 row 8; acceptance §10 items 2 and 5 (for `deny.toml`).

---

## Files that change

```
deny.toml   — create: the supply-chain policy, byte-identical to evidence/probe-workspace/deny.toml (spec §6)
```

## Order of work

### Task 1 — Add the supply-chain policy

**Files:**
- Create `deny.toml`, with exactly the content of `.claudestacks/sdlc/2026-11-22-workspace-skeleton/evidence/probe-workspace/deny.toml`.

**Change:** The workspace gains its supply-chain policy:
- `[graph] all-features = true`
- `[advisories]`: an empty `ignore`
- `[licenses]`: allow `Apache-2.0` only, with `confidence-threshold = 0.8`, no exceptions, and `[licenses.private] ignore = false`
- `[bans]`: `multiple-versions = "deny"` and `wildcards = "deny"`, with empty `allow`, `deny`, `skip` and `skip-tree`
- `[sources]`: unknown registries and git sources denied, with only the crates.io index allowed

No Rust code changes.

**Proof:** no unit test applies, because the task adds no Rust code. The policy is proven by the tool run below, which passes once the file exists:
- `cargo deny check` prints `advisories ok, bans ok, licenses ok, sources ok` and exits 0 (`evidence/probe-deny.txt`).

**Done when:**
- These commands pass:
  - `cargo deny check` (cargo-deny 0.20.2) exits 0 with that line.
  - `diff .claudestacks/sdlc/2026-11-22-workspace-skeleton/evidence/probe-workspace/deny.toml deny.toml` prints nothing.
- The Definition of Done is green. Its five commands run directly:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps`
  - `cargo test --workspace --all-targets --all-features`
  - `cargo test --workspace --all-features --doc`
- Each exits 0 with zero `warning` or `error` lines.
- Commit `build(workspace): add the cargo-deny supply-chain policy`.

---

## Risks

- **Only Apache-2.0 is allowed.** The first external dependency under another licence fails `cargo deny check` with exit 4 (`evidence/probe-deny-license.txt`). That is intended: the milestone adding such a dependency extends `allow` in the same change (spec §6). This plan adds no dependency.
- **The advisory database is fetched at check time.** The check was proven clean on 2026-11-22 (`evidence/probe-deny.txt`). The workspace has no external crates, so no advisory can name one of its packages.
- **Option not taken:** putting `deny` inside the `dod` gate. The spec keeps it a separate task and job (§5, §7).

## Proof

- `cargo deny check` exits 0 on the committed tree.
- The five Definition-of-Done commands stay green.
- `deny.toml` is byte-identical to the probe copy.
