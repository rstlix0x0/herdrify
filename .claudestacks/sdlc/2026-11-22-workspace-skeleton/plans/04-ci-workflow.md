---
status: approved
created: 2026-11-22
depends-on: [02, 03]
built-on: 38eeb1508131
---

# CI Workflow Implementation Plan

**Goal:** Add the GitHub Actions workflow that runs the repository's checks on every pull request.

**Architecture:** One workflow, `.github/workflows/ci.yml`:
- **Triggers:** pushes to `main`, every pull request, and manual dispatch.
- **Workflow settings:** superseded runs on the same ref are cancelled, and permissions are `contents: read` only.
- **`dod` job:** an `ubuntu-latest` / `macos-latest` / `windows-latest` matrix with `fail-fast: false` and a 45-minute timeout. It installs the pinned toolchain with `rustup toolchain install`, caches with `Swatinem/rust-cache@v2`, installs `cargo-make@0.37.24` through `taiki-e/install-action@v2`, then runs one `shell: bash` step: `cargo make dod` (plan 03) with its output kept in `target/dod.log`, failing the step if the colour-stripped log has any `warning` or `error` line (spec §5, §7).
- **`deny` job:** `ubuntu-latest`, with a 15-minute timeout. It installs the toolchain and `cargo-deny@0.20.2`, then runs `cargo deny check` against plan 02's policy (spec §7).

**Tech Stack:** GitHub Actions; `actions/checkout@v5`, `Swatinem/rust-cache@v2`, `taiki-e/install-action@v2`; rustup 1.29.3 semantics for `rustup toolchain install`.

**Content authority:** spec §7, and §9 rows 5 and 6 (the CI half), 10 (the CI half; plan 05 owns the `CLAUDE.md` half), 11 and 12; acceptance §10 items 5 (for `ci.yml`) and 7.

---

## Files that change

```
.github/workflows/ci.yml   — create: the CI workflow, byte-identical to evidence/probe-workspace/.github/workflows/ci.yml (spec §7)
```

## Order of work

### Task 1 — Add the CI workflow

**Files:**
- Create `.github/workflows/ci.yml`, with exactly the content of `.claudestacks/sdlc/2026-11-22-workspace-skeleton/evidence/probe-workspace/.github/workflows/ci.yml`.

**Change:**
- The repository gains CI with the triggers, concurrency, permissions, env block (`CARGO_TERM_COLOR`, `CARGO_INCREMENTAL`, `CARGO_NET_RETRY`, `RUSTUP_MAX_RETRIES`, `RUST_BACKTRACE`) and the two jobs above.
- In the `dod` job's last step:
  - a cargo warning that leaves `cargo make dod` at exit 0 makes the step print the warning lines and exit 1
  - a failing gate step makes it exit with cargo-make's status before the check runs

  (spec §5, §9, `evidence/probe-ci-step-warning-check.txt`).
- No `.gitattributes`, `rustfmt.toml` or `clippy.toml` is added (spec §7, §11).
- No Rust code changes.

**Proof:** no unit test applies, because the task adds a workflow file and no Rust code. The spec proved the file's structure with a yaml-rust2 0.10.4 probe (`evidence/probe-ci-yaml.txt`). It also ran the `dod` step's parsed script on clean, warning and failing copies (`evidence/probe-ci-step-warning-check.txt`). Byte identity with the probed file carries both proofs over:
- `diff .claudestacks/sdlc/2026-11-22-workspace-skeleton/evidence/probe-workspace/.github/workflows/ci.yml .github/workflows/ci.yml` prints nothing.

**Done when:**
- That `diff` prints nothing.
- `(set -o pipefail; mkdir -p target && cargo make dod 2>&1 | tee target/dod.log)` exits 0, `sed $'s/\x1b\\[[0-9;]*m//g' target/dod.log | grep -E '^ *(warning|error)'` prints nothing, and the Definition of Done is green.
- `git status --porcelain` lists no `.gitattributes`, `rustfmt.toml`, `.rustfmt.toml` or `clippy.toml`.
- Commit `ci(repo): run the Definition-of-Done gate and cargo-deny on GitHub Actions`.

---

## Risks

- **Riskiest step: the first real run.** That this exact file passes on GitHub cannot be shown before it is pushed (spec §7). This includes the warning-line check on the ubuntu and windows runners, whose `sed` and `grep` were not run locally: it was proven with macOS's only. The author accepted proof at the first pull request. The shared blocks are proven on all three runners by airsl run 44676722443 (`evidence/airsl-ci-jobs.txt`). The four `dod` lines and two `deny` lines that differ from airsl's workflow are each justified in spec §7 (`evidence/ci-vs-airsl-diff.txt`). Acceptance §10 item 7 is the first pull request's run (see Proof).
- **Windows line endings.** No `.gitattributes` or rustfmt config is added. airsl's Windows `dod` job passed `cargo fmt --check` the same way (spec §7, `evidence/airsl-format-config-absent.txt`).
- **Tool versions are pinned.** CI installs `cargo-make@0.37.24` and `cargo-deny@0.20.2`, the versions the gate was proven with (spec §1, §9).
- **Options not taken:** CI on every branch push (the author chose `main` plus pull requests, spec §7), and airsl's Developer Mode step and `cargo make examples` (both airsl-only).

## Proof

- `ci.yml` is byte-identical to the probe copy, and `cargo make dod` stays green.
- After `execute`, the author pushes the branch and opens its first pull request. All three `dod` legs and the `deny` job must succeed on that run (acceptance §10 item 7). This is the author's act, because pushing is outside `execute`.
