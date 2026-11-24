---
status: approved
created: 2026-11-23
built-on: e674a715bd9e
adrs: [0001, 0002, 0003]
---

# Spec: herdrify-app SDK

This spec delivers `herdrify-app` 0.1.0, a standalone async Rust client for herdr-app. It is
proven against herdr-app 0.1.17 and accepts 0.1.16 and newer. It covers:
- the five subcommands `--version`, `commands`, `ls`, `status` and `do`, with typed arguments
  for all 77 commands
- pure models of Herdr's layout, theme and keys files
- a public, fakeable `Transport` port with a process adapter
- `HerdrApp`, a version-checked entrypoint that takes plain values
- tests against a fake transport, the shipped files, and a throwaway isolated window

It adds no herdrify concept: no components, grants or bundles.

All paths under `evidence/` are relative to this chain directory. `probe-*/` directories hold the
Rust probe sources, and `probe-*-output.txt` files hold their raw output. A citation such as
`probe-gaps-output.txt:21` names a line, and `V`, `T`, `G` and the like name the probe's line
labels. Every live probe output starts with the herdr-app version it ran on, and all of them read
`herdr-app 0.1.17`:
- `probe-cli-output.txt:4`
- `probe-window-output.txt:1`
- `probe-commands-output.txt:1`
- `probe-gaps-output.txt:1`
- `probe-views-output.txt:1`

Every window probe ran against an isolated window and an isolated core:
- its own `HERDR_APP_HOME` under `/tmp`
- its own `HC_SESSION`
- XDG directories under `/tmp`
- every inherited `HC_*` variable removed

The person's core was only read, with `hc ls --json`, and it never held a probe terminal
(`probe-window-output.txt:2,315,336`; `probe-commands-output.txt:2,727,827,836`).

**History.** The installed herdr-app updated itself from 0.1.16 to 0.1.17 during this design. A
first round of probes ran partly on each version; all window and CLI probes were then re-run on
0.1.17. The 0.1.16 command list is kept as `evidence/herdr-app-0.1.16-commands.json` (75
commands); 0.1.17 adds `pane.rename` and `pane.resize`
(`evidence/herdr-app-0.1.17-commands.json`).

**This revision** replaces `spec-superseded-2026-11-23.md`. The plan-set review found two spec
defects, and the revision settles six readings it left open:
- defect: `Float.bounds` was taken from a pre-0.1.17 run; 0.1.17 has no such field (§5.7, F14)
- defect: the doc grep matched the example name `inspect`; it is now word-bounded and catches
  plurals and multi-digit numbers (§2)
- `Call` owns a `Client` clone, as hc's does, so its boxed future needs no lifetime, and the shape
  is compile-probed (§2, §7, §8)
- the `InvalidArgument.argument` word and the `as` setter's name (§8)
- `rename("")` (§8, §11)
- the scope of `(no output)` (§10)
- `Ack`'s exact refusal (§5.8)
- the argv JSON key order (§5.1)

## 1. Premises

**Versions.**
- Toolchain: rustc 1.94, pinned by `rust-toolchain.toml`.
- herdr-app 0.1.17 (above) and hc 0.1.21 (`ps-window-core-output.txt:3`).
- Crates: tokio 1.54.1, serde 1.0.229, serde_json 1.0.154, thiserror 2.0.21. These are the
  versions every probe locked (`probe-*/Cargo.lock`), and the workspace already declares them
  (`Cargo.toml:22-25`).

**ADRs relied on.**
- **ADR 0001:** `herdrify-app` sits at L0 with no herdrify dependency. SDKs publish first.
- **ADR 0002:**
  - async only, on tokio
  - no Cargo features
  - `Send` futures
  - a public `Transport` that runs one `herdr-app` argv and returns stdout, stderr and exit code
  - `Client<T: Transport>`
  - pure models of the layout, theme and keys files
  - hand-written command arguments
  - version below 0.1.16 refused, newer accepted and reported
- **ADR 0003:** the models must express what herdrify generates. The SDK enforces none of
  herdrify's rules.

**Upstream documentation.** herdr-app rewrites `AGENTS.md`, `herdr.d.ts` and `gpui-kit.d.ts` in
its home on every start (`evidence/herdr-app-0.1.17-home/AGENTS.md:3`). The 0.1.17 copies in
`evidence/herdr-app-0.1.17-home/` and the shipped files in `evidence/fresh-home-0.1.17/` come
from the fresh home the isolated 0.1.17 window wrote. Every `AGENTS.md:N` below is a line of that
0.1.17 copy.

**Fixtures.** The shipped layouts and themes in the author's own home are byte-identical to the
fresh 0.1.17 ones (`fixtures-cmp-output.txt`). `fresh-home-0.1.17/` (layouts main, terminals and
win98; themes atom-one-dark and win98; `keys.json`) and `herdr-app-0.1.17-commands.json` become
the test fixtures.

**The person's Herdr is never touched.** No test or probe writes into `~/.config/herdr-app` or
acts on the person's window (§12.6).

## 2. Architecture

The author approved this section on its own on 2026-11-23. Two amendments followed with the
component approvals:
- `files/identifiers.rs`
- `commands/words.rs`

The plan-review revision placed types the first draft left unplaced. The present revision changes
two §2 items that need the author's explicit yes:
- the static-dispatch bullet (`Call` owns a `Client` clone; `Client` holds `Arc<T>`)
- the word-bounded doc grep

The layers, the module tree and the dependency direction are unchanged.

Clean architecture, with ports and adapters inside the crate.

```
crates/herdrify-app/
├─ Cargo.toml  README.md  CHANGELOG.md
├─ src/
│  ├─ lib.rs  prelude.rs          table of contents only
│  ├─ files/        DOMAIN  Herdr's file shapes          serde, serde_json, thiserror · no tokio, no I/O
│  │  ├─ mod.rs
│  │  ├─ error.rs              FileError
│  │  ├─ identifiers.rs        NodeId, ModName, LayoutName, ThemeName, TerminalAddress, ColorName,
│  │  │                        CommandName, Chord, EmptyValueError
│  │  ├─ number.rs             Number, NumberError
│  │  ├─ color.rs              Color, HexColor, ColorError
│  │  ├─ layout/               layouts/<name>.json: five roles
│  │  │  ├─ mod.rs
│  │  │  ├─ node.rs            Layout, Node, NodeKind, Size, Padding, Shadow, FocusStyle
│  │  │  ├─ content.rs         Content, PaneFrame, PaneHeader, HeaderPosition, Reveal
│  │  │  ├─ paint.rs           Paint, Fit, Line
│  │  │  ├─ look.rs            Look, ThemeChoice, RootOptions, Titlebar, ClonedFrom
│  │  │  └─ floating.rs        FloatingBlock, Edge, Align
│  │  ├─ theme.rs              Theme
│  │  └─ keys.rs               Keys, Section, Action, KeyActionError
│  ├─ protocol/     DOMAIN  herdr-app's CLI language       serde, serde_json, thiserror · no tokio, no I/O
│  │  ├─ mod.rs
│  │  ├─ invocation.rs         Invocation
│  │  ├─ outcome.rs            RunOutput, Exit, Failure, classify
│  │  ├─ version.rs            AppVersion, InvalidAppVersion
│  │  ├─ identifiers.rs        SpaceId, TabId, PaneId, OverlayId, SpaceName, TabName, PaneName,
│  │  │                        TerminalName, MachineName, Task, AppName, Folder, FolderError, Argv
│  │  ├─ catalog.rs            CommandInfo, ArgInfo, KnownCommand, KNOWN_COMMANDS
│  │  ├─ listing.rs            Listing, SpaceEntry, TabEntry, TerminalEntry, RecentSpace, AgentStatus
│  │  ├─ status.rs             WindowStatus, Float
│  │  └─ commands/             trait Command + the 77 typed commands, by group
│  │     ├─ mod.rs
│  │     ├─ command.rs         trait Command (pure trait)
│  │     ├─ outputs.rs         Created, TabMade, Split, Opened, Kept, Parked, Ack, RawReply
│  │     ├─ words.rs           input word enums, argument enums, Position, Px, Weight, FiniteF64,
│  │     │                     ArgumentRangeError
│  │     ├─ space.rs  tab.rs  pane.rs  agent.rs  layout.rs  app.rs
│  │     └─ terminal/          two roles
│  │        ├─ mod.rs
│  │        ├─ lifecycle.rs    open, keep, park, kill, peek, show, pin, right_click
│  │        └─ selection.rs    select_all, select, adjust_selection, clear_selection, copy, paste,
│  │                           paste_image, scroll, search
│  ├─ port/         PORT
│  │  ├─ mod.rs
│  │  ├─ transport.rs          Transport (pure trait)
│  │  └─ error.rs              TransportError
│  ├─ driver/       APPLICATION
│  │  ├─ mod.rs
│  │  ├─ client.rs             Client<T>
│  │  └─ error.rs              Error
│  ├─ herdr_app/    ENTRYPOINT
│  │  ├─ mod.rs
│  │  ├─ entry.rs              HerdrApp
│  │  ├─ builder.rs            HerdrAppBuilder
│  │  ├─ call.rs               Call
│  │  ├─ spaces.rs             SpaceOps
│  │  ├─ tabs.rs               TabOps
│  │  ├─ panes.rs              PaneOps
│  │  ├─ terminals.rs          TerminalOps, AgentOps
│  │  ├─ layouts.rs            LayoutOps
│  │  └─ window.rs             WindowOps, ModOps, OverlayOps
│  └─ adapter/      ADAPTER
│     ├─ mod.rs
│     └─ process.rs            ProcessTransport, ProcessTransportBuilder
├─ examples/   inspect/ · drive/ · files/     (one folder each: main.rs + README.md)
└─ tests/
   ├─ fixtures/                herdr-app-0.1.17-commands.json, layouts/{main,terminals,win98}.json,
   │                           themes/{atom-one-dark,win98}.json, keys.json
   ├─ support/                 fake_transport.rs · live_window.rs
   ├─ catalog.rs
   ├─ files_roundtrip.rs
   ├─ client_fake.rs
   └─ live.rs
```

**Why each module exists.**

| Module | Load-bearing reason |
|---|---|
| `files` | The layout, theme and keys models are pure and usable without any process. `protocol` needs `Layout` for `layout.set` |
| `protocol` | ADR 0002 calls for sans-IO logic. Argv building, output classification and version parsing are bytes in, values out (M-IMPL-IO) |
| `port` | The public, fakeable seam of ADR 0002 |
| `driver` | Turns a command into an invocation, and an outcome into a typed reply or `Error`. Named `driver` because `client/client.rs` fails clippy `module_inception` (`2026-11-22-herdrify-hc/evidence/probe-inception-output.txt`) |
| `herdr_app` | The composition root: the version-checked `HerdrApp`, plain values |
| `adapter` | The only code that spawns a process (M-MOCKABLE-SYSCALLS) |

**Dependency rule.** Every dependency points inward.

| Layer | May depend on | Must not depend on | External crates |
|---|---|---|---|
| `files` | nothing internal | all others | serde, serde_json, thiserror |
| `protocol` | `files` | `port`, `driver`, `herdr_app`, `adapter` | serde, serde_json, thiserror |
| `port` | `protocol` | `driver`, `herdr_app`, `adapter` | none |
| `driver` | `port`, `protocol`, `files` | `herdr_app`, `adapter` | serde_json; tokio only in `#[cfg(test)]` and doctests |
| `herdr_app` | all inner layers and `adapter` | — | none directly; tokio only in tests and doctests |
| `adapter` | `port`, `protocol` | `driver`, `herdr_app` | tokio (`process`) |

**Enforcement.** The reviewer runs these, and each must print nothing:
```sh
grep -rn 'crate::\(protocol\|port\|driver\|herdr_app\|adapter\)' crates/herdrify-app/src/files
grep -rn 'crate::\(port\|driver\|herdr_app\|adapter\)' crates/herdrify-app/src/protocol
grep -rn 'crate::\(driver\|herdr_app\|adapter\)' crates/herdrify-app/src/port
grep -rn 'crate::\(herdr_app\|adapter\)' crates/herdrify-app/src/driver
grep -rn 'crate::\(driver\|herdr_app\)' crates/herdrify-app/src/adapter
grep -rn 'tokio' crates/herdrify-app/src/files crates/herdrify-app/src/protocol crates/herdrify-app/src/port
grep -rniwE 'adrs?|specs?|plans? [0-9]+|milestones?|claudestacks|phases? [0-9]+|tasks? [0-9]+' crates/herdrify-app/src crates/herdrify-app/tests crates/herdrify-app/examples crates/herdrify-app/README.md
```

The last grep matches whole words only (`-w`), with plurals and multi-digit numbers. Run with
`/usr/bin/grep` (BSD grep 2.6.0, the script `grep-word.sh`; `grep-word-output.txt`):
- no match: `cargo run --example inspect`, `specific`, `a readr value`, `respect the order`
- match: `see the spec`, `see the specs`, `per ADR 0002`, `per the ADRs`, `Plan 3 says`,
  `see plan 03`, `Task 12`, `Phase 10`, `the milestone`, `claudestacks-sdlc`

`driver` and `herdr_app` are left out of the tokio grep because their unit tests and doctests
need a runtime, as in the hc precedent (`2026-11-22-herdrify-hc/spec.md:251`).

**Guideline rules applied** (`claudestacks-guideline-rust` 0.1.5, `references/`):
- **`mod-rs-export-only.md`:**
  - `lib.rs`, `prelude.rs` and every `mod.rs` hold only `//!` docs, `mod` and `pub use`
  - every module opens with the four-question `//!` block
  - public items are re-exported flat from their layer module with `#[doc(inline)]`
  - the crate root re-exports exactly `HerdrApp`, `HerdrAppBuilder`, `Client`, `Error`,
    `Transport`, `TransportError` and `ProcessTransport`
  - `prelude` re-exports `HerdrApp`, `Error` and the argument enums of §5.9
- **`modularity.md`:**
  - one concept per file
  - no `*Manager`, `*Util` or `*Helper`
  - errors live with their context: `files/error.rs`, `port/error.rs`, `driver/error.rs`, or the
    concept's own file (`ColorError`, `KeyActionError`, `NumberError`, `InvalidAppVersion`,
    `FolderError`, `ArgumentRangeError`)
  - no crate-wide `errors.rs`
  - no `<concept>_<qualifier>.rs` file
  - one shared `EmptyValueError`
  - one `CommandName`, in `files`, reused by `protocol` (Rule 2)
- **`static-dispatch.md`:**
  - `Client<T: Transport>` and `HerdrApp<T: Transport = ProcessTransport>`
  - exactly one trait object: `Call`'s `IntoFuture` future is `Pin<Box<dyn Future<Output =
    Result<C::Output, Error>> + Send>>` with a `// dyn:` comment (exception #5: `IntoFuture` must
    name its future type, and a transport's async future cannot be named on stable). `Call<T, C>`
    owns a `Client<T>` clone, so the future is `'static`. This is the shape of
    `crates/herdrify-hc/src/herdr_core/call.rs:57-60,154-167` (`Call<T, P>` owning `client:
    Client<T>`, `BoxedCall<O>`). The app's own shape (an RPITIT `Transport`, `Client<T>` over
    `Arc<T>` with an inherent `async fn run<C: Command>`, and `Call<T, C>` with bounds `C: Command +
    Send + 'static`, `C::Output: Send`) compiles on rustc 1.94.1, and `tokio::spawn` accepts its
    future (`probe-call-output.txt`).
  - `Client<T>` holds its transport in an `Arc<T>`, so it is `Clone` for every `T`
    (M-SERVICES-CLONE: an `Arc` inner for cheap clones, not dispatch).
  - `Box<Content>` in `PaneHeader` breaks a recursive type, which the reference allows ("breaks a
    recursive type"): `Content → PaneFrame → PaneHeader → Content` without it is `error[E0072]:
    recursive types … have infinite size` (seen while building `probe-model`)
- **`strong-types.md`:**
  - identifier newtypes
  - required arguments are constructor parameters
  - each one-of group and each two-state argument is an enum (§5.9)
  - `confirmed` is an argument-less setter
  - no `bool` public parameter
  - file-model `Option<bool>` fields are wire mirrors
- **`unit-test-mandate.md`:**
  - colocated tests in every logic file
  - `port/transport.rs` and `commands/command.rs` are exemption #3
  - `lib.rs`, `prelude.rs` and every `mod.rs` are exemption #1
  - each exemption is cited in its file's `//!`
- **`doc-comment-discipline.md`:** the last enforcement grep.
- **`workspace.md`:** workspace dependencies only, and inherited metadata and lints.
- **`microsoft-guidelines.md`:**
  - M-TYPES-SEND
  - M-OOBE: the default `cargo test` needs no herdr-app
  - M-ESSENTIAL-FN-INHERENT
  - M-DONT-LEAK-TYPES: no tokio type in the public API. `serde_json::{Value, Map}` are part of the
    contract, for `extra`, props, `raw` and `RawReply`. `serde_json::Number` stays private inside
    `Number`.
  - M-SERVICES-CLONE: `Client`, `HerdrApp` and `ProcessTransport` are `Clone`
- **The Rust Definition of Done:** `cargo make dod` with zero warnings.

## 3. CLI facts the design rests on

| # | Fact | Evidence |
|---|---|---|
| F1 | `herdr-app --version` prints `herdr-app 0.1.17\n` and exits 0 | `probe-cli-output.txt:1-5` |
| F2 | `herdr-app commands` prints a JSON array of 77 objects with exactly the keys `about`, `args` and `name`. Each arg has exactly `name` and `required`: 100 args, 25 required, 77 unique names. It needs no window, and its output is identical with an empty `HERDR_APP_HOME`, with the author's home, and to the saved fixture | `probe-cli-output.txt:7-12` |
| F3 | With no window, `ls`, `status` and `do` exit 2 with stderr `herdr-app: no Herdr window at <home>/herdr-app.sock: <reason>\n`. A home too long for a socket gives `path must be shorter than SUN_LEN` | `probe-cli-output.txt:14-43,56-61` |
| F4 | `do` args that are not JSON exit 2 with `herdr-app: args are not JSON: …`, before any window is contacted. A JSON value that is not an object exits 2 with ``herdr-app: expected --<arg>, got "[1]"``. A bare `do` exits 2 with the usage line | `probe-cli-output.txt:44-55`; `probe-window-output.txt:170-174` |
| F5 | Unknown subcommands and flags exit 2 with ``herdr-app: unknown argument "…"`` and the help | `probe-cli-output.txt:62-79` |
| F6 | The CLI writes nothing into an empty home | `probe-cli-output.txt:80` |
| F7 | `herdr-app` with no arguments, given an isolated home, launches a second window process beside the person's. It answered `status` within 0.6–3.2 s | `probe-window-output.txt:4-6`; `probe-commands-output.txt:3,728`; `probe-gaps-output.txt:2`; `probe-views-output.txt:3` |
| F8 | The person's window process (`Herdr`) has `herdr-core connect` as a child (`ps-window-core-output.txt:1-2`). With `HC_SESSION` and the XDG variables set, the isolated window's terminals land in the isolated core and never in the person's | `probe-window-output.txt:313,315` |
| F9 | `do` success exits 0 with a JSON object: `{"ok":true}` for most commands, else a §5.8 shape | `probe-commands-output.txt:729-831` |
| F10 | A refusal exits 1 with `{"error": "<text>"}` on stdout and empty stderr | `probe-window-output.txt:109-168` |
| F11 | Both the JSON-object form and the flag form work, and `{}` works for commands with no arguments | `probe-window-output.txt:47-81` |
| F12 | herdr-app checks argument types inconsistently: many wrong-typed values are silently accepted (`space.new` takes `argv: 7`), others are refused | `probe-commands-output.txt` phase T (`T` lines, 4-724) |
| F13 | `ls` prints `{spaces:[{id,name,folder,machine,active,tabs:[{id,name,active,settled,terminals:[{terminal,pane,status,app,name,sub_agents}]}]}],recent:[{name,folder}]}`. `sub_agents` holds terminal addresses | `probe-window-output.txt:186-291`; `probe-views-output.txt:9-47` |
| F14 | `status` prints `{zoom,font_size,ui_font_size,space,tab,focused_pane,layout,agent,floats,overlays,errors}`. `floats` maps a node id to `{edge,align,vertical,dragging,hovered}` (`probe-views-output.txt:98-104`). `overlays` and `errors` hold strings. A broken `layouts/bad.json` is not reported | `probe-window-output.txt:8-26`; `probe-views-output.txt:48-146` |
| F15 | After `window.close`, every call answers exit 1 `{"error":"the window did not answer"}`, `app.quit` included, and the process does not exit | `probe-commands-output.txt:831-834` |
| F16 | `do app.quit` makes the window process exit 0. SIGTERM makes it exit by signal 15. Afterwards `status` exits 2 | `probe-window-output.txt:317-334`; `probe-gaps-output.txt:47-49` |
| F17 | A fresh window shows the welcome overlay, with null `space`, `tab`, `focused_pane` and `agent` | `probe-window-output.txt:8-26` |

## 4. Files (domain)

### 4.1 Shape rules

The full shape of §4.5–§4.10 was built as a probe and run over the six 0.1.17 fixtures: each is
JSON-equal after a round trip, has no duplicate keys, and re-serializes stably
(`probe-model-output.txt:1-6`).

- **Plain data.** Every model struct is `#[non_exhaustive]`, with public fields and constructors.
- **Nothing unnamed is lost.** Every object-shaped struct, and every struct variant of an untagged
  enum, has `#[serde(flatten)] pub extra: serde_json::Map<String, Value>`. Evidence
  (`probe-model-output.txt:7-9`):
  - `{"terminal":"t7","x":1}` → `Terminal` with `x` kept
  - `{"px":1,"weight":2}` → `Px` with `weight` kept

  A key repeated in the source keeps its last value (`probe-messages-output.txt:8`).
- **Unknown words are kept.** Every file word list ends with `#[serde(untagged)] Other(String)`
  (`probe-model-output.txt:38`: `"type":"grid"` → `Other("grid")`).
- **Unknown or invalid objects are kept.** `Content`, `Size` and `Paint` end with
  `Other(Map<String, Value>)`, and `ThemeChoice` with `Other(Value)`. An object whose known
  variant fails, including on an invalid inner value, lands in `Other` and is written back. It is
  not a `FileError` (`probe-model-output.txt:10-14`):
  - `{"mod":""}` → `Content::Other`
  - `{"px":"x"}` → `Size::Other`
  - an unknown content object → `Other`

  Only a value that is neither a matching variant nor an object is refused (§10).
- **Exact numbers.** Every number is a `Number` (§4.3).
- **No flattened tagged enum.** One beside a flattened map writes its keys twice
  (`probe-serde-shape-output.txt` S2), so nodes are flat.
- **Serialization.** `to_json()` is `serde_json::to_string_pretty` plus `"\n"`: two-space
  indentation (`probe-messages-output.txt` M7). Named fields come out in declaration order, then
  `extra` keys in alphabetical order (`probe-crates-output.txt:13`). The result is JSON-equal to
  the source, not byte-equal.
- **Parsing.** `Layout::parse`, `Theme::parse` and `Keys::parse` take `&str` and return
  `Result<_, FileError>`. The types also implement `Serialize` and `Deserialize`.

### 4.2 Identifiers (`files/identifiers.rs`)

`NodeId`, `ModName`, `LayoutName`, `ThemeName`, `TerminalAddress`, `ColorName`, `CommandName` and
`Chord` are `#[serde(try_from = "String", into = "String")]` newtypes:
- `new(impl Into<String>) -> Result<Self, EmptyValueError>`
- `as_str`, `Display` and `FromStr`
- the only refusal is the empty string; whitespace-only values are accepted

| Type | `label` |
|---|---|
| `NodeId` | `node id` |
| `ModName` | `mod name` |
| `LayoutName` | `layout name` |
| `ThemeName` | `theme name` |
| `TerminalAddress` | `terminal address` |
| `ColorName` | `colour name` |
| `CommandName` | `command name` |
| `Chord` | `key chord` |

An empty name in a typed field that is not inside an untagged enum (`Theme.colors` keys, a `Keys`
chord, `Action::Command.command`) is a `FileError` whose detail is the `EmptyValueError` text
(`probe-model-output.txt:30`: `a command name cannot be empty`). Inside `Content`, `Size` or
`Paint` it makes the object fall to `Other` (§4.1).

### 4.3 `Number` (`files/number.rs`)

`Number` wraps a private `serde_json::Number` and serializes transparently.
- **Form kept:** `120.0 → 120.0`, `268 → 268`, `4.286 → 4.286`, `0.0 → 0.0`, `-5 → -5`,
  `-0.0 → -0.0`.
- **Exponents:** `1e3 → 1000.0`, which is JSON-equal (`probe-messages-output.txt` M6).

API:
- `From<u32>`, `From<i64>`, `From<u64>`
- `TryFrom<f64>`: `NumberError { value }` for NaN and the infinities, because
  `Number::from_f64(NaN)` is `None` (M6)
- `as_f64()`
- `is_integer()`

### 4.4 Colours (`files/color.rs`)

`HexColor` is `#` followed by exactly 6 or 8 ASCII hex digits, in any case.

`Color` is `Hex(HexColor)` or `Named(ColorName)`. Parsing rule:
- starts with `#`: must be a valid `HexColor`, else `ColorError::NotHex`
- otherwise: a `ColorName`
- empty: `ColorError::Empty`

Checked examples (`probe-messages-output.txt:37-41`):

| Input | Result |
|---|---|
| `"#ffffff0d"` | ok, `Hex` |
| `"canvas"` | ok, `Named` |
| `"#12345"` | `NotHex` |

Layouts may use theme names or `#rrggbb[aa]` (AGENTS.md:289). Themes use `#rrggbb`
(AGENTS.md:339-341); the shipped themes use 7-character colours only.

### 4.5 `Layout` (`files/layout/`)

`Layout` is `#[serde(transparent)]` over its root `Node` (AGENTS.md:225).

### 4.6 `Node` (`files/layout/node.rs`)

All fields are optional unless marked. Sources: AGENTS.md:225-289, 321-376, 378-408 and 410-434.

| Field | Type |
|---|---|
| `id` | `NodeId` |
| `kind` (wire `type`, **required**) | `NodeKind`: `Block`, `Row`, `Column`, `Stack`, `Other(String)` |
| `content` | `Content` |
| `children` | `Vec<Node>`. An absent list and `[]` stay distinct |
| `active` | `NodeId` |
| `header` | `serde_json::Value` (a stack header; AGENTS.md:233-234 documents only the word `"tabs"` and "a mod") |
| `size` | `Size`: `Px { px: Number, extra }`, `Weight { weight: Number, extra }`, `Other(Map)` |
| `min`, `max`, `radius`, `gap` | `Number` |
| `padding` | `Padding`: `Even(Number)` or `Pair([Number; 2])` |
| `background` | `Paint` |
| `border`, `divider` | `Line` |
| `shadow` | `Shadow`: `Sm`, `Md`, `Lg`, `Other` |
| `focus` | `FocusStyle { border: Option<Line>, extra }` |
| `hidden`, `resizable`, `movable`, `window_drag` | `bool` |
| `floating` | `Vec<FloatingBlock>` |
| `look` | `#[serde(flatten)] Look` |
| `root` | `#[serde(flatten)] RootOptions` |
| `extra` | `#[serde(flatten)] Map` |

Constructors:
- `Node::block(Content)`
- `Node::row(Vec<Node>)`
- `Node::column(Vec<Node>)`
- `Node::stack(Vec<Node>, NodeId)`

Parsing accepts any combination as written.

### 4.7 `Look` and `RootOptions` (`files/layout/look.rs`)

`Look` (AGENTS.md:321-376):

| Field | Type |
|---|---|
| `theme` | `ThemeChoice`: `Name(ThemeName)`, `ByAppearance { light, dark, extra }`, `Other(Value)` |
| `colors` | `BTreeMap<ColorName, Color>` |
| `font`, `ui_font` | `String` |
| `font_size`, `font_weight`, `terminal_opacity`, `terminal_line_height`, `terminal_cell_width`, `terminal_scroll_speed` | `Number` |
| `terminal_padding` | `Padding` |

`RootOptions` (AGENTS.md:259-288, 206):

| Field | Type |
|---|---|
| `titlebar` | `Titlebar`: `None` (wire `"none"`) or `Other` |
| `traffic_lights` | `[Number; 2]` |
| `window_blur` | `bool` |
| `preview` | `String` |
| `terminal_menu`, `toast` | `ModName` |
| `link_handlers` | `Vec<ModName>` (new in 0.1.17, AGENTS.md:279-281) |
| `cloned_from` | `ClonedFrom { name: LayoutName, version: Value, extra }` (`probe-commands-output.txt:801`) |

Both structs are flattened into `Node`. Their keys never reach `extra`
(`probe-model-output.txt:1-3,38`).

### 4.8 `Content`, `PaneFrame`, `Paint`, `Line`, `FloatingBlock`

**`Content`** is untagged, tried in this order (AGENTS.md:227-231, 378-408):

| Variant | Shape |
|---|---|
| `Mod` | `{ mod: ModName, props?: Map, extra }` |
| `Terminal` | `{ terminal: TerminalAddress, extra }` |
| `Tab` | `{ tab: String, pane?: PaneFrame, extra }` |
| `Agent` | `{ agent: String, extra }` |
| `Other` | `Map` |

Checked (`probe-model-output.txt:7,10,12-14`): the `Agent`, `Tab`-with-frame, unknown and
empty-`mod` cases.

**`PaneFrame`**:
- `gap`, `radius`: `Number`
- `background`: `Paint`
- `border`, `focus`: `Line`
- `header`: `PaneHeader { content: Box<Content>, height?: Number, position?: HeaderPosition{Top,
  Bottom, Other}, reveal?: Reveal{Always, Hover, Other}, extra }`
- `lone`: `bool`
- `extra`

**`Paint`** is untagged: `Color(Color)`, `Gradient { gradient: Vec<Color>, angle?: Number, extra
}`, `Image { image: String, fit?: Fit{Cover, Contain, Other}, extra }`, `Other(Map)`
(AGENTS.md:239-241).

**`Line`** is `{ width?: Number, color?: Color, extra }`.

**`FloatingBlock`** is `{ id?: NodeId, content: Content, edge?: Edge{Top, Right, Bottom, Left,
Other}, align?: Align{Start, Center, End, Other}, peek?: Number, extra }` (AGENTS.md:410-422).

### 4.9 `Theme` (`files/theme.rs`)

From AGENTS.md:339-345:

| Field | Type |
|---|---|
| `background`, `foreground`, `selection` (required) | `HexColor` |
| `cursor` | `HexColor` |
| `ansi` (required) | `[HexColor; 16]` |
| `colors` | `BTreeMap<ColorName, HexColor>` (win98 uses names beyond AGENTS.md's list, such as `ring`) |
| `extra` | `Map` |

### 4.10 `Keys` (`files/keys.rs`)

From AGENTS.md:515-544.

**`Keys`** is a `BTreeMap<Section, BTreeMap<Chord, Action>>`.

**`Section`** is parsed from the key string (`probe-model-output.txt:31-37`):

| Input | Result |
|---|---|
| `"global"` | `Global` |
| `"terminal"` | `Terminal` |
| `"layout:<name>"` with a non-empty name | `Layout(LayoutName)` |
| `"mod:<name>"` with a non-empty name | `Mod(ModName)` |
| anything else, including `"layout:"`, `"mod:"` and `"weird"` | `Other(String)` |

**`Action`**, rules applied in order (`probe-model-output.txt:16-30`):

| Input | Result |
|---|---|
| the exact string `"terminal"` | `Terminal` |
| a string starting `"send:"` | `Send(rest)`; `"send:"` gives `Send("")` |
| any other string | split at the **first** `:` into `Run { command: CommandName, value: Option<String> }`: `"pane.split:right"` → `Run{pane.split, "right"}`; `"space.new:{a:b}"` → `Run{space.new, "{a:b}"}`; `"terminal.copy"` → `Run{terminal.copy, None}` |
| a string with an empty command name (`":x"`, `""`) | refused: `KeyActionError` |
| an object whose `command` is a string and whose `args` is absent or an object | `Command { command: CommandName, args: Option<Map>, extra }`. An empty `command` is refused with `EmptyValueError` (`a command name cannot be empty`) |
| any other object (no `command`, or non-object `args`), number, bool, null or array | `Other(Value)`, kept |

Each form serializes back as written: JSON-equal, and the keys of `Command` re-ordered
(`probe-model-output.txt:21`).

## 5. Protocol (domain)

### 5.1 `Invocation` (`protocol/invocation.rs`)

| Constructor | Argv |
|---|---|
| `version()` | `["--version"]` |
| `commands()` | `["commands"]` |
| `ls()` | `["ls"]` |
| `status()` | `["status"]` |
| `run(&CommandName, &Map)` | `["do", <name>, <compact JSON>]` |

`run` always uses the JSON-object form, with `{}` for no arguments (F11). Each argv element is
one process argument, with no shell (`probe-window-output.txt:37-45`).

The JSON element is `serde_json::to_string` of the `Map`. Typed commands reach `run` through
`serde_json::to_value`, so their keys come out in alphabetical order:
`{"name":"api","argv":["claude"],"focus":false}` as a struct becomes
`{"argv":["claude"],"focus":false,"name":"api"}` (`probe-ack-output.txt:6-7`). herdr-app reads
the object by key: alphabetical objects such as `{"argv":…,"name":…,"tab":…}` and
`{"as":…,"name":"main"}` succeeded (`probe-commands-output.txt:729,800`). Tests compare this
element as parsed JSON, never as text.

`label()` gives `--version`, `commands`, `ls`, `status` or `do <name>`.

### 5.2 `RunOutput` and `classify` (`protocol/outcome.rs`)

`RunOutput { exit: Exit, stdout: Vec<u8>, stderr: Vec<u8> }`, where `Exit` is `Code(i32)` or
`Signal` (`probe-crates-output.txt:5`).

`classify(&RunOutput) -> Result<&[u8], Failure>` applies these rules in order:
1. `Code(0)`: `Ok(stdout)`.
2. `Code(1)`, and stdout is a JSON object whose `error` is a string: `Refused { message }`.
3. `Code(2)`: take stderr as lossy UTF-8, trim trailing whitespace, and strip one leading
   `"herdr-app: "`. If the rest starts with `"no Herdr window at "`, it is `NoWindow { message }`;
   otherwise `Cli { message }`.
4. Anything else: `Unexpected { exit, stderr }`, with stderr as lossy UTF-8, trimmed.

Checked examples:

| Input | Result | Evidence |
|---|---|---|
| exit 1 `{"error":"tab.new focus is true or false"}` | `Refused` | `probe-window-output.txt:136-142` |
| exit 2 `herdr-app: no Herdr window at /tmp/hzp/herdr-app.sock: No such file or directory (os error 2)\n` | `NoWindow` with message `no Herdr window at /tmp/hzp/herdr-app.sock: No such file or directory (os error 2)` | `probe-cli-output.txt:14-19` |
| exit 2 `herdr-app: args are not JSON: key must be a string at line 1 column 2\n` | `Cli` with message `args are not JSON: key must be a string at line 1 column 2` | `probe-cli-output.txt:44-49` |
| exit 1 `{"error":"the window did not answer"}` | `Refused` | F15 |

### 5.3 `AppVersion` (`protocol/version.rs`)

`AppVersion::parse(&str)` reads stdout as follows:
- trim surrounding whitespace
- require the prefix `"herdr-app "`
- parse the rest as `MAJOR.MINOR.PATCH[-PRE][+BUILD]`:
  - each number is ASCII digits with no leading zero unless it is `0`
  - PRE is dot-separated, non-empty identifiers of `[0-9A-Za-z-]`, with numeric identifiers free
    of leading zeros
  - BUILD is accepted and dropped
- anything else is `InvalidAppVersion { text }`

Ordering is semver 2.0 precedence. `Display` gives `X.Y.Z[-PRE]`.
- `AppVersion::MINIMUM` is `0.1.16` (ADR 0002).
- `AppVersion::PROVEN` is `0.1.17`.

Checked examples:

| Input | Result |
|---|---|
| `"herdr-app 0.1.17\n"` | `0.1.17` (F1) |
| `"herdr-app 0.1.16-rc.1"` | below `MINIMUM` |
| `"herdr-app 0.1.18+abc"` | `0.1.18` |
| `""`, `"herdr-app 01.1.16"`, `"herdr-app 0.1"`, `"x 0.1.16"` | `InvalidAppVersion` |

### 5.4 Identifiers (`protocol/identifiers.rs`)

Each is a non-empty string newtype with the same shape as §4.2:

| Type | `label` |
|---|---|
| `SpaceId` | `space id` |
| `TabId` | `tab id` |
| `PaneId` | `pane id` |
| `OverlayId` | `overlay id` |
| `SpaceName` | `space name` |
| `TabName` | `tab name` |
| `PaneName` | `pane name` |
| `TerminalName` | `terminal name` |
| `MachineName` | `machine name` |
| `Task` | `task` |
| `AppName` | `app name` |

Two have extra rules:
- **`Folder`** holds an absolute UTF-8 path. `Folder::new(impl AsRef<Path>)`:
  - refuses an empty path with `EmptyValueError { label: "folder" }`
  - refuses a relative path, `~/…` included, with `FolderError::Relative { path }`. std calls
    `x/y`, `./x` and `~/code` relative and `/tmp/x` absolute (`probe-crates-output.txt:23-27`).
  - refuses a non-UTF-8 path with `FolderError::NotUtf8 { path }`

  The SDK never resolves or expands a path.
- **`Argv`** is a non-empty `Vec<String>`. An empty list is refused with
  `EmptyValueError { label: "argv" }`.

The protocol reuses `NodeId`, `ModName`, `LayoutName`, `ThemeName`, `TerminalAddress` and
`CommandName` from `files`.

### 5.5 `catalog.rs`

- **`CommandInfo { name: CommandName, args: Vec<ArgInfo>, about: String }`** and
  **`ArgInfo { name: String, required: bool }`** decode `commands` (F2).
- **`KNOWN_COMMANDS: &[KnownCommand]`** lists the 77 commands as herdr-app 0.1.17 lists them.
  `KnownCommand { name: &'static str, args: &'static [(&'static str, bool)] }` holds the arg names
  and listed `required` flags, in the listed order. It is unit-tested in `catalog.rs`.

### 5.6 `listing.rs` (`ls`, F13)

| Type | Fields |
|---|---|
| `Listing` | `spaces: Vec<SpaceEntry>`, `recent: Vec<RecentSpace>` |
| `SpaceEntry` | `id: SpaceId`, `name: String`, `folder: String`, `machine: Option<MachineName>`, `active: bool`, `tabs: Vec<TabEntry>` |
| `TabEntry` | `id: TabId`, `name: String`, `active: bool`, `settled: bool`, `terminals: Vec<TerminalEntry>` |
| `TerminalEntry` | `terminal: TerminalAddress`, `pane: Option<PaneId>`, `status: Option<AgentStatus>`, `app: Option<AppName>`, `name: Option<String>`, `sub_agents: Vec<TerminalAddress>` |
| `RecentSpace` | `name: String`, `folder: String` |

`AgentStatus` is open: `Working`, `Blocked`, `Done`, `Idle`, `Other(String)`
(`evidence/herdr-app-0.1.17-home/herdr.d.ts:94`). Unknown fields are ignored; `Client::raw`
reads them.

### 5.7 `status.rs` (`status`, F14)

**`WindowStatus`:**

| Field | Type |
|---|---|
| `zoom`, `font_size`, `ui_font_size` | `f64` |
| `space` | `Option<SpaceId>` |
| `tab` | `Option<TabId>` |
| `focused_pane` | `Option<PaneId>` |
| `layout` | `LayoutName` |
| `agent` | `Option<TerminalAddress>` |
| `floats` | `BTreeMap<NodeId, Float>` |
| `overlays` | `Vec<OverlayId>` |
| `errors` | `Vec<String>` |

**`Float`**: `edge: Edge`, `align: Align`, and `vertical`, `dragging` and `hovered` as `bool`,
exactly the five keys 0.1.17 prints (`probe-views-output.txt:98-104`). Any other key is ignored
like every unknown output field. Output numbers are `f64` because outputs are never written back.

### 5.8 Command outputs (`commands/outputs.rs`)

| Type | Shape | Evidence |
|---|---|---|
| `Created` | `{space: SpaceId, tab: TabId, terminal: TerminalAddress}` | `probe-commands-output.txt:729-730` |
| `TabMade` | `{space, tab, terminal?: TerminalAddress, pane?: PaneId}`: a shell tab has `terminal`, a mod tab `pane` | `:738-739` |
| `Split` | `{pane: PaneId, terminal?: TerminalAddress}` | `:750-751` |
| `Opened` | `{terminal, space?, tab?, pane?}`: place tab gives space and tab, right and down give pane | `:763-765` |
| `Kept` | `{pane: PaneId, tab?: TabId, terminal?: TerminalAddress, pending?: bool}`. A first keep gave all four; a repeat keep gave only `pane` | `probe-gaps-output.txt:6,8` |
| `Parked` | `{terminal, pending: bool}` | `probe-gaps-output.txt:9` |
| `Ack` | an object whose `ok` is `true`; other fields are ignored. `Ack` decodes a `Map` first, then checks `ok`. An object without `ok: true` fails with exactly `expected "ok": true` (no position); a non-object fails with serde's ``invalid type: integer `7`, expected a map at line 1 column 1`` (`probe-ack-output.txt:1-5`). Either becomes `Error::Decode` | §5.10 |
| `RawReply` | `serde_json::Value` | §5.10 "no success recorded" |

### 5.9 Argument types (`commands/words.rs`)

**Closed input enums** (author's ruling: inputs closed, outputs open; a newer word goes through
`Client::raw`):

| Enum | Words | Evidence (`probe-commands-output.txt` unless named) |
|---|---|---|
| `SplitDirection` | `right`, `down` | `:203` |
| `Direction` | `left`, `right`, `up`, `down` | `:412` (pane.go); `:423` (pane.resize) |
| `Place` | `tab`, `right`, `down` | about `terminal.open`, `terminal.keep` |
| `ClickTarget` | `program`, `menu` | `:297` |
| `SelectMode` | `cell`, `word`, `line` | `:505` |
| `SelectionMove` | `left`, `right`, `up`, `down`, `page_up`, `page_down`, `home`, `end`, `line_start`, `line_end` | `:523` |
| `ScrollEdge` | `top`, `bottom` | `:549` |
| `SettingsSection` | `look`, `machines`, `alerts`, `updates`, `privacy` | AGENTS.md:679, the union with about `app.settings` |
| `FeedbackFrom` | `person`, `agent` | AGENTS.md:681 |

**Argument enums and numbers:**

| Type | Variants and wire form |
|---|---|
| `Focus` | `Show` (`true`), `Background` (`false`) |
| `Pin` | `Pinned`, `Unpinned` |
| `Machine` | `ThisComputer` (`""`), `Named(MachineName)` (AGENTS.md:578-580) |
| `Rename<N>` | `Automatic` (`""`), `To(N)`. herdr-app takes `""` for space, tab and pane (`probe-gaps-output.txt:38-39`; `probe-commands-output.txt:760`) |
| `Position` | 1-based; `Position::new(u32) -> Result<Position, ArgumentRangeError>` |
| `SpaceRef`, `TabRef` | `Index(Position)` or `Id(…)` |
| `TabMoveTo` | `Index(Position)` or `By(i32)` |
| `PaneTarget` | `Pane(PaneId)` or `Terminal(TerminalAddress)` |
| `SwapWith` | `Direction(Direction)` or `Target(PaneId)` |
| `Scroll` | `Lines(i64)`, `Pages(FiniteF64)` or `To(ScrollEdge)` |
| `At` | `Cursor` (`"cursor"`) or `Cell { row: u32, col: u32 }` |
| `Resize` | `Px(Px)` or `Weight(Weight)` |
| `TabContent` | `Shell { argv?, task?, folder? }` or `Mod { module: ModName, props?: Map }` |
| `PaneContent` | `Shell { argv? }` or `Mod { module, props? }` |
| `Px`, `Weight`, `FiniteF64` | `f64` newtypes with `new(f64) -> Result<_, ArgumentRangeError>` |

**`ArgumentRangeError { argument: &'static str, value: String, rule: &'static str }`**, Display
`<value> is out of range: <rule>`:

| Type | `argument` | Refused when | `rule` |
|---|---|---|---|
| `Position` | `index` | 0 | `positions start at 1` |
| `Px` | `px` | non-finite or negative | `must be finite and not negative` |
| `Weight` | `weight` | non-finite or ≤ 0 | `must be finite and above 0` |
| `FiniteF64` | `pages` | non-finite | `must be finite` |

Examples: `0 is out of range: positions start at 1`;
`-5 is out of range: must be finite and not negative`. herdr-app itself silently takes `px: -5`
and `2.5` for `layout.resize` (`probe-gaps-output.txt:40-41`), and refuses `index 0` only as "no
such space" (`:42`).

### 5.10 The 77 commands

Every command struct implements:
```rust
pub trait Command: Serialize {
    const NAME: &'static str;
    type Output: DeserializeOwned;
}
```

- **R** marks a constructor parameter; other arguments are optional setters.
- An argument enum serializes to the wire keys it stands for.
- **Proof** is a line of `probe-commands-output.txt` (`:N`) or `probe-gaps-output.txt` (`G:N`)
  showing that output from a successful run.
- **"no success recorded"** marks a command with no recorded success; its output is `RawReply`:
  - never run, because of a shared install or the network: `app.update`, `app.update_core`,
    `app.check_updates`
  - never run, because they act on the person's other apps: `app.hide`, `app.hide_others`,
    `app.show_all`
  - never run, because of the system clipboard: `terminal.copy`, `terminal.paste`,
    `terminal.paste_image`
  - never run, because of Herdr's feedback flow: `app.feedback`
  - run, but always refused with "that pane isn't on screen" in an isolated window, even on a pane
    of a visible two-pane tab: `pane.resize` (`:761-762`; G:19-20)

| Command | Arguments | Output | Proof |
|---|---|---|---|
| `space.new` | name `SpaceName`, folder `Folder`, tab `TabName`, argv `Argv`, task `Task`, machine `Machine`, focus `Focus` | Created | :729-730 |
| `space.close` | id `SpaceId`, `.confirmed()` | Ack | :824 |
| `space.rename` | id `SpaceId`, **R** name `Rename<SpaceName>` | Ack | :731; G:38 |
| `space.move` | **R** id `SpaceId`, **R** index `Position` | Ack | :732 |
| `tab.new` | name `TabName`, space `SpaceId`, **R** content `TabContent`, focus `Focus` | TabMade | :738-739 |
| `tab.close` | id `TabId`, `.confirmed()` | Ack | :823 |
| `tab.rename` | id `TabId`, **R** name `Rename<TabName>` | Ack | :740; G:39 |
| `tab.move` | id `TabId`, **R** to `TabMoveTo` | Ack | :741-742 |
| `pane.split` | **R** direction `SplitDirection`, pane `PaneId`, **R** content `PaneContent` | Split | :750-751 |
| `pane.close` | pane `PaneId` | Ack | :822 |
| `pane.rename` | pane `PaneId`, **R** name `Rename<PaneName>` | Ack | :759-760 |
| `pane.resize` | **R** direction `Direction`, pane `PaneId`, px `Px` | RawReply | no success recorded |
| `terminal.open` | argv `Argv`, folder `Folder`, place `Place`, name `TerminalName` | Opened | :763-765 |
| `terminal.keep` | **R** terminal `TerminalAddress`, place `Place` | Kept | G:6,8 |
| `terminal.park` | **R** terminal | Parked | G:9 |
| `terminal.right_click` | **R** to `ClickTarget`, terminal | Ack | :780-781 |
| `terminal.pin` | **R** terminal, **R** pinned `Pin` | Ack | :782-783 |
| `terminal.kill` | **R** terminal | Ack | G:10 |
| `space.select` | **R** `SpaceRef` | Ack | :733-734 |
| `space.next`, `space.prev` | — | Ack | :735-736 |
| `tab.select` | **R** `TabRef` | Ack | :743-744 |
| `tab.settle`, `tab.unsettle` | id `TabId` | Ack | :747-748 |
| `tab.next`, `tab.prev` | — | Ack | :745-746 |
| `pane.zoom` | pane `PaneId` | Ack | :752-753 |
| `pane.focus` | **R** `PaneTarget` | Ack | :754-755 |
| `pane.go` | **R** direction `Direction` | Ack | :756 |
| `pane.swap` | pane `PaneId`, **R** with `SwapWith` | Ack | :757; G:21 |
| `terminal.peek` | **R** terminal | Ack | :766 |
| `terminal.show` | terminal, pane `PaneId` | Ack | :768-769 |
| `terminal.select_all` | terminal | Ack | :771 |
| `terminal.select` | terminal, at `At`, mode `SelectMode` | Ack | :772-773; G:12,15 |
| `terminal.adjust_selection` | terminal, **R** direction `SelectionMove` | Ack | G:13-14 |
| `terminal.clear_selection` | terminal | Ack | :775; G:16 |
| `terminal.copy`, `terminal.paste`, `terminal.paste_image` | — | RawReply | no success recorded |
| `terminal.scroll` | terminal, **R** `Scroll` | Ack | :776-778 |
| `terminal.search` | — | Ack | :779 |
| `agent.focus` | **R** terminal | Ack | :788 |
| `agent.next`, `agent.prev` | — | Ack | :789-790 |
| `layout.switch` | name `LayoutName` | Ack | :791-793 |
| `layout.clone` | **R** name `LayoutName`, as `LayoutName` | Ack | :800 |
| `mod.clone` | **R** name `ModName`, as `ModName` | Ack | :805 |
| `layout.set` | **R** layout `files::Layout` | Ack | :802 |
| `layout.toggle`, `layout.show`, `layout.hide` | **R** id `NodeId` | Ack | :794-796 |
| `layout.resize` | **R** id `NodeId`, **R** `Resize` | Ack | :797-798 |
| `layout.select` | **R** stack `NodeId`, **R** child `NodeId` | Ack | :799 |
| `layout.reset` | name `LayoutName` | Ack | :803 |
| `mod.open` | **R** name `ModName` | Ack | :806-807 |
| `overlay.close` | id `OverlayId` | Ack | :767 |
| `app.help`, `app.welcome`, `app.whats_new`, `app.palette` | — | Ack | :808-814 |
| `app.feedback` | message `String`, from `FeedbackFrom` | RawReply | no success recorded |
| `app.settings` | section `SettingsSection` | Ack | :816 |
| `app.check_updates`, `app.update_core`, `app.update` | — | RawReply | no success recorded |
| `app.reset` | — | Ack | :825 |
| `app.theme` | **R** name `ThemeName` | Ack | :818 |
| `app.zoom_in`, `app.zoom_out`, `app.zoom_reset` | — | Ack | :819-821 |
| `app.quit` | — | Ack | `probe-window-output.txt:317-323` |
| `app.hide`, `app.hide_others`, `app.show_all` | — | RawReply | no success recorded |
| `window.close` | — | Ack; afterwards the window stops answering (F15), which its doc comment says | :831-832 |
| `window.minimize`, `window.maximize` | — | Ack | :828-830 |

Totals: 77 commands, of which 60 Ack, 6 shaped outputs and 11 RawReply.

Requiredness and one-of groups, from the probes:
- **Required in practice though listed optional:** herdr-app refuses `space.rename`, `tab.rename`
  and `pane.rename` without a name (`probe-commands-output.txt:71,170,386`).
- **Refused when both are given:** `tab.move`, `pane.swap` and `terminal.scroll`
  (`probe-gaps-output.txt:27,31-32`).
- **Silently resolved when both are given:** `tab.new` and `pane.split` take `mod` over `argv`
  (`:35-36`); `space.select`, `tab.select` and `pane.focus` with both forms answered as for the id
  (`:24,26,29`).
- **Accepted with neither:** `layout.resize` with neither `px` nor `weight` (`:33`); the SDK still
  requires one.

## 6. Port (`port/`)

```rust
pub trait Transport: Send + Sync + 'static {
    fn run(&self, invocation: &Invocation)
        -> impl Future<Output = Result<RunOutput, TransportError>> + Send;
}
```

`TransportError` (`port/error.rs`):
- `NotFound { program: String }` when the spawn fails with `io::ErrorKind::NotFound`
  (`probe-crates-output.txt:1`)
- `Io { program: String, source: io::Error }` otherwise

## 7. Driver (`driver/client.rs`)

`Client<T: Transport>` holds `Arc<T>` and is `Clone` for every `T`. `Client::new(transport)` does
no I/O.

| Method | Invocation | Result |
|---|---|---|
| `version()` | `--version` | `AppVersion` |
| `check_version()` | `--version` | `AppVersion`, or `UnsupportedVersion` below `MINIMUM` |
| `commands()` | `commands` | `Vec<CommandInfo>` |
| `ls()` / `status()` | `ls` / `status` | `Listing` / `WindowStatus` |
| `run<C: Command>(C)` | `do C::NAME <json>` | `C::Output` |
| `raw(Invocation)` | any | `serde_json::Value`. `raw(Invocation::version())` is `Decode`, because its stdout is not JSON, and the doc comment says so |

Flow of one call:
1. Build the `Invocation`. `run` serializes `C` with `serde_json::to_value`; a value that is not
   an object is `InvalidArgument { argument: "command", detail: "a command must serialize to a
   JSON object" }`.
2. `transport.run`.
3. `classify`.
4. Decode stdout with `serde_json::from_slice`. `version` reads stdout as lossy UTF-8 and calls
   `AppVersion::parse`.

Concurrent calls are allowed. Each runs its own process, with no ordering promise.

## 8. Entrypoint (`herdr_app/`)

`HerdrApp<T: Transport = ProcessTransport>` holds a `Client<T>` and its checked `AppVersion`. Like
`Client`, it is `Clone` for every `T` (a hand-written impl, not a derive that would demand
`T: Clone`).

| Item | Does |
|---|---|
| `HerdrApp::local().await` | `HerdrApp::builder().connect()` |
| `HerdrApp::builder()` → `HerdrAppBuilder` | `.program(impl AsRef<OsStr>)`, `.home(impl AsRef<Path>)`, `.env(k, v)`, `.env_remove(k)`, then `.connect().await`: a `ProcessTransport` plus `check_version` |
| `HerdrApp::from_client(Client<T>).await` | `check_version` |
| `version()` | `&AppVersion` |
| `client()` | `&Client<T>` |
| `ls()`, `status()`, `commands()` | as on `Client` |

The version is checked once, at construction.

**Operation groups.** Each method returns a `Call<T, C>`, which owns a clone of the app's
`Client<T>`. Parameter types are plain values:
- names and ids: `impl Into<String>`
- argv: `impl IntoIterator<Item = impl Into<String>>`
- folders: `impl AsRef<Path>`
- index: `u32`; by: `i32`; lines: `i64`
- pages, px and weight: `f64`

A value its type refuses becomes `Error::InvalidArgument { argument, detail }` at `.await`, with
the refusing error's Display as `detail`, and nothing runs. `argument` is the wire name, as
`herdr-app commands` lists it, of the argument the refused value fills: `name`, `tab`, `id`,
`index` and so on. Examples:
- `invalid index: 0 is out of range: positions start at 1`
- `invalid name: a space name cannot be empty`
- `invalid argv: a argv cannot be empty`

`rename(name)`, on spaces, tabs and panes, takes a non-empty name: `rename("")` is
`invalid name: a <space|tab|pane> name cannot be empty`. Only `rename_automatic()` (spaces, tabs)
and `unname()` (panes) send `""`.

| Group (file) | Methods |
|---|---|
| `spaces()` (`spaces.rs`) | `new_space` · `close` · `rename(name)` · `rename_automatic` · `move_to(id, index)` · `select_index(index)` · `select_id(id)` · `next` · `prev` |
| `tabs()` (`tabs.rs`) | `new_tab` · `new_mod_tab(mod)` · `close` · `rename(name)` · `rename_automatic` · `move_to(index)` · `move_by(by)` · `select_index(index)` · `select_id(id)` · `settle` · `unsettle` · `next` · `prev` |
| `panes()` (`panes.rs`) | `split(direction)` · `split_mod(direction, mod)` · `close` · `rename(name)` · `unname` · `resize(direction)` · `zoom` · `focus(pane)` · `focus_terminal(t)` · `go(direction)` · `swap(direction)` · `swap_with(target)` |
| `terminals()` (`terminals.rs`) | `open` · `keep(t)` · `park(t)` · `peek(t)` · `show` · `kill(t)` · `pin(t)` · `unpin(t)` · `right_click(to)` · `select_all` · `select` · `adjust_selection(dir)` · `clear_selection` · `copy` · `scroll_lines(lines)` · `scroll_pages(pages)` · `scroll_to(edge)` · `paste` · `paste_image` · `search` |
| `agents()` (`terminals.rs`) | `focus(t)` · `next` · `prev` |
| `layouts()` (`layouts.rs`) | `switch(name)` · `next` · `clone(name)` · `set(layout)` · `toggle(id)` · `show(id)` · `hide(id)` · `resize_px(id, px)` · `resize_weight(id, weight)` · `select(stack, child)` · `reset` |
| `mods()` (`window.rs`) | `clone(name)` · `open(name)` |
| `overlays()` (`window.rs`) | `close` |
| `window()` (`window.rs`) | `help` · `welcome` · `whats_new` · `feedback` · `palette` · `settings` · `check_updates` · `update_core` · `reset` · `theme(name)` · `zoom_in` · `zoom_out` · `zoom_reset` · `update` · `quit` · `hide` · `hide_others` · `show_all` · `close` · `minimize` · `maximize` |

Together they reach all 77 commands: 9 + 13 + 12 + 20 + 3 + 11 + 2 + 1 + 21 = 92 methods, with
several methods per command where a one-of group splits.

**`Call`** (`call.rs`):
- Optional arguments are setters named after the argument (`.name("api")`,
  `.argv(["claude"])`, `.focus(Focus::Background)`, `.confirmed()`, `.px(30.0)`). The one
  exception is `as`, a Rust keyword: its setter is `.as_name(..)` on `layout.clone` and
  `mod.clone`, on the typed commands and on `Call` alike. Its refusal still names the argument
  `as`.
- The first conversion failure is stored, later setters do nothing, and `.await` returns it
  without running herdr-app.
- `Call<T, C>` holds `client: Client<T>` and `command: Result<C, Error>`, and implements
  `IntoFuture<Output = Result<C::Output, Error>>` through the one justified, `'static` boxed
  future of §2 (bounds `C: Command + Send + 'static`, `C::Output: Send`).
- Setters are written per command; a crate-private `macro_rules!` in `call.rs` may generate them.

## 9. Adapter (`adapter/process.rs`)

`ProcessTransport` holds `program: OsString` (default `"herdr-app"`, found on `PATH`),
`home: Option<PathBuf>` and an ordered list of env changes. It is `Clone`.
- `ProcessTransport::herdr_app()` gives the default.
- `ProcessTransport::builder()` offers `.program`, `.home`, `.env`, `.env_remove` and `.build()`.

`run` does this:
1. `tokio::process::Command` with the invocation's argv.
2. stdin null, stdout and stderr piped.
3. Set `HERDR_APP_HOME` when `home` is set, then apply the env changes in order.
4. `kill_on_drop(true)`, then `.output().await`.

Behaviour:
- The child inherits the parent's environment, `HC_*` and `HERDR_APP_HOME` included, unless they
  are removed; the type's docs say so.
- Dropping the future kills the child (`probe-crates-output.txt:2-4`).
- There is no internal timeout.

## 10. Errors (exact `Display`)

`message` is herdr-app's text with `herdr-app: ` and trailing whitespace removed (§5.2).
`invocation` is `Invocation::label()`. Only `Unexpected` substitutes `(no output)` for an empty
stderr. `Refused`, `NoWindow` and `Cli` show `message` as it is, even when empty.

**`driver::Error`:**

| Variant | `Display` |
|---|---|
| `Transport(TransportError)` | transparent |
| `Refused { invocation, message }` | `herdr-app <invocation> refused: <message>` — e.g. `herdr-app do tab.new refused: tab.new focus is true or false` |
| `NoWindow { message }` | `herdr-app has no window to answer: <message>` |
| `Cli { message }` | `herdr-app did not accept the invocation: <message>` |
| `Unexpected { exit: Code(n), stderr }` | `herdr-app exited with code <n>: <stderr>` |
| `Unexpected { exit: Signal, stderr }` | `herdr-app was ended by a signal: <stderr>` |
| `Decode { invocation, detail }` | `cannot read the output of herdr-app <invocation>: <detail>` — e.g. `cannot read the output of herdr-app ls: missing field `spaces` at line 1 column 13` (`probe-messages-output.txt:10`) |
| `InvalidVersion(InvalidAppVersion)` | transparent |
| `UnsupportedVersion { found, minimum }` | `herdr-app <found> is older than <minimum>, the oldest version this crate supports` |
| `InvalidArgument { argument, detail }` | `invalid <argument>: <detail>` |

**The smaller errors:**

| Type | `Display` |
|---|---|
| `TransportError::NotFound { program }` | ``cannot start `<program>`: it was not found`` |
| `TransportError::Io { program, source }` | ``cannot start `<program>`: <source>`` |
| `FileError { file, detail }` | `invalid <layout\|theme\|keys> file: <detail>` |
| `ColorError::Empty` | `a colour cannot be empty` |
| `ColorError::NotHex { value }` | `"<value>" is not a hex colour: use #rrggbb or #rrggbbaa` |
| `KeyActionError { value }` | `"<value>" is not a key action: its command name is empty` |
| `NumberError { value }` | `<value> is not a finite number` |
| `EmptyValueError { label }` | `a <label> cannot be empty` |
| `FolderError::Relative { path }` | `"<path>" is not an absolute path` |
| `FolderError::NotUtf8 { path }` | `"<lossy path>" is not UTF-8` |
| `ArgumentRangeError` | `<value> is out of range: <rule>` (§5.9) |
| `InvalidAppVersion { text }` | `cannot read a herdr-app version from <text:?>` — e.g. `cannot read a herdr-app version from "herdr-app 0.1"` |

`FileError.detail` is serde_json's `Display`, verbatim. The pinned shapes
(`probe-messages-output.txt`; `probe-model-output.txt`):

| Case | Detail |
|---|---|
| missing field | ``missing field `spaces` at line 1 column 13`` (`:10`) |
| wrong type | ``invalid type: integer `7`, expected a string at line 1 column 15`` (`:3`) |
| wrong length | `invalid length 1, expected an array of length 16 at line 1 column 87` (`:2`) |
| duplicate named field | ``duplicate field `background` at line 1 column 36`` (`:7`) |
| empty text | `EOF while parsing a value at line 1 column 0` (`:4`) |
| trailing text | `trailing characters at line 1 column 240` (`:5`) |
| byte-order mark | `expected value at line 1 column 1` (`:6`) |
| invalid UTF-8 | `invalid unicode code point at line 1 column 13` (`:12`) |
| bad colour in a direct field | ``"#12345" is not a hex colour: use #rrggbb or #rrggbbaa at line 1 column 18`` (`:37`) |
| a non-object, non-colour value in `Paint` | `data did not match any variant of untagged enum Paint …` (`probe-messages-output.txt:39`; `probe-model-output.txt:15`) |
| key action with an empty name | ``":x" is not a key action: its command name is empty …`` (`probe-model-output.txt:28`) |

## 11. Edge cases

| Case | Ruling | Owner |
|---|---|---|
| empty id, name, address or chord | `EmptyValueError` | identifiers |
| whitespace-only name | accepted as written | identifiers |
| `""` rename | typed: `Rename::Automatic`; `HerdrApp`: `rename("")` is `InvalidArgument`, `rename_automatic()`/`unname()` send `""` | commands, `HerdrApp` |
| index 0, out-of-range px/weight/pages | `ArgumentRangeError`; through `HerdrApp` → `InvalidArgument` | `words.rs`, `call.rs` |
| empty argv | `EmptyValueError` | identifiers |
| argv with spaces or quotes | one JSON string each | `invocation.rs` |
| relative, `~/…` or non-UTF-8 folder | `FolderError` | identifiers |
| a mod's `<mod>.<command>` | `Client::raw(Invocation::run(..))` | driver |
| stdout empty or not the shape | `Decode` | driver |
| `Ack` with extra fields | accepted | `outputs.rs` |
| unknown output word or field | `Other(String)`, or ignored | listing, status |
| version unreadable, below 0.1.16 | `InvalidVersion`, `UnsupportedVersion` | version, driver |
| herdr-app not on `PATH` | `TransportError::NotFound` | adapter |
| no window, home too long | `NoWindow` | outcome |
| after `window.close` | `Refused` "the window did not answer" | outcome |
| future dropped | child killed | adapter |
| concurrent calls | independent processes | driver |
| unknown file key, word or object shape | kept (`extra`, `Other`) | files |
| invalid value inside a `Content`/`Size`/`Paint` object | that object kept as `Other` | files |
| repeated unknown key | last value kept | files |
| repeated named field, trailing text, empty file, BOM | `FileError` | files |
| keys `"send:"` | `Send("")` | keys |
| keys `"layout:"` | `Section::Other` | keys |
| keys value with `:` | split at the first `:` | keys |
| keys action of another JSON kind | `Action::Other` | keys |
| node fields that don't fit its kind | kept as written | layout |
| `children: []` vs absent | distinct | layout |

## 12. Testing

### 12.1 Colocated unit tests

Each logic file tests its rules, including the rejection side:
- `number.rs`: the forms of §4.3; NaN and the infinities.
- `color.rs`: §4.4's examples, both cases, both lengths, empty.
- Both `identifiers.rs`: empty; `Folder` relative, `~/`, absolute; non-UTF-8 (a `#[cfg(unix)]`
  test built from `OsStr::from_bytes`); `Argv` empty.
- `keys.rs`: every row of §4.10; `Section`; the round trip.
- Every layout file: the §4.6–§4.8 shapes; `extra` in every variant; `Other` cases.
- `theme.rs`: required fields; the `ansi` length.
- `invocation.rs`: each argv; argv with spaces.
- `outcome.rs`: every rule of §5.2 with its evidence string.
- `version.rs`: every example and the precedence.
- `words.rs`: each wire form; each `ArgumentRangeError` row.
- Each command file: one full and one minimal instance per command.
- `outputs.rs`: each recorded output; `Ack` with an extra field accepted; `{"ok":false}` refused.
- `catalog.rs`: `KNOWN_COMMANDS` has 77 unique names.
- `driver/client.rs`: each method over an in-file fake (`#[tokio::test]`), including the
  non-object command.
- `herdr_app/*`: `Call` stores the first refusal and runs nothing.
- `adapter/process.rs`: `NotFound` for a missing program. A `#[cfg(unix)]` test spawns
  `sh -c 'printf %s "$HERDR_APP_HOME"'` through `.program("sh")` to check env order and
  `HERDR_APP_HOME`.

### 12.2 `tests/fixtures/`

Copied byte for byte from:
- `evidence/herdr-app-0.1.17-commands.json`
- `evidence/fresh-home-0.1.17/layouts/{main,terminals,win98}.json`
- `evidence/fresh-home-0.1.17/themes/{atom-one-dark,win98}.json`
- `evidence/fresh-home-0.1.17/keys.json`

### 12.3 `tests/catalog.rs` (offline)

- `KNOWN_COMMANDS` equals the fixture: the same 77 names, and per command the same arg names in
  order with the same `required` flags.
- For each of the 77 command types: its `NAME` is in `KNOWN_COMMANDS`, and an instance with every
  field set serializes only keys that are arg names of that command. Every enum variant is
  covered across the test.

### 12.4 `tests/files_roundtrip.rs` (offline)

Each fixture layout, theme and keys file must:
1. parse
2. serialize with `to_json` (pretty) to a value JSON-equal to the source
3. contain no duplicate key: the `to_json` text has the same length as
   `serde_json::to_string_pretty` of the same value after one pass through `serde_json::Value`
   (the `probe-model` method)
4. re-parse and re-serialize to the same text

### 12.5 `tests/client_fake.rs` (offline)

`tests/support/fake_transport.rs` replays scripted `RunOutput`s and records the invocations.
Cases:
- each §5.8 output from its recorded stdout
- every `classify` path into its `Error`, with exact Display
- `check_version` accepting 0.1.16, 0.1.17 and 0.2.0, and refusing 0.1.15 and 0.1.16-rc.1
- `raw` returning an unnamed field
- `raw(version())` → `Decode`
- `HerdrApp::from_client` checking once
- a `Call` with an empty name or index 0 → `InvalidArgument`, with no invocation recorded

### 12.6 `tests/live.rs`

`tests/support/live_window.rs` provides the harness.

**Isolation.** Every herdr-app and hc process the harness starts gets:
- `HERDR_APP_HOME=/tmp/hfa<pid>-<n>`
- `HC_SESSION=hfa<pid>-<n>`
- `XDG_STATE_HOME`, `XDG_CONFIG_HOME` and `XDG_RUNTIME_DIR` under `/tmp/hfa<pid>-<n>/x/`
- every inherited `HC_*` variable and `HERDR_APP_HOME` removed

These are the names probe-commands used (`probe-commands-output.txt:3,726,728,835`). A leftover
`/tmp/hfa<pid>-<n>` is removed before use. A unit test in `live_window.rs` asserts the env list.
The harness never runs a command against the person's window or default core.

**Skip rules.** The test prints `skipped: <reason>` and passes when:
- `herdr-app` is not on `PATH`
- for the window test: `hc` is not on `PATH`, the target is not unix, or the launched `herdr-app`
  exits before answering `status` (no window can be opened: headless, no display)

A window process that is alive but has not answered `status` within 45 s is a **failure**.

**Drift test** (no window; `HERDR_APP_HOME` is an empty scratch directory):
- `herdr-app --version` must be at least `MINIMUM` (fails otherwise).
- It compares the installed `commands` with `KNOWN_COMMANDS`: names, arg names, `required`.
  - Installed version == `PROVEN` (0.1.17): any difference **fails**, listing it.
  - Any other version: the test passes and prints the full difference (added, removed, changed)
    as `warning: herdr-app <version> differs from the proven 0.1.17: …`.

**Window test** (one test function, so one window):
- **Launch:** spawn `herdr-app` with stdout and stderr null, and poll `status` every 300 ms.
- **Scenario:**
  1. `ls` and `status` decode.
  2. `space.new` with argv `["sh","-c","sleep 600"]` → `Created`.
  3. `tab.new` shell → `TabMade` with a terminal; mod `files` → `TabMade` with a pane.
  4. `pane.split` argv → `Split` with a terminal; mod → `Split` without one.
  5. `pane.rename` → `Ack`.
  6. `terminal.open` → `Opened` with a tab; place right → `Opened` with a pane.
  7. A terminal from `hc run --bg --json --no-parent` in the isolated core: `terminal.keep` →
     `Kept`, then `terminal.park` → `Parked`. A refused park ("pane change in progress",
     `probe-gaps-output.txt:7`) is retried once after 500 ms.
  8. `tab.next` → `Ack`.
  9. `do tab.new {"bogus":1}` through `Client::raw` → `Refused` (`probe-window-output.txt:117-123`).
  10. `layout.clone {name:"main", as:"hfa-<pid>"}`, then `layout.set` with the fixture
      `main.json` as `files::Layout` → `Ack`.
- **Teardown** (also run by a Drop guard if the scenario panics):
  1. `app.quit`; wait up to 20 s
  2. `kill -TERM <pid>` (F16); wait up to 10 s
  3. `Child::kill`
  4. `hc server stop` in the isolated session
  5. remove the home
- **Never sent:** `window.close` and the eleven "no success recorded" commands.

### 12.7 Doctests and examples

- Every public item with behaviour has a doctest. Doctests needing herdr-app are `no_run`.
- `examples/inspect`: version, `ls`, `status`.
- `examples/drive`: spaces, tabs and panes through `HerdrApp`.
- `examples/files`: builds and prints a layout, a theme and keys.

The examples compile under `cargo test --all-targets` and are run by no test.

## 13. Dependencies and supply chain

`crates/herdrify-app/Cargo.toml` gains:

```toml
[dependencies]
tokio      = { workspace = true, features = ["process"] }
serde      = { workspace = true }
serde_json = { workspace = true }
thiserror  = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["macros", "rt-multi-thread", "time"] }
```

The workspace already locks these crates for `herdrify-hc` (`crates/herdrify-hc/Cargo.toml`), so
`cargo deny check` sees no new crate.

The package becomes version `0.1.0`:
- the `publish = false` line is removed
- `keywords = ["herdr", "terminal", "async", "client", "layout"]`
- `categories = ["api-bindings", "asynchronous"]`
- the root dependency entry becomes `herdrify-app = { path = "crates/herdrify-app", version =
  "0.1.0" }`

## 14. Documentation changes

- `CLAUDE.md` Status: `herdrify-app` is implemented at 0.1.0, with live tests described like
  hc's.
- `docs/herdr-and-airsl.md` § herdr-app: the version row becomes 0.1.17, and rows are added for
  F1–F17 with evidence paths into this chain.
- `crates/herdrify-app/README.md`, `CHANGELOG.md` (0.1.0) and `examples/README.md`.

## 15. Acceptance

- `cargo make dod` passes with zero warnings. On the author's Mac with herdr-app 0.1.17, this
  includes the drift test (strict) and the window test.
- `cargo deny check` passes.
- `cargo publish --dry-run -p herdrify-app` passes at 0.1.0.
- The §2 enforcement greps print nothing.

## 16. Non-goals

- The `shot`, `check`, `feedback` and `--skill` subcommands, and opening a window from the public
  API. Only the test harness launches one.
- herdr-app's window socket. ADR 0002 fixes the transport as the CLI's argv.
- Reading or writing files in a Herdr home; that belongs to `herdrify-herdr`.
- herdrify's ports, adapters, allowlist or key rules.
- Typing the eleven RawReply outputs.
- Publishing to crates.io. Only the dry run is in scope.
- Release automation, changelogs or versions of other crates.
- Proving compatibility with herdr-app versions other than 0.1.17.
