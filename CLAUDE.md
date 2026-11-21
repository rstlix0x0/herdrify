# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Status

Pre-code. The repo holds only `README.md` (the plan), `LICENSE` (Apache-2.0) and a Cargo
`.gitignore`. There is no Cargo workspace, no build, no tests yet. Do not assume commands,
crate layouts or APIs beyond what `README.md` states; when the workspace lands, add the real
build/lint/test commands here.

## What herdrify is

Rust tools and libraries for [Herdr](https://herdr.dev): SDKs for `herdr-core` and `herdr-app`,
plus a framework of add/remove-one-at-a-time components written in Lua and run under
[airsl](https://github.com/airsstack/airsl).

## Planned architecture

```text
 Herdr app
 └─ mods/herdrify-host/main.js    the ONLY JavaScript: generic renderer
       │  exec("herdrify render <component>")
       ▼
 herdrify (Rust)
 └─ airsl engine ── components/<name>/main.lua
       │   gets only the grants its component.toml declares AND the user approved
       ▼
 UI tree (JSON) ─► rendered with the active theme's color names
```

Design invariants (from the README):

- Herdr runs only JavaScript mods, so one generic JS host renders every component; all
  component logic lives in Lua.
- Components are capability-scoped: `[grants]` in `component.toml` (e.g. `proc = ["gh"]`) is
  the whole of what a component may touch. `~/.herdrify/config.toml` caps the maximum grant.
- Components use theme color names, never fixed colors.
- `herdrify remove` must undo exactly what `add` did, driven by `~/.herdrify/herdrify.lock`.

## Planned crates

| Crate | Job |
|---|---|
| `herdrify-core` | herdr-core client: JSON-RPC over `hc connect`, typed from `hc schema` |
| `herdrify-app` | herdr-app client: `herdr-app do` commands, layout, keys, theme types |
| `herdrify-lua` | runs components under airsl, host modules behind grants |
| `herdrify-compose` | builds the Herdr layout and themes from a frame and its components |
| `herdrify-pkg` | fetch, plan, apply, check, lock component installs |
| `herdrify-cli` | the `herdrify` command |

Component kinds: `integration · tool · panel · theme · frame`. A component is a directory with
`component.toml` (name, kind, slot, entry, `[grants]`, `[keys]`) and its Lua entry file.

## Roadmap order

1. `herdrify-core` SDK → 2. `herdrify-app` SDK → 3. Lua frames/themes composed into a layout →
4. `add`/`remove`/lock with grant approval → 5. JS host + widgets → 6. components →
7. events/live updates → 8. `new` generator and docs.
