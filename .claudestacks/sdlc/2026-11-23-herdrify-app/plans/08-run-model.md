---
status: approved
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
