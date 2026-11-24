---
status: done
created: 2026-11-23
depends-on: [02]
built-on: aab842c3c7d3
---

# Protocol Identifiers Implementation Plan

**Goal:** Give the `protocol` layer its identifier newtypes, including `Folder` and `Argv`.

**Architecture:** The first files of the `protocol` domain layer: pure, with no tokio and no I/O,
depending only on `files` (spec §2). `protocol/mod.rs` is export-only, and `lib.rs` gains
`pub mod protocol;`. The six names shared with `files` (`NodeId`, `ModName`, `LayoutName`,
`ThemeName`, `TerminalAddress`, `CommandName`) are reused through `crate::files`, never redefined
(modularity Rule 2). The shared rules of plan 02's Architecture apply verbatim, doctests on every
public item with behaviour included.

**Tech Stack:** Rust 1.94, serde 1.0.229, thiserror 2.0.21.

**Content authority:** spec §5.4; §10 (`FolderError` rows, `EmptyValueError`); §11 (the folder
and argv rows).

---

## Files that change

```
crates/herdrify-app/src/lib.rs                    — modify: pub mod protocol
crates/herdrify-app/src/protocol/mod.rs           — create: export-only table of contents of the protocol layer
crates/herdrify-app/src/protocol/identifiers.rs   — create: the spec §5.4 newtypes, Folder, FolderError, Argv
```

## Order of work

### Task 1 — Add the protocol module and its name newtypes

**Files:**
- Modify `crates/herdrify-app/src/lib.rs`.
- Create `crates/herdrify-app/src/protocol/mod.rs` — the `//!` block (herdr-app's CLI language,
  sans-IO).
- Create `crates/herdrify-app/src/protocol/identifiers.rs`.

**Change:**
- `SpaceId`, `TabId`, `PaneId`, `OverlayId`, `SpaceName`, `TabName`, `PaneName`, `TerminalName`,
  `MachineName`, `Task` and `AppName`.
- Each has the same shape as `files` names (plan 02 Task 1), with the `EmptyValueError` from
  `files` and spec §5.4's labels.

**Proof:**
- `each_protocol_name_refuses_empty_with_its_label` — `new("")` on each of the eleven gives its
  §5.4 label.
- `protocol_names_round_trip_through_serde` — `"sp1"` into `SpaceId` and back; `""` refused.

**Done when:** `cargo test -p herdrify-app protocol::identifiers` passes and the Definition of Done
is green. Commit `feat(herdrify-app): add the protocol layer and its name newtypes`.

### Task 2 — Add Folder and Argv

**Files:**
- Modify `crates/herdrify-app/src/protocol/identifiers.rs`.

**Change:**
- `Folder::new(impl AsRef<Path>) -> Result<Folder, FolderError>` stores an absolute UTF-8 path:
  - empty → `FolderError::Empty(EmptyValueError { label: "folder" })`
  - relative → `FolderError::Relative { path }`
  - non-UTF-8 → `FolderError::NotUtf8 { path }`

  It serializes as its string, with `as_str` and `Display`, and never resolves or expands.
- `FolderError` Displays:
  - `Relative`: `"<path>" is not an absolute path`
  - `NotUtf8`: `"<lossy path>" is not UTF-8`
  - `Empty`: transparent
- `Argv`: a non-empty `Vec<String>`. `Argv::new(impl IntoIterator<Item = impl Into<String>>)`
  refuses an empty list with `EmptyValueError { label: "argv" }`. It serializes as a JSON array.

**Proof:**
- `folder_accepts_absolute_paths` — `/tmp/x` is accepted and serializes as `"/tmp/x"`.
- `folder_refuses_relative_paths` — `x/y`, `./x` and `~/code` are each `Relative`, with Display
  `"x/y" is not an absolute path`.
- `folder_refuses_empty` — `""` displays `a folder cannot be empty`.
- `folder_refuses_non_utf8` (`#[cfg(unix)]`) — a path built from the bytes `b"/tmp/\xff"` is
  `NotUtf8`.
- `argv_refuses_empty_and_keeps_order` — `Argv::new(Vec::<String>::new())` displays
  `a argv cannot be empty`; `["sh","-c","echo a b"]` serializes as that array.

**Done when:** `cargo test -p herdrify-app protocol::identifiers` passes and the Definition of Done
is green. Commit `feat(herdrify-app): add Folder and Argv`.

---

## Risks

- `EmptyValueError`'s Display grammar gives `a argv cannot be empty`. That is the spec's exact
  rule (§10 `a <label> cannot be empty`), so the test pins it as written.

## Proof

- `cargo make dod` is green.

## Review findings

Task 1 (reviewed with plan 06 Task 1): SPEC COMPLIANT, 0🔴 0🟡, blocking: none.
- 🔵 open: `files/identifiers.rs:47` — `name!` takes `fmt`, `FromStr`, `Serialize`, `Deserialize`, `EmptyValueError` from the caller's scope; `protocol/identifiers.rs` imports them only for the macro. Absolute paths (`::std::fmt`, `$crate::files::EmptyValueError`) would make it hygienic.
- 🔵 open: `files/identifiers.rs:66` — the generated doctest uses `"herdrify-host"` for every type (e.g. `SpaceId::new("herdrify-host")`).
- 🔵 open: `files/mod.rs:41` — `pub(crate) use identifiers::name;` sits inside the `#[doc(inline)] pub use` run.
- 🔵 open: `protocol/identifiers.rs:14`, `protocol/mod.rs:5` — docs say the layer reuses the six `files` names; nothing uses them yet (true once later plans land).
- ❓ resolved: the eleven labels were checked against spec §5.4's table by hand; all eleven match.

Task 2: SPEC COMPLIANT, 0🔴 1🟡, blocking: none.
- 🟡 open: `protocol/identifiers.rs:167` — no test that reading a `Folder` refuses `"x/y"` or `""`; reverting `try_from = "String"` would go unnoticed.
- 🔵 open: `protocol/identifiers.rs:133` — `Empty(#[from] EmptyValueError)` converts any `EmptyValueError` (e.g. a `mod name` one) into `FolderError::Empty`.
- 🔵 open: `protocol/identifiers.rs:159` — the `Folder` doctest depends on `std::env::temp_dir()` being absolute UTF-8.
- 🔵 open: `protocol/identifiers.rs:177` — the documented check order (empty, UTF-8, absolute) is untested for a non-UTF-8 relative path.
- ❓ open: `Argv::new([""])` is accepted (items may be empty); matches the task, which asks only for a non-empty list.
- Verified: `cargo test -p herdrify-app --lib protocol::identifiers` → `test result: ok. 7 passed; 0 failed`.

## Deviations

- 2026-11-24 — Definition of Done: `cargo make dod` fails only in `herdrify-hc` `tests/live.rs::live_schema_matches_the_fixture`, because the installed `hc --version` is `hc 0.1.22` (environment drift, outside this chain). Gate run instead: workspace fmt-check, clippy `-D warnings`, `RUSTDOCFLAGS="-D warnings" cargo doc`, plus `cargo test -p herdrify-app` all-targets and `--doc` → `exit=0`, no warning/error lines; `106 passed`, `3 passed`, `89 passed`.
- 2026-11-24 — Task 1: the `files` `name!` macro was made crate-visible (`pub(crate) use`) and given a leading module-name argument so its generated doctests import from `files` or `protocol`; the eleven protocol names reuse it instead of a copy.
- 2026-11-24 — Task 2: the plan is silent on reading; `Folder` and `Argv` deserialize through `try_from`, refusing what `new` refuses, mirroring the name newtypes. Added `Argv::as_slice` to read the list. Unit tests use `C:\tmp\x` on Windows, where `/tmp/x` is not absolute.

## Fix pass (2026-11-24, on the author's word "fix all of it")

Every open finding above is resolved:
- fixed: `name!` is hygienic (`::core::fmt`, `::core::str::FromStr`, `::serde::…`, `$crate::files::EmptyValueError`); `protocol/identifiers.rs` no longer imports anything only for the macro.
- fixed: `name!` takes an example value; each of the 19 names has its own doctest value (`SpaceId` `"sp1"`, `LayoutName` `"main"`, …).
- fixed: `files/mod.rs` — `pub(crate) use identifiers::name;` now sits after the public re-exports, with its own comment.
- checked: the "reuses the `files` names" sentence is now true (`CommandName` in catalog/invocation, `TerminalAddress` in listing/status, `NodeId`/`LayoutName` in status).
- fixed: `FolderError::Empty` dropped `#[from]`; `Folder::new` builds it directly.
- fixed: the `Folder` doctest uses a `cfg!(windows)` literal instead of `temp_dir()`.
- fixed: new tests `folder_refuses_bad_values_on_read` and `folder_checks_utf8_before_absoluteness`. Mutation-checked: making `TryFrom<String>` return `Ok(Self(value))` fails the first (`folder_refuses_bad_values_on_read --- FAILED`); checking absoluteness before UTF-8 fails the second (`folder_checks_utf8_before_absoluteness --- FAILED`); the file was restored and `cmp`-checked afterwards.
- resolved ❓: `Argv` items stay unchecked (the spec requires only a non-empty list); its doc says so, `argv[0]` included.
- Gate after the pass: `dod-app.sh` → `exit=0`, no warning/error lines; `140 passed`, `3 passed`, `123 passed`.
