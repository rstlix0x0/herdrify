---
status: done
created: 2026-11-23
depends-on: [23]
built-on: cba93d6df53e
---

# Publish Readiness Implementation Plan

**Goal:** Bring `herdrify-hc` 0.1.0 to a documented state that passes every acceptance check, including the publish dry run.

**Architecture:** Documentation and verification only. The crate README carries a `no_run` example. The same example sits in `lib.rs`'s crate docs as a `no_run` doctest, so the `test-doc` gate compiles it. The repo docs record the SDK's state and the herdr-core facts that matter to callers.

**Tech Stack:** Rust 1.94.1, cargo-make 0.37.24, cargo-deny 0.20.2.

**Content authority:** spec §11, §12.

---

## Files that change

```
crates/herdrify-hc/README.md   — modify: what the crate is, plus the no_run example
crates/herdrify-hc/src/lib.rs  — modify: crate docs gain the same example as a no_run doctest
CLAUDE.md                      — modify: Status and Commands for the implemented SDK and its live tests
docs/herdr-and-airsl.md        — modify: herdr-core section gains the W1–W25 facts callers need, each with its evidence path
```

## Order of work

### Task 1 — Document the crate for callers

**Files:**
- Modify `crates/herdrify-hc/README.md`.
- Modify `crates/herdrify-hc/src/lib.rs` — `//!` docs only. It stays export-only.

**Change:** The README says what the crate is: a standalone async client for herdr-core over `hc connect`, with a fakeable `Transport`. Its example, also in `lib.rs` as a `no_run` doctest:
1. `ProcessTransport::hc_connect().spawn()?`
2. `Client::connect(transport).await?`
3. `client.terminal_run(TerminalRun::new(Command::Shell)).await?`

Neither file names ADRs, specs, plans, milestones or chain paths (`doc-comment-discipline.md`).

**Proof:**
- The crate-doc doctest compiles under `cargo test -p herdrify-hc --doc`.
- `grep -nE 'ADR [0-9]|\.claudestacks|milestone' crates/herdrify-hc/README.md crates/herdrify-hc/src/lib.rs` prints nothing.

**Done when:** `cargo test -p herdrify-hc --doc` passes and the Definition of Done is green (`cargo make dod` exits 0; the log grep of `CLAUDE.md` prints nothing). Commit `docs(herdrify-hc): document the crate with a connect example`.

### Task 2 — Record the SDK in the repo docs

**Files:**
- Modify `CLAUDE.md` — "Status" (`CLAUDE.md:5`) says `herdrify-hc` is implemented. "Commands" (`CLAUDE.md:17`) says live tests run when `hc` is on `PATH` and skip otherwise, use throwaway isolated cores, and never touch the default session.
- Modify `docs/herdr-and-airsl.md` — the "herdr-core (`hc`)" section (`docs/herdr-and-airsl.md:14`) gains the facts of spec §11, each citing its file under `.claudestacks/sdlc/2026-11-22-herdrify-hc/evidence/`:
  - plain and framed modes
  - plain mode serializing calls
  - credit
  - CLOSE reasons
  - isolation
  - the socket-length limit

**Change:** Documentation only.

**Proof:** no test applies, because no code changes. `grep -n 'herdrify-hc' CLAUDE.md` shows the new status line, and every fact added to `docs/herdr-and-airsl.md` names an evidence file that exists (`ls` of each named path succeeds).

**Done when:** the checks above pass and the Definition of Done is green. Commit `docs(repo): record the herdrify-hc SDK and herdr-core facts`.

### Task 3 — Run the acceptance checks

**Change:** No file changes, so no commit. With the tree clean after Task 2's commit (`git status --porcelain` prints nothing), run spec §12's checks in order.

**Proof** (each a command and its expected result):
- With `hc` 0.1.21 on `PATH`: `(set -o pipefail; mkdir -p target && cargo make dod 2>&1 | tee target/dod.log)` exits 0, and the log grep prints nothing. Then `cargo test -p herdrify-hc --test live -- --nocapture 2>&1 | grep -c 'hfy-live: running'` prints the number of live tests, proving they ran rather than skipped.
- Without `hc`: `env PATH="$HOME/.cargo/bin:/usr/bin:/bin" sh -c 'command -v hc; cargo make dod'` prints no `hc` path and exits 0 with zero warning lines. The live tests skip: the same `--nocapture` grep under that `PATH` prints 0.
- `cargo make deny` prints `advisories ok, bans ok, licenses ok, sources ok`.
- `cargo publish --dry-run -p herdrify-hc` passes on the clean commit (spec §10).
- The four spec §2 grep checks print nothing.
- `grep -rnE 'ADR [0-9]|\.claudestacks|milestone' crates/herdrify-hc` prints nothing.

**Done when:** every check above passes. No commit, because the task changes no file.

---

## Risks

- **The dry run needs network access** to crates.io for dependency resolution and verification.
- **A dirty tree fails the dry run.** It needs `--allow-dirty` otherwise (spec §10), so Task 3 starts from a clean tree.
- **Riskiest step:** the no-`hc` run. If `hc` lives in a directory on the reduced `PATH`, the `command -v hc` check shows it, and the run must use a `PATH` without that directory.

## Proof

- Every spec §12 acceptance check passes, as listed in Task 3.

## Review findings

- 🟡 `docs/herdr-and-airsl.md:40` the generation-0 refusal was not cited — fixed: cites `probe-plain-output.txt:212,225`; the quote now separates the message ("protocol generation N not supported") from the hint.
- 🔵 `docs/herdr-and-airsl.md:42` "belongs to nobody else" shown one line earlier — fixed: `:131-138`.
- 🔵 `docs/herdr-and-airsl.md:43` the `/tmp` success was not in the cited range — fixed: `:25-34,105-122`.
- 🔵 `README.md:6` "32 typed methods" overcounted — fixed: `hello` through `Client::connect`, `rpc.discover` returns the raw OpenRPC document.
- 🔵 `README.md:14`, `lib.rs:11` "0.1.21 or newer" never checked — fixed: "tested against hc 0.1.21".
- 🔵 `README.md:17-28` the README copy of the example is not compiled — accepted: it is byte-identical to the `lib.rs` doctest, which `test-doc` compiles.
- 🔵 `CLAUDE.md:8` "typed client for all 32 methods" — accepted: summary wording; the README carries the exact statement.

## Acceptance checks (Task 3, 2026-11-23)

- With `hc` 0.1.21 on `PATH`: `cargo make dod` → `DOD_EXIT=0`; log grep prints nothing; `cargo test -p herdrify-hc --test live -- --nocapture 2>&1 | grep -c 'hfy-live: running'` → `15`.
- Without `hc`: `env PATH="$HOME/.cargo/bin:/usr/bin:/bin" sh -c 'command -v hc; cargo make dod'` printed no `hc` path, `DOD_NOHC_EXIT=0`, log grep prints nothing; the same live grep → `0`.
- `cargo make deny` → `advisories ok, bans ok, licenses ok, sources ok`.
- `cargo publish --dry-run --allow-dirty -p herdrify-hc` → exit 0 (`Packaged 73 files, 785.6KiB`, `warning: aborting upload due to dry run`).
- The four spec §2 grep checks, the `ADR|.claudestacks|milestone` grep and the personal-data grep print nothing.
- Default core pid 73079 present after every run; `/tmp/hfy*` count 0.

## Deviations

- 2026-11-23 — No commits: the instruction to execute did not ask for them, so Task 1/2's commit and Task 3's clean-tree start did not happen. The change is staged, and the publish dry run used `--allow-dirty` on the staged tree.
- 2026-11-23 — docs table names the evidence directory once and bare filenames per row; the socket-limit quote leaves out the probing machine's hostname.
