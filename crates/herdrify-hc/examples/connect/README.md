# Connect to a core

Everything starts with `HerdrCore`, the entrypoint. It builds the command that reaches the core,
connects, and then hands out every method from plain values. The calls are the same whatever
carries the connection.

## What you can do

- Reach the core that `hc connect` reaches, your default session or the one `HC_SESSION` names:
  `HerdrCore::local()`.
- Pick another core with `HerdrCore::builder()`:
  - `.session(name)`: a named core on this machine. `hc connect` starts it when it is not running,
    and it keeps running afterwards.
  - `.ssh(host)`: the core on another machine, through `ssh <host> hc connect`.
  - `.command([...])`: any other command that speaks the protocol on stdin and stdout.
  - `.env(key, value)` and `.env_remove(key)`: the command's environment, such as dropping
    `HC_SCOPE` inside a Herdr terminal.
- Bring your own byte stream with `HerdrCore::over(transport)`: a `StreamTransport` over a socket, a
  pipe or a process you started yourself, or your own `Transport` for a fake in tests.
- Ask the core about itself: its version and methods (`core.hello()`), its process and terminal
  counts (`core.server().status()`), its terminals (`core.terminals().list()`).
- Skip the entrypoint: build a `ProcessTransport`, connect a `Client` and send the typed requests
  from `herdrify_hc::protocol` yourself. `HerdrCore` is built on this path, and
  `core.client()` reaches it from the entrypoint.

## Run

```sh
cargo run -p herdrify-hc --example connect                     # the core hc connect reaches
cargo run -p herdrify-hc --example connect -- --session <name> # a named core on this machine
cargo run -p herdrify-hc --example connect -- --ssh <host>     # the core on <host>, over ssh
cargo run -p herdrify-hc --example connect -- --stream         # a StreamTransport built by hand
cargo run -p herdrify-hc --example connect -- --client         # the Client and typed requests
```

Output captured on hc 0.1.21:

```text
connected to hc 0.1.21, 32 methods
core pid 13739, up 0 s, 0 live and 0 finished terminals
```

## Where to look

| File | Shows |
|---|---|
| [`entrypoint.rs`](entrypoint.rs) | `HerdrCore::local`, the builder with a session or ssh, and `HerdrCore::over` a hand-built transport |
| [`status.rs`](status.rs) | `hello`, `server().status()` and `terminals().list()` on a `HerdrCore` over any transport |
| [`client.rs`](client.rs) | the same report with `ProcessTransport`, `Client::connect` and the typed requests |
