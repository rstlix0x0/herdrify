---
status: done
created: 2026-11-23
depends-on: [02]
built-on: aab842c3c7d3
---

# Layout Model Implementation Plan

**Goal:** Model `layouts/<name>.json` as a typed `Layout` that keeps everything it does not name.

**Architecture:** `files/layout/` is a folder with five role files and an export-only `mod.rs`
(spec §2), and no file is named `layout.rs`. The shared rules of plan 02's Architecture apply
verbatim. Model rules:
- Every struct is `#[non_exhaustive]` with public fields.
- Every object-shaped struct, and every struct variant of an untagged enum, ends with
  `#[serde(flatten)] pub extra: serde_json::Map<String, Value>`.
- Every word list ends with `#[serde(untagged)] Other(String)`.
- Every number is `files::Number`.
- Every optional field is `#[serde(skip_serializing_if = "Option::is_none")]`.
- The node is flat: an explicit `type` field.
- `PaneHeader.content` is `Box<Content>`, which breaks the recursive type (spec §2).
- Every struct is buildable outside the crate: all-optional structs derive `Default`, and the rest
  have a named `new` (Task rows below).

**Tech Stack:** Rust 1.94, serde 1.0.229, serde_json 1.0.154.

**Content authority:** spec §4.1, §4.5–§4.8; §10 (the `Paint` message, layout `FileError`s); §11
(the files rows).

---

## Files that change

```
crates/herdrify-app/src/files/mod.rs              — modify: mod layout and re-exports
crates/herdrify-app/src/files/error.rs            — modify: crate-private FileError::from_serde (first used by Layout::parse)
crates/herdrify-app/src/files/layout/mod.rs       — create: export-only table of contents of the layout model
crates/herdrify-app/src/files/layout/paint.rs     — create: Paint, Fit, Line
crates/herdrify-app/src/files/layout/content.rs   — create: Content, PaneFrame, PaneHeader, HeaderPosition, Reveal
crates/herdrify-app/src/files/layout/floating.rs  — create: FloatingBlock, Edge, Align
crates/herdrify-app/src/files/layout/look.rs      — create: Look, ThemeChoice, RootOptions, Titlebar, ClonedFrom
crates/herdrify-app/src/files/layout/node.rs      — create: Layout, Node, NodeKind, Size, Padding, Shadow, FocusStyle
```

## Order of work

### Task 1 — Add Paint, Fit and Line

**Files:**
- Create `crates/herdrify-app/src/files/layout/mod.rs` and `files/layout/paint.rs`.
- Modify `crates/herdrify-app/src/files/mod.rs` — `mod layout;` and the re-exports.

**Change:**
- `Line { width?: Number, color?: Color, extra }`, deriving `Default`.
- `Fit`: `Cover`, `Contain`, `Other(String)`.
- `Paint`, untagged, in this order:
  - `Color(Color)`
  - `Gradient { gradient: Vec<Color>, angle?: Number, extra }`
  - `Image { image: String, fit?: Fit, extra }`
  - `Other(Map)`

  With helpers `Paint::color(Color)` and `Paint::gradient(Vec<Color>)`.

**Proof:**
- `paint_parses_each_shape` — `"border"`, `{"gradient":["#2a6cbd","#0b2a57"],"angle":165}` and
  `{"image":"a.jpg","fit":"cover"}` give `Color`, `Gradient` and `Image`, each round-tripping
  JSON-equal.
- `paint_variants_keep_extra` — `{"gradient":["#000000"],"x":1}` keeps `x` in `Gradient`'s
  `extra`, and `{"image":"a.jpg","x":1}` in `Image`'s.
- `paint_keeps_unknown_objects` — `{"board":1}` and `{"gradient":["#12"]}` are `Other` and
  round-trip.
- `paint_refuses_a_non_object_non_colour` — `7` fails with a message starting `data did not match
  any variant of untagged enum Paint`.
- `line_keeps_extra` — `{"width":1,"color":"border","dash":2}` keeps `dash`.
- Doctests on `Paint::color` and `Line::default`.

**Done when:** `cargo test -p herdrify-app files::layout::paint` passes and the Definition of Done
is green. Commit `feat(herdrify-app): add the layout paint types`.

### Task 2 — Add Content and the pane frame

**Files:**
- Create `crates/herdrify-app/src/files/layout/content.rs`.
- Modify `crates/herdrify-app/src/files/layout/mod.rs`.

**Change:**
- `Content`, untagged, in this order:
  - `Mod { module: ModName (wire "mod"), props?: Map, extra }`
  - `Terminal { terminal: TerminalAddress, extra }`
  - `Tab { tab: String, pane?: PaneFrame, extra }`
  - `Agent { agent: String, extra }`
  - `Other(Map)`

  With constructors `Content::module(ModName)` (no props) and `Content::terminal(TerminalAddress)`.
- `PaneFrame { gap?, radius?, background?, border?, focus?, header?, lone?, extra }`, deriving
  `Default`.
- `PaneHeader { content: Box<Content>, height?, position?, reveal?, extra }`, with
  `PaneHeader::new(Content)`.
- `HeaderPosition`: `Top`, `Bottom`, `Other`. `Reveal`: `Always`, `Hover`, `Other`.

**Proof:**
- `content_parses_each_variant` — `{"mod":"files","props":{"role":"x"}}`, `{"terminal":"t7"}`,
  `{"tab":"active","pane":{"gap":6,"lone":false,"zz":1}}` and `{"agent":"focused"}` give `Mod`,
  `Terminal`, `Tab` (with `zz` in the frame's `extra`) and `Agent`, each round-tripping.
- `content_variants_keep_extra` — `x:1` added to each of the four objects lands in that variant's
  own `extra`.
- `content_falls_back_to_other` — `{"mod":""}` and `{"board":"x"}` are `Other` and round-trip.
- `pane_frame_parses_the_agents_md_example` — the JSON at AGENTS.md:383-387 parses, with header
  content `Mod(pane)`, height 30, position `Top` and reveal `Always`, and round-trips.
- `pane_header_keeps_extra` — `{"content":{"mod":"pane"},"x":1}` keeps `x`.
- `module_constructor_writes_no_props` — `Content::module(name)` serializes as `{"mod":"<name>"}`.
- Doctests on `Content::module` and `PaneHeader::new`.

**Done when:** `cargo test -p herdrify-app files::layout::content` passes and the Definition of
Done is green. Commit `feat(herdrify-app): add layout content and pane frames`.

### Task 3 — Add FloatingBlock

**Files:**
- Create `crates/herdrify-app/src/files/layout/floating.rs`.
- Modify `crates/herdrify-app/src/files/layout/mod.rs`.

**Change:**
- `FloatingBlock { id?, content, edge?, align?, peek?, extra }`, with
  `FloatingBlock::new(Content)`.
- `Edge`: `Top`, `Right`, `Bottom`, `Left`, `Other`.
- `Align`: `Start`, `Center`, `End`, `Other`.

**Proof:**
- `floating_block_parses_the_agents_md_example` — the block at AGENTS.md:413 round-trips.
- `floating_block_keeps_extra` — an added `x:1` lands in `extra`.
- `edge_and_align_keep_unknown_words` — `"middle"` is `Align::Other("middle")` and serializes
  back.
- A doctest on `FloatingBlock::new`.

**Done when:** `cargo test -p herdrify-app files::layout::floating` passes and the Definition of
Done is green. Commit `feat(herdrify-app): add floating blocks`.

### Task 4 — Add Look and RootOptions

**Files:**
- Create `crates/herdrify-app/src/files/layout/look.rs`.
- Modify `crates/herdrify-app/src/files/layout/mod.rs`.

**Change:**
- `Look` and `RootOptions` with spec §4.7's fields, both deriving `Default` and having no
  `extra` (they are flattened into `Node`, whose `extra` takes the rest).
- `ThemeChoice`, untagged: `Name(ThemeName)`, `ByAppearance { light, dark, extra }`,
  `Other(Value)`.
- `Titlebar`: `None` (wire `"none"`), `Other`.
- `ClonedFrom { name: LayoutName, version: Value, extra }`, with `ClonedFrom::new(LayoutName,
  Value)`.

**Proof:**
- `theme_choice_parses_each_form` — `"graphite"`, `{"light":"paper","dark":"ink","x":1}` (with `x`
  kept) and `7` give `Name`, `ByAppearance` and `Other`.
- `root_options_parse_link_handlers_and_cloned_from` — `{"link_handlers":["my-browser"],
  "cloned_from":{"name":"main","version":"0.1.17","x":1},"titlebar":"none"}` fills the three
  fields, with `x` in `ClonedFrom.extra`.
- `titlebar_keeps_unknown_words` — `"custom"` is `Titlebar::Other`.
- A doctest on `ThemeChoice` building a by-appearance choice.

**Done when:** `cargo test -p herdrify-app files::layout::look` passes and the Definition of Done is
green. Commit `feat(herdrify-app): add layout look and root options`.

### Task 5 — Add Node and Layout

**Files:**
- Create `crates/herdrify-app/src/files/layout/node.rs`.
- Modify `crates/herdrify-app/src/files/layout/mod.rs`.
- Modify `crates/herdrify-app/src/files/error.rs` — crate-private `FileError::from_serde(FileKind,
  &serde_json::Error)`, keeping serde's Display verbatim as `detail`.

**Change:**
- `Node`, with every field of spec §4.6 in that order:
  - `kind` renamed `type`
  - `#[serde(flatten)] look`, then `#[serde(flatten)] root`, then `#[serde(flatten)] extra`
- `NodeKind`: `Block`, `Row`, `Column`, `Stack`, `Other(String)`.
- `Size`, untagged: `Px { px, extra }`, `Weight { weight, extra }`, `Other(Map)`, with helpers
  `Size::px(Number)` and `Size::weight(Number)`.
- `Padding`, untagged: `Even(Number)`, `Pair([Number; 2])`.
- `Shadow`: `Sm`, `Md`, `Lg`, `Other`.
- `FocusStyle { border?: Line, extra }`, deriving `Default`.
- Constructors: `Node::block(Content)`, `Node::row(Vec<Node>)`, `Node::column(Vec<Node>)` and
  `Node::stack(Vec<Node>, NodeId)`.
- `Layout(Node)`, `#[serde(transparent)]`:
  - `Layout::new(Node)`
  - `Layout::parse(&str) -> Result<Layout, FileError>` with `FileKind::Layout`
  - `to_json() -> String`: pretty plus `"\n"`
  - `root()` and `root_mut()`

**Proof:**
- `node_keeps_children_absent_and_empty_apart` — `{"type":"row"}` and
  `{"type":"row","children":[]}` re-serialize as written.
- `node_keeps_unknown_type_and_keys` — `{"type":"grid","link_handlers":["my-browser"],
  "zz":[1,2.0]}` is `Other("grid")`, with `link_handlers` in `root` and `zz` in `extra`, and
  round-trips with `2.0` kept.
- `node_keeps_fields_that_do_not_fit_its_kind` — `{"type":"block","children":[]}` round-trips.
- `node_flattened_structs_take_their_keys` — a node with `theme`, `font_size` and `titlebar` has
  none in `extra`.
- `size_keeps_extra_and_falls_back` — `{"px":1,"weight":2}` is `Px` with `weight` kept;
  `{"weight":2,"x":1}` is `Weight` with `x` kept; `{"px":"x"}` is `Other`.
- `focus_style_keeps_extra` — `{"border":{"width":1},"x":1}` keeps `x`.
- `node_repeated_unknown_key_keeps_last` — `{"type":"row","x":1,"x":2}` has `extra.x == 2`.
- `node_constructors_build_documented_shapes` — `Node::stack(children, active)` serializes with
  `type:"stack"`, `children` and `active`.
- `layout_parse_reports_file_errors` — each displays exactly:
  - `""` → `invalid layout file: EOF while parsing a value at line 1 column 0`
  - a BOM-prefixed layout → `invalid layout file: expected value at line 1 column 1`
  - a valid layout plus ` x` → a detail starting `trailing characters at line 1 column`
  - `{"type":"row","type":"row"}` → a detail starting ``duplicate field `type` ``
- `layout_to_json_is_pretty_with_newline` — the output ends with `"\n"` and uses two-space
  indentation.
- Doctests on `Node::block`, `Layout::parse` and `Layout::to_json`.

**Done when:** `cargo test -p herdrify-app files::layout` passes and the Definition of Done is
green. Commit `feat(herdrify-app): add the layout node and Layout`.

---

## Risks

- Untagged-enum order decides the match: keep spec §4.6–§4.8's orders exactly.
- A missed `skip_serializing_if` writes `null`; plan 06's fixture round trip catches it.

## Proof

- Plan 06 runs the three shipped layouts end to end. `cargo make dod` is green.

## Review findings

Reviewer verdict: compliant with amendments, `blocking: none`. It re-ran `cargo make dod`: exit 0,
warning/error grep empty; `cargo test -p herdrify-app files` gave 64 passed, plus 58 doctests.
The probe crate was in the session scratchpad.

Open (non-blocking, not fixed):
- 🟡 `files/layout/content.rs:27` — a known key repeated inside an untagged object falls to
  `Other` silently: `{"mod":"a","mod":"b"}` → `Other({"mod":"b"})` (same for `Paint`, `Size`).
  `Layout::parse` refuses a repeated named field only on a node; its doc reads wider.
- 🟡 `files/layout/node.rs:3` — an explicit `null` on any optional field reads as `None` and is
  dropped on write (`"gap":null`, `"header":null`, `"theme":null`, `"props":null`,
  `"pane":null` do not round-trip JSON-equal). Docs say every field is kept; no test pins it.
- 🟡 `files/layout/node.rs:224` — a public `extra` map can hold a key that a named or flattened
  field also writes; `to_json` then writes it twice and `Layout::parse` refuses its own output
  (`invalid layout file: duplicate field `gap` at line 4 column 7`). Unguarded and undocumented.
- 🔵 `content.rs:314` (and paint, floating, look, node) — `read_back`/`fail` test helpers are
  copied into five files.
- 🔵 `look.rs:26` — `look.rs` and `node.rs` import from each other (the `Padding` deviation).
- 🔵 `look.rs:306,331` — `return Ok(fail("…")?);` instead of `return Err(...)`.
- 🔵 `node.rs:56` — the `# Errors` list of `Layout::parse` is too narrow (any node lacking
  `type`; invalid values such as `"#12"` are refused too).
- 🔵 `node.rs:74` — doctests use `.unwrap_err()` (also in the plan 02 doctests).
- 🔵 `node.rs:91` — "alphabetical" `extra` order is serde_json's byte order and holds only
  without `serde_json/preserve_order`; no test with two extra keys pins it.
- 🔵 `node.rs:108` — the `to_json` fallback is unreachable (checked), but would write `"\n"`.
- 🔵 `node.rs:286` — `Node::row`, `Node::column`, `Node::stack` and `Layout::new` have no
  `# Examples` (a unit test covers them).
- 🔵 `paint.rs:100` — struct variants of `Paint`, `Content` and `ThemeChoice` are not
  `#[non_exhaustive]`. The reviewer judged this correct: marking them would make the
  variants without helpers unbuildable outside the crate. The cost is that a later field
  addition is semver-major.
- 🔵 Unplanned helper `ThemeChoice::by_appearance` (used by the `ThemeChoice` doctest).

## Deviations

- 2026-11-23 — `Look.terminal_padding` (Task 4) needs `Padding`, which Files-that-change places
  in `node.rs`, created by Task 5. Task 4 created `node.rs` holding only `Padding`, and Task 5
  extended it. The final module tree matches the plan.
- 2026-11-23 — `Content::Tab.pane` is `Option<Box<PaneFrame>>`, not `Option<PaneFrame>`: clippy
  `large_enum_variant` under `-D warnings` rejected the unboxed field. Serialization is unchanged.
- 2026-11-23 — Test names `pane_frame_parses_the_agents_md_example` and
  `floating_block_parses_the_agents_md_example` were built as `*_documented_example`, keeping
  file references out of source. The JSON is the same (AGENTS.md:383-387 and :413, checked by the
  reviewer).
- 2026-11-23 — One review for the whole plan, and no per-task commits (as in plan 02).
