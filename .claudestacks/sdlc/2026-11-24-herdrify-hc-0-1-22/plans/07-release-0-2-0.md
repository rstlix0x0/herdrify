---
status: done
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

## Review findings

Reviewer verdict (Tasks 1–2): COMPLIANT, 0🔴 1🟡 9🔵 2❓, no drift. Re-run: `cargo check --workspace`, fmt, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace`, `cargo test -p herdrify-hc --doc` (98), `cargo metadata --locked` all ok; counts recounted (36 methods / 39 schemas on 0.1.22; 32 / 36 on 0.1.21); every CHANGELOG name exists in `src`.

- 🟡 `CHANGELOG.md:13` "for files too large for one frame" was false (a put in pieces is capped at 8 MiB, whose base64 fits one 16 MiB frame payload) and clashed with the concepts word **frame** — fixed: pieces of at most 1 MiB, up to 8 MiB per put, so a big file never holds up the connection; needs a framed connection and core 0.1.22+ (`evidence/probe-022-output.txt:72-73`, `:163-165`).
- ❓ `src/herdr_core/put.rs:1-2` made the same claim — fixed the same way. `cargo test -p herdrify-hc --doc` → `test result: ok. 98 passed`.
- 🔵 `CHANGELOG.md:9` lead line repeated the bullet at `:30` — fixed: removed.
- 🔵 `CHANGELOG.md` Added missed `protocol::InvalidTransferId`, `Error::DOCS_OPTION`, `TransferId::random` — fixed: added, with `TransferId::new`.
- 🔵 `examples/README.md:28` "written against hc 0.1.22" was untrue (examples unchanged since 0.1.21) — fixed: written against hc 0.1.21, unchanged for 0.2.0, built in the gate.
- 🔵 `examples/connect/README.md:37-38` sample output was retyped, not captured — fixed: the captured 0.1.21 lines restored byte for byte from `HEAD` and labelled "Output captured on hc 0.1.21:". No example was run.
- 🔵 `docs/architecture.md:26` ADR column out of order — fixed: "0001, 0007, 0008".
- 🔵 `docs/herdr-and-airsl.md:3-6` intro put all raw output in `adrs/evidence/` — fixed: rows naming a chain's evidence folder are the exception; ragged reflow fixed.
- 🔵 `docs/herdr-and-airsl.md:28-29` "observed on hc 0.1.21 and 0.1.22" was too broad — fixed: the last two rows (put in pieces, `docs` filter) were observed on 0.1.22, the rest on 0.1.21.
- 🔵 `CLAUDE.md:91-95` Grants (0008) left out the `docs` rule — fixed: "Components cannot set the `docs` param of `events.subscribe`: it is refused." (ADR 0008's Decision line).
- ❓ "0.1.21 still supported" while `live_schema_matches_the_fixture` fails on 0.1.21 — fixed: README and CHANGELOG say every live test passes on hc 0.1.21 except the schema drift check, which pins 0.1.22, and the put tests skip there (`evidence/live-tests-0.1.21-after.txt`).
- 🔵 nothing fails the gate if the version, counts or ADR numbers in prose revert — declined: they are prose; the plan grep and the publish dry run cover the version and counts.

Plan grep after the fixes: 5 hits, all deliberate 0.1.21 history (README support line, the 0.1.21 count row, examples README, the connect example's captured-output label and line).

## Gate

With hc 0.1.22 and herdr-app 0.1.17 on `PATH`, `HC_*` unset:
- `(set -o pipefail; mkdir -p target && cargo make dod 2>&1 | tee target/dod.log)` → exit 0. `sed $'s/\x1b\\[[0-9;]*m//g' target/dod.log | grep -E '^ *(warning|error)'` prints nothing. Live tests ran, none skipped: herdrify-app live 10 passed; herdrify-hc live 19 passed, live_herdr_core 5 passed; lib 363, client_fake 106, schema_coverage 9, adapter_stream 4, herdrify-app lib 253; doctests 447 + 29 + 98.
- `cargo make deny` → exit 0: `advisories ok, bans ok, licenses ok, sources ok`.
- `cargo clippy -p herdrify-hc --all-targets --all-features --target x86_64-pc-windows-msvc -- -D warnings` → exit 0, `Finished`.
- `cargo publish --dry-run -p herdrify-hc --allow-dirty` → exit 0: `Packaged 110 files, 985.6KiB (208.0KiB compressed)`, `Verifying herdrify-hc v0.2.0`, `warning: aborting upload due to dry run`.
- Leftovers: `/tmp/hf*` identical before and after (`hfa48416-0`, `hfa48416-1`, `hfy.4K8V`, all 2026-11-23). The only `--hc-launchd` process is pid 99744, from 2026-11-23, not started by this run.

## Deviations

- 2026-11-24: Tasks 1–2 ran alongside plan 06 Task 2 (disjoint files); the gate (Task 3) ran after plan 06 was done.
- 2026-11-24: The connect example's sample output keeps the captured 0.1.21 lines (labelled) instead of the plan's "connected to hc 0.1.22, 36 methods": that line was never captured, and no example was run against a core. The examples README says the examples were written against 0.1.21 instead of the plan's "written against hc 0.1.22", for the same reason.
- 2026-11-24: The strict `cargo publish --dry-run -p herdrify-hc` without `--allow-dirty` is left for after the author commits, as the plan says.
- 2026-11-24: Not committed (instruction: do not commit).
