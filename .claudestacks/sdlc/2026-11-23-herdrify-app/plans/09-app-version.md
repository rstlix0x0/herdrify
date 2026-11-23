---
status: approved
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
