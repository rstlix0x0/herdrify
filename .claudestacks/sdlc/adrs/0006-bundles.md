---
status: approved
created: 2026-11-22
sealed: 1848c5581948
---

# ADR 0006: Bundles

## Context

Engineers want to share a whole setup at once: a frame, a theme, key bindings and several
components. Others should be able to install it in one step, apply it, and remove it in one step.

**How Herdr shares setups.** All AGENTS.md lines below are in
`evidence/herdr-app-0.1.16-agents-excerpts.md`. Herdr shares a layout or a mod as a folder "laid
out like this home":
- a layout brings `layouts/<name>.json` plus the `mods/<name>/` it needs
- a mod brings only `mods/<name>/`, and maybe a `keys.json` (AGENTS.md:455-459)

A mod folder must contain `main.js` (AGENTS.md:877). When a name is taken, Herdr's guide tells the
agent to rename the incoming item, rewriting "the mod's own `mods/<name>/` paths"
(AGENTS.md:468-470). That format carries JavaScript and raw Herdr JSON, which ADR 0003 refuses.

**Why herdrify cannot rename on a clash.** A component's name is load-bearing:
- it keys `components/<name>@<commit>/` and `state/<name>/` (ADR 0005)
- it keys the component's lock entry
- it appears in its generated layout blocks and key actions (ADR 0003)

Renaming would mean rewriting all of those, and the component's own Lua, which may refer to its
name.

Single components are already installed with `herdrify add <repo> --path <dir>` (`README.md:74`).

## Options considered

1. **A herdrify bundle.** A `bundle.toml` lists components, either inside the bundle folder or as
   pinned references to other repositories. A bundle holds only Lua and TOML, installs with
   `herdrify add`, and appears in the lock as one entry that owns its members.
   - Pros: a whole setup installs, applies and is removed as one unit.
   - Cons: herdrify must resolve references and reference-count shared components.
2. **No bundle concept.** Components are shared one repository at a time, and people assemble them
   by hand.
   - Pros: nothing new to build.
   - Cons: a setup cannot be shared or removed as one unit.
3. **Reuse Herdr's folder format.**
   - Pros: familiar to Herdr users.
   - Cons: it carries JavaScript and raw Herdr JSON, which ADR 0003 refuses.

## Decision

Option 1.

**What a bundle declares.** `bundle.toml` gives:
- a `name`
- a `version`, which is shown to people and recorded in the lock
- its members

A member is either a component folder inside the bundle, or a reference `{ repo, path, rev }`
pinned to a revision. A member must be a component. Bundles do not nest.

**Telling a bundle from a component.** `herdrify add <repo> --path <dir>` reads the folder at
`<dir>`:
- `bundle.toml` means a bundle
- `component.toml` means a single component
- both, or neither, is refused

**Component identity.** A component's identity is its git URL plus its path:
- for a referenced member, the reference's `repo` and `path`
- for an inline member, the bundle's repo and the member's path inside it

Only one revision of a given name is installed at a time.

**Installing.**
- `herdrify add` on a bundle produces one plan and one approval for every member's grants, in a
  single all-or-nothing step.
- Nothing is written if any of these fails:
  - a fetch
  - an ADR 0003 file-type check
  - a grant over the ceiling (ADR 0004)
  - a clash
  - the person declining
- A revision already in `cache/` installs without fetching (ADR 0005).
- A member that is already installed with the same identity and revision is shared. Its approved
  grants carry over, and only grants new to the plan are asked about.

**Applying the setup.**
- A bundle may contain at most one `frame` and at most one `theme` member.
- `add` makes them the active frame and theme in `config.toml` and shows that in the plan.
- The lock records the previous active frame and theme. `remove` restores them.
- Key bindings come from the members' `[keys]`. Two members, or a member and an installed
  component, binding the same key is refused (ADR 0003).

**Shared members.** A component used by several bundles is reference-counted in the lock.
`herdrify remove <bundle>` removes only members nothing else still uses.

**Clashes.** A member is refused, and herdrify never renames, when its name matches an installed
component with:
- a different identity, or
- a different revision

**Single components.** `herdrify add <repo> --path <dir>` with a `component.toml` keeps working.

## Consequences

**Benefits**

- A whole setup is shared, installed, applied and removed as one unit.
- Bundles can share components through pinned references.

**Costs**

- **More lock logic.** Resolution, reference counting and restoring the active frame and theme all
  need tests.
- **One revision per name.** Two bundles that pin different revisions of the same component cannot
  both be installed, and neither can two bundles that ship different components with the same name.
- **No raw Herdr content.** Bundle authors cannot ship Herdr layouts, mods or `keys.json`.
- **References can break.** A pinned reference whose repository disappears blocks installing the
  bundle on any machine that does not have that revision cached.
- **One frame and one theme.** A bundle cannot offer the person a choice of frames or themes.
