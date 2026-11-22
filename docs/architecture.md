# Architecture

How herdrify is put together. Each decision on this page is recorded in an approved ADR under
[`.claudestacks/sdlc/adrs/`](../.claudestacks/sdlc/adrs/), and the ADR is the authority. The terms
are defined in [concepts.md](concepts.md).

## Layers

herdrify uses ports and adapters ([ADR 0001](../.claudestacks/sdlc/adrs/0001-ports-and-adapters-architecture.md)).

```text
 L2  herdrify-cli       composition root: wires real adapters, terminal Approver,
                        user and author commands
 L1  herdrify-lua       Runtime port: airsl engine, host modules, grants → airsl Policy
     herdrify-herdr     Window + Terminals ports: Herdr home files, herdr-app, hc
     herdrify-store     Store port: $HERDRIFY_HOME
     herdrify-git       Source port: fetch repo@rev into the cache
     herdrify-testkit   fakes for every port except Runtime
 L0  herdrify-core      domain types, ports, use cases (add, remove, render, check, compose)
     herdrify-hc        herdr-core SDK, standalone
     herdrify-app       herdr-app SDK, standalone
```

| Crate | Job | ADR |
|---|---|---|
| `herdrify-core` | domain types, the six ports, use cases, grant logic, Lua API description. No I/O. | 0001, 0004, 0007 |
| `herdrify-hc` | async herdr-core client: JSON-RPC over `hc connect`, sans-IO protocol, public `Transport` | 0002 |
| `herdrify-app` | async herdr-app client: commands, layout/theme/keys models, public `Transport` | 0002 |
| `herdrify-lua` | `Runtime` adapter: runs components under airsl, host modules behind grants | 0001, 0004 |
| `herdrify-herdr` | `Window` + `Terminals` adapter over both SDKs and Herdr's home | 0001, 0003 |
| `herdrify-store` | `Store` adapter over `$HERDRIFY_HOME` | 0005 |
| `herdrify-git` | `Source` adapter: fetches repositories | 0001 |
| `herdrify-testkit` | published fakes, for herdrify's tests and `herdrify test` | 0001, 0007 |
| `herdrify-cli` | the `herdrify` binary | 0001 |

## Dependency rules

```text
 herdrify-cli ──► adapters ──► herdrify-core
                     │
                     └──► herdrify-hc, herdrify-app   (no herdrify deps)

 normal dependencies point only downward · dev-dependencies are exempt
```

- **`herdrify-core` is pure.** It has no I/O, and its normal dependencies include no tokio, no airsl
  and no SDK.
- **Ports are traits used through generics**, never `Box<dyn …>`. The ports are `Runtime`,
  `Window`, `Terminals`, `Store`, `Source` and `Approver`.
- **Ports are async.** Each method returns `impl Future<Output = …> + Send`. Implementors are
  `Send + Sync + 'static`, shared through `Arc`.
- **The SDKs depend on no herdrify crate.** `herdrify-herdr` maps them onto core's ports.

## Sync Lua, async ports

airsl is synchronous, while the SDKs and ports are async. They meet inside `herdrify-lua`:

```text
 #[tokio::main]  herdrify-cli
      │
      ▼  core use case (async)
      ├──► herdrify-herdr ──► SDK clients            async, native
      └──► herdrify-lua
             spawn_blocking( airsl eval main.lua )   sync engine on a blocking thread
                   │  Lua calls herdrify.hc.read(…)
                   ▼
             Handle::block_on( terminals.read(…) )  sync → async
```

`block_on` works on a blocking thread. It panics on an async worker thread, so every eval runs
inside `spawn_blocking`. This was proven against airsl 0.1.4 and tokio 1.54.1
(`.claudestacks/sdlc/adrs/evidence/probe-bridge-output.txt`).

## Testing

| What | Where | Against |
|---|---|---|
| use cases | `herdrify-core/tests/` (integration tests) | `herdrify-testkit` fakes |
| pure logic | colocated `#[cfg(test)]` in core | fakes written in the same file |
| `herdrify-store` | the crate's tests | a temporary `$HERDRIFY_HOME` |
| `herdrify-git` | the crate's tests | a local git repository |
| `herdrify-herdr` | the crate's tests | a temporary `HERDR_APP_HOME` and a running `hc` |
| `herdrify-lua` | the crate's tests | the real airsl engine, plus a test that the Lua API description matches the host modules |

Why use cases are integration tests: a `#[cfg(test)]` unit test inside core cannot use testkit's
fakes. It sees a second copy of core and fails with E0277
(`.claudestacks/sdlc/adrs/evidence/probe-devdep-cycle-output.txt`).

## Trust boundary

There is one sandboxed level, with no trusted tier for installs
([ADR 0003](../.claudestacks/sdlc/adrs/0003-sandbox-only-herdrify-sole-writer.md)).

```text
 ┌──────────── trusted ─────────────────────────┐   ┌──── sandboxed (installed) ────┐
 │ herdrify binary · herdrify-host main.js      │◄──│ components + bundles          │
 │ allowlisted Herdr mods (empty props)         │   │ .lua .toml .md LICENSE only   │
 │ files herdrify generates in Herdr's home     │   │ run under approved grants     │
 └──────────────────────────────────────────────┘   └───────────────────────────────┘
```

- **herdrify is the only writer of what it creates in Herdr's home.** That covers generated layouts,
  generated themes, `mods/herdrify-host/`, and its own `keys.json` sections:
  `layout:herdrify-<frame>` and `mod:herdrify-host`.
- **Generated keys are restricted.** They bind only to `herdrify-host.<command>` and to
  `layout.toggle`, `layout.show` and `layout.hide` on generated blocks. `send:` actions and `argv`
  commands are refused.
- **Grants are owned by core.** The kinds are `fs`, `env`, `proc` and `hc`, capped by the ceiling in
  `config.toml`. A request over the ceiling refuses the plan
  ([ADR 0004](../.claudestacks/sdlc/adrs/0004-grants-owned-by-core.md)).

## Runtime flows

### `herdrify render <component>`

```text
 herdrify-host (JS, in Herdr) ──exec──► herdrify render github-ci
   Store: lock → component dir + approved grants
   Runtime: airsl Policy (fs/env/proc) + herdrify.hc module (hc grant)
   eval main.lua → UI tree { ui = 1, … } ──stdout──► host draws it with theme colors
```

### `herdrify add` / `remove`

```text
 add <repo> --path <dir>
   Source.fetch → cache/          (or use the cached revision)
   core: parse bundle.toml | component.toml, check files, grants, keys, clashes
   core: plan  ──► Approver (all-or-nothing)
   Store: components/<name>@<commit>/ , lock entries
   core: compose ──► Window: regenerate layouts, themes, keys sections, record hashes

 remove <name>
   drop lock entry ─► regenerate from the remaining set
   refuse if a generated file or section was edited since herdrify wrote it
```

## Author SDK

Authors get a Lua API, generated editor stubs and JSON Schemas, and the commands `new`, `check`,
`test`, `render` and `dev`. Tests run on the real runtime with every other port faked by
`herdrify-testkit` ([ADR 0007](../.claudestacks/sdlc/adrs/0007-author-sdk.md)).
