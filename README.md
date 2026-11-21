# herdrify

Rust tools and libraries for [Herdr](https://herdr.dev): SDKs for `herdr-core` and `herdr-app`,
and a framework of components you can add and remove one at a time, written in Lua and run
under [airsl](https://github.com/airsstack/airsl).

> **Status: planned.** Nothing is built yet. This README describes the plan.

## What it is

```text
herdrify
├─ SDKs          talk to herdr-core (terminals, agents, machines) and herdr-app (the window)
├─ framework     a layout made of components you add and remove one at a time
├─ components    integrations, tools, panels, layouts and themes, written in Lua
└─ CLI           add · remove · install · new · check · core
```

## How it fits together

```text
 Herdr app
 └─ mods/herdrify-host/main.js    the only JavaScript: draws what Lua describes
       │  exec("herdrify render <component>")
       ▼
 herdrify (Rust)
 └─ airsl engine ── components/<name>/main.lua
       │   gets only the grants its manifest declares and you approved
       ▼
 UI tree (JSON) ─► rendered with the active theme's colors
```

- **Components are Lua.** Each one declares what it needs (programs, files) and gets nothing else.
- **One JavaScript host.** Herdr runs only JavaScript mods, so a single generic mod renders every
  component.
- **Theme aware.** Components use the theme's color names, never fixed colors, so they follow
  every theme, light or dark.

## Planned crates

| Crate | Job |
|---|---|
| `herdrify-core` | herdr-core client: JSON-RPC over `hc connect`, typed from `hc schema` |
| `herdrify-app` | herdr-app client: `herdr-app do` commands, layout, keys and theme types |
| `herdrify-lua` | runs components under airsl, with host modules behind grants |
| `herdrify-compose` | builds the Herdr layout and themes from a frame and its components |
| `herdrify-pkg` | fetch, plan, apply, check and lock component installs |
| `herdrify-cli` | the `herdrify` command |

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

[grants]
proc = ["gh"]

[keys]
"cmd-alt-g" = "layout.toggle:github-ci"
```

## Planned commands

```text
herdrify add <repo> --path components/github-ci   plan, approve grants, apply, lock
herdrify remove github-ci                         undo exactly what was added
herdrify install <repo> --path <dir>              a plain Herdr bundle, as it is
herdrify new component <name>                     a new component skeleton
herdrify ls                                       frame, slots, components
herdrify check                                    validate components and the layout
herdrify core …                                   run, send to, read and watch terminals
```

Everything herdrify keeps lives in `~/.herdrify`:

```text
~/.herdrify/
├─ config.toml       frame, sources, the most any component may be granted
├─ cache/            fetched repositories, by commit
└─ herdrify.lock     what each component installed, from where, with which grants
```

## Roadmap

1. `herdrify-core` SDK
2. `herdrify-app` SDK
3. Lua frames and themes, composed into a Herdr layout
4. `add` / `remove` / lock, with grant approval
5. The JavaScript host and its widgets: list, table, text, badge, button, sparkline
6. Components: code tree, git, GitHub CI, Kubernetes, Grafana
7. Events and live updates
8. `new` generator and documentation

## License

Apache-2.0. See [LICENSE](LICENSE).
