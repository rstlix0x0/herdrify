# herdrify-app examples

Each folder is one use case: a program that walks through it step by step, and a README that says
what you can do with the SDK there.

The first two use `HerdrApp`, the entrypoint: it runs `herdr-app` for you, checks its version
once and takes plain values (strings, numbers), and every call is chained options then `.await`:

```rust
use herdrify_app::prelude::*;

async fn make() -> Result<(), Error> {
    let app = HerdrApp::local().await?;
    let created = app.spaces().new_space().name("api").focus(Focus::Background).await?;
    app.tabs().new_tab().name("build").space(created.space.as_str()).await?;
    Ok(())
}
```

Under it is the `Client` with the typed commands of `herdrify_app::protocol` and a transport you
pick, for full control. `HerdrApp::client()` reaches it at any time.

| Use case | What you can do | Run |
|---|---|---|
| [`inspect`](inspect/) | Read the version, the spaces, tabs and terminals, and the state of a running window, changing nothing | `cargo run -p herdrify-app --example inspect` |
| [`drive`](drive/) | Make a space, a tab and a pane in a running window, then close what you made | `cargo run -p herdrify-app --example drive` |
| [`files`](files/) | Build a layout, a theme and a keys file in code, write them as JSON and read them back, with no window | `cargo run -p herdrify-app --example files` |

## Before you run them

`inspect` and `drive` need `herdr-app` on `PATH` (they were written against herdr-app 0.1.17) and a
running window. They act on the window that `herdr-app` reaches, which is your everyday window
unless you point it elsewhere, for example with `HerdrApp::builder().home(..)`. `drive` only closes
what it made itself.

`files` needs neither: it works on values in memory and prints them.
