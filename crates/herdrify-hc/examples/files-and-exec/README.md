# Run commands and work with files

Not everything needs a terminal. The core runs a command and hands back its output and exit code,
and reads and writes files on its machine. Through `ssh <host> hc connect`, that machine is the
remote one.

## What you can do

- Run a command with `core.exec(["prog", "arg"])`, or a shell line with `core.exec_line("...")`,
  and get its stdout, stderr and exit code. A failing command is not an error of the call; its exit
  code says how it went.
- Give it stdin with `.input(bytes)`, a working directory with `.cwd(path)`, extra environment with
  `.env([..])`, and a time limit with `.timeout(duration)`.
- Write a file with `core.files().write(path, data)`, read it whole or a range of lines with
  `.read(path)` (`.offset(line)`, `.limit(lines)`), and replace exact pieces of it with
  `.edit(path, [(old, new)])`.
- Do any of it on a machine added to the core with `.on(machine)`.

## Run

```sh
cargo run -p herdrify-hc --example files-and-exec
```

```text
uname -s printed "Darwin"
sh -c 'exit 3' exited with Some(3)
wc -c counted 5 bytes of input
the 1 s time limit stopped it: true
made /tmp/tmp.tCBPoEP9YV
wrote 34 bytes to /tmp/tmp.tCBPoEP9YV/notes.txt
it holds 3 lines:
first line
second line
third line
after the edit, line 2 reads "2nd line"
removed /tmp/tmp.tCBPoEP9YV
```

(The scratch directory is wherever `mktemp -d` puts it on the core's machine; the path is shortened
here.)

## Where to look

| File | Shows |
|---|---|
| [`exec.rs`](exec.rs) | commands with output, a failing exit code, stdin, a time limit; the scratch directory |
| [`files.rs`](files.rs) | write, read, edit, and read a range of lines |
