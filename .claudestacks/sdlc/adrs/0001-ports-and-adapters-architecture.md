---
status: approved
created: 2026-11-22
sealed: fcf0e259ccee
---

# ADR 0001: Ports and adapters architecture

## Context

herdrify's logic covers several jobs: parsing manifests, checking grants against a ceiling,
planning an install, computing what `remove` must undo, composing a frame into a layout, and
validating a UI tree. That logic has to deal with six outside parties:

- **herdr-core**, through JSON-RPC 2.0 over `hc connect` (`evidence/hc-0.1.21-help.txt:41`,
  `evidence/probe-facts-output.txt:6`)
- **herdr-app**, through `herdr-app do <command>` (`evidence/herdr-app-0.1.16-help.txt:6`)
- **the airsl Lua engine**
- **the filesystem**
- **git**
- **the person**, who approves grants

The author wants every piece of that logic tested with fakes: no running Herdr, no real programs,
no network, no person at a prompt.

The README splits the work into six crates (`README.md:43-48`):

- `herdrify-core` is the "herdr-core client" (`README.md:43`).
- `herdrify-compose` builds the layout and themes (`README.md:46`).
- `herdrify-pkg` fetches, plans, applies and locks installs (`README.md:47`).

Our inference from those job descriptions is that compose and pkg would reach the filesystem, git
and Herdr directly. Testing their logic would then need a real Herdr home and real repositories.

Two forces shape the seams.

**Behaviour injection.** The rust guideline makes "Generic with trait bound" the default way to
inject behaviour when "the production `SqlStorage` and a `MockStorage` for tests" are both expected
(`claudestacks-guideline-rust/0.1.5/skills/rust-guidelines/references/static-dispatch.md:12`).

**airsl is synchronous.** All the following citations are to airsl 0.1.4, in
`evidence/airsl-0.1.4-excerpts.md`.
- `Engine::eval` and `Engine::eval_to` are plain `fn`s (`src/engine.rs:215,237`).
- A recursive grep of `src/` for `async fn|impl Future` exits 1. The same search shape finds those
  `eval` functions.
- No tokio appears in its dependency tree. `cargo tree -e normal -i tokio` in the probe shows tokio
  reached only from the probe crate itself.

Meanwhile the SDKs that talk to Herdr are async (ADR 0002), so herdrify contains a sync/async
boundary.

## Options considered

1. **Ports and adapters with a pure domain crate.**
   - `herdrify-core` holds the domain types, the ports (a trait for every outside party) and the use
     cases, with no I/O.
   - Adapter crates implement the ports. The CLI is the composition root that wires the real
     adapters together.
   - Pros: logic is testable with fakes; normal dependencies point one way; the SDKs stay separate
     products.
   - Cons: more crates; types are mapped at each boundary; every outside party needs a trait.
2. **The README's split by feature.** Six crates, each doing its own I/O (`README.md:43-48`).
   - Pros: fewer crates and no port traits.
   - Cons: compose and pkg logic would reach real systems, so their tests need real systems.
     Nothing gives fakes a seam.
3. **One crate with modules.**
   - Pros: the fastest start, with one manifest.
   - Cons:
     - nothing enforces dependency direction, so a module can reach for I/O from anywhere
     - the SDKs cannot be published on their own

## Decision

Option 1. The workspace has nine crates in three layers.

```
 L2  herdrify-cli       composition root: wires real adapters, terminal Approver,
                        user and author commands
 L1  herdrify-lua       Runtime port: airsl engine, host modules, grants → airsl Policy
     herdrify-herdr     Window + Terminals ports: Herdr home files, herdr-app, hc
     herdrify-store     Store port: $HERDRIFY_HOME (ADR 0005)
     herdrify-git       Source port: fetch repo@rev into the cache
     herdrify-testkit   fakes for every port except Runtime (ADR 0007)
 L0  herdrify-core      domain types, ports, use cases (add, remove, render, check, compose)
     herdrify-hc        herdr-core SDK, standalone (ADR 0002)
     herdrify-app       herdr-app SDK, standalone (ADR 0002)
```

**Dependency rules**

- **Normal dependencies point only downward.** Dev-dependencies are exempt.
- **Core's ports are six traits:** `Runtime`, `Window`, `Terminals`, `Store`, `Source` and
  `Approver`.
  - `Approver` asks the person to approve a plan. The CLI implements it on the terminal, and
    `herdrify-testkit` fakes it.
- **Core has no I/O.** Its normal dependencies include no tokio, no airsl and no SDK.
- **The SDKs depend on no herdrify crate.** `herdrify-hc` and `herdrify-app` are adapted to core's
  ports by `herdrify-herdr`.

**Ports**

- Ports are traits used through generics, not `Box<dyn …>` (the static-dispatch rule above).
- Ports are async. Each port method returns `impl Future<Output = …> + Send`.
- Port implementors are `Send + Sync + 'static` and shared through `Arc`. This is required because:
  - a host module that holds a port must satisfy `HostModule: Send + Sync`
    (`src/modules/registry.rs:97`)
  - airsl enables mlua's `send` feature (airsl `Cargo.toml:107-114`)

  Both citations are in `evidence/airsl-0.1.4-excerpts.md`.

**Where core's tests live**

The probe in `evidence/probe-devdep-cycle-output.txt` shows the split:

- A `#[cfg(test)]` unit test inside a core crate cannot use fakes from a crate that depends on core.
  It fails with E0277 and "multiple different versions of crate `probe_core`" (lines 10 and 18,
  exit 101).
- An integration test under `tests/` can use them (line 5, exit 0).

So:

- Core's use-case tests live in `herdrify-core/tests/`. They dev-depend on `herdrify-testkit` and
  tokio.
- Core's colocated unit tests use fakes written inside the same file.

**The sync/async bridge**

- The airsl engine runs inside `tokio::task::spawn_blocking`.
- A host module that needs an async port calls it through a captured
  `tokio::runtime::Handle::block_on`.

The probe confirms both halves. On a blocking thread this returns the value
(`evidence/probe-bridge-output.txt:2`). When the same eval runs on an async worker thread, the call
panics with "Cannot start a runtime from within a runtime" (`evidence/probe-bridge-output.txt:7`).
Probe versions are airsl 0.1.4 and tokio 1.54.1 (`evidence/README.md`).

## Consequences

**Benefits**

- Every use case is tested in `herdrify-core/tests/` against fakes from `herdrify-testkit`.
- Each adapter is tested against its real outside party:
  - `herdrify-store`: a temporary `$HERDRIFY_HOME`
  - `herdrify-git`: a local git repository
  - `herdrify-herdr`: a temporary `HERDR_APP_HOME` (`evidence/herdr-app-0.1.16-help.txt:19`) and a
    running `hc`
  - `herdrify-lua`: the real airsl engine
- The two SDKs ship on their own (ADR 0002).
- A future library facade over core and the adapters would be one new crate. That facade is out of
  scope now.

**Costs**

- **Nine crates to version and publish.** Publishing goes dependency-first: SDKs and core, then the
  adapters, then the CLI.
- **Boundary mapping.** Domain types and SDK types are mapped in `herdrify-herdr`. That mapping is
  code with its own tests.
- **Strict port bounds.** Every port implementor must be `Send + Sync + 'static`, and every port
  future must be `Send`. That limits what an adapter may hold across `.await`.
- **Use-case tests are integration tests.** They see only core's public API, not its private items
  (probe above).
- **A bridging rule.** A host module that calls `block_on` panics when the eval runs on an async
  worker thread (probe above). The lua adapter owns the rule that every eval runs inside
  `spawn_blocking`.
- **Stale names.** `herdrify-core` changes meaning. The README (`README.md:43,94`) and CLAUDE.md
  use it for the herdr-core client and must be updated. The name also invites confusion with
  herdr-core itself.
- **Port churn.** Core's ports are a public API, so while herdrify is 0.x a port change ripples into
  every adapter and fake.
