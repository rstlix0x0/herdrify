# Concepts

One word per concept. The approved ADRs in
[`.claudestacks/sdlc/adrs/`](../.claudestacks/sdlc/adrs/) are the authority.

## The things people install

| Term | Meaning | ADR |
|---|---|---|
| **component** | A folder with `component.toml` and Lua files, run under airsl with only its approved grants. | 0001, 0003 |
| **kind** | A component's role, declared in its manifest: `integration`, `tool`, `panel`, `theme` or `frame`. | 0005 |
| **frame** | A component of kind `frame`. herdrify generates a Herdr layout from it. | 0003 |
| **theme** | A component of kind `theme`. herdrify generates a `themes/<name>.json` from it. | 0003 |
| **bundle** | A shareable setup. A folder with `bundle.toml` listing components, each either inside the bundle or a pinned `{ repo, path, rev }` reference. Holds at most one frame and one theme. Bundles do not nest. | 0006 |
| **identity** | What makes two components the same: git URL plus path. Only one revision of a name is installed at a time. | 0006 |

A component or bundle folder may contain only `*.lua`, `*.toml`, `*.md` and `LICENSE` files
(ADR 0003).

## Safety

| Term | Meaning | ADR |
|---|---|---|
| **grant** | What a component may touch. The kinds are `fs`, `env`, `proc` and `hc`. | 0004 |
| **hc grant** | Access to herdr-core. `read = "own" \| "all"`, `run = [programs]`, `send = "own"`. 18 of hc's 32 methods are never exposed. | 0004 |
| **ceiling** | The most any component may be granted, set in `config.toml`. A request over it refuses the plan. | 0004 |
| **approval** | The person accepts a whole plan or declines it. Nothing in between. | 0004, 0006 |
| **sandboxed** | The only trust level for installed things. | 0003 |
| **host** | `herdrify-host`, herdrify's one JavaScript mod inside Herdr. It draws every component's UI tree. | 0003 |
| **allowlisted Herdr mods** | Herdr's own shipped mods that a generated layout may use, always with empty props. | 0003 |

## Rendering

| Term | Meaning | ADR |
|---|---|---|
| **UI tree** | The JSON a component returns. The host draws it with the active theme's color names. | 0007 |
| **render contract** | `ui = 1` in `component.toml`. The host refuses a version it does not know. | 0007 |
| **Lua API description** | Data in `herdrify-core` describing every Lua function: name, parameters, return type and required grant. Editor stubs are generated from it. | 0007 |

## State

| Term | Meaning | ADR |
|---|---|---|
| **herdrify home** | `$HERDRIFY_HOME`, default `~/.herdrify`. | 0005 |
| **Herdr home** | `HERDR_APP_HOME`, default `~/.config/herdr-app`. Herdr reads layouts, keys, mods and themes from here. | 0003 |
| **config.toml** | The person's intent: active frame and theme, git sources, ceiling, enabled components. The only record of intent. | 0005 |
| **lock** | `herdrify.lock`. What is installed, plus a `[generated]` table of every file and section herdrify wrote in Herdr's home, with hashes. | 0005 |
| **regenerate** | After every `add` or `remove`, herdrify rebuilds its Herdr files from the installed set. | 0005 |
| **state** | `state/<name>/`, a component's writable data. Kept on `remove`, deleted with `--purge`. | 0005 |

## Code

| Term | Meaning | ADR |
|---|---|---|
| **port** | A trait in `herdrify-core` for an outside party: `Runtime`, `Window`, `Terminals`, `Store`, `Source`, `Approver`. | 0001 |
| **adapter** | A crate implementing ports against the real world. | 0001 |
| **fake** | A port implementation in `herdrify-testkit`, used by tests and by `herdrify test`. | 0001, 0007 |
| **SDK** | `herdrify-hc` and `herdrify-app`: standalone async clients for herdr-core and herdr-app. | 0002 |
| **Transport** | The public async trait an SDK client talks through: stdio, ssh, or a fake. | 0002 |
