---
status: approved
created: 2026-11-22
sealed: bdc5077dd066
---

# ADR 0007: The author SDK for components and bundles

## Context

Under ADR 0003, authors write only Lua, TOML and Markdown. To work well they need five things:

1. An API they can discover.
2. Validation before they publish.
3. Tests that need no running Herdr.
4. A quick way to see their component working.
5. A stable render contract between what their component returns and what `herdrify-host` draws,
   so a component keeps working as the host evolves.

Herdr solves the first need for mod authors by generating type files beside its guide on every
start: `gpui-kit.d.ts` and `herdr.d.ts` (AGENTS.md:44-47, in
`evidence/herdr-app-0.1.16-agents-excerpts.md`).

airsl ships two relevant commands:
- `airsl check` "Compile every Lua file under a path without running any of it"
- `airsl test` "Run the Lua test files under a directory"

(`evidence/airsl-0.1.4-help.txt:7-8`.)

Two architectural facts bear on this:

- **Host-module functions carry no type information.** They are closures created inside
  `HostModule::install` (airsl 0.1.4 `src/modules/registry.rs:97-117`, in
  `evidence/airsl-0.1.4-excerpts.md`), so nothing can read their types back out. Any type stubs
  need a description that exists independently of those closures.
- **Fakes come from testkit.** ADR 0001 provides fakes for every port except `Runtime`, from
  `herdrify-testkit`. Only integration tests can use them
  (`evidence/probe-devdep-cycle-output.txt`).

## Options considered

1. **A Lua SDK delivered through the `herdrify` CLI.**
   - **One description of the Lua API.** `herdrify-core` holds it as data, and three things use
     it:
     - `herdrify-lua` implements it, with a test that fails when the two differ
     - editor stubs are generated from it
     - JSON Schemas are generated from core's manifest types
   - **Commands:** `herdrify new`, `check`, `test`, `render` and `dev`.
   - Pros: stubs are checked against the real modules by a test, and authors test with herdrify's
     own fakes.
   - Cons: an API description to maintain, plus the generators and test plumbing.
2. **Hand-written stubs and docs, with authors using `airsl check` and `airsl test` directly.**
   - Pros: less code.
   - Cons:
     - nothing ties the stubs to the real modules
     - authors would hand-write Lua stand-ins for herdrify's host modules, which can drift from the
       real ones
3. **Documentation only.**
   - Pros: cheapest.
   - Cons: authors find mistakes only by installing into a live Herdr.

## Decision

Option 1.

**The API description.** `herdrify-core` holds the Lua API as data. For every module and function
it records:
- the name
- the parameters and their types
- the return type
- the grant the function requires (ADR 0004)

`herdrify-lua` builds its host modules from it. A test in `herdrify-lua` fails when an installed
function is missing from the description, or a described function is missing from the modules.

**`herdrify sdk emit`** writes:
- the Lua editor stubs, from the API description
- JSON Schemas for `component.toml` and `bundle.toml`, from core's manifest types

The schemas check shape only.

**`herdrify check <dir>`** works offline on a component or bundle folder. It runs:
- the shape rules
- the ADR 0003 file-type allowlist and keys rules
- the ADR 0004 grant syntax rules, including the `proc` and `terminal.run` rules
- a compile of every Lua file, as `airsl check` does

It does not check the person's ceiling, clashes, or pinned references. `add` checks those against
installed state.

**`herdrify test <dir>`** runs the component's Lua tests:
- on the real `herdrify-lua` runtime
- with every other port faked by `herdrify-testkit`
- seeded through a `herdrify.test` Lua module that sets up fake terminals, files and store contents

**`herdrify render <component>`** prints the component's UI tree as JSON on stdout. `herdrify-host`
runs the same command.

**`herdrify dev <dir>`** links a folder as `dev/<name>` through the normal plan and approval
(ADR 0005). It then re-renders the component in a running Herdr when a file in the folder changes.

**The render contract.**
- A component declares `ui = 1` in `component.toml`.
- herdrify includes that version in the JSON it hands to `herdrify-host`.
- The host refuses a version it does not know, and shows the component name and the version.

**`herdrify-testkit`** is a published crate. herdrify's integration tests use it as a
dev-dependency (ADR 0001), and `herdrify test` uses it at runtime.

**Order of work.**
- The API description, the render contract, `check` and `render` are built together with the host
  and its widgets.
- `new`, `test`, `dev` and `sdk emit` follow.

## Consequences

**Benefits**

- Authors get completion, validation, offline tests and a live loop from one tool.
- Editor stubs cannot silently diverge from the host modules, because a test fails first.

**Costs**

- **More to maintain.** The API description is a second statement of the Lua API, kept honest only
  by the test in `herdrify-lua`.
- **Schemas are only a first check.** They validate shape. The other rules exist only in
  `herdrify check` and `herdrify add`, so an editor's schema check alone is not enough.
- **Fakes become public API.** `herdrify-testkit` is published, so a change to a fake is a release.
- **`dev` needs Herdr.** It needs a running Herdr; `check`, `test` and `render` do not.
- **Contract versions accumulate.** A change to the render contract needs a new `ui` version, and
  the host must keep supporting the old ones.
