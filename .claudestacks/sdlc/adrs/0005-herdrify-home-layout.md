---
status: approved
created: 2026-11-22
sealed: 1a36ec0fff7d
---

# ADR 0005: The ~/.herdrify home layout

## Context

herdrify has to keep:

- the person's configuration and grant ceiling (ADR 0004)
- fetched sources
- installed components
- per-component writable data
- a lock recording what each install did, so `remove` can undo it exactly (`README.md:75`)

The README promises "Everything herdrify keeps lives in `~/.herdrify`" and sketches `config.toml`,
`cache/` and `herdrify.lock` there (`README.md:83-90`). The author wants the home organised the way
vim organises plugins: one directory per installed package.

Several forces shape the layout.

- **Generated files live in Herdr's home, not herdrify's.** Herdr takes layouts, keys, mods and
  themes from its own home (AGENTS.md:78-81, 333-338, in
  `evidence/herdr-app-0.1.16-agents-excerpts.md`). That home is `~/.config/herdr-app` unless
  `HERDR_APP_HOME` says otherwise (`evidence/herdr-app-0.1.16-help.txt:19`). The lock must record
  those files.
- **One generated file can have many contributors.** A generated layout comes from a frame plus
  several components. herdrify's keys sit in sections of a `keys.json` that the person also edits
  (ADR 0003). So "what one component wrote" is not always a whole file.
- **Authors need a development loop.** `herdrify dev` (ADR 0007) runs a component straight from its
  working folder.
- **Updates should be reversible**, so a bad update can be rolled back.
- **Tests need a relocatable home** (ADR 0001).

## Options considered

1. **One vim-like home.** Everything herdrify stores lives under one directory, and every kind of
   component shares one `components/` tree. A component's kind comes from its manifest.
   - Pros: herdrify's stored state is in one place, and each component's kind is recorded once.
   - Cons: it ignores the XDG split.
2. **XDG split.** Config in `~/.config/herdrify`, data in `~/.local/share/herdrify`, cache in
   `~/.cache/herdrify`.
   - Pros: matches platform conventions.
   - Cons: three places to find, back up and clean, and it breaks the README's promise.
3. **Directories by kind** (`themes/`, `frames/`, `components/`).
   - Pros: the kinds show in a listing.
   - Cons: the kind is stated twice, in the manifest and in the path, and the two can disagree.

## Decision

Option 1, in this shape. It supersedes the README's sketch, which has no `components/`, `dev/` or
`state/` and keys `cache/` by commit.

```
$HERDRIFY_HOME (default ~/.herdrify)
├─ config.toml        edited by the person: active frame and theme, git sources,
│                     grant ceiling, enabled components. The ONLY record of intent.
├─ herdrify.lock      written by herdrify: what is installed and what herdrify wrote
├─ cache/git/<host>/<repo-path>/      fetched repositories
├─ components/<name>@<commit>/        installed, immutable, any kind
├─ dev/<name>                         symlinks to local components under development
└─ state/<name>/                      writable data for that component
```

**The lock** has two parts.

- **One entry per component and bundle.** It records:
  - the source (a git URL and path, or `dev`)
  - the commit (absent for `dev`)
  - the approved grants
  - what the entry contributes to generated files: layout blocks, its theme, its key bindings
- **One `[generated]` table.** It records:
  - Herdr's absolute home path
  - every whole file herdrify created there, with its hash: generated layouts, generated themes,
    `mods/herdrify-host/`
  - every `keys.json` section herdrify owns, with the hash of that section

**Regeneration.** Generated files belong to herdrify as a whole, never to one component. After any
`add` or `remove`, herdrify regenerates them from the remaining installed set. A file or section
that is no longer needed is deleted. If its current hash no longer matches the lock, the person
edited it, and `remove` refuses, naming the path.

**Updates.** An installed component is never modified. An update installs `<name>@<newcommit>`
beside the old one and switches the lock entry. `herdrify gc` deletes `components/<name>@<commit>`
directories that no lock entry names.

**Removal.** `remove` deletes the lock entry and regenerates. It keeps `state/<name>/` so a
reinstall finds its data. `remove --purge` also deletes `state/<name>/`.

**Development.** A `dev/<name>` symlink is installed through the normal plan and approval, with
source `dev`. It is refused when its name matches an installed component.

**Sources.** Only git repositories. Local folders come in through `dev/`.

**Relocation.** `$HERDRIFY_HOME` overrides the default location, the same way `HERDR_APP_HOME`
does for Herdr.

## Consequences

**Benefits**

- All of herdrify's stored state lives in one directory.
- Rolling back means pointing the lock at the previous `<name>@<commit>`.
- `remove` works for shared files too, because it regenerates instead of subtracting.
- Adapter tests run against a temporary `$HERDRIFY_HOME`.
- `state/<name>/` is a natural target for a component's `fs` write grant.

**Costs**

- **Not XDG.** The layout does not follow XDG conventions.
- **Deleting the home is not a clean uninstall.** The files herdrify generated in Herdr's home stay
  behind, and without the lock nothing can attribute them.
- **Old revisions pile up.** Superseded `<name>@<commit>` directories use disk until `herdrify gc`
  runs.
- **Edits block removal.** A person who edits a generated file or section must undo the edit, or
  delete it by hand, before `remove` will proceed.
- **The lock is critical.** A lost or damaged lock leaves Herdr-side files herdrify can no longer
  attribute.
