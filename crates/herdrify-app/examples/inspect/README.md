# Look at a running window

Ask herdr-app how its window is doing, and change nothing.

## What you can do

- Connect with `HerdrApp::local()`. It runs `herdr-app` and checks its version once; the version
  it found is `app.version()`.
- List the window with `app.ls().await?`: its spaces, each space's tabs, each tab's terminals, and
  the spaces closed recently.
- Read the window's state with `app.status().await?`: the layout in use, the zoom, the space and tab
  that are shown, the overlays open, and any errors the window reports.

## Run

It needs `herdr-app` on `PATH` and a running window.

```sh
cargo run -p herdrify-app --example inspect
```

It prints the version, one line for each space, tab and terminal, and the window state.

## Where to look

| File | Shows |
|---|---|
| [`main.rs`](main.rs) | the version, `ls` and `status` |
