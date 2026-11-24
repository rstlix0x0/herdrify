---
status: done
created: 2026-11-23
depends-on: [07]
built-on: aab842c3c7d3
---

# Run Model Implementation Plan

**Goal:** Model one herdr-app run, from the argv that starts it to the classified outcome.

**Architecture:** Two pure files in `protocol` (spec §2):
- `invocation.rs` builds argv.
- `outcome.rs` holds `RunOutput`, `Exit`, `Failure` and `classify`.

The port (plan 14) moves these values; nothing here does I/O. The shared rules of plan 02's
Architecture apply verbatim, doctests included. The JSON element of a `do` argv is always
compared as parsed JSON, never as text (spec §5.1: map keys come out alphabetical).

**Tech Stack:** Rust 1.94, serde_json 1.0.154.

**Content authority:** spec §5.1, §5.2; §3 F3–F5, F10, F11, F15.

---

## Files that change

```
crates/herdrify-app/src/protocol/invocation.rs   — create: Invocation
crates/herdrify-app/src/protocol/outcome.rs      — create: RunOutput, Exit, Failure, classify
crates/herdrify-app/src/protocol/mod.rs          — modify: mods and re-exports
```

## Order of work

### Task 1 — Add Invocation

**Files:**
- Create `crates/herdrify-app/src/protocol/invocation.rs`.
- Modify `crates/herdrify-app/src/protocol/mod.rs`.

**Change:**
- `Invocation`, with the constructors `version()`, `commands()`, `ls()`, `status()` and
  `run(&CommandName, &Map<String, Value>)`.
- `argv() -> Vec<String>`, exactly as in the spec §5.1 table; `run` writes the map as compact
  JSON (`{}` when empty).
- `label() -> String`: `--version`, `commands`, `ls`, `status` or `do <name>`.

**Proof:**
- `invocation_argv_per_constructor` — the five argv lists of spec §5.1.
- `invocation_run_writes_compact_json` — `run("tab.new", {"name":"api","argv":["claude"]})` gives
  an argv whose first two elements are `do` and `tab.new`, and whose third parses as JSON equal to
  `{"argv":["claude"],"name":"api"}`, with no whitespace in it. An empty map gives exactly `"{}"`.
- Doctests on `Invocation::run` and `classify` (Task 2).
- `invocation_keeps_spaces_and_quotes` — an argv value `echo "a b"` arrives in the JSON element
  unchanged after a parse.
- `invocation_label_per_constructor` — the five labels.

**Done when:** `cargo test -p herdrify-app protocol::invocation` passes and the Definition of Done
is green. Commit `feat(herdrify-app): add Invocation`.

### Task 2 — Add RunOutput and classify

**Files:**
- Create `crates/herdrify-app/src/protocol/outcome.rs`.
- Modify `crates/herdrify-app/src/protocol/mod.rs`.

**Change:**
- `RunOutput { exit: Exit, stdout: Vec<u8>, stderr: Vec<u8> }`, where `Exit` is `Code(i32)` or
  `Signal`.
- `Failure`:
  - `Refused { message }`
  - `NoWindow { message }`
  - `Cli { message }`
  - `Unexpected { exit, stderr }`
- `classify(&RunOutput) -> Result<&[u8], Failure>`, applying spec §5.2's four rules in order.

**Proof:**
- `classify_exit_zero_is_stdout` — `Code(0)` returns the stdout bytes.
- `classify_refusal` — `Code(1)` with `{"error":"tab.new focus is true or false"}` is `Refused`
  with that message.
- `classify_exit_one_without_error_object_is_unexpected` — `Code(1)` with stdout `oops` is
  `Unexpected`.
- `classify_no_window` — `Code(2)` with stderr `herdr-app: no Herdr window at
  /tmp/hzp/herdr-app.sock: No such file or directory (os error 2)\n` is `NoWindow` with the
  message stripped of `herdr-app: ` and the newline.
- `classify_cli_error` — `Code(2)` with `herdr-app: args are not JSON: key must be a string at
  line 1 column 2\n` is `Cli` with `args are not JSON: key must be a string at line 1 column 2`.
- `classify_signal_and_other_codes` — `Signal` and `Code(3)` are `Unexpected` with the trimmed
  stderr.

**Done when:** `cargo test -p herdrify-app protocol::outcome` passes and the Definition of Done is
green. Commit `feat(herdrify-app): classify herdr-app run outcomes`.

---

## Risks

- `classify` matches herdr-app's English prefix `no Herdr window at `. A reworded herdr-app
  message turns `NoWindow` into `Cli`, which is still a typed error.

## Proof

- `cargo make dod` is green.

## Review findings

Tasks 1 and 2 reviewed together: SPEC COMPLIANT, 0🔴 4🟡 7🔵, blocking: none.
- 🟡 open: `protocol/invocation.rs:105-107` — the doc says `to_value` maps come out alphabetical; that holds only while serde_json's `preserve_order` feature is off (reviewer probe, serde_json 1.0.154: default `{"argv":["claude"],"name":"api"}`, with `preserve_order` `{"name":"api","argv":["claude"]}`). Tests already compare parsed JSON.
- 🟡 open: `protocol/mod.rs:7-18` — the `//!` block does not list the run outcome, and its non-responsibility "reading what it prints" contradicts `outcome.rs` (carried to the next `protocol/mod.rs` edit).
- 🟡 open: `protocol/outcome.rs:21-28`, `:54-82` — the pub enums `Exit` and `Failure` have no `# Examples`.
- 🟡/❓ open: `protocol/outcome.rs:58-82` — `Failure` implements neither `Display` nor `std::error::Error`. The coder found a Display in §5.2/§10 only for the driver's error, which wraps it; `Failure` is a classification value that the driver layer surfaces.
- 🔵 open: `protocol/invocation.rs:23-40` — `Invocation` itself has no `# Examples` (its methods do).
- 🔵 open: `protocol/invocation.rs:75` — `ls()` doc says "lists herdr-app's windows"; `herdr-app --help` 0.1.17 says it lists spaces, tabs and terminals in the open window.
- 🔵 open: `protocol/invocation.rs:122` — the doctest compares the JSON element as text (safe with one key).
- 🔵 open: `protocol/outcome.rs:56-57` — the doc says every `message` loses the `herdr-app: ` prefix; `Refused.message` is the stdout `error` string unchanged.
- 🔵 open: `protocol/outcome.rs` — untested: exit 2 without the `herdr-app: ` prefix, a doubled prefix, and a refusal in herdr-app's real pretty-printed multi-line shape.
- 🔵 open: the six `outcome` tests return `()` (no fallible calls), as `files/error.rs:119` and others already do.
- Verified: `cargo test -p herdrify-app --lib protocol::` → `test result: ok. 17 passed; 0 failed` (the ten new tests included).

## Deviations

- 2026-11-24 — Definition of Done: `cargo make dod` fails only in `herdrify-hc` `tests/live.rs::live_schema_matches_the_fixture` (installed `hc 0.1.22`, environment drift outside this chain). Gate run instead: workspace fmt-check, clippy `-D warnings`, `RUSTDOCFLAGS="-D warnings" cargo doc`, plus `cargo test -p herdrify-app` all-targets and `--doc` → `exit=0`, no warning/error lines; `116 passed`, `3 passed`, `98 passed`.
- 2026-11-24 — Tasks 1 and 2 share `protocol/mod.rs`, so they ran one after the other and had one review.
- 2026-11-24 — Task 2: `Failure` has no `Display`; spec §5.2/§10 define Display only for the driver's error. Rule 3 trims trailing whitespace (`trim_end`, "trailing" in the rule); rule 4 trims both ends ("trimmed").

## Fix pass (2026-11-24, on the author's word "fix all of it")

Every open finding above is resolved:
- fixed: the `Invocation::run` doc says key order follows serde_json's `Map` (sorted by default, insertion order with `preserve_order`), so compare parsed JSON; the doctest now compares the element as parsed JSON.
- fixed: `Invocation`, `Exit` and `Failure` have `# Examples`.
- fixed: the `ls()` doc says it lists the spaces, their tabs and terminals, in the open window.
- fixed: the `Failure` doc says which variant holds what (`Refused`: stdout `error` unchanged; `NoWindow`/`Cli`: stderr, trailing whitespace and one `herdr-app: ` removed; `Unexpected`: stderr trimmed).
- resolved ❓: `Failure` stays without `Display`/`Error`. It is documented as a classification of a finished run that the client turns into its error (`Error::from_failure(label, Failure)`, plan 15), which carries the message; no message text was invented.
- fixed: new tests `classify_no_window_without_prefix`, `classify_strips_one_prefix` and `classify_recorded_pretty_refusal` (the 3-line pretty refusal from `probe-window-output.txt:120-122`).
- fixed: every test returns `Result`; the outcome tests use `?` helpers (`succeeded()`, `failed()`), since `Ok(())` after plain asserts trips clippy `unnecessary_wraps`.
- Gate after the pass: `dod-app.sh` → `exit=0`, no warning/error lines; `140 passed`, `3 passed`, `123 passed`.
