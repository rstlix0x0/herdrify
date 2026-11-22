---
status: approved
created: 2026-11-22
depends-on: [01, 02]
built-on: 38eeb1508131
---

# Gate Tasks Implementation Plan

**Goal:** Make `cargo make dod` run the five Definition-of-Done commands in order as one gate.

**Architecture:** A root `Makefile.toml` for cargo-make 0.37.24:
- `[config]` turns off workspace fan-out (`default_to_workspace = false`), skips cargo-make's core tasks, and requires `min_version = "0.37.24"`.
- `dod`, aliased as `default`, depends on `fmt-check`, `clippy`, `doc`, `test` and `test-doc`, in that order. `doc` sets `RUSTDOCFLAGS = "-D warnings"` in its own env.
- `fmt` is the only task that rewrites files.
- `deny` runs `cargo deny check` against plan 02's `deny.toml`. It is not part of `dod` (spec §5).

**Tech Stack:** cargo-make 0.37.24, cargo 1.94.1, cargo-deny 0.20.2.

**Content authority:** spec §5, and §9 rows 5 and 6 (the local half) and 11; acceptance §10 items 1 and 5 (for `Makefile.toml`).

---

## Files that change

```
Makefile.toml   — create: the dod/fmt/deny tasks, byte-identical to evidence/probe-workspace/Makefile.toml (spec §5)
```

## Order of work

### Task 1 — Add the cargo-make gate

**Files:**
- Create `Makefile.toml`, with exactly the content of `.claudestacks/sdlc/2026-11-22-workspace-skeleton/evidence/probe-workspace/Makefile.toml`.

**Change:**
- `cargo make dod` runs, in order (the `default` task aliases it, as the probe file declares):
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `cargo doc --workspace --all-features --no-deps`, with `RUSTDOCFLAGS="-D warnings"`
  - `cargo test --workspace --all-targets --all-features`
  - `cargo test --workspace --all-features --doc`
- `cargo make fmt` runs `cargo fmt --all`.
- `cargo make deny` runs `cargo deny check`.
- `[env]` sets `CARGO_TERM_COLOR = "always"`.
- No Rust code changes.

**Proof:** no unit test applies, because the task adds no Rust code. The gate is proven by the tool run below, which passes once the file exists. It uses spec §8's two log-check commands, the same pair `CLAUDE.md` will list (plan 05).
- First `cargo clean`, so the run starts with no `target/` (acceptance §10 item 1).
- `(set -o pipefail; mkdir -p target && cargo make dod 2>&1 | tee target/dod.log)` exits 0 (spec §5, `evidence/probe-ci-step-warning-check.txt`).
- `target/dod.log` shows the five `Execute Command:` lines in the order above (`evidence/probe-gate-full.txt:7,8,19,39,105`).
- `sed $'s/\x1b\\[[0-9;]*m//g' target/dod.log | grep -E '^ *(warning|error)'` prints nothing. This is spec §5's pass criterion: `grep` exits 1 here, and that exit means clean.

**Done when:**
- These commands pass:
  - The Proof run above: from a cleaned `target/`, the first log-check command exits 0 and the second prints nothing.
  - `cargo make deny` exits 0, printing `advisories ok, bans ok, licenses ok, sources ok`.
  - `diff .claudestacks/sdlc/2026-11-22-workspace-skeleton/evidence/probe-workspace/Makefile.toml Makefile.toml` prints nothing.
- The Definition of Done is green. From this commit on, `cargo make dod` is the one-command form of its five commands.
- Commit `build(workspace): add the cargo-make Definition-of-Done gate`.

---

## Risks

- **The gate is proven on cargo-make 0.37.24 only** (spec §1, §9). A newer cargo-make is not covered. `min_version` refuses an older one. CI (plan 04) and `CLAUDE.md` (plan 05) install exactly 0.37.24.
- **`doc` without its env would pass despite warnings.** Plain `cargo doc` exits 0 on rustdoc warnings. The `[tasks.doc.env]` block is what makes the step a gate, so it must not be dropped or moved.
- **Option not taken:** a `deny` dependency inside `dod`. The spec keeps them separate (§5).

## Proof

- `cargo make dod` exits 0 with zero `warning` or `error` lines on the committed tree, and runs exactly the five commands in order.
- `cargo make deny` exits 0.
- `Makefile.toml` is byte-identical to the probe copy.
