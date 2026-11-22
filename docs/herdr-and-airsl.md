# Herdr and airsl: facts herdrify builds on

These facts were observed on 2026-11-22. The raw output behind each row is in
[`.claudestacks/sdlc/adrs/evidence/`](../.claudestacks/sdlc/adrs/evidence/). Probe again before
relying on a fact for a newer version.

| Tool | Version |
|---|---|
| hc (herdr-core) | 0.1.21 |
| herdr-app | 0.1.16 |
| airsl | 0.1.4 (mlua 0.12.2) |
| tokio (probe) | 1.54.1 |

## herdr-core (`hc`)

| Fact | Evidence |
|---|---|
| `hc schema` is an OpenRPC 1.3.2 document: 32 methods, 36 schemas | `probe-facts-output.txt:3-7,40` |
| JSON-RPC 2.0, one message per line, or framed after `hello`. Protocol generation 1. Errors carry a stable `data.kind`. | `probe-facts-output.txt:6` |
| `hc connect` serves the API over stdio; the remote form is `ssh box hc connect` | `hc-0.1.21-help.txt:41` |
| Streams: `events.subscribe` and `terminal.attach` | `probe-facts-output.txt:21,33` |
| `hc exec`, `hc file`, `machine.exec` and `file.write` reach any program and any file | `hc-0.1.21-help.txt:27,29` |

## herdr-app

| Fact | Evidence |
|---|---|
| Home is `~/.config/herdr-app`; `HERDR_APP_HOME` picks another | `herdr-app-0.1.16-help.txt:19` |
| 75 commands; each arg has only `name` and `required`, with no types | `probe-facts-output.txt:42-44` |
| `herdr.d.ts` types `run(name, args?: Record<string, unknown>)`, so command args are untyped there too | `herdr-app-0.1.16-dts-excerpts.md` |
| 4 commands take `argv`: `space.new`, `tab.new`, `pane.split`, `terminal.open` | `probe-facts-output.txt:45` |
| Mods are trusted JS with unrestricted commands and network | AGENTS.md:1140 |
| A layout block's content is `{ "mod", "props" }`; the shipped `toast` mod runs the command named in its props | AGENTS.md:224, `mods/toast/main.js:27` |
| `keys.json` sections: `global`, `layout:<name>`, `terminal`, `mod:<name>`; the more specific one wins | AGENTS.md:523-525 |
| Any edit makes `keys.json` the person's, so default shortcut upgrades stop | AGENTS.md:510-512 |
| Shipped layouts and mods are read-only and replaced on every start | AGENTS.md:188-193 |

AGENTS.md lines are in `herdr-app-0.1.16-agents-excerpts.md`.

## airsl

| Fact | Evidence |
|---|---|
| Synchronous: `eval` and `eval_to` are plain `fn`s; no `async fn`; no tokio in the tree | `airsl-0.1.4-excerpts.md` |
| `GrantSet` has three axes: `fs`, `env`, `proc` | `src/sandbox/grant_set.rs:54-59` |
| `proc` matches the program name as written, not a resolved path | `src/sandbox/grants.rs:221-226` |
| Third-party host modules, under a host-chosen root table | `src/modules/registry.rs:1-6`, `src/builder.rs:78-86` |
| Ships its own extension manifest, `Ceiling`, `negotiate` and `Approver`; herdrify does not use them (ADR 0004) | `src/extension/*` |
| `block_on` inside `spawn_blocking` works; on an async worker it panics | `probe-bridge-output.txt:2,7` |

airsl source lines are in `airsl-0.1.4-excerpts.md`.
