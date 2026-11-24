# herdrify-app

A standalone async client for herdr-app, and models of the files it reads. It runs the `herdr-app`
program to reach the Herdr window, and the transport is a trait you can replace, so tests can run
against a fake.

- `HerdrApp`, one entrypoint: it runs `herdr-app`, checks its version once and reaches every
  command from plain values. Each call is chained options, then `.await`.
- All 77 commands of `herdr-app commands` as typed values, with typed replies for the ones that
  answer, plus `ls` and `status` for the window.
- `ProcessTransport` runs the `herdr-app` program. The `Transport` trait is the seam to anything
  else that can run a command.
- The layout, theme and keys files as plain data, with no I/O (see below).

## Example

This opens a space and a tab in the window that `herdr-app` reaches, then splits the pane. It needs
`herdr-app` on `PATH` and a running window; the crate was tested against herdr-app 0.1.17.

```rust,no_run
use herdrify_app::prelude::*;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    let app = HerdrApp::local().await?;
    let created = app
        .spaces()
        .new_space()
        .name("api")
        .argv(["claude"])
        .focus(Focus::Background)
        .await?;
    app.tabs().new_tab().name("build").space(created.space.as_str()).await?;
    app.panes().split(SplitDirection::Right).await?;
    Ok(())
}
```

## `HerdrApp` and `Client`

`HerdrApp` is built on `Client`. Use `HerdrApp` when you want to say what to do with plain values:
operations are grouped by what they act on (`spaces()`, `tabs()`, `panes()`, `terminals()`,
`agents()`, `layouts()`, `mods()`, `overlays()`, `window()`), and a value a command refuses comes
back as `Error::InvalidArgument` when the call is awaited, without running anything.

Use `Client` when you want full control: build the transport yourself, connect a `Client` and send
the typed commands of `herdrify_app::protocol`. `HerdrApp::client` reaches the `Client` underneath,
and `HerdrApp::from_client` wraps one you built.

```rust,no_run
use herdrify_app::protocol::TabNext;
use herdrify_app::{Client, Error, ProcessTransport};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    let client = Client::new(ProcessTransport::herdr_app());
    client.check_version().await?;
    client.run(TabNext::new()).await?;
    Ok(())
}
```

## The file models

`herdrify_app::files` holds the three files herdr-app reads, as plain data with no I/O:

- `Layout`, the layout file: a tree of nodes with the content, frames, fills and look they hold.
- `Theme`, the theme file, with its colours.
- `Keys`, the keys file, its sections and the action each chord is bound to.

Each one parses from text and writes back to JSON. A file keeps every key and number it does not
name, in the form it was read, so a file you read and write back means the same to herdr-app.
Reading and writing the file on disk is left to you.

## Tests

The live tests in `tests/live.rs` open a `herdr-app` window of their own. Each run has its own
`HERDR_APP_HOME` and `hc` session in a scratch directory, and none of the `HC_*` variables or
`HERDR_APP_HOME` of the environment it runs in, so it never reaches a window or core you are
running. The tests skip when `herdr-app` is not on `PATH`, and the window test also skips when `hc`
is missing or no window can open.

## More examples

[`examples/`](https://github.com/rstlix0x0/herdrify/tree/main/crates/herdrify-app/examples) holds
one runnable program per use case, each with a README on what you can do there:

| Use case | Run |
|---|---|
| read the version, the window listing and its status | `cargo run -p herdrify-app --example inspect` |
| open spaces, tabs and panes through `HerdrApp` | `cargo run -p herdrify-app --example drive` |
| build, check and print a layout, a theme and keys | `cargo run -p herdrify-app --example files` |

`inspect` and `drive` need `herdr-app` on `PATH` and a running window. `files` runs anywhere.

Part of [herdrify](https://github.com/rstlix0x0/herdrify). Licensed under Apache-2.0.
