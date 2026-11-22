---
status: approved
created: 2026-11-22
depends-on: [03, 04]
built-on: 38eeb1508131
---

# Status Docs Implementation Plan

**Goal:** Bring the repository's status documentation in line with the built workspace, as spec §8 words it.

**Architecture:** Text edits only.
- `CLAUDE.md`'s `## Status` loses its pre-code wording.
- A new `## Commands` section, placed directly after `## Status`, lists:
  - the toolchain install and the pinned tool install
  - the gate (plan 03), formatting and the supply-chain check (plans 02 and 03)
  - the version-line run (plan 01)
  - the CI sentence (plan 04)
- `README.md`'s status block changes its opening sentence only.

**Tech Stack:** Markdown.

**Content authority:** spec §8, and §9 rows 5 (the local check) and 10 (the local half; `CLAUDE.md` lists the installs; plan 04 owns the CI half); acceptance §10 item 6.

---

## Files that change

```
CLAUDE.md   — modify: Status first paragraph reworded, "When the workspace lands…" sentence removed, new ## Commands section (spec §8)
README.md   — modify: status block opening sentence reworded (spec §8)
```

## Order of work

### Task 1 — Update CLAUDE.md and README.md to the built workspace

**Files:**
- Modify `CLAUDE.md`:
  - In `## Status`, replace the first paragraph's text "Pre-code. There is no Cargo workspace, build or test yet. The architecture is decided:" with "The Cargo workspace exists: nine crates, no features yet. The architecture is decided:".
  - Delete the sentence "When the workspace lands, add the real build/lint/test commands here."
  - Insert the `## Commands` section, exactly as the outer ````markdown fence in spec §8 shows it, directly after `## Status` and before `## What herdrify is`. It holds the `sh` block of eight command lines with their comments, including the two log-check lines, and the CI sentence.
- Modify `README.md`:
  - In the status block, replace "> **Status: planned.** Nothing is built yet. This README describes the plan." with "> **Status: early.** The Cargo workspace and its nine crates exist; no features are built yet. This README describes the plan."
  - Keep the rest of the block exactly as it is: the words "The decisions behind" that end README.md line 7, and the whole of line 8 ("> it are in …"). Only the opening sentence's text is replaced, in place; nothing is rewrapped (spec §8: "The rest of the block stays as it is").

**Change:**
- Both files describe the built state.
- No other section of either file changes.
- No file under `docs/`, `.claudestacks/` or `crates/` changes.

**Proof:** no unit test applies, because the task adds no Rust code. The checks are text assertions, which fail before the edit and pass after:
- `grep -c "Pre-code" CLAUDE.md` prints `0`.
- `grep -c "When the workspace lands" CLAUDE.md` prints `0`.
- `grep -n "The Cargo workspace exists: nine crates, no features yet. The architecture is decided:" CLAUDE.md` finds one line.
- `grep -n "^## " CLAUDE.md` lists `## Status`, then `## Commands`, then `## What herdrify is`, in that order.
- The `## Commands` section of `CLAUDE.md` equals the text inside spec §8's outer fence, character for character:
  run from the repository root, `diff <(awk '/^````markdown$/{f=1;next} /^````$/{f=0} f' .claudestacks/sdlc/2026-11-22-workspace-skeleton/spec.md) <(awk '/^## Commands$/{f=1} /^## What herdrify is$/{f=0} f' CLAUDE.md | sed '$d')` prints nothing. The final `sed` drops the one blank line that separates the section from `## What herdrify is`.
- `grep -c "Nothing is built yet" README.md` prints `0`, and `grep -c "Status: early" README.md` prints `1`.
- `git diff --stat` lists only `CLAUDE.md` and `README.md`.

**Done when:**
- Those checks hold.
- `(set -o pipefail; mkdir -p target && cargo make dod 2>&1 | tee target/dod.log)` exits 0, `sed $'s/\x1b\\[[0-9;]*m//g' target/dod.log | grep -E '^ *(warning|error)'` prints nothing, and the Definition of Done is green.
- Commit `docs(repo): document the workspace commands and update the status`.

---

## Risks

- **The commands must match what plans 01–04 built.** Every command in the new section was proven in the spec:
  - `rustup toolchain install`: `evidence/probe-rustup-install.txt`
  - `cargo install … @<ver>`: `evidence/probe-cargo-install-help.txt:3,6`
  - `cargo make dod`: §5
  - `cargo make deny`: §6
  - `cargo run -q -p herdrify-cli`: `evidence/probe-run.txt`
  - the two log-check lines: §5, `evidence/probe-ci-step-warning-check.txt` (bash and zsh)

  The section is copied from §8, not rewritten.
- **Wrapping the README blockquote** could tempt a rewording. Only the opening sentence changes.
- **Option not taken:** putting the commands inside `## Status`. The spec places them in their own section (§8).

## Proof

- Every `grep` check above holds on the committed tree.
- `git diff --stat HEAD~1` on this plan's commit lists only `CLAUDE.md` and `README.md`.
- `cargo make dod` stays green.
