# ADR evidence

The raw facts the ADRs in `..` cite. Every file is command output captured on 2026-11-22, on
macOS (Darwin 25.3.0), with these versions:

| Tool | Version | Captured in |
|---|---|---|
| hc (herdr-core) | 0.1.21 | `hc-version.txt` |
| herdr-app | 0.1.16 | `herdr-app-version.txt` |
| airsl CLI | 0.1.4 | `airsl-cli-version.txt` |
| airsl crate | 0.1.4 (mlua 0.12.2) | `probe/Cargo.lock` |
| tokio | 1.54.1 | `probe/Cargo.lock` |
| rustc / cargo | 1.99.0 | `rustc-version.txt` |

| File | Produced by |
|---|---|
| `hc-0.1.21-help.txt` | `hc --help` |
| `hc-0.1.21-schema.json` | `hc schema` |
| `herdr-app-0.1.16-help.txt` | `herdr-app --help` |
| `herdr-app-0.1.16-commands.json` | `herdr-app commands` |
| `herdr-app-0.1.16-agents-excerpts.md` | numbered excerpts of `~/.config/herdr-app/AGENTS.md` and `mods/toast/main.js` |
| `herdr-app-0.1.16-dts-excerpts.md` | grep over `~/.config/herdr-app/herdr.d.ts` |
| `airsl-0.1.4-help.txt` | `airsl --help` |
| `airsl-0.1.4-excerpts.md` | grep/sed over the published airsl 0.1.4 crate source |
| `probe-facts-output.txt` | `probe/src/bin/facts.rs` over the two JSON files above |
| `probe-bridge-output.txt` | `probe/src/bin/bridge.rs` (airsl 0.1.4 + tokio) |
| `probe-devdep-cycle-output.txt` | `probe-devdep-cycle/` (see below) |

Re-run the probes from `probe/`:

```sh
cargo run -q --bin facts -- ../hc-0.1.21-schema.json ../herdr-app-0.1.16-commands.json
cargo run -q --bin bridge
```

`probe/` is a standalone crate (its own `[workspace]`), never a member of the herdrify workspace.

## Dev-dependency cycle probe

`probe-devdep-cycle/` is a standalone two-crate workspace. Its `probe-core` crate dev-depends on
`probe-testkit`, and `probe-testkit` depends on `probe-core`. `probe-devdep-cycle-output.txt`
holds the output of running, from inside that directory:

```sh
cargo test -q -p probe-core --test integration   # integration test using the fake
cargo test -q -p probe-core --lib                # #[cfg(test)] unit test using the fake
```
