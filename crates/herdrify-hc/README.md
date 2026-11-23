# herdrify-hc

A standalone async client for herdr-core. It speaks JSON-RPC to a core through `hc connect`, and
the transport is a trait you can replace, so tests can run against a fake.

- `HerdrCore`, one entrypoint: it starts `hc connect` (or `ssh <host> hc connect`, or a session you
  name) and reaches every method from plain values. Each call is chained options, then `.await`.
- All 32 methods of the core's API: `hello` through `Client::connect`, the rest as typed `Client`
  methods with typed parameters and results (`rpc.discover` returns the raw OpenRPC document).
- Concurrent calls on one connection, over the framed protocol.
- Event subscriptions and terminal attach streams, with flow-control credit granted for you.
- `ProcessTransport` runs `hc connect` (or any command that speaks the protocol, such as
  `ssh box hc connect`). `StreamTransport` carries the protocol over any async byte pair.

## Example

This runs a command in a terminal on the core that `hc connect` reaches, reads what it printed and
removes the terminal. It needs `hc` on `PATH`; the crate was tested against hc 0.1.21.

```rust,no_run
use herdrify_hc::prelude::*;

#[tokio::main]
async fn main() -> Result<(), HerdrCoreError> {
    let core = HerdrCore::local().await?;
    let started = core.terminals().run(["sh", "-c", "echo hello"]).await?;
    started.terminal.wait().await?;
    println!("{}", started.terminal.read().await?.text.trim_end());
    started.terminal.rm().await?;
    Ok(())
}
```

`HerdrCore` is built on `Client`. For full control, build the transport yourself, connect a
`Client` and send the typed requests of `herdrify_hc::protocol`:

```rust,no_run
use herdrify_hc::protocol::{Command, TerminalRun};
use herdrify_hc::{Client, ProcessTransport};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport = ProcessTransport::hc_connect().spawn()?;
    let client = Client::connect(transport).await?;
    let terminal = client.terminal_run(TerminalRun::new(Command::Shell)).await?;
    println!("started {}", terminal.address);
    Ok(())
}
```

## More examples

[`examples/`](https://github.com/rstlix0x0/herdrify/tree/main/crates/herdrify-hc/examples) holds one
runnable program per use case, each with a README on what you can do there:

| Use case | Run |
|---|---|
| connect to a core: local, a named session, over ssh, your own transport, the raw `Client` | `cargo run -p herdrify-hc --example connect` |
| run and drive terminals | `cargo run -p herdrify-hc --example terminals` |
| follow core events and attach to a live terminal | `cargo run -p herdrify-hc --example streams` |
| run commands and work with files | `cargo run -p herdrify-hc --example files-and-exec` |

Part of [herdrify](https://github.com/rstlix0x0/herdrify). Licensed under Apache-2.0.
