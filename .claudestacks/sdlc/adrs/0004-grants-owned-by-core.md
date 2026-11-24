---
status: superseded
created: 2026-11-22
sealed: b33ca3358102
---

# ADR 0004: Grants owned by herdrify-core, including an hc grant

## Context

Every component declares grants in its manifest. `~/.herdrify/config.toml` caps them (the ceiling),
and the person approves what remains. All airsl citations below are to airsl 0.1.4, in
`evidence/airsl-0.1.4-excerpts.md`.

**airsl's grant model**

- `GrantSet` has three grant axes: `fs`, `env` and `proc` (`src/sandbox/grant_set.rs:54-59`). It is
  `#[non_exhaustive]` (`:53`).
- A `Policy` is built from those axes with `with_fs`, `with_env` and `with_proc`, and `fs` takes
  read and write roots (`src/sandbox/grant_set.rs:43-50`).
- airsl accepts host modules from other crates (`src/modules/registry.rs:1-6`) under a root table
  the host chooses (`src/builder.rs:78-86`).
- A module reads its grants at install time and checks them when a function is called:
  `InstallContext::grants` (`src/modules/registry.rs:52`) and the pattern described at `:103-105`.
- airsl says the authority a module enforces should be the engine's own object, because "A module
  built with its own copy of the grants could disagree with the engine"
  (`src/modules/registry.rs:21-27`).

**How `proc` matching works.** `proc` matches "on the program name as written, not on a resolved
path". The airsl docs add that such an allowlist "is only as strong as the write grants sitting
alongside it" (`src/sandbox/grants.rs:221-226`). The check itself is `self.executables.contains(program)`
(`:255-257`).

**airsl's extension system.** airsl ships its own extension machinery:
- an `extension.toml` manifest (`src/extension/manifest.rs:40`)
- `Ceiling` (`src/extension/ceiling.rs:18`)
- `negotiate`, which "Intersects `manifest`'s request with `ceiling`", turning required misses into
  denials and optional misses into reductions (`src/extension/negotiate.rs:269-275`)
- `Approver` (`src/extension/approver.rs:68`)

**Components need herdr-core, and airsl has no axis for it.** Granting the `hc` program through
`proc` would grant everything:
- `hc exec` runs any program, and `hc file read|write|edit` touches any file
  (`evidence/hc-0.1.21-help.txt:27,29`).
- The RPC API has 32 methods (`evidence/probe-facts-output.txt:7-39`), including `machine.exec` and
  `file.write` (`:27,29`).
- `terminal.run` alone has several ways to start an arbitrary program (`:12`, with descriptions in
  `evidence/hc-0.1.21-schema.json`):
  - `line`, a shell line instead of argv
  - no argv at all, which starts that machine's shell
  - an argv[0] that names a template
  - `env`
  - `keep`, which replaces a lost program with a shell
  - `on`
- herdr-app can also start programs, through `terminal.open {argv}` (AGENTS.md:616, in
  `evidence/herdr-app-0.1.16-agents-excerpts.md`).

## Options considered

1. **herdrify-core owns the grant model.**
   - Core defines its own `Grant` kinds (`fs`, `env`, `proc`, `hc`), a `Ceiling` and the
     intersection.
   - The lua adapter maps `fs`, `env` and `proc` onto an airsl `Policy`. It enforces `hc` in
     herdrify's own host module.
   - Pros: grant logic is tested without Lua, and new kinds are possible.
   - Cons: core duplicates the intersection airsl has, and enforcement happens in two places.
2. **airsl is the source of truth.** `component.toml` is translated to airsl's manifest, and
   `negotiate` and `Approver` do the work.
   - Pros: less code.
   - Cons:
     - airsl 0.1.4 has no axis for `hc`
     - core would depend on airsl, against ADR 0001, or the logic would live only in the adapter,
       where testing it needs Lua
3. **Extend airsl upstream with host-defined grant kinds, then use option 2.**
   - Pros: one model.
   - Cons: herdrify's releases wait on airsl changes, and airsl gains host-specific concepts.

## Decision

Option 1. `herdrify-core` defines four grant kinds: `fs`, `env`, `proc` and `hc`.

**The `hc` grant**

```toml
[grants.hc]
read = "own"              # "own" or "all"
run  = ["cargo", "npm"]   # programs terminal.run may start, by exact argv[0]
send = "own"              # only "own"
```

An absent part grants nothing. Every one of the 32 RPC methods falls in exactly one row below.

| Group | Methods | Rule |
|---|---|---|
| `read` | `terminal.list`, `terminal.get`, `terminal.read`, `terminal.wait`, `events.subscribe`, `templates.list` | `"own"`: only terminals this component started. `"all"`: every terminal. `doc` events are dropped from `events.subscribe` before they reach Lua. |
| `run` | `terminal.run` | See the `terminal.run` rules below. |
| own lifecycle | `terminal.kill`, `terminal.rm` | Allowed with `run`, on terminals this component started only. |
| `send` | `terminal.send`, `terminal.keys`, `terminal.interrupt` | On terminals this component started only. |
| internal | `hello`, `rpc.discover` | Used by the adapter itself, never by components. |
| never exposed | `server.*` (3), `machine.*` (6), `file.*` (3), `doc.*` (2), `terminal.put`, `terminal.keep`, `terminal.label`, `terminal.attach` | 18 methods. |

That is 6 + 1 + 2 + 3 + 2 + 18 = 32.

**`terminal.run` rules**

- A component may set only `argv`, `cwd`, `name`, `size`, `timeout` and `wait`.
- `argv` must be non-empty.
- `argv[0]` must exactly equal an entry in `run`, and must not name a template. herdrify checks
  `templates.list` before each call.
- `cwd` must lie inside the component's approved `fs` read roots.
- herdrify sets `labels` itself, to the ownership label that "own" checks. Components cannot set,
  change or remove labels, because `terminal.label` is never exposed.
- Every other parameter is refused: `line`, `env`, `keep`, `on`, `parent`, `outlive_parent`,
  `die_with_parent`, `display`, `rm`, `no_notify`, `no_parent`, `caller`.

**`proc` entries**

- An entry must be a bare program name, containing no `/` or `\`.
- An entry is refused if its lowercased name is `hc` or `herdr-app`.
- These rules apply in a component manifest and in the ceiling.

**Ceiling and approval**

- The ceiling in `~/.herdrify/config.toml` caps every kind: list kinds by inclusion, `read` by
  `"own"` < `"all"`.
- A request that exceeds the ceiling makes the plan refuse. The refusal names the grant and the
  ceiling's value. Nothing is silently reduced.
- Core computes this check. The `Approver` port then asks the person (ADR 0001).

**Enforcement**

- The lua adapter builds an airsl `Policy` from the approved `fs`, `env` and `proc`.
- The `herdrify.hc` host module is built holding the approved `hc` grant, and checks it on every
  call.

## Consequences

**Benefits**

- Every grant rule is tested in core with fakes: parsing, the ceiling, the method table, the
  `terminal.run` rules and the `proc` checks.
- Components get herdr-core access through a closed method table, with no path to a shell through
  `terminal.run`.

**Costs**

- **Duplicated logic.** Core re-implements intersection logic that airsl's `negotiate` already has.
- **The `hc` grant lives outside airsl.** Two enforcement points must agree: airsl for `fs`, `env`
  and `proc`, and herdrify's module for `hc`. The `hc` grant reaches its module as a separate copy,
  which is exactly what airsl's design warns against (`src/modules/registry.rs:21-27`). The
  adapter's tests must keep the two in step.
- **`read = "all"` is broad.** It exposes every terminal's screen and history, including the
  person's own and anything secret on them. The approval must say so plainly.
- **Some allowed programs run arbitrary code anyway.** Shells and interpreters do, and so do tools
  such as `cargo` and `npm`, which run build scripts. This holds for `proc` entries and for `run`
  entries alike, and herdrify does not judge it beyond showing the entry at approval.
- **PATH shadowing.** Program names are matched as written (`src/sandbox/grants.rs:223-226`). An
  `fs` write grant over a directory on `PATH` lets a component plant its own copy of any granted
  program. The approval shows the write grant; herdrify does not cross-check it against `PATH`.
