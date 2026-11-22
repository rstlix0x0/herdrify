---
status: approved
created: 2026-11-22
depends-on: [01]
built-on: ca0f990f7018
---

# Identifiers Implementation Plan

**Goal:** Give every herdr-core identifier its own newtype that refuses an empty string exactly where the core does.

**Architecture:** The first files of the `protocol` layer, the pure sans-IO domain (spec §2). `protocol/mod.rs` is export-only (`//!` docs, `mod`, `#[doc(inline)] pub use`). `lib.rs` gains `pub mod protocol;`. Every new `src` file opens with the four-question `//!` block of `mod-rs-export-only.md` and has a colocated `#[cfg(test)] mod tests`. The workspace lints deny `unwrap_used` and `panic` and warn on `expect_used`, test code included (`Cargo.toml:31-33`), so tests return `Result` and use `?`. Every `pub` item is re-exported from `protocol/mod.rs`, because `unreachable_pub` warns (`Cargo.toml:25`).

**Tech Stack:** Rust 1.94.1, serde 1.0.229, thiserror 2.0.21.

**Content authority:** spec §4.7, §8.1 (`EmptyValueError` row and the `what` words).

---

## Files that change

```
crates/herdrify-hc/src/lib.rs                         — modify: pub mod protocol
crates/herdrify-hc/src/protocol/mod.rs                — create: export-only table of contents for the domain layer
crates/herdrify-hc/src/protocol/empty_value_error.rs  — create: EmptyValueError
crates/herdrify-hc/src/protocol/identifiers.rs        — create: the identifier newtypes of spec §4.7 and DocVersion
```

## Order of work

### Task 1 — Add the protocol module and EmptyValueError

**Files:**
- Modify `crates/herdrify-hc/src/lib.rs` — add `pub mod protocol;`. Keep its `//!` block; it stays export-only.
- Create `crates/herdrify-hc/src/protocol/mod.rs` — `//!` block naming the layer (pure, no tokio, no I/O), `mod empty_value_error;` and its re-export.
- Create `crates/herdrify-hc/src/protocol/empty_value_error.rs` — `EmptyValueError { what: &'static str }`.

**Change:** `EmptyValueError` is the one shared error for every refusing newtype and for `Argv` (`modularity.md` Rule 2). Its `Display` is `{what} must not be empty` (spec §8.1). It derives `Debug`, `Clone`, `PartialEq`, `Eq` and implements `std::error::Error` through `thiserror`. It is `Send + Sync + 'static`.

**Proof:**
- `empty_value_error_display_names_the_value` — `EmptyValueError { what: "argv" }` displays exactly `argv must not be empty`.
- `empty_value_error_is_send_sync_static` — a compile-time bound check that it is `Send + Sync + 'static`.

**Done when:** `cargo test -p herdrify-hc empty_value_error` passes and the Definition of Done is green (`cargo make dod` exits 0; the log grep of `CLAUDE.md` prints nothing). Commit `feat(herdrify-hc): add the protocol module and EmptyValueError`.

### Task 2 — Add the refusing identifier newtypes

**Files:**
- Create `crates/herdrify-hc/src/protocol/identifiers.rs` — the nine refusing newtypes.
- Modify `crates/herdrify-hc/src/protocol/mod.rs` — `mod identifiers;` and the re-exports.

**Change:** `TerminalRef`, `TerminalName`, `DocName`, `LabelKey`, `LabelValue`, `MachineName`, `SessionName`, `KeyName`, `MethodName`. Each wraps a `String` and has the shape of spec §4.7 "Shape of the refusing newtypes": `new` returning `Result<Self, EmptyValueError>`, `TryFrom<&str>`, `FromStr`, `as_str`, `Display`, and `#[serde(transparent)]` serialization with deserialization that goes through `new`. They refuse `""` only. Whitespace and characters the core refuses (`a b`, `a=b`, `" "`) are accepted (spec §4.7 "Values the core refuses"). The `what` word for each is exactly the spec §8.1 list: `terminal reference`, `terminal name`, `doc name`, `label key`, `label value`, `machine name`, `session name`, `key name`, `method name`. All derive `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash`, `PartialOrd` and `Ord`, so they can key maps.

**Proof:**
- `each_refusing_newtype_refuses_empty_with_its_what_word` — for each of the nine, `new("")` is `Err(EmptyValueError { what })` with that type's word above.
- `refusing_newtypes_accept_values_the_core_refuses` — `TerminalName::new("a b")`, `LabelKey::new("a=b")` and `TerminalRef::new(" ")` are `Ok`.
- `refusing_newtype_deserialization_goes_through_new` — `"t7"` deserializes into `TerminalRef` and serializes back to `"t7"`; `""` fails to deserialize into each of the nine.
- `refusing_newtype_parsing_matches_new` — `FromStr` and `TryFrom<&str>` give the same result as `new` for `""` and `"t7"`.

**Done when:** `cargo test -p herdrify-hc identifiers` passes and the Definition of Done is green. Commit `feat(herdrify-hc): add the refusing identifier newtypes`.

### Task 3 — Add the accepting identifier newtypes and DocVersion

**Files:**
- Modify `crates/herdrify-hc/src/protocol/identifiers.rs` — the four accepting newtypes and `DocVersion`.
- Modify `crates/herdrify-hc/src/protocol/mod.rs` — re-exports.

**Change:** `TerminalAddress`, `CallerId`, `RemotePath`, `FileName` have an infallible `new` and `From<String>`, plus `as_str`, `Display` and `#[serde(transparent)]`. Each doc comment says the core decides what is valid, so the SDK checks nothing (spec §4.7 "Shape of the accepting newtypes"). `DocVersion(pub u64)` is a transparent integer newtype with a public field and `get()`, so callers write `DocVersion(1)`. `RemotePath`'s doc comment says the path is on the core's machine, not the local one.

**Proof:**
- `accepting_newtypes_accept_empty` — `CallerId::new("")`, `FileName::new("")`, `RemotePath::new("")` and `TerminalAddress::new("")` construct, and each serializes as `""`.
- `accepting_newtypes_round_trip` — `"rohan/t7"` deserializes into `TerminalAddress` and serializes back unchanged.
- `doc_version_is_a_transparent_integer` — `DocVersion` serializes as `3` and deserializes from `3`.

**Done when:** `cargo test -p herdrify-hc identifiers` passes and the Definition of Done is green. Commit `feat(herdrify-hc): add the accepting identifier newtypes and DocVersion`.

---

## Risks

- **Over-validation.** Adding any check beyond `""` would refuse a value a newer core accepts, which breaks the compatibility promise (spec §1). The tests in Task 2 pin this down.
- **Option not taken:** one generic `NonEmpty<T>` type. The spec names a distinct type per identifier (§4.7).

## Proof

- `cargo test -p herdrify-hc` passes.
- `grep -rn 'tokio' crates/herdrify-hc/src/protocol` prints nothing (spec §2 enforcement).
- The Definition of Done is green.
