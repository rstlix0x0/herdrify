---
status: done
created: 2026-11-24
built-on: 2db7af9df077
---

# Transfer Id Implementation Plan

**Goal:** Add `TransferId`, the validated id of a put in pieces.

**Architecture:** A new concept file, `protocol/transfer.rs`, holds `TransferId` and its error
`InvalidTransferId` (spec §2.1, §2.2). It does not go in `identifiers.rs`, whose documented rule
is to refuse only the empty string. `protocol/mod.rs` and `prelude.rs` re-export it flat.

**Tech Stack:** Rust 1.94, serde 1.0.229, thiserror (workspace), std `RandomState`.

**Content authority:** spec §3.1, §2.4 (strong-types, M-ERRORS-CANONICAL-STRUCTS, M-TYPES-SEND,
M-PUBLIC-DEBUG).

---

## Files that change

```
crates/herdrify-hc/src/protocol/transfer.rs   — create: TransferId, InvalidTransferId, their tests
crates/herdrify-hc/src/protocol/mod.rs        — modify: mod transfer; pub use TransferId, InvalidTransferId; //! Non-responsibilities reworded
crates/herdrify-hc/src/prelude.rs             — modify: re-export TransferId
```

## Order of work

### Task 1 — Add TransferId with its rule

**Files:**
- Create `crates/herdrify-hc/src/protocol/transfer.rs`. It starts with the four-question `//!`
  block (what, why, depends on, how tested), plus Responsibilities and Non-responsibilities.
- Modify `crates/herdrify-hc/src/protocol/mod.rs`: `mod transfer;` and
  `#[doc(inline)] pub use transfer::{InvalidTransferId, TransferId};`.
- Modify `crates/herdrify-hc/src/prelude.rs`: add `TransferId` to the `protocol` re-export.

**Change:**
- `TransferId(String)`, a private tuple struct. It is valid when it has 16 to 64 characters and
  every one is an ASCII hex digit, either case. Nothing is trimmed, and case is kept.
- Constructors: `new(impl Into<String>) -> Result<Self, InvalidTransferId>`, `TryFrom<&str>`,
  `TryFrom<String>` and `FromStr`. All of them apply the rule.
- `Deserialize` goes through `TryFrom<String>` (`#[serde(try_from = "String")]`), so a bad id
  fails to decode. `Serialize` is transparent, as a plain string.
- `as_str()`, `Display` (the value as given), `Debug`, `Clone`, `PartialEq`, `Eq` and `Hash`.
- `InvalidTransferId { value: String }`, using `thiserror`. Its message is
  `transfer id {value:?} must be 16 to 64 hex digits`. It derives `Debug`, `Clone`, `PartialEq`
  and `Eq`.
- `# Examples` on every public item: a happy path, and one refused value. `new` and the
  `TryFrom` impls carry `# Errors`, naming `InvalidTransferId`.
- `protocol/mod.rs`'s `//!` Non-responsibilities say the core decides "which values the core
  accepts beyond the empty string" (`src/protocol/mod.rs:28`). Reword it so `TransferId`, which
  validates fully, is the stated exception.

**Proof:** each table row goes through every entry point: `new`, `TryFrom<&str>`,
`TryFrom<String>`, `FromStr` and serde `Deserialize` from a JSON string.
- `transfer_id_accepts_the_valid_examples`: through each entry, `"0123456789abcdef"`,
  `"a"` × 64 and `"ABCDEF0123456789"` are accepted. The last keeps its case in `as_str()` and
  `Display`.
- `transfer_id_refuses_the_invalid_examples`: through each entry, `"0123456789abcde"`,
  `"b"` × 65, `"zzzzzzzzzzzzzzzz"`, `""`, `" 0123456789abcdef"` and `"0123456789abcdeé"` are
  each refused. For the non-serde entries, the refusal is `InvalidTransferId` holding that
  value.
- `invalid_transfer_id_message_is_exact`: `TransferId::new("xyz")` reads
  `transfer id "xyz" must be 16 to 64 hex digits`.
- `transfer_id_serializes_as_a_plain_string`: `serde_json::to_string` gives `"\"0123456789abcdef\""`.
- `invalid_transfer_id_is_send_sync_static`: compile-time bounds, as the sibling error types have.

**Done when:** `cargo test -p herdrify-hc --lib protocol::transfer` passes. The Definition of
Done is green except `live_schema_matches_the_fixture`, the one known failure on hc 0.1.22
(spec P1), which plan 05 Task 2 fixes. Commit `feat(herdrify-hc): add TransferId`.

### Task 2 — Add TransferId::random

**Files:**
- Modify `crates/herdrify-hc/src/protocol/transfer.rs`.

**Change:** `TransferId::random() -> TransferId` gives 32 lowercase hex digits, from two
`std::collections::hash_map::RandomState::new().hash_one(..)` values, each formatted as 16 hex
digits. No dependency is added. Its doc says the id is not a secret: it only tells open puts
apart. It has `# Examples`.

**Proof:**
- `random_ids_are_32_lowercase_hex_digits_and_valid`: `random()` has length 32, every character
  is in `0-9a-f`, and `TransferId::new(random().as_str())` is ok.
- `random_ids_differ`: two calls give different values.

**Done when:** `cargo test -p herdrify-hc --lib protocol::transfer` passes. The Definition of
Done is green except the known `live_schema_matches_the_fixture` failure (Task 1). Commit
`feat(herdrify-hc): add TransferId::random`.

---

## Risks

- `TryFrom<String>` and `TryFrom<&str>` are both implemented, and they must not leave a generic
  `TryFrom` ambiguous at call sites. Name the types in the tests.
- `random_ids_differ` could in principle collide. Two 128-bit values from fresh `RandomState`s
  colliding is not a realistic failure.

## Proof

- `cargo test -p herdrify-hc --lib` and `cargo test -p herdrify-hc --doc` pass.
- `cargo clippy -p herdrify-hc --all-targets --all-features -- -D warnings` is clean.

## Review findings

Reviewer verdict: SPEC COMPLIANT, 0🔴 0🟡 5🔵, nothing blocking. DoD re-run green (`-p herdrify-hc`: fmt, clippy -D warnings, doc -D warnings, lib 351, doc 87, client_fake 98, schema_coverage 7).

- 🔵 `transfer.rs:97` comment said "two hashes of the same input" while the code hashes 0 and 1 — fixed: the comment now says the two independently seeded hashes take distinct inputs. `cargo test -p herdrify-hc --lib` → `test result: ok. 354 passed`.
- 🔵 `transfer.rs:273` test helper used `unwrap_or_default()` on JSON encoding — fixed: the helper returns `Result` and propagates with `?`. Same command, 354 passed; clippy clean.
- 🔵 `transfer.rs:308` deserialize refusal only checked `is_err()` — fixed: it now asserts the message contains `must be 16 to 64 hex digits`. Same command, 354 passed.
- 🔵 `InvalidTransferId` kept `value` private behind a `value()` accessor, unlike its sibling errors — fixed: `pub value: String`, accessor removed. `cargo test --workspace --all-targets --all-features --no-fail-fast` → only `live_schema_matches_the_fixture` fails (known); herdrify-hc lib 363, client_fake 106, doc 98.
- 🔵 no compile-time Send/Sync check on `TransferId` itself — fixed: `transfer_id_is_send_sync_static`. `cargo test -p herdrify-hc --lib protocol::transfer` → 8 passed.

## Deviations

- 2026-11-24: Run interleaved with plans 02–04 in shared batches, because the plans' file sets are disjoint task by task (01/T1 ran beside 02/T1, 03/T1 and 03/T2).
- 2026-11-24: Not committed. The instruction to execute said "do not commit"; the plan's commit messages are left for the author.
- 2026-11-24: Board row at start: `plan 01 approved … execute 01` (clean).
