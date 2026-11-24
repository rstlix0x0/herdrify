# Herdr and airsl: facts herdrify builds on

These facts were observed on 2026-11-22; the `herdrify-app` section below on 2026-11-23, and the
`hc` 0.1.22 rows on 2026-11-24. The raw output behind each row is in
[`.claudestacks/sdlc/adrs/evidence/`](../.claudestacks/sdlc/adrs/evidence/), except where a row
names the evidence folder of its own chain. Probe again before relying on a fact for a newer
version.

| Tool | Version |
|---|---|
| hc (herdr-core) | 0.1.22 (0.1.21 for the facts observed before it) |
| herdr-app | 0.1.16 (the herdr-app table), 0.1.17 (the `herdrify-app` section) |
| airsl | 0.1.4 (mlua 0.12.2) |
| tokio (probe) | 1.54.1 |

## herdr-core (`hc`)

| Fact | Evidence |
|---|---|
| `hc schema` is an OpenRPC 1.3.2 document. On 0.1.22: 36 methods, 39 schemas (on 0.1.21: 32 methods, 36 schemas). | `.claudestacks/sdlc/2026-11-24-herdrify-hc-0-1-22/evidence/method-counts.txt:1-3`, `.claudestacks/sdlc/2026-11-24-herdrify-hc-0-1-22/evidence/schema-counts.txt:1-5` |
| JSON-RPC 2.0, one message per line, or framed after `hello`. Protocol generation 1. Errors carry a stable `data.kind`. | `probe-facts-output.txt:6` |
| `hc connect` serves the API over stdio; the remote form is `ssh box hc connect` | `hc-0.1.21-help.txt:41` |
| Streams: `events.subscribe` and `terminal.attach` | `probe-facts-output.txt:21,33` |
| `hc exec`, `hc file`, `machine.exec` and `file.write` reach any program and any file | `hc-0.1.21-help.txt:27,29` |

### Using `hc connect` from a program (`herdrify-hc`)

`herdrify-hc` 0.2.0 is the async client built on these facts, observed against throwaway cores.
The put in pieces and the `docs` filter rows were observed on `hc` 0.1.22 (the last two rows); the
rest were observed on `hc` 0.1.21. The raw output behind each row is in
[`.claudestacks/sdlc/2026-11-22-herdrify-hc/evidence/`](../.claudestacks/sdlc/2026-11-22-herdrify-hc/evidence/),
and the Evidence column names the file inside it. The rows that carry a full path, the
`hc` 0.1.22 ones, are in
[`.claudestacks/sdlc/2026-11-24-herdrify-hc-0-1-22/evidence/`](../.claudestacks/sdlc/2026-11-24-herdrify-hc-0-1-22/evidence/).

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
| The put in pieces (`terminal.put_begin`, `put_chunk`, `put_finish`, `put_abort`) works on this core's terminals only: `nowhere/t1` and `local/t1` fail with `machine_unknown` and the hint "this core only runs local terminals". A transfer id is 16 to 64 hex digits (15 and 65 are refused). A piece is at most 1 MiB and a put at most 8 MiB (one byte more is refused). | `.claudestacks/sdlc/2026-11-24-herdrify-hc-0-1-22/evidence/probe-022-extra-output.txt:15,19`, `.claudestacks/sdlc/2026-11-24-herdrify-hc-0-1-22/evidence/probe-022-output.txt:39-73,128-133` |
| The `docs` filter on `events.subscribe` needs 0.1.22. On 0.1.22 `{"docs":["probe-doc"]}` delivers only that document's events. 0.1.21 accepts `docs` and ignores it: it delivered every doc event and the terminal events. | `.claudestacks/sdlc/2026-11-24-herdrify-hc-0-1-22/evidence/probe-022-output.txt:168-181`, `.claudestacks/sdlc/2026-11-24-herdrify-hc-0-1-22/evidence/probe-021-output.txt:159-175` |

## herdr-app

| Fact | Evidence |
|---|---|
| Home is `~/.config/herdr-app`; `HERDR_APP_HOME` picks another | `herdr-app-0.1.16-help.txt:19` |
| 75 commands on 0.1.16 (77 on 0.1.17); each arg has only `name` and `required`, with no types | `probe-facts-output.txt:42-44`; 0.1.17: `.claudestacks/sdlc/2026-11-23-herdrify-app/evidence/probe-cli-output.txt:9` |
| `herdr.d.ts` types `run(name, args?: Record<string, unknown>)`, so command args are untyped there too | `herdr-app-0.1.16-dts-excerpts.md` |
| 4 commands take `argv`: `space.new`, `tab.new`, `pane.split`, `terminal.open` | `probe-facts-output.txt:45` |
| Mods are trusted JS with unrestricted commands and network | AGENTS.md:1140 |
| A layout block's content is `{ "mod", "props" }`; the shipped `toast` mod runs the command named in its props | AGENTS.md:224, `mods/toast/main.js:27` |
| `keys.json` sections: `global`, `layout:<name>`, `terminal`, `mod:<name>`; the more specific one wins | AGENTS.md:523-525 |
| Any edit makes `keys.json` the person's, so default shortcut upgrades stop | AGENTS.md:510-512 |
| Shipped layouts and mods are read-only and replaced on every start | AGENTS.md:188-193 |

AGENTS.md lines are in `herdr-app-0.1.16-agents-excerpts.md`.

### Using the `herdr-app` CLI from a program (`herdrify-app`)

`herdrify-app` 0.1.0 is the async client built on these facts. They were observed on `herdr-app`
0.1.17, against isolated homes and isolated windows. The raw output behind each row is in
[`.claudestacks/sdlc/2026-11-23-herdrify-app/evidence/`](../.claudestacks/sdlc/2026-11-23-herdrify-app/evidence/),
and the Evidence column names the file inside it.

| Fact | Evidence |
|---|---|
| F1. `herdr-app --version` prints `herdr-app 0.1.17\n` and exits 0 | `probe-cli-output.txt:1-5` |
| F2. `herdr-app commands` prints a JSON array of 77 objects with exactly the keys `about`, `args` and `name`. Each arg has exactly `name` and `required`: 100 args, 25 required, 77 unique names. It needs no window, and its output is identical with an empty `HERDR_APP_HOME`, with the author's home, and to the saved fixture | `probe-cli-output.txt:7-12` |
| F3. With no window, `ls`, `status` and `do` exit 2 with stderr `herdr-app: no Herdr window at <home>/herdr-app.sock: <reason>\n`. A home too long for a socket gives `path must be shorter than SUN_LEN` | `probe-cli-output.txt:14-43,56-61` |
| F4. `do` args that are not JSON exit 2 with `herdr-app: args are not JSON: …`, before any window is contacted. A JSON value that is not an object exits 2 with ``herdr-app: expected --<arg>, got "[1]"``. A bare `do` exits 2 with the usage line | `probe-cli-output.txt:44-55`, `probe-window-output.txt:170-174` |
| F5. Unknown subcommands and flags exit 2 with ``herdr-app: unknown argument "…"`` and the help | `probe-cli-output.txt:62-79` |
| F6. The CLI writes nothing into an empty home | `probe-cli-output.txt:80` |
| F7. `herdr-app` with no arguments, given an isolated home, launches a second window process beside the person's. It answered `status` within 0.6–3.2 s | `probe-window-output.txt:4-6`, `probe-commands-output.txt:3,728`, `probe-gaps-output.txt:2`, `probe-views-output.txt:3` |
| F8. The person's window process (`Herdr`) has `herdr-core connect` as a child (`ps-window-core-output.txt:1-2`). With `HC_SESSION` and the XDG variables set, the isolated window's terminals land in the isolated core and never in the person's | `probe-window-output.txt:313,315` |
| F9. `do` success exits 0 with a JSON object: `{"ok":true}` for most commands, else a command-specific shape | `probe-commands-output.txt:729-831` |
| F10. A refusal exits 1 with `{"error": "<text>"}` on stdout and empty stderr | `probe-window-output.txt:109-168` |
| F11. Both the JSON-object form and the flag form work, and `{}` works for commands with no arguments | `probe-window-output.txt:47-81` |
| F12. herdr-app checks argument types inconsistently: many wrong-typed values are silently accepted (`space.new` takes `argv: 7`), others are refused | `probe-commands-output.txt` (`T` lines, 4-724) |
| F13. `ls` prints `{spaces:[{id,name,folder,machine,active,tabs:[{id,name,active,settled,terminals:[{terminal,pane,status,app,name,sub_agents}]}]}],recent:[{name,folder}]}`. `sub_agents` holds terminal addresses | `probe-window-output.txt:186-291`, `probe-views-output.txt:9-47` |
| F14. `status` prints `{zoom,font_size,ui_font_size,space,tab,focused_pane,layout,agent,floats,overlays,errors}`. `floats` maps a node id to `{edge,align,vertical,dragging,hovered}` (`probe-views-output.txt:98-104`). `overlays` and `errors` hold strings. A broken `layouts/bad.json` is not reported | `probe-window-output.txt:8-26`, `probe-views-output.txt:48-146` |
| F15. After `window.close`, every call answers exit 1 `{"error":"the window did not answer"}`, `app.quit` included, and the process does not exit | `probe-commands-output.txt:831-834` |
| F16. `do app.quit` makes the window process exit 0. SIGTERM makes it exit by signal 15. Afterwards `status` exits 2 | `probe-window-output.txt:317-334`, `probe-gaps-output.txt:47-49` |
| F17. A fresh window shows the welcome overlay, with null `space`, `tab`, `focused_pane` and `agent` | `probe-window-output.txt:8-26` |

## airsl

| Fact | Evidence |
|---|---|
| Synchronous: `eval` and `eval_to` are plain `fn`s; no `async fn`; no tokio in the tree | `airsl-0.1.4-excerpts.md` |
| `GrantSet` has three axes: `fs`, `env`, `proc` | `src/sandbox/grant_set.rs:54-59` |
| `proc` matches the program name as written, not a resolved path | `src/sandbox/grants.rs:221-226` |
| Third-party host modules, under a host-chosen root table | `src/modules/registry.rs:1-6`, `src/builder.rs:78-86` |
| Ships its own extension manifest, `Ceiling`, `negotiate` and `Approver`; herdrify does not use them (ADR 0008) | `src/extension/*` |
| `block_on` inside `spawn_blocking` works; on an async worker it panics | `probe-bridge-output.txt:2,7` |

airsl source lines are in `airsl-0.1.4-excerpts.md`.
