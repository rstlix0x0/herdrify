# Drive the window

Make a space, a tab and a pane in a running window, then close what you made.

## What you can do

- Make a space with `app.spaces().new_space()`, and give it a name, a program to run (`.argv(..)`)
  and `.focus(Focus::Background)` to leave your focus where it is. The reply holds the new space,
  tab and terminal.
- Add a tab to a given space with `app.tabs().new_tab().space(id)`.
- Split a given pane with `app.panes().split(SplitDirection::Right).pane(id)`. Without `.pane(..)`
  it splits the focused pane.
- Find the pane of a terminal in the listing from `app.ls()`.
- Close what you made, by identifier: `app.panes().close().pane(id)`,
  `app.tabs().close().id(id).confirmed()` and `app.spaces().close().id(id).confirmed()`.
  Without `.confirmed()`, `space.close` is refused. `tab.close` only asks, and only when it would
  close the last tab of a space.

## Run

It needs `herdr-app` on `PATH` and a running window.

```sh
cargo run -p herdrify-app --example drive
```

It prints each step as it goes: the space it made, the tab, the split, and the closes. It touches
only what it made, and closes the space again even when a step in between fails.

## Where to look

| File | Shows |
|---|---|
| [`main.rs`](main.rs) | make the space, add a tab, split a pane, close them; the clean-up that always runs |
