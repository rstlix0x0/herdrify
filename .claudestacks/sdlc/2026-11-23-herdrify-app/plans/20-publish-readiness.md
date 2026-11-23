---
status: approved
created: 2026-11-23
depends-on: [06, 18, 19]
built-on: aab842c3c7d3
---

# Publish Readiness Implementation Plan

**Goal:** Make `herdrify-app` 0.1.0 pass spec §15's acceptance checks.

**Architecture:** Manifest, crate docs, examples and the repository docs (spec §12.7, §13, §14,
§15). Examples follow hc's layout: one folder per use case with `main.rs` and a `README.md`, plus
`examples/README.md`, discovered by Cargo with no `[[example]]` entries
(`crates/herdrify-hc/Cargo.toml` has none). Shipped READMEs use no planning vocabulary, and the
word-bounded §2 grep enforces it.

**Tech Stack:** Cargo, Rust 1.94.

**Content authority:** spec §12.7, §13 (version, keywords, categories, workspace entry), §14, §15.

---

## Files that change

```
crates/herdrify-app/Cargo.toml                 — modify: version 0.1.0, publish line removed, keywords, categories
Cargo.toml                                     — modify: herdrify-app workspace entry version 0.1.0
Cargo.lock                                     — modify: the version bump
crates/herdrify-app/README.md                  — modify: replace the stub with the crate's README
crates/herdrify-app/CHANGELOG.md               — create: 0.1.0
crates/herdrify-app/examples/README.md         — create: index of the three examples
crates/herdrify-app/examples/inspect/main.rs   — create: version, ls, status
crates/herdrify-app/examples/inspect/README.md — create: what it shows and how to run it
crates/herdrify-app/examples/drive/main.rs     — create: spaces, tabs and panes through HerdrApp
crates/herdrify-app/examples/drive/README.md   — create: what it does and how to run it
crates/herdrify-app/examples/files/main.rs     — create: build, check and print a layout, a theme and keys
crates/herdrify-app/examples/files/README.md   — create: what it prints and how to run it
CLAUDE.md                                      — modify: Status (herdrify-app implemented at 0.1.0, live tests)
docs/herdr-and-airsl.md                        — modify: herdr-app version 0.1.17 and rows F1–F17
```

## Order of work

### Task 1 — Add the examples

**Files:**
- The seven `examples/` files above.

**Change:**
- `inspect` and `drive` use `HerdrApp::local()`. They need herdr-app and a window, so no test runs
  them.
- `files` builds three values:
  - a `Layout` whose block names `herdrify-host` (via `Content::module`, no props)
  - a `Theme`
  - `Keys` with a `layout:herdrify-main` section

  For each, it prints `to_json`, re-parses the printed text, and exits non-zero if the re-parsed
  value differs.

**Proof:**
- `cargo build -p herdrify-app --examples` compiles all three.
- `cargo run -q -p herdrify-app --example files` exits 0, which proves the three documents
  re-parse equal.

**Done when:** both commands succeed and the Definition of Done is green. Commit
`docs(herdrify-app): add examples per use case`.

### Task 2 — Bump to 0.1.0 and write the crate docs

**Files:**
- Modify `crates/herdrify-app/Cargo.toml`, `Cargo.toml` and `Cargo.lock`.
- Modify `crates/herdrify-app/README.md`.
- Create `crates/herdrify-app/CHANGELOG.md`.

**Change:** spec §13's version, keywords, categories and workspace entry. The README covers:
- what the crate is
- `HerdrApp` vs `Client`
- the file models
- that live tests open an isolated window

**Proof:**
- `cargo publish --dry-run -p herdrify-app --allow-dirty` passes. Drop `--allow-dirty` when run
  on the committed tree in Task 4.

**Done when:** the dry run passes and the Definition of Done is green. Commit
`chore(herdrify-app): prepare 0.1.0 for publishing`.

### Task 3 — Update the repository docs

**Files:**
- Modify `CLAUDE.md` and `docs/herdr-and-airsl.md`.

**Change:** spec §14:
- `CLAUDE.md`'s Status names `herdrify-app` 0.1.0 and its live tests
- the herdr-app table's version becomes 0.1.17, with rows F1–F17 citing
  `.claudestacks/sdlc/2026-11-23-herdrify-app/evidence/` files

**Proof:**
- `grep -n 'herdrify-app' CLAUDE.md` shows the new Status sentence.
- Every evidence path added to `docs/herdr-and-airsl.md` exists: each one is checked with `ls`.

**Done when:** both checks hold and the Definition of Done is green. Commit
`docs: record herdrify-app 0.1.0 and the herdr-app 0.1.17 facts`.

### Task 4 — Run the acceptance checks

**Change:** No file changes. Spec §15 run in full on the committed tree:
- `cargo make dod`, with the `CLAUDE.md` log grep
- `cargo deny check`
- `cargo publish --dry-run -p herdrify-app`
- the seven §2 enforcement greps, the last word-bounded (`-w`)
- `grep -rn 'Box<dyn' crates/herdrify-app/src`, showing only `herdr_app/call.rs` under its
  `// dyn:` line

**Proof:**
- each command exits 0, each §2 grep prints nothing, and the `Box<dyn` grep has its one hit

**Done when:** all of §15 holds. There is no commit if nothing changed.

---

## Risks

- `cargo publish --dry-run` packages only files under the crate. The test fixtures live under the
  crate's `tests/`, so they ship, as in hc.

## Proof

- Spec §15 holds in full.
