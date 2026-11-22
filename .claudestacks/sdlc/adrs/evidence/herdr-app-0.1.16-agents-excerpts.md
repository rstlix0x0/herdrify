# herdr-app 0.1.16: AGENTS.md excerpts

Source: `~/.config/herdr-app/AGENTS.md`, which herdr-app 0.1.16 writes on every start (its line 3 says so). Each block is the raw output of the command shown, run on 2026-11-22. The command prefixes each line with its line number in that file.

~~~~text
$ awk 'NR>=3 && NR<=3 {print NR": "$0}' AGENTS.md
3: <!-- Written by the Herdr app on every start. Edits here are overwritten. -->
~~~~

~~~~text
$ awk 'NR>=33 && NR<=37 {print NR": "$0}' AGENTS.md
33: fetch URLs, keep state per mod or shared between mods, animate natively.
34: 
35: You can't: use HTML, CSS, a web view, Node APIs or `setTimeout`; draw
36: outside your block except through overlays; change how a terminal itself
37: draws text; read or change other mods' code at run time.
~~~~

~~~~text
$ awk 'NR>=44 && NR<=47 {print NR": "$0}' AGENTS.md
44: https://gpui-kit.com. The Herdr app carries its own copy with additions
45: (`frames`, `loop`), so **the type files here are the truth**:
46: `gpui-kit.d.ts` for elements and `herdr.d.ts` for the app's data and
47: commands. They're rewritten on every start to match this version.
~~~~

~~~~text
$ awk 'NR>=188 && NR<=193 {print NR": "$0}' AGENTS.md
188: **Herdr's own layouts and mods are read-only.** They are replaced whole
189: on every start. Use settings.json for look (fonts, sizes, theme, colors,
190: sounds), where the layout leaves them unset. Herdr (main) has no look of
191: its own, so your settings decide it; Herdr 98 and Terminals bring their own
192: look. The theme picker overrides any layout. To change a shipped layout's
193: look beyond that, or its structure/code, clone it and edit the clone:
~~~~

~~~~text
$ awk 'NR>=194 && NR<=208 {print NR": "$0}' AGENTS.md
194: 
195: ```sh
196: herdr-app do layout.clone '{"name":"main","as":"my-layout"}'
197: herdr-app do mod.clone '{"name":"files","as":"my-files"}'
198: ```
199: 
200: The default clone name is `<name>-copy`; an existing name is refused.
201: `layout.clone` switches to the clone. In the palette: Duplicate layout
202: or Duplicate mod. Clones are yours and never updated. They record
203: `cloned_from: {name, version}` and copy the layout and its clicks as-is,
204: including its look. A cloned layout keeps its mod references:
205: clone a mod too and point your layout's `content.mod` to it to change code.
206: Mod clones rewrite their own `mods/<name>/` asset paths and self-opening
207: overlay names to the new name.
208: 
~~~~

~~~~text
$ awk 'NR>=333 && NR<=338 {print NR": "$0}' AGENTS.md
333: - `themes/<name>.json`: `{ "background", "foreground", "selection",
334:   "cursor"?, "ansi": [16 colors], "colors"?: {...} }`, all `#rrggbb`; panel
335:   colors are derived from these, `colors` overrides them. Herdr ships
336:   `atom-one-dark`. The original unedited Atom One Dark and Win98 theme files
337:   are upgraded to readable terminal colors; edited theme files are kept.
338:   A theme named like a layout (`themes/win98.json`) is
~~~~

~~~~text
$ awk 'NR>=453 && NR<=474 {print NR": "$0}' AGENTS.md
453: ### Layouts and mods other people made
454: 
455: People share layouts at herdr.dev/layouts and mods at herdr.dev/mods. Each
456: is a folder laid out like this home: a layout has `layouts/<name>.json` and
457: the `mods/<name>/` it needs; a mod is a piece for the person's own window,
458: so it has only `mods/<name>/` and maybe a `keys.json` with the keys it
459: expects. Both have a `RECIPE.md` (the brief to build it from scratch) and a
460: `preview.png`. The files are one build of the recipe. Asked to add one
461: (`herdr.dev/layouts/<owner>/<name>`, `herdr.dev/mods/<owner>/<name>`, or a
462: folder on GitHub):
463: 
464: 1. Read the recipe and every file before writing anything. Before adding,
465:    tell the person if a mod reaches the network, runs programs (`exec`,
466:    `hc`) or reads files outside its own folder.
467: 2. Copy the layout file to `layouts/` and each mod folder to `mods/`. Leave
468:    the recipe and preview out. Never overwrite: if a name is taken (yours
469:    or Herdr's), rename the new one: its file or folder, the `mod` names in
470:    the layout, and the mod's own `mods/<name>/` paths.
471: 3. Fit it to this setup only where its assumptions don't hold (a missing
472:    program, a name clash) or the person asks. If the files don't work on
473:    this version, or the person wants it different, build it from the
474:    recipe instead.
~~~~

~~~~text
$ awk 'NR>=508 && NR<=513 {print NR": "$0}' AGENTS.md
508: ## Keys: `keys.json`
509: 
510: On upgrade, an untouched keys file from an earlier release gets the current
511: shortcuts. Any edit, even whitespace, keeps the entire file as yours; new default
512: shortcuts must then be added by hand.
513: 
~~~~

~~~~text
$ awk 'NR>=524 && NR<=534 {print NR": "$0}' AGENTS.md
524:   `terminal` (a terminal has focus), `mod:<name>`. The more specific section
525:   wins: `layout:terminals` over `global`, `terminal` over both.
526: - An action is a command (`name`, or `name:value`: the value fills the
527:   command's first argument), `send:<text>` (types into the terminal),
528:   `<mod>.<command>` (delivered to that mod's `nextCommand()`), or `terminal`
529:   (the program gets the key).
530: - Named, typed command arguments use an object instead of a string:
531:   `"ctrl-shift-r": {"command": "layout.resize", "args": {"id": "sidebar", "px": 240}}`.
532:   `command` names an app command; `args` is an object (optional, defaults to `{}`).
533:   It can pass several arguments, numbers, booleans, arrays or nested objects.
534:   String actions keep their literal first-argument shorthand, including numeric
~~~~

~~~~text
$ awk 'NR>=571 && NR<=571 {print NR": "$0}' AGENTS.md
571: - Spaces: `space.new {name?, folder?, tab?, argv?, task?, machine?, focus?}`
~~~~

~~~~text
$ awk 'NR>=580 && NR<=581 {print NR": "$0}' AGENTS.md
580: - Tabs: `tab.new {name?, space?, folder?, argv?, task?, focus?}` (argv: an
581:   agent to run, task: its first message). Agents run in your interactive
~~~~

~~~~text
$ awk 'NR>=596 && NR<=597 {print NR": "$0}' AGENTS.md
596: - Panes: `pane.split {direction: "right" | "down", pane?, argv? | mod?, props?}` (a new
597:   terminal, or a mod pane), `pane.close {pane?}`,
~~~~

~~~~text
$ awk 'NR>=616 && NR<=617 {print NR": "$0}' AGENTS.md
616: - Terminals: `terminal.open {argv?, folder?, place?, name?}` (`folder` sets the
617:   starting directory for tabs and right/down splits, with or without `argv`), `terminal.kill {terminal}`,
~~~~

~~~~text
$ awk 'NR>=873 && NR<=880 {print NR": "$0}' AGENTS.md
873: ## Mods: `mods/<name>/`
874: 
875: ```
876: ~/.config/herdr-app/mods/<name>/
877:   main.js       the mod (required)
878:   mod.json    where it first appears: { "place": "right" | "left" | "top" | "bottom" | "none", "size": 320 }
879:   *.js, *.svg   more modules ("./ui.js"), its own icons
880: ```
~~~~

~~~~text
$ awk 'NR>=1061 && NR<=1072 {print NR": "$0}' AGENTS.md
1061:   machine, branch), `machines()` (name, null for this computer; layout:
1062:   `loading`, `ready`, or `unavailable` on this connection; loading ends on
1063:   the first read even if it fails, and offline machines are unavailable),
1064:   `agents()` (status, app, space, tab, pinned, focused; "done" turns "idle"
1065:   once someone looks at it on any Herdr client), `terminals()`
1066:   (every core terminal), `focused()`, `layout()`, `theme()` (every color by
1067:   name: background (the terminals', cards), canvas (behind cards), chrome
1068:   (sidebars, bars), surface, popover, raised, hover, selected, border,
1069:   input (a stronger border), foreground, muted, dim, accent,
1070:   accent_foreground, solid and solid_foreground (a primary button),
1071:   selection, working, blocked, done, idle, unknown, ansi0..15; light and
1072:   dark themes name the same ones), `settings()` (with `dark`: the
~~~~

~~~~text
$ awk 'NR>=1100 && NR<=1106 {print NR": "$0}' AGENTS.md
1100:   (selecting text), so give it a mod block to drag by, a title bar:
1101:   `openOverlay({ kind: "window", key: "t7", width: 600, height: 400, layout: { type: "column", children: [{ type: "block", size: { px: 28 }, content: { mod: "my-titlebar" } }, { type: "block", content: { terminal: "t7" } }] } })`.
1102: - Machine: `exec(cmd, args, { cwd, on, timeout })` (the user's real environment; use this,
1103:   not `process.run`; `on: space.machine` runs there, omitted means local).
1104:   `timeout` is milliseconds, defaults to 30,000 and is capped at 600,000 (10 minutes),
1105:   locally and remotely. It rejects on timeout, if the program cannot start or the machine cannot be reached.
1106:   `hc(args)` (`hc read <t>` gives a terminal's text),
~~~~

~~~~text
$ awk 'NR>=1140 && NR<=1142 {print NR": "$0}' AGENTS.md
1140: Mods are trusted code: commands and network access (including `fetch`) are unrestricted.
1141: Other built-ins: `localStorage` (per mod copy), `console.log`, `fetch`,
1142: `import * as fs from "fs/promises"` (read in home, write under `~/.config/herdr-app`).
~~~~


## Added after review

~~~~text
$ awk 'NR>=74 && NR<=90 {print NR": "$0}' AGENTS.md
74: ## The files
75: 
76: | File | What it decides | Applied |
77: |---|---|---|
78: | `layouts/<name>.json` | the window and its whole look: blocks, theme, fonts, backgrounds, pane frames; one file per layout (`main`, `terminals`, `win98`, any you add) | live |
79: | `keys.json` | what every key does | live |
80: | `settings.json` | the person: zoom, sounds, fonts for every layout, worktree directory | live |
81: | `mods/<name>/` | panels: small JavaScript programs | live (hot reload) |
82: | `programs.json` | how programs show (name, icon, tile), on top of the built-in table | live |
83: | `icons/` | the shipped program icons | written by Herdr (put your own elsewhere) |
84: | `frame/` | the app's own screens (welcome, ...) | rewritten by Herdr on every start: don't edit, it won't stick |
85: | `view.json` | this screen's selected space/tab/pane, zoom, settled tabs, recent spaces and seen attention | written by Herdr |
86: | `shared.json`, `data/`, `seen.json`, `installed.json`, `window.json`, `windows.json` | mod state, bookkeeping, where the window was (size, maximized), mods' windows (spots, kept ones) | written by Herdr |
87: 
88: Shared state is saved atomically in the background, with the latest change winning.
89: If `shared.json` is invalid, Herdr logs a warning and keeps it as
90: `shared.json.corrupt-<time>` before starting empty.
~~~~

~~~~text
$ awk 'NR>=222 && NR<=226 {print NR": "$0}' AGENTS.md
222: A layout is a tree of nodes. Four types:
223: 
224: - `block`: shows content. `"content"` is `{ "mod": "<folder>", "props": {...} }`,
225:   `{ "terminal": "t7" }`, `{ "tab": "active" }` (the shown tab's
226:   terminals as it arranges them), or `{ "agent": "focused" }` (one agent's
~~~~

~~~~text
$ awk 'NR>=1142 && NR<=1143 {print NR": "$0}' AGENTS.md
1142: `import * as fs from "fs/promises"` (read in home, write under `~/.config/herdr-app`).
1143: 
~~~~

~~~~text
$ awk 'NR>=24 && NR<=28 {print NR": "$0}' mods/toast/main.js
24:       .cursor_pointer()
25:       .hover((s) => s.bg(t.hover))
26:       .on_click(() => {
27:         if (p.action?.name) run(p.action.name, p.action.args ?? {});
28:         closeOverlay(p.overlay);
~~~~
