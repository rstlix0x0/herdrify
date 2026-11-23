# herdrify-hc

A standalone async client for herdr-core. It speaks JSON-RPC to a core through `hc connect`, and
the transport is a trait you can replace, so tests can run against a fake.

- All 32 methods of the core's API: `hello` through `Client::connect`, the rest as typed `Client`
  methods with typed parameters and results (`rpc.discover` returns the raw OpenRPC document).
- Concurrent calls on one connection, over the framed protocol.
- Event subscriptions and terminal attach streams, with flow-control credit granted for you.
- `ProcessTransport` runs `hc connect` (or any command that speaks the protocol, such as
  `ssh box hc connect`). `StreamTransport` carries the protocol over any async byte pair.

## Example

This starts a terminal on the core that `hc connect` reaches. It needs `hc` on `PATH`; the crate
was tested against hc 0.1.21.

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

Part of [herdrify](https://github.com/rstlix0x0/herdrify). Licensed under Apache-2.0.
