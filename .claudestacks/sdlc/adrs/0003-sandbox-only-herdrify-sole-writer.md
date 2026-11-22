---
status: approved
created: 2026-11-22
sealed: 9e14ef8ea9c9
---

# ADR 0003: Sandbox only, with herdrify as the sole writer of its Herdr files

## Context

herdrify's promise is that a component gets only the grants its manifest declares and the person
approved. Herdr gives a shared folder three ways around that promise. All AGENTS.md and
`mods/toast/main.js` lines cited below are in `evidence/herdr-app-0.1.16-agents-excerpts.md`.

**Herdr runs JavaScript mods as trusted code.**
- "Mods are trusted code: commands and network access (including `fetch`) are unrestricted"
  (AGENTS.md:1140).
- A mod's `exec` runs in "the user's real environment" (AGENTS.md:1102).

**Herdr's plain JSON files can start programs.**
- A `keys.json` action can be:
  - `send:<text>`, which "types into the terminal"
  - a command object with arbitrary `args` (AGENTS.md:526-534)
- Exactly four of the 75 herdr-app commands take an `argv`: `space.new`, `tab.new`, `pane.split`
  and `terminal.open` (`evidence/probe-facts-output.txt:45`; AGENTS.md:571,580,596,616).

**A layout block names a mod and passes it props.** A block's content is
`{ "mod": "<folder>", "props": {...} }` (AGENTS.md:224). Herdr's own shipped mods can act on those
props. For example, the shipped `toast` mod runs whatever command its props name:
`if (p.action?.name) run(p.action.name, p.action.args ?? {});` (`mods/toast/main.js:27`).

Herdr takes these files from its home. Layouts live in `layouts/<name>.json`, keys in `keys.json`,
and mods in `mods/<name>/` (AGENTS.md:78-81). Themes live in `themes/<name>.json`
(AGENTS.md:333-338). The home is `~/.config/herdr-app`, and `HERDR_APP_HOME` selects another
(`evidence/herdr-app-0.1.16-help.txt:19`). Herdr's own layouts and mods there are read-only and are
replaced on every start (AGENTS.md:188-193).

A bundle that could ship JavaScript, a `keys.json`, or a layout passing props to a Herdr mod could
run any program and bypass every grant.

## Options considered

1. **One sandboxed level, with herdrify as the sole writer.**
   - Bundles and components may contain only an allowlist of file types.
   - herdrify generates every file it puts in Herdr's home.
   - Pros: one unconditional promise for everything a person installs, and validation covers only
     herdrify's own formats.
   - Cons: engineers cannot ship their own JavaScript, assets or raw Herdr files.
2. **Two trust levels.** Sandboxed bundles, plus "trusted" bundles that may ship JavaScript after a
   warning.
   - Pros: the most flexibility.
   - Cons: the promise becomes conditional, and its strength rests on a click-through warning.
3. **Accept raw Herdr JSON after validation.** Bundles may ship `layouts/*.json` and `keys.json`,
   and herdrify checks every action, argument and prop against an allowlist.
   - Pros: authors reuse Herdr's formats directly.
   - Cons: the validator would need to know:
     - every command and argument that can start a program, across 75 commands
       (`evidence/probe-facts-output.txt:42`)
     - every prop every Herdr mod acts on

     It would also have to keep up as Herdr changes, and one miss is an escape.

## Decision

Option 1.

**What a bundle or component may contain.**
- Only these files: `*.lua`, `*.toml`, `*.md`, and files named `LICENSE` or `LICENSE.*`.
- Any other file is refused at plan time, with the file's path in the message.

**What runs as trusted code.** Two things, and nothing a person installs through herdrify:
- `herdrify-host`, herdrify's own host mod, which is the only JavaScript herdrify installs
- the allowlisted Herdr mods named below

**What herdrify writes in Herdr's home.** herdrify is the only writer of the files and sections it
creates there, all generated from Lua and TOML.

Herdr's home is `HERDR_APP_HOME` when it is set, else `~/.config/herdr-app`. The lock records the
absolute path (ADR 0005).

- **Layouts** are generated from frame components. A block in a generated layout names either:
  - `herdrify-host`, or
  - a mod from a fixed allowlist of Herdr's shipped mods, always with empty props

  The allowlist lives in `herdrify-core`, versioned with the herdr-app versions it supports. A mod
  joins it only after its source has been read and shown to start nothing from absent props.
- **Themes** are generated from theme components, as `themes/<name>.json` in Herdr's theme shape
  (AGENTS.md:333-338).
- **Keys** come from the `[keys]` tables of component manifests.
  - herdrify writes them only into `keys.json` sections it owns. Herdr's sections are `global`,
    `layout:<name>`, `terminal` and `mod:<name>` (AGENTS.md:523-525). herdrify owns
    `layout:herdrify-<frame>` and `mod:herdrify-host`.
  - A key may bind only to:
    - `herdrify-host.<command>`, which is delivered to the host mod
    - `layout.toggle`, `layout.show` or `layout.hide` (one argument, `id`;
      `evidence/herdr-app-0.1.16-commands.json:589,599,609`) on a block herdrify generated
  - `send:` actions and commands that take an `argv` are refused.
  - Two components binding the same key is refused at plan time.

**Commands.** `herdrify install` is dropped. It installed "a plain Herdr bundle, as it is"
(`README.md:76`), meaning raw layouts, mods and keys.

## Consequences

**Benefits**

- Every component and bundle a person installs runs sandboxed, with no exception. The only trusted
  code is herdrify's own host and the vetted Herdr mods.
- Plan-time validation is a file-type allowlist plus herdrify's own Lua and TOML formats.

**Costs**

- **Limited flexibility for authors.**
  - Engineers cannot add widgets, JavaScript or assets. How far they can go is set by
    `herdrify-host`'s widget set and the UI-tree contract.
  - Frames can use only the allowlisted Herdr mods, with no props.
- **Ongoing review work.** Each herdr-app version herdrify supports needs those mods' sources read
  again.
- **Herdr's default shortcuts stop updating.** Herdr treats `keys.json` as the person's file once
  anything edits it: "Any edit, even whitespace, keeps the entire file as yours; new default
  shortcuts must then be added by hand" (AGENTS.md:510-512).
  - The first time herdrify writes keys, that file stops getting Herdr's new default shortcuts.
  - `herdrify remove` deletes herdrify's sections but does not try to restore the file's untouched
    state.
- **herdrify's keys win inside its layout.** A more specific section beats `global`
  (AGENTS.md:524-525), so while a herdrify layout shows, its keys override the person's global keys.
- **Raw Herdr content goes elsewhere.** People install raw Herdr mods and layouts the Herdr way,
  outside herdrify.
- **Format tracking.** herdrify must follow Herdr's layout, theme and keys formats through
  `herdrify-app`'s models (ADR 0002).
