---
status: done
created: 2026-11-23
depends-on: [15, 16]
built-on: aab842c3c7d3
---

# HerdrApp Entrypoint Implementation Plan

**Goal:** Give callers `HerdrApp`, a version-checked entrypoint that reaches all 77 commands from
plain values.

**Architecture:** `herdr_app/` is the composition root (spec §2, §8):
- `entry.rs` holds `HerdrApp<T = ProcessTransport>`.
- `builder.rs` holds `HerdrAppBuilder`.
- `call.rs` holds `Call<T, C>`.
- Six group files hold the operation structs.

`Call<T, C>` owns `client: Client<T>` (a cheap `Arc` clone) and `command: Result<C, Error>`, like
`crates/herdrify-hc/src/herdr_core/call.rs:57-60`. Its `IntoFuture` future is the one justified
`'static` `Pin<Box<dyn Future<Output = Result<C::Output, Error>> + Send>>`, under a `// dyn:`
comment naming static-dispatch exception #5 (`call.rs:154-167` there).

`lib.rs` gains `pub mod herdr_app;`, the root re-exports of spec §2 and `pub mod prelude;`. The
shared rules of plan 02's Architecture apply verbatim; doctests needing herdr-app are `no_run`.

**Tech Stack:** Rust 1.94; tokio 1.54.1 in tests only.

**Content authority:** spec §8; §2 (root re-exports, prelude, the `// dyn:` rule); §11 (the
plain-value and rename rows).

---

## Files that change

```
crates/herdrify-app/src/lib.rs                   — modify: pub mod herdr_app, pub mod prelude, root re-exports, crate docs
crates/herdrify-app/src/prelude.rs               — create: HerdrApp, Error and the argument enums
crates/herdrify-app/src/herdr_app/mod.rs         — create: export-only table of contents of the entrypoint
crates/herdrify-app/src/herdr_app/entry.rs       — create: HerdrApp
crates/herdrify-app/src/herdr_app/builder.rs     — create: HerdrAppBuilder
crates/herdrify-app/src/herdr_app/call.rs        — create: Call, its setters and IntoFuture
crates/herdrify-app/src/herdr_app/spaces.rs      — create: SpaceOps
crates/herdrify-app/src/herdr_app/tabs.rs        — create: TabOps
crates/herdrify-app/src/herdr_app/panes.rs       — create: PaneOps
crates/herdrify-app/src/herdr_app/terminals.rs   — create: TerminalOps, AgentOps
crates/herdrify-app/src/herdr_app/layouts.rs     — create: LayoutOps
crates/herdrify-app/src/herdr_app/window.rs      — create: WindowOps, ModOps, OverlayOps
```

## Order of work

### Task 1 — Add HerdrApp and its builder

**Files:**
- Modify `crates/herdrify-app/src/lib.rs` — `pub mod herdr_app;`.
- Create `herdr_app/mod.rs`, `entry.rs` and `builder.rs`.

**Change:**
- `HerdrApp<T>` holds `Client<T>` and `AppVersion`, and is `Clone`:
  - `local()`, `builder()` and `from_client(client)`
  - `version()`, `client()`, `ls()`, `status()` and `commands()`
- `HerdrAppBuilder`: `.program`, `.home`, `.env`, `.env_remove` and `.connect()`, which builds a
  `ProcessTransport` and runs `check_version` once.

**Proof** (an in-file scripted transport):
- `from_client_checks_the_version_once` — exactly one `--version` invocation is recorded, and
  `version()` returns it.
- `from_client_refuses_old_versions` — `herdr-app 0.1.15` gives `UnsupportedVersion`.
- `ls_status_commands_delegate` — each sends its subcommand and decodes.
- A `no_run` doctest on `HerdrApp::local` and one on `HerdrApp::builder`.

**Done when:** `cargo test -p herdrify-app herdr_app` passes and the Definition of Done is green.
Commit `feat(herdrify-app): add HerdrApp`.

### Task 2 — Add Call and the space, tab and pane groups

**Files:**
- Create `herdr_app/call.rs`, `spaces.rs`, `tabs.rs` and `panes.rs`.
- Modify `herdr_app/mod.rs` and `entry.rs` (`spaces()`, `tabs()`, `panes()`).

**Change:**
- `Call<T, C>`:
  - a crate-private `try_with` records the first refusal; later setters do nothing
  - `IntoFuture` runs the command, or returns the recorded `InvalidArgument` without running
- `SpaceOps`, `TabOps` and `PaneOps`, with every method of spec §8's rows, the parameter types of
  §8, and setters on `Call` named after each optional argument.
- Refusals become `InvalidArgument { argument: <wire argument name>, detail: <refusing error's
  Display> }`.
- `rename(name)` refuses `""`; `rename_automatic()` and `unname()` send `""`.

**Proof:**
- `every_space_tab_pane_method_builds_its_command` — every method in the three rows runs once
  against the scripted transport, and its recorded argv names the expected command, with the JSON
  element compared as parsed JSON. Examples:
  - `spaces().new_space().name("api").argv(["claude"]).focus(Focus::Background)` →
    `space.new {"argv":["claude"],"focus":false,"name":"api"}`
  - `tabs().move_by(-1)` → `tab.move {"by":-1}`
  - `panes().unname()` → `pane.rename {"name":""}`
- `call_keeps_the_first_refusal` — `spaces().new_space().name("").name("ok")` returns
  `InvalidArgument { argument: "name", detail: "a space name cannot be empty" }`, displaying
  `invalid name: a space name cannot be empty`, and records no invocation.
- `rename_refuses_empty` — `tabs().rename("")` displays `invalid name: a tab name cannot be
  empty`; `tabs().rename_automatic()` sends `{"name":""}`.
- `index_zero_is_invalid_argument` — `spaces().select_index(0)` displays `invalid index: 0 is out
  of range: positions start at 1`, with no invocation.
- `relative_folder_is_invalid_argument` — `.folder("x/y")` displays `invalid folder: "x/y" is not
  an absolute path`.
- `empty_argv_is_invalid_argument` — `.argv(Vec::<String>::new())` displays `invalid argv: a argv
  cannot be empty`.
- `call_future_is_send` — a compile-time check.
- `no_run` doctests on `spaces()`, `tabs()` and `panes()`.

**Done when:** `cargo test -p herdrify-app herdr_app` passes and the Definition of Done is green.
Commit `feat(herdrify-app): add Call and the space, tab and pane operations`.

### Task 3 — Add the terminal, agent, layout, mod, overlay and window groups

**Files:**
- Create `herdr_app/terminals.rs`, `layouts.rs` and `window.rs`.
- Modify `herdr_app/mod.rs`, `entry.rs` and `call.rs`.

**Change:**
- `TerminalOps`, `AgentOps`, `LayoutOps`, `ModOps`, `OverlayOps` and `WindowOps`, with every
  method of spec §8's rows. Parameter types: lines `i64`; pages, px and weight `f64`.
- `window().close()`'s doc comment says the window stops answering afterwards.
- `layouts().clone(name)` and `mods().clone(name)` take `as` through `.as_name(..)`, whose refusal
  names the argument `as` (spec §8).

**Proof:**
- `every_remaining_method_builds_its_command` — every method in the six rows runs once and sends
  its expected command, e.g. `terminals().scroll_pages(0.5)` → `{"pages":0.5}`.
- `every_command_is_reachable` — the commands recorded across this test and
  `every_space_tab_pane_method_builds_its_command`'s list together are exactly the 77 names of
  `KNOWN_COMMANDS`.
- `out_of_range_numbers_are_invalid_argument` — `layouts().resize_px("sidebar", -5.0)` displays
  `invalid px: -5 is out of range: must be finite and not negative`, and
  `terminals().scroll_pages(f64::NAN)` displays `invalid pages: NaN is out of range: must be
  finite`.
- `no_run` doctests on each group accessor.

**Done when:** `cargo test -p herdrify-app herdr_app` passes and the Definition of Done is green.
Commit `feat(herdrify-app): add the terminal, layout and window operations`.

### Task 4 — Add the root re-exports and the prelude

**Files:**
- Modify `crates/herdrify-app/src/lib.rs` — the root re-exports `HerdrApp`, `HerdrAppBuilder`,
  `Client`, `Error`, `Transport`, `TransportError` and `ProcessTransport`, `pub mod prelude;`, and
  crate `//!` docs with a `no_run` connect example.
- Create `crates/herdrify-app/src/prelude.rs` — export-only, citing exemption #1, re-exporting
  `HerdrApp`, `Error` and the §5.9 argument enums.

**Change:** The public surface named in spec §2.

**Proof:**
- A doctest in `lib.rs` names the seven root items and compiles.
- The `lib.rs` `no_run` example compiles.

**Done when:** `cargo test -p herdrify-app --doc` passes and the Definition of Done is green.
Commit `feat(herdrify-app): add the root re-exports and the prelude`.

---

## Risks

- Setter volume in `call.rs`. A crate-private `macro_rules!` may generate per-command setters
  (spec §8); it stays in `call.rs`.
- `grep -rn 'Box<dyn' crates/herdrify-app/src` must show only `call.rs`, under its `// dyn:` line.

## Proof

- `cargo make dod` is green, and the `Box<dyn` grep shows exactly one justified hit.

## Review findings

Reviewer (one pass, 2026-11-24, over Tasks 1–4): blocking set = the 🔴 below. Gate re-run: `dod-app.sh` → `exit=0`, no warning/error lines; clippy `-D warnings` on `x86_64-pc-windows-msvc` green (type check only).

- fixed 🔴 Task 2: `new_tab()` and `split()` hard-coded shell content, so `tab.new` folder/argv/task/props and `pane.split` argv/props were unreachable through `Call`. `Call<T, TabNew>` now has `.argv`, `.task`, `.folder` and `.props`; `Call<T, PaneSplit>` has `.argv` and `.props`. They go through crate-private `TabNew::content_mut`/`PaneSplit::content_mut`, and a setter of the other form changes nothing. New tests `tab_content_setters_set_the_program_and_the_mod_properties`, `pane_content_setters_set_the_program_and_the_mod_properties`, `a_setter_of_the_other_form_changes_nothing`, `the_refused_content_setters_name_their_wire_argument` (red first: E0599). `cargo test -p herdrify-app herdr_app` → `test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 198 filtered out`; `dod-app.sh` → `exit=0`, 250 unit, 5 + 3 integration, 447 + 29 doctests.
- open 🟡 `herdr_app/builder.rs:163`: the test writes and execs a script in a multi-threaded test binary where other tests fork; possible ETXTBSY flake on Linux (same pattern as `adapter/process.rs`).
- open 🟡 `herdr_app/entry.rs:58`: the hand-written `Clone` (no `T: Clone` bound) has no test over a non-`Clone` transport; reverting to `#[derive(Clone)]` would pass the gate.
- open 🔵 `herdr_app/builder.rs:81`: `home(impl AsRef<Path>)` differs from `ProcessTransportBuilder::home(impl Into<PathBuf>)`.
- open 🔵 `herdr_app/call.rs:138`: the `// dyn:` line cites "exception #5" without naming the list.
- open 🔵 `herdr_app/fake.rs:42`: unused `#[derive(Default)]` on `Script`.
- open 🔵 `herdr_app/fake.rs:128`: redundant `#[cfg(test)]` on `mod tests` inside a test-only module.
- open 🔵 `herdr_app/layouts.rs:96`: the `layouts().set` doctest uses `.expect(..)`.
- open 🔵 `herdr_app/mod.rs:9`: "a program that only wants herdr-app needs this module alone" — examples also import `protocol` or the prelude.
- open 🔵 `herdr_app/spaces.rs:57`: the private `call` helper and hand-written `Debug` are repeated in nine Ops structs.
- open 🔵 `herdr_app/terminals.rs:43`: "acts on the focused terminal unless `.terminal(..)`" is false for keep, park, peek, kill, pin, unpin and `agents().focus`.
- open 🔵 `herdr_app/terminals.rs:101`: "briefly" is not supported by the command's description.
- open 🔵 `herdr_app/window.rs:140`: "Two commands end the running app or restart it" omits `quit()`; `close` quits only on Linux.
- open 🔵 `herdr_app/window.rs:308`: "a call made after this one is refused" is not checkable and "refused" means something else in this crate.
- open 🔵 (plan drift, recorded below): `herdr_app/fake.rs` and the split per-group tests.

## Deviations

- 2026-11-24: `cargo make dod` is replaced by the scoped gate (workspace fmt, clippy and doc; herdrify-app tests and doctests); `cargo make dod` fails only in herdrify-hc's `tests/live.rs` `live_schema_matches_the_fixture` against the installed hc 0.1.22, outside this chain.
- 2026-11-24: the `grep -rn 'Box<dyn' crates/herdrify-app/src` check is judged on library code. Tests and doctests of plans 02–16 return `Box<dyn std::error::Error>` (about 120 hits); `grep -rn 'Box<dyn' crates/herdrify-app/src | grep -v 'std::error::Error'` → `herdr_app/call.rs:141` (`BoxedCall`, under its `// dyn:` line) and the test alias `adapter/process.rs:284`.
- 2026-11-24: "an in-file scripted transport" became one test-only `herdr_app/fake.rs` (`Script`), because `Client` keeps its transport private and five test modules read the recorded argv.
- 2026-11-24: `every_space_tab_pane_method_builds_its_command` and `every_remaining_method_builds_its_command` are split into one test per group file (`every_space_method_builds_its_command`, …); `every_command_is_reachable` checks the set of sent names equals the 77 `KNOWN_COMMANDS`.
- 2026-11-24: beyond spec §8's examples, `space.new` has `.this_computer()` and every optional argument of every command has a setter (terminal open/keep/show/select, `app.settings`, `app.feedback`, `layout.reset`, `overlay.close`, the tab/split content setters).
- 2026-11-24: the prelude holds `HerdrApp`, `Error`, the nine closed §5.9 enums, `Focus` and `At` (the enums `HerdrApp` methods take); `Pin` and `Machine` are left out.

## Fix pass (2026-11-24, on the author's word "fix all of open findings")

- fixed 🟡 builder.rs:163: the script test retries `connect` on `io::ErrorKind::ExecutableFileBusy` (up to 20 × 50 ms).
- fixed 🟡 entry.rs:58: test `a_clone_runs_through_the_same_transport_even_when_the_transport_is_not_clone` over a non-`Clone` transport; mutation `#[derive(Clone)]` → `error[E0599]`; restored.
- fixed 🔵 builder.rs:81: `home(impl Into<PathBuf>)`, passed straight through.
- fixed 🔵 call.rs:138: the `// dyn:` line names "justified exception 5 in the static-dispatch rules".
- fixed 🔵 spaces.rs:57: crate-private `ops_plumbing!` in `call.rs` generates `new`, the private `call` and `Debug` for all nine Ops structs; `OverlayOps::close` goes through it.
- fixed 🔵 fake.rs:42/128 (unused derive, redundant cfg), layouts.rs:96 (doctest without `.expect`), mod.rs:9, terminals.rs:43/101, window.rs:140/308 (wording checked against the 0.1.17 commands fixture).
- recorded 🔵 plan drift: already under Deviations.
- `cargo test -p herdrify-app --lib herdr_app` → `53 passed`; `--doc herdr_app` → `182 passed`.
- Gate after the pass (all five fixes together): `dod-app.sh` → `exit=0`, no warning/error lines; 253 unit, catalog 5, client_fake 8, files_roundtrip 3, live 10 (ran, not skipped), 447 + 29 doctests; `cargo clippy -p herdrify-app --all-targets --all-features --target x86_64-pc-windows-msvc -- -D warnings` clean; `cargo deny check` → `advisories ok, bans ok, licenses ok, sources ok`; the seven §2 greps print nothing; `pgrep -fl herdr-app` prints nothing.
