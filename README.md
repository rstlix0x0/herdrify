# herdrify

Rust tools and libraries for [Herdr](https://herdr.dev): standalone async SDKs for `herdr-core` and
`herdr-app`, plus a framework of sandboxed components you can add and remove one at a time. The
components are written in Lua and run under [airsl](https://github.com/airsstack/airsl).

> **Status: early.** The Cargo workspace and its nine crates exist; no features are built yet. This README describes the plan. The decisions behind
> it are in [`.claudestacks/sdlc/adrs/`](.claudestacks/sdlc/adrs/) and [`docs/`](docs/).

## What it is

```text
herdrify
├─ SDKs          herdrify-hc (herdr-core) · herdrify-app (herdr-app), async, standalone
├─ framework     a Herdr layout made of components you add and remove one at a time
├─ components    integrations, tools, panels, frames and themes, written in Lua
├─ bundles       a whole setup (frame, theme, keys, components) shared as one unit
└─ CLI           add · remove · ls · check · render · gc · core · new · test · dev · sdk
```

## How it fits together

```text
 Herdr app
 └─ mods/herdrify-host/main.js    the only JavaScript: draws what Lua describes
       │  exec("herdrify render <component>")
       ▼
 herdrify (Rust)
 └─ airsl engine ── components/<name>@<commit>/main.lua
       │   gets only the grants its manifest declares, within your ceiling, that you approved
       ▼
 UI tree (JSON, ui = 1) ─► drawn with the active theme's colors
```

- **Always sandboxed.** Components and bundles contain only `.lua`, `.toml`, `.md` and `LICENSE`
  files. There is no third-party JavaScript and no trusted tier.
- **herdrify writes the Herdr files.** herdrify generates the layouts, themes and key bindings it
  puts in Herdr's home. It never copies them from a component or bundle.
- **One JavaScript host.** Herdr runs only JavaScript mods, so a single generic mod renders every
  component.
- **Theme aware.** Components use the theme's color names, never fixed colors.

## Crates

| Crate | Job |
|---|---|
| `herdrify-core` | domain types, ports, use cases, and grant logic. No I/O. |
| `herdrify-hc` | herdr-core client: async JSON-RPC over `hc connect`, public `Transport` |
| `herdrify-app` | herdr-app client: async commands, layout/theme/keys models, public `Transport` |
| `herdrify-lua` | runs components under airsl, with host modules behind grants |
| `herdrify-herdr` | writes Herdr's files and drives herdr-app and hc for herdrify |
| `herdrify-store` | `~/.herdrify`: config, lock, installed components, cache, state |
| `herdrify-git` | fetches component and bundle repositories |
| `herdrify-testkit` | fakes for every port, for tests and `herdrify test` |
| `herdrify-cli` | the `herdrify` command |

See [docs/architecture.md](docs/architecture.md) for layers, dependency rules and testing.

## A component

```text
components/github-ci/
├─ component.toml
└─ main.lua
```

```toml
name  = "github-ci"
kind  = "integration"        # integration · tool · panel · theme · frame
slot  = "right"
entry = "main.lua"
ui    = 1                    # render contract version

[grants]
proc = ["gh"]

[grants.hc]                  # optional: herdr-core access
read = "own"                 # "own" or "all" terminals

[keys]
"cmd-alt-g" = "layout.toggle:github-ci"
```

## A bundle

```text
rust-dev/
├─ bundle.toml               name, version, members
└─ components/cargo-panel/   an inline member
```

A member is either a component folder inside the bundle or a pinned `{ repo, path, rev }`
reference. A bundle holds at most one frame and one theme. Installing a bundle applies them, and
removing it restores what you had before.

## Planned commands

```text
herdrify add <repo> --path <dir>        a component or a bundle: plan, approve, apply, lock
herdrify remove <name> [--purge]        undo exactly what add did (--purge: also its state)
herdrify ls                             frame, theme, components, bundles
herdrify check <dir>                    validate a component or bundle, offline
herdrify render <component>             the component's UI tree as JSON
herdrify gc                             delete installed revisions nothing uses
herdrify core …                         run, send to, read and watch terminals

herdrify new component|bundle <name>    a new skeleton
herdrify test <dir>                     run a component's Lua tests against fakes
herdrify dev <dir>                      link a local component, live in Herdr
herdrify sdk emit                       Lua editor stubs + JSON Schemas
```

Everything herdrify keeps lives in `~/.herdrify` (or `$HERDRIFY_HOME`):

```text
~/.herdrify/
├─ config.toml                  frame, theme, sources, ceiling, enabled
├─ herdrify.lock                what is installed and every file herdrify wrote to Herdr
├─ cache/git/<host>/<repo>/     fetched repositories
├─ components/<name>@<commit>/  installed components, immutable
├─ dev/<name>                   local components under development
└─ state/<name>/                each component's writable data
```

## Roadmap

1. `herdrify-hc` SDK
2. `herdrify-app` SDK
3. `herdrify-core` domain and `herdrify-store`; Lua frames and themes composed into a Herdr layout
4. `add` / `remove` / lock, grants and approval, bundles
5. The JavaScript host and its widgets, the Lua API and the `ui = 1` render contract, `check` and
   `render`
6. Components: code tree, git, GitHub CI, Kubernetes, Grafana
7. Events and live updates
8. Author tooling (`new`, `test`, `dev`, `sdk emit`) and documentation

## License

Apache-2.0. See [LICENSE](LICENSE).
