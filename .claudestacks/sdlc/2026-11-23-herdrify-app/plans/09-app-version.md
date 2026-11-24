---
status: done
created: 2026-11-23
depends-on: [08]
built-on: aab842c3c7d3
---

# App Version Implementation Plan

**Goal:** Parse and order herdr-app versions against the supported minimum.

**Architecture:** One pure concept file, `protocol/version.rs`, holding `AppVersion`, its public
`PreIdentifier` and `InvalidAppVersion` (spec §2), all re-exported from `protocol/mod.rs`. The
shared rules of plan 02's Architecture apply verbatim, with a doctest on `AppVersion::parse`. This
plan depends on plan 08 only to order the edits of `protocol/mod.rs`.

**Tech Stack:** Rust 1.94, thiserror 2.0.21.

**Content authority:** spec §5.3; §10 (`InvalidAppVersion`).

---

## Files that change

```
crates/herdrify-app/src/protocol/version.rs   — create: AppVersion, InvalidAppVersion
crates/herdrify-app/src/protocol/mod.rs       — modify: mod version and re-exports
```

## Order of work

### Task 1 — Add AppVersion

**Files:**
- Create `crates/herdrify-app/src/protocol/version.rs`.
- Modify `crates/herdrify-app/src/protocol/mod.rs`.

**Change:**
- `AppVersion { major, minor, patch, pre: Vec<PreIdentifier> }`, where `PreIdentifier` is
  `Numeric(u64)` or `Alpha(String)`.
- `AppVersion::parse(&str)`, by spec §5.3's rules.
- `Ord` implementing semver 2.0 precedence, and `Display` as `X.Y.Z[-PRE]`.
- `const MINIMUM` (`0.1.16`) and `const PROVEN` (`0.1.17`).
- `InvalidAppVersion { text }`, Display `cannot read a herdr-app version from <text:?>`.

**Proof:**
- `parses_the_installed_output` — `"herdr-app 0.1.17\n"` is `0.1.17` and equals `PROVEN`.
- `drops_build_metadata` — `"herdr-app 0.1.18+abc"` is `0.1.18`.
- `pre_release_ranks_below_release` — `0.1.16-rc.1 < MINIMUM`; `0.1.16-alpha < 0.1.16-alpha.1 <
  0.1.16-beta < 0.1.16-rc.1`; numeric identifiers rank below alphanumeric ones.
- `refuses_malformed_text` — `""`, `"herdr-app 01.1.16"`, `"herdr-app 0.1"`, `"x 0.1.16"` and
  `"herdr-app 0.1.16-"` are `InvalidAppVersion`, and `"herdr-app 0.1"` displays
  `cannot read a herdr-app version from "herdr-app 0.1"`.
- `orders_against_minimum` — `0.1.15 < MINIMUM <= 0.1.16 < 0.2.0`.

**Done when:** `cargo test -p herdrify-app protocol::version` passes and the Definition of Done is
green. Commit `feat(herdrify-app): add AppVersion`.

---

## Risks

- None specific.

## Proof

- `cargo make dod` is green.

## Review findings

Task 1: SPEC COMPLIANT, 0🔴 2🟡 3🔵, blocking: none. The reviewer probed semver 2.0 precedence (the semver.org §11 chain, numeric < alphanumeric, numeric by value, longer list higher, leading zeros refused in core and numeric pre) and found it correct.
- 🟡 open: `protocol/version.rs:9`, `:77` — the docs say text that is not `MAJOR.MINOR.PATCH[-PRE][+BUILD]` is refused, but the BUILD part is not checked (`0.1.16+`, `0.1.16+a+b` parse). The code follows spec §5.3 ("BUILD is accepted and dropped"); only the two doc lines overstate it.
- 🟡 open: `protocol/version.rs:41`, `:66-74` — public fields and a directly buildable `PreIdentifier` let callers skip `parse` (`Alpha("1")` prints as `0.1.16-1` yet differs from the parsed value; `Alpha("")` prints text `parse` refuses).
- 🔵 open: `protocol/version.rs:43` — numeric-below-alphanumeric comes from the variant declaration order under `#[derive(Ord)]`, with no note (a test guards it).
- 🔵 open: `protocol/version.rs:161` — no test with a hyphen inside the build part (`0.1.16+build-1`), which depends on splitting `+` before `-`.
- 🔵 open: `protocol/version.rs:250` — only a trailing `\n` is tested; leading whitespace and `\r\n` pass by probe but are untested.
- Verified: `cargo test -p herdrify-app --lib protocol::version` → `test result: ok. 5 passed; 0 failed`.

## Deviations

- 2026-11-24 — Definition of Done: `cargo make dod` fails only in `herdrify-hc` `tests/live.rs::live_schema_matches_the_fixture` (installed `hc 0.1.22`, environment drift outside this chain). Gate run instead: workspace fmt-check, clippy `-D warnings`, `RUSTDOCFLAGS="-D warnings" cargo doc`, plus `cargo test -p herdrify-app` all-targets and `--doc` → `exit=0`, no warning/error lines; `121 passed`, `3 passed`, `104 passed`.
- 2026-11-24 — `protocol/mod.rs`'s `//!` block was brought up to date for the whole layer here, carrying plan 08's open finding: it now lists the run outcome and the version.

## Fix pass (2026-11-24, on the author's word "fix all of it")

Every open finding above is resolved:
- fixed: the module, `parse` and `InvalidAppVersion` docs say the build part after the first `+` is dropped without being checked (spec §5.3).
- fixed: `AppVersion`'s fields are private, with `const fn major()`, `minor()`, `patch()` and `fn pre() -> &[PreIdentifier]`. New tests `printed_version_parses_back_equal` and `exposes_the_parsed_parts`. Nothing else in the crate used the fields.
- fixed: a `//` note says `PreIdentifier`'s variant order sets numeric below alphanumeric.
- fixed: `drops_build_metadata` covers `herdr-app 0.1.16+build-1`; `parses_the_installed_output` covers `"  herdr-app 0.1.17\r\n"`.
- fixed: every test returns `Result`.
- Gate after the pass: `dod-app.sh` → `exit=0`, no warning/error lines; `140 passed`, `3 passed`, `123 passed`.
