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

### Using `hc connect` from a program (`herdrify-hc`)

`herdrify-hc` 0.1.0 is the async client built on these facts. They were observed on `hc` 0.1.21
against throwaway cores. The raw output behind each row is in
[`.claudestacks/sdlc/2026-11-22-herdrify-hc/evidence/`](../.claudestacks/sdlc/2026-11-22-herdrify-hc/evidence/),
and the Evidence column names the file inside it.

| Fact | Evidence |
|---|---|
| A connection starts in plain mode: JSON-RPC 2.0, one compact message per line. The reply to `hello {"generation":1,"framed":true}` is still a plain line; frames start right after it in the same byte stream. | `probe-plain-output.txt:583-590`, `probe-framed-output.txt:36-42` |
| A frame is `[channel u32 LE][kind u8][len u32 LE][payload]`, payload at most 16 MiB. Kinds: 0 DATA, 1 CREDIT, 2 CLOSE, 3 RESIZE, 4 FOCUS, 5 FACTS, 6 AUTOMATIC_INPUT. | `upstream-v0.1.21-schema-README.md:47-58`, `probe-framed-output.txt:38-39` |
| A plain JSON line written after the switch to frames makes the core close the connection. | `probe-framed-output.txt:460-464` |
| In plain mode calls on one connection run one at a time: `terminal.list` waited about 3 s behind `terminal.wait`. In framed mode, channel 0 runs calls side by side: the later `terminal.list` was answered first. | `probe-plain-output.txt:405-409`, `probe-framed-output.txt:118-122` |
| In plain mode, after `events.subscribe` the connection silently ignores further requests. `terminal.attach` needs a framed connection (`needs_framed`). | `probe-plain-output.txt:444-448`, `probe-framed-output.txt:397` |
| A stream (`events.subscribe`, `terminal.attach`) starts with 256 KiB of credit. With no CREDIT sent, event delivery stopped at exactly 262144 bytes and resumed after CREDIT. Channel 0 uses no CREDIT: 415110 bytes of replies arrived with none sent. | `upstream-v0.1.21-schema-README.md:103`, `probe-framed-output.txt:255-258`, `probe-review-output.txt` (`== R1`) |
| When a program in an attached terminal exits, the core sends CLOSE on the attach channel with `{"exit_code":0,"finished_ms":…,"reason":"exited","signal":null}`. A CLOSE from the client gets an empty CLOSE back, and no more data on that channel; channel 0 keeps working and channel numbers are not reused. | `probe-framed-output.txt:303-304,370-371,384-385`, `probe-methods-output.txt:326-366` |
| `hello` with generation 2 or 0 fails with `-32000 unsupported_generation` ("protocol generation N not supported", hint "this core speaks generation 1"). | `probe-plain-output.txt:212,225`, `probe-framed-output.txt:102-103` |
| Closing `hc connect`'s stdin makes it exit 0 after it sends any in-flight reply. `hc connect` starts a stopped isolated core by itself. | `probe-plain-output.txt:603-623`, `probe-plain-output.txt:109-122` |
| A core that belongs to nobody else is reached by `HC_SESSION` (or `hc connect --session <name>`, the same core) together with `XDG_STATE_HOME`, `XDG_CONFIG_HOME` and `XDG_RUNTIME_DIR` pointed at a scratch directory, and every inherited `HC_*` variable removed. | `probe-plain-output.txt:2`, `probe-plain-output.txt:131-138` |
| A socket path over 103 bytes is refused on macOS (`the socket path for session … is too long here (110 bytes; this system allows 103)`). A `mktemp` directory under `/var/folders` gave 110 bytes; a short directory under `/tmp` works. | `probe-plain-output.txt:25-34,105-122` |

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
