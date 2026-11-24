---
status: approved
created: 2026-11-24
depends-on: [01, 02, 03, 04, 05, 06]
built-on: 2db7af9df077
---

# Release 0.2.0 Implementation Plan

**Goal:** Ship `herdrify-hc` 0.2.0 with every document saying what it now supports.

**Architecture:** No code. The crate and its workspace pin move to 0.2.0, because `Error` gains
a variant (spec §4.3, §9), and the CHANGELOG records the release. The crate docs, the repo docs
and `CLAUDE.md` move to hc 0.1.22, 36 methods and ADR 0008. The full gate, `cargo deny` and the
publish dry run close the chain.

**Tech Stack:** Cargo, cargo-make 0.37.24, cargo-deny 0.20.2.

**Content authority:** spec §9, §10.

---

## Files that change

```
crates/herdrify-hc/Cargo.toml                    — modify: version 0.2.0
Cargo.toml                                       — modify: [workspace.dependencies] herdrify-hc version 0.2.0
Cargo.lock                                       — modify: follows the version
crates/herdrify-hc/CHANGELOG.md                  — modify: the [0.2.0] entry
crates/herdrify-hc/README.md                     — modify: 36 methods; tested against hc 0.1.22, 0.1.21 supported
crates/herdrify-hc/src/lib.rs                    — modify: "tested against hc 0.1.22"
crates/herdrify-hc/examples/README.md            — modify: "written against hc 0.1.22"
crates/herdrify-hc/examples/connect/README.md    — modify: sample output "connected to hc 0.1.22, 36 methods"
CLAUDE.md                                        — modify: herdrify-hc at 0.2.0, 36 methods, ADRs 0001–0008, ADR 0008 grants line
docs/concepts.md                                 — modify: ADR 0008 in place of 0004; 22 of hc's 36 methods
docs/architecture.md                             — modify: ADR 0008 in place of 0004
docs/herdr-and-airsl.md                          — modify: hc 0.1.22 facts; new rows for the put in pieces and docs; ADR 0008
```

## Order of work

### Task 1 — Bump the version and write the CHANGELOG

**Files:**
- Modify `crates/herdrify-hc/Cargo.toml:4`: `version = "0.2.0"`.
- Modify `Cargo.toml:14`: `herdrify-hc = { path = "crates/herdrify-hc", version = "0.2.0" }`.
- Modify `Cargo.lock`, through `cargo check -p herdrify-hc` (no hand edits).
- Modify `crates/herdrify-hc/CHANGELOG.md`: a `## [0.2.0] - <the date of the release commit>`
  entry above 0.1.0, with these sections:
  - **Added:**
    - the four `Client::terminal_put_*` calls and their request and reply types
    - `TransferId`
    - `TerminalHandle::put_begin` and `put_in_pieces`
    - `TerminalOps::put_chunk`, `put_finish` and `put_abort`
    - `EventsSubscribe::docs` and the `docs` option of `HerdrCore::events`
  - **Changed (breaking):** `Error::NeedsNewerCore`. `docs` is refused on cores older than 0.1.22.
  - **Changed:** tested against hc 0.1.22; 0.1.21 is still supported.

**Change:** Versions and the release note only.

**Proof:**
- `cargo check --workspace` resolves with herdrify-hc 0.2.0. `herdrify-herdr` takes it through
  `{ workspace = true }`.

**Done when:** `cargo check --workspace` passes and the Definition of Done is green. Commit
`chore(herdrify-hc): release 0.2.0`.

### Task 2 — Bring the docs to hc 0.1.22 and ADR 0008

**Files:**
- Modify `crates/herdrify-hc/README.md:8,18`.
- Modify `crates/herdrify-hc/src/lib.rs:19`.
- Modify `crates/herdrify-hc/examples/README.md:28`.
- Modify `crates/herdrify-hc/examples/connect/README.md:37`.
- Modify `CLAUDE.md`:
  - `:7-8`: herdrify-hc at 0.2.0, all 36 methods
  - `:14`: "(0001–0008)"
  - `:91`: "Grants (0008)"
  - `:95`: "22 of 36 methods are never exposed"
- Modify `docs/concepts.md:24-27`: ADR 0008, and "22 of hc's 36 methods".
- Modify `docs/architecture.md:26,29,110`: ADR 0008, with the link to
  `0008-grants-owned-by-core-hc-0-1-22.md`.
- Modify `docs/herdr-and-airsl.md`:
  - `:10`: hc 0.1.22
  - `:19`: 36 methods, 39 schemas, citing the chain's `method-counts.txt` and
    `schema-counts.txt`
  - `:27`: herdrify-hc 0.2.0, observed on 0.1.21 and 0.1.22
  - `:97`: ADR 0008
  - two new rows citing the chain's evidence:
    - the put in pieces: this core's terminals only, 16–64 hex ids, at most 1 MiB per piece and
      8 MiB per put
    - the `docs` filter: needs 0.1.22, and 0.1.21 ignores it

**Change:** Wording only. Each sentence states what the spec or its evidence states.

**Proof:**
- `grep -rn '32 methods\|hc 0.1.21\|18 of\|0001–0007' CLAUDE.md docs crates/herdrify-hc/README.md
  crates/herdrify-hc/examples crates/herdrify-hc/src/lib.rs` prints only lines that describe
  0.1.21 history on purpose (the 0.1.21 support statement and the evidence-folder sentence).
- `RUSTDOCFLAGS="-D warnings" cargo doc -p herdrify-hc --no-deps` is clean.

**Done when:** the grep shows no stale line and the Definition of Done is green. Commit
`docs(herdrify-hc): document hc 0.1.22 support and ADR 0008`.

### Task 3 — Run the full gate

No file changes of its own.

**Change:** Run each check and keep its result for the plan's `## Review findings`:
- `(set -o pipefail; mkdir -p target && cargo make dod 2>&1 | tee target/dod.log)` with hc 0.1.22
  installed
- `sed $'s/\x1b\\[[0-9;]*m//g' target/dod.log | grep -E '^ *(warning|error)'` prints nothing
- `cargo make deny`
- `cargo clippy -p herdrify-hc --all-targets --all-features --target x86_64-pc-windows-msvc -- -D warnings`
- `cargo publish --dry-run -p herdrify-hc --allow-dirty`
- After the runs, `ls /tmp | grep '^hf'` and the `--hc-launchd` process list show no leftover
  core from these tests.

**Proof:**
- `cargo make dod` exits 0. Its log has no warning or error line. The herdrify-hc and
  herdrify-app live tests ran, not skipped.
- `cargo deny` reports advisories, bans, licenses and sources ok.
- The Windows clippy check is clean.
- The dry run reports `Verifying herdrify-hc v0.2.0`.

**Done when:** every check above passes. The strict dry run without `--allow-dirty` runs again
once the author commits.

---

## Risks

- **`cargo make dod` also runs herdrify-app's live window scenario**, which opens an isolated
  window. That was the author's approved choice in the herdrify-app chain. It is unchanged here.
- **Spec §10 asks for the dry run on the committed tree.** Execute does not commit, so it runs
  with `--allow-dirty` and records that in `## Deviations`. When the author asks for the commit,
  the session that makes it runs `cargo publish --dry-run -p herdrify-hc` (no `--allow-dirty`)
  right after, and reports the result. That is how the herdrify-app chain closed its own dry run.
- **Doc wording drift.** The grep in Task 2 is the guard. A deliberate 0.1.21 mention, such as
  "0.1.21 still supported", stays.

## Proof

- `cargo make dod` is green with zero warnings with hc 0.1.22 installed.
- `cargo make deny` is ok.
- `cargo publish --dry-run -p herdrify-hc --allow-dirty` verifies v0.2.0.
