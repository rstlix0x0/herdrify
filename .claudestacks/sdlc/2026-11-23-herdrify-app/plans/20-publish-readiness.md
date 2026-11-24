---
status: done
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

## Review findings

Reviewer (one pass, 2026-11-24, over Tasks 1–3): plan-compliant, blocking set empty. `cargo build -p herdrify-app --examples` ok; `cargo run -q -p herdrify-app --example files` exit 0; `cargo publish --dry-run -p herdrify-app --allow-dirty` exit 0, 81 files, all 7 `examples/` files packaged; the word-bounded grep over README, CHANGELOG and examples prints nothing; every F1–F17 evidence path exists and supports its row; `drive` makes and closes only its own space, tab and pane, by id, and never sends `window.close` or `app.quit`. `inspect` and `drive` were built, never run.

- open 🟡 package: no `LICENSE` in the package; hc ships `crates/herdrify-hc/LICENSE -> ../../LICENSE`.
- open 🟡 `examples/drive/README.md:15-16`: "without `.confirmed()` herdr-app refuses" is false for tabs (`evidence/probe-commands-output.txt:160-166`); only `space.close` refuses.
- open 🔵 `examples/README.md:8`: a `rust` fence on a fragment using top-level `?`.
- open 🔵 `examples/drive/main.rs:42-44`: when the walk and the close both fail, the close error is dropped.
- open 🔵 `examples/files/README.md:31-84`: the sample lacks the final blank line; nothing keeps it in step with the program.
- open 🔵 `README.md:5-6`, `CHANGELOG.md:15`: "it starts `herdr-app`" can read as launching a window; "runs" is clearer.
- open 🔵 `CLAUDE.md:39-40`: "Each opens its own isolated herdr-app window": only `isolated_window_scenario` does.
- open 🔵 `docs/herdr-and-airsl.md:3,10`: the header date and the 0.1.17 version row now sit over older rows cited to 0.1.16 files.
- open 🔵 `docs/herdr-and-airsl.md:50`: "(77 on 0.1.17)" is cited to a 0.1.16 file; `evidence/probe-cli-output.txt:9` holds it.
- open 🔵 (plan drift): the live-test sentence went into `CLAUDE.md`'s Commands section beside hc's, not Status.

Task 4 (acceptance, 2026-11-24, on the working tree):

- `sh dod-app.sh` → `exit=0`, no warning/error lines; 250 unit, catalog 5, client_fake 8, files_roundtrip 3, live 5 (ran, 3.34 s), 447 + 29 doctests.
- `cargo make dod` → `rc=105`; its only failure: `test live_schema_matches_the_fixture ... FAILED` (`crates/herdrify-hc/tests/live.rs:626`, `-p herdrify-hc --test live`), outside this chain.
- `cargo deny check` → `advisories ok, bans ok, licenses ok, sources ok`.
- `cargo publish --dry-run -p herdrify-app --allow-dirty` → `Packaged 81 files, 783.7KiB (154.1KiB compressed)`, `Verifying herdrify-app v0.1.0`, `warning: aborting upload due to dry run`.
- The seven §2 greps: the first five and the word-bounded seventh print nothing. The sixth (`tokio` in files/protocol/port) printed `src/port/transport.rs:54:/// # #[tokio::main(flavor = "current_thread")]`, a doctest added by plan 14's fix pass. Fixed: the doctest now polls the run once with `std::task::Waker::noop()` and asserts the exit; `cargo test -p herdrify-app --doc port::transport` → `test result: ok. 2 passed`; the grep then prints nothing.
- `grep -rn 'Box<dyn' crates/herdrify-app/src | grep -v 'std::error::Error'` → `herdr_app/call.rs:143` (`BoxedCall`, under its `// dyn:` line) and the test alias `adapter/process.rs:284`.
- After the runs `pgrep -fl herdr-app` printed nothing.

## Deviations

- 2026-11-24: `cargo make dod` is replaced by the scoped gate (workspace fmt, clippy and doc; herdrify-app tests and doctests, live tests included); `cargo make dod`'s only failure is herdrify-hc's `live_schema_matches_the_fixture` against the installed hc 0.1.22, outside this chain.
- 2026-11-24 (Task 4): spec §15 asks for the checks "on the committed tree" and a dry run without `--allow-dirty`. Nothing is committed without the author's word, so every check ran on the working tree and the dry run carried `--allow-dirty`. Re-run `cargo publish --dry-run -p herdrify-app` once the tree is committed.
- 2026-11-24 (Task 4): the `Box<dyn` check is judged on library code (see plan 17's Deviations).
- 2026-11-24 (Task 2): the CHANGELOG links `https://semver.org/`, because hc's `/spec/v2.0.0.html` path matches the word-bounded grep.

## Fix pass (2026-11-24, on the author's word "fix all of open findings")

- fixed 🟡 package: `crates/herdrify-app/LICENSE -> ../../LICENSE`, as hc; `cargo package -p herdrify-app --list --allow-dirty` lists `LICENSE`; dry run → `Packaged 83 files, 814.2KiB (163.7KiB compressed)`.
- fixed 🟡 drive/README.md: `space.close` is refused without `.confirmed()`; `tab.close` only asks, on the last tab.
- fixed 🔵 examples/README.md:8 (a complete `async fn`), drive/main.rs (the close error is printed and the space named as left behind), files/README.md (sample equals `cargo run -q -p herdrify-app --example files`, `diff` empty), README/CHANGELOG/inspect README ("runs", not "starts"), CLAUDE.md (only the window scenario opens a window), docs/herdr-and-airsl.md (header dates and version rows match their cited files; "77 on 0.1.17" cites `2026-11-23-herdrify-app/evidence/probe-cli-output.txt:9`).
- declined 🔵 a drift guard for the `files` sample: no gate runs examples; `tests/files_roundtrip.rs` covers the models.
- recorded 🔵 plan drift (Commands vs Status): already under Review findings.
- Gate after the pass (all five fixes together): `dod-app.sh` → `exit=0`, no warning/error lines; 253 unit, catalog 5, client_fake 8, files_roundtrip 3, live 10 (ran, not skipped), 447 + 29 doctests; `cargo clippy -p herdrify-app --all-targets --all-features --target x86_64-pc-windows-msvc -- -D warnings` clean; `cargo deny check` → `advisories ok, bans ok, licenses ok, sources ok`; the seven §2 greps print nothing; `pgrep -fl herdr-app` prints nothing.
