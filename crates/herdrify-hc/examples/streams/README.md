# Follow a core as it happens

Calls ask a question and get one answer. Streams keep answering: core events report every change to
every terminal, and an attach stream is a live terminal, byte by byte. Both run on the same
connection as your calls, and the SDK grants the flow-control credit as you read them.

## What you can do

- Subscribe to core events with `core.events()`, for every terminal or narrowed with
  `.terminal(..)`, `.children_of(..)` or `.labels([..])`; a `TerminalHandle`'s `.events()` is one
  terminal's. The subscription starts with a snapshot of the terminals that exist
  (`Subscription::terminals`), then `next()` yields each event: created, finished, status, title,
  foreground process, size, labels and more.
- Attach to a terminal with its handle's `.attach(cols, rows)` and read its output as it is printed:
  raw terminal bytes, escape sequences included, ready for a terminal emulator.
- Type into it with `AttachStream::input` (raw bytes, `\r` is Enter), resize it with `resize`, and
  detach with `close`.
- Know why a stream ended: `CloseReason::Detached` when you closed it, `CloseReason::Ended` with
  the exit code when the program exited.

## Run

```sh
cargo run -p herdrify-hc --example streams
```

```text
subscribed; 0 terminals exist already
started t1
  t1 created
  t1 runs bash (pid 16779)
  t1 finished: Exited, exit code Some(3)
attached to t2
the shell answered 42 after 302 bytes of output
the shell ended: Exited, exit code Some(0)
```

## Where to look

| File | Shows |
|---|---|
| [`events.rs`](events.rs) | subscribe, start a command, follow its events to its exit, close the subscription |
| [`attach.rs`](attach.rs) | attach to a shell, type into it, read until it answers, resize, see it exit |
