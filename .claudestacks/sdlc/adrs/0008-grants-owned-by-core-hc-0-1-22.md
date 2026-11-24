---
status: approved
created: 2026-11-24
sealed: afef33032b82
supersedes: [0004]
---

# ADR 0008: Grants owned by herdrify-core, with the hc table for herdr-core 0.1.22

## Context

ADR 0004 decided that `herdrify-core` owns the grant model, including an `hc` grant with a closed
method table. Its table states that "every one of the 32 RPC methods falls in exactly one row"
(0004, *Decision*). That no longer holds.

**What herdr-core 0.1.22 changed.** `hc schema` now lists 36 methods
(`evidence/hc-0.1.22-method-counts.txt:2-3`). The full difference between the 0.1.21 and 0.1.22
documents is in `evidence/hc-schema-0.1.21-to-0.1.22.diff`. Its left side is herdrify-hc's
0.1.21 fixture, which is the same document as `evidence/hc-0.1.21-schema.json`. Nothing is
removed. Two additions bear on grants:

- **Four new methods:** `terminal.put_begin`, `terminal.put_chunk`, `terminal.put_finish` and
  `terminal.put_abort` (`evidence/hc-0.1.22-method-counts.txt:24-27`). Each is described as
  "Between cores": a put that comes in pieces, so a big file never holds up a line
  (`evidence/hc-schema-0.1.21-to-0.1.22.diff:139,234,311,368`). Together they do what
  `terminal.put` does: they write a file beside a terminal. A probe on 0.1.22 sent all four and
  read the file back (`evidence/probe-hc-0.1.22-put-docs-output.txt:14-29,34-35`). The probe's
  source is in the herdrify-hc 0.1.22 chain
  (`../2026-11-24-herdrify-hc-0-1-22/evidence/probe-022/`).
- **A new optional `docs` param on `events.subscribe`.** It narrows by document name, and "naming
  narrows, so documents alone hear only them" (`evidence/hc-schema-0.1.21-to-0.1.22.diff:433`).
  Probed on 0.1.22: `{"docs":["probe-doc"]}` delivered only that document's events. It sent no
  terminal events and an empty terminal snapshot
  (`evidence/probe-hc-0.1.22-put-docs-output.txt:170-181`). With no filter, the stream carries
  every `doc` event and every terminal event (`:212-226`).

**The rest of what ADR 0004 relied on still holds**, checked against the versions installed
today (`evidence/adr-0008-unchanged-forces.txt`):
- `terminal.run` is identical in the 0.1.21 and 0.1.22 documents (`:18-19`).
- `hc exec` and `hc file read|write|edit` are still there (`evidence/hc-0.1.22-help.txt:28,30`).
- airsl is 0.1.5, and every airsl source file cited below is byte-identical to 0.1.4 (`:1-11`).
- herdr-app is 0.1.17 and still has `terminal.open` (`:12-15`).

**The forces behind the grant model** are recorded here in full, because this ADR replaces 0004.
The airsl citations are to airsl 0.1.4 (identical in 0.1.5, as above), in
`evidence/airsl-0.1.4-excerpts.md`.

- **airsl's grant model.**
  - `GrantSet` has three grant axes: `fs`, `env` and `proc` (`src/sandbox/grant_set.rs:54-59`). It
    is `#[non_exhaustive]` (`:53`).
  - A `Policy` is built from those axes, and `fs` takes read and write roots
    (`src/sandbox/grant_set.rs:43-50`).
  - airsl accepts host modules from other crates (`src/modules/registry.rs:1-6`), under a root
    table the host chooses (`src/builder.rs:78-86`).
  - A module reads its grants at install time and checks them on each call
    (`src/modules/registry.rs:52,103-105`).
  - airsl warns that "A module built with its own copy of the grants could disagree with the
    engine" (`src/modules/registry.rs:21-27`).
- **`proc` matching.** `proc` matches "on the program name as written, not on a resolved path",
  and is "only as strong as the write grants sitting alongside it"
  (`src/sandbox/grants.rs:221-226,255-257`).
- **airsl's extension system** has `extension.toml` (`src/extension/manifest.rs:40`), `Ceiling`
  (`src/extension/ceiling.rs:18`), `negotiate`, which "Intersects `manifest`'s request with
  `ceiling`" (`src/extension/negotiate.rs:269-275`), and `Approver`
  (`src/extension/approver.rs:68`).
- **Components need herdr-core, and airsl has no axis for it.** Granting the `hc` program through
  `proc` would grant everything:
  - `hc exec` runs any program, and `hc file` touches any file (`evidence/hc-0.1.22-help.txt:28,30`).
  - The API includes `machine.exec` and `file.write` (`evidence/hc-0.1.22-method-counts.txt:28,30`).
  - `terminal.run` alone has several ways to start an arbitrary program
    (`evidence/probe-facts-output.txt:12`; the param descriptions are in
    `evidence/hc-0.1.21-schema.json`, unchanged in 0.1.22):
    - `line`, a shell line instead of argv
    - no argv at all, which starts that machine's shell
    - an argv[0] that names a template
    - `env`
    - `keep`, which replaces a lost program with a shell
    - `on`
  - herdr-app can also start programs, through `terminal.open {argv}` (AGENTS.md:616, in
    `evidence/herdr-app-0.1.16-agents-excerpts.md`; still present in 0.1.17,
    `evidence/adr-0008-unchanged-forces.txt:15`).

## Options considered

**Who owns the grant model.** These are ADR 0004's options, restated because this ADR replaces
it. The forces above have not changed, so neither has the choice.

1. **herdrify-core owns the grant model.** Core defines its own kinds (`fs`, `env`, `proc`,
   `hc`), a `Ceiling` and the intersection. The lua adapter maps `fs`, `env` and `proc` onto an
   airsl `Policy`, and enforces `hc` in herdrify's own host module.
   - Pros: grant logic is tested without Lua, and new kinds are possible.
   - Cons: core duplicates airsl's intersection, and enforcement happens in two places.
2. **airsl is the source of truth.** `component.toml` is translated to airsl's manifest, and
   `negotiate` and `Approver` do the work.
   - Pros: less code.
   - Cons: airsl has no axis for `hc`. Either core would depend on airsl, against ADR 0001, or the
     logic would live only in the adapter, where testing it needs Lua.
3. **Extend airsl upstream with host-defined grant kinds, then use option 2.**
   - Pros: one model.
   - Cons: herdrify's releases wait on airsl changes, and airsl gains host-specific concepts.

**Where the 0.1.22 additions go.**

A. **The four `terminal.put_*` methods are never exposed, and components cannot set `docs`.**
   - Pros:
     - Puts: matches `terminal.put`, which is already never exposed and does the same thing.
       The table stays closed.
     - `docs`: doc events are dropped before Lua whatever a component asks, so `docs` would give
       a component nothing. Set alone, it would also silently empty the terminal snapshot and
       stop terminal events (Context, the 0.1.22 probe). Refusing it removes a way to get an
       empty stream by mistake.
   - Cons: a component that wants to send a large file has no way to do it.
B. **Expose the chunked put under a new grant part** (for example `put = "own"`, on terminals the
   component started).
   - Pros: components could hand files to their own terminals.
   - Cons: it writes files on the terminal's machine, which is file-write power outside the `fs`
     grant and its approval. A new grant part also widens the approval text and the ceiling. No
     component needs it yet.
C. **Allow `docs` on `events.subscribe` and still drop doc events.**
   - Pros: one fewer param rule.
   - Cons: `docs` could then only narrow away terminal events, with doc events dropped anyway.
     This gives nothing the `terminal`, `children_of` and `labels` filters don't give.
D. **Leave the table at 32 and say nothing.**
   - Pros: no ADR for each herdr-core release.
   - Cons: the table stops being closed. Whether a component may call a method in no row would be
     undefined.

## Decision

On ownership, option 1: `herdrify-core` owns the grant model. On the 0.1.22 additions, option A:
the four `terminal.put_*` methods are never exposed, and components cannot set `docs`.

`herdrify-core` defines four grant kinds: `fs`, `env`, `proc` and `hc`.

**The `hc` grant**

```toml
[grants.hc]
read = "own"              # "own" or "all"
run  = ["cargo", "npm"]   # programs terminal.run may start, by exact argv[0]
send = "own"              # only "own"
```

An absent part grants nothing. Every one of herdr-core 0.1.22's 36 RPC methods falls in exactly
one row below.

| Group | Methods | Rule |
|---|---|---|
| `read` | `terminal.list`, `terminal.get`, `terminal.read`, `terminal.wait`, `events.subscribe`, `templates.list` | `"own"`: only terminals this component started. `"all"`: every terminal. `doc` events are dropped from `events.subscribe` before they reach Lua, and components cannot set its `docs` param. |
| `run` | `terminal.run` | See the `terminal.run` rules below. |
| own lifecycle | `terminal.kill`, `terminal.rm` | Allowed with `run`, on terminals this component started only. |
| `send` | `terminal.send`, `terminal.keys`, `terminal.interrupt` | On terminals this component started only. |
| internal | `hello`, `rpc.discover` | Used by the adapter itself, never by components. |
| never exposed | `server.*` (3), `machine.*` (6), `file.*` (3), `doc.*` (2), `terminal.put`, `terminal.put_begin`, `terminal.put_chunk`, `terminal.put_finish`, `terminal.put_abort`, `terminal.keep`, `terminal.label`, `terminal.attach` | 22 methods. |

That is 6 + 1 + 2 + 3 + 2 + 22 = 36.

A method that a later herdr-core adds is in no row, so it is never exposed until an ADR places it.

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

**`events.subscribe` params**

- In 0.1.22 its params are `children_of`, `docs`, `labels` and `terminal`
  (`evidence/adr-0008-unchanged-forces.txt:16-17`). A component may set `terminal`,
  `children_of` and `labels`; `docs` is refused.

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

**What changed from ADR 0004**

- The table counts 36 methods. The four `terminal.put_*` methods are never exposed.
- `events.subscribe` has a stated param rule: `docs` is refused.
- A method added by a later herdr-core is never exposed until an ADR places it.

**Benefits**

- Every grant rule is tested in core with fakes: parsing, the ceiling, the method table, the
  `terminal.run` and `events.subscribe` rules, and the `proc` checks.
- Components get herdr-core access through a closed method table, with no path to a shell
  through `terminal.run` and no way to write files beside a terminal.

**Costs**

- **Duplicated logic.** Core re-implements intersection logic that airsl's `negotiate` already has.
- **The `hc` grant lives outside airsl.** Two enforcement points must agree: airsl for `fs`, `env`
  and `proc`, and herdrify's module for `hc`. The `hc` grant reaches its module as a separate
  copy, which is exactly what airsl warns against (`src/modules/registry.rs:21-27`). The
  adapter's tests must keep the two in step.
- **`read = "all"` is broad.** It exposes every terminal's screen and history, including the
  person's own and anything secret on them. The approval must say so plainly.
- **Some allowed programs run arbitrary code anyway.** Shells and interpreters do, and so do tools
  such as `cargo` and `npm`, which run build scripts. This holds for `proc` and `run` entries
  alike. herdrify does not judge it beyond showing the entry at approval.
- **PATH shadowing.** Program names are matched as written (`src/sandbox/grants.rs:223-226`). An
  `fs` write grant over a directory on `PATH` lets a component plant its own copy of any granted
  program. The approval shows the write grant; herdrify does not cross-check it against `PATH`.
- **No file hand-off for components.** A component cannot send a file to a terminal, small or
  large. Changing that takes a new ADR.
- **Each herdr-core release needs a look**, at its methods and at its params. A new method stays
  unreachable until an ADR places it. A new param on an exposed method is a different case: only
  `terminal.run` and `events.subscribe` have a param allowlist. For the other exposed methods,
  whether a new param reaches components depends on what the adapter forwards. That is left to
  the `herdrify-herdr` design, and each release's diff must be checked for it.
