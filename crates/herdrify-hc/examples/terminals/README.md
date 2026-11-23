# Run and drive terminals

A terminal on a core runs one program, with a screen the core keeps. You start it, talk to it,
read it, and remove it when you are done with it.

## What you can do

- Start a program in a terminal with `core.terminals()`: `.run(["prog", "arg"])` for a program and
  its arguments, `.run_line("...")` for a shell line, `.run_shell()` for the machine's shell. Chain
  options before the `.await`: `.name(..)`, `.cwd(..)`, `.env([..])`, `.labels([..])`,
  `.size(cols, rows)`, `.on(machine)` and more.
- Get back a `TerminalHandle` (in `started.terminal`) that names the terminal for every later call,
  or make one for an existing terminal with `core.terminals().handle("t3")`.
- Wait for it with `.wait()`: until it exits, until text appears in its output (`.output(..)`), or
  until a timeout (`.timeout(..)`).
- Read its screen with `.read()`, even after it has finished.
- Type into it: `.send(text)` pastes text and presses Enter, `.keys([..])` presses named keys,
  `.interrupt()` presses Ctrl-C and reports the effect, `.put(name, data)` stores a file for it.
- Look it up and mark it: `core.terminals().list()`, `.get()`, `.set_labels([..])`,
  `.remove_labels([..])`, `.keep(Keep::On)`.
- End it with `.kill()` and forget it with `.rm()`.
- Tell a refusal apart from a broken connection: `error.refusal()` is the core's refusal, with a
  stable `ErrorKind` such as `NotFound`.

## Run

```sh
cargo run -p herdrify-hc --example terminals
```

```text
started t1 (Live)
wait ended: Exit, exit code Some(0)
it printed: hello from herdrify-hc
removed t1
terminal.get refused it as not_found: no terminal "t1"
started a shell at t2
the shell answered 42
interrupted sleep with ctrl+c: unknown
killed it: Finished
removed it
```

## Where to look

| File | Shows |
|---|---|
| [`lifecycle.rs`](lifecycle.rs) | run → wait → read → remove on the returned handle, and matching the `NotFound` refusal |
| [`interact.rs`](interact.rs) | a shell that stays open: send, read until a line appears, interrupt, kill, remove |
