# Layout, theme and keys files

herdr-app reads three JSON files: a layout, a theme and a keys file. The `files` module has a
type for each, so a program can build one, write it, or read one and edit it, with no window and
no herdr-app involved.

## What you can do

- Build a layout from nodes: `Node::block(Content::module(ModName::new("herdrify-host")?))` is a
  block that shows a mod with no props, `Node::row(children)` holds blocks side by side, and
  `Layout::new(root)` makes the file.
- Build a theme with `Theme::new(background, foreground, selection, ansi)`, then set its cursor and
  named colours.
- Build a keys file with `Keys::new()` and `keys.insert(section, chord, action)`. A section such as
  `layout:herdrify-main` is `Section::Layout(LayoutName::new("herdrify-main")?)`.
- Write any of them with `to_json()` and read text back with `Layout::parse`, `Theme::parse` and
  `Keys::parse`. Keys the types do not name are kept, so reading a file and writing it back does not
  lose them.

## Run

It needs no herdr-app and no window.

```sh
cargo run -p herdrify-app --example files
```

For each of the three it prints the JSON, parses that text again, and exits non-zero when the value
read back differs from the value written.

```text
--- layout ---
{
  "type": "row",
  "children": [
    {
      "id": "host",
      "type": "block",
      "content": {
        "mod": "herdrify-host"
      }
    }
  ]
}
the layout read back equals the layout written

--- theme ---
{
  "background": "#1d1f21",
  "foreground": "#c5c8c6",
  "selection": "#373b41",
  "cursor": "#c5c8c6",
  "ansi": [
    "#1d1f21",
    "#cc6666",
    "#b5bd68",
    "#f0c674",
    "#81a2be",
    "#b294bb",
    "#8abeb7",
    "#c5c8c6",
    "#969896",
    "#d54e53",
    "#b9ca4a",
    "#e7c547",
    "#7aa6da",
    "#c397d8",
    "#70c0b1",
    "#eaeaea"
  ],
  "colors": {
    "accent": "#81a2be"
  }
}
the theme read back equals the theme written

--- keys ---
{
  "layout:herdrify-main": {
    "ctrl-b": "pane.split:right",
    "ctrl-n": "tab.next"
  }
}
the keys read back equals the keys written

```

## Where to look

| File | Shows |
|---|---|
| [`main.rs`](main.rs) | building a layout, a theme and keys, and the write and read-back check |
