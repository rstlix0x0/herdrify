# herdrify-hc examples

Each folder is one use case: a program that walks through it step by step, and a README that says
what you can do with the SDK there.

They use `HerdrCore`, the entrypoint: it starts `hc connect` for you and takes plain values
(strings, numbers, durations), and every call is chained options then `.await`:

```rust
let core = HerdrCore::local().await?;
let shell = core.terminals().run(["sh"]).await?.terminal;
let counted = core.exec(["wc", "-c"]).input("hello").await?;
```

Under it is the `Client` with the typed requests of `herdrify_hc::protocol` and a transport you
pick, for full control. [`connect`](connect/) shows that path side by side with the entrypoint
(`--client`), and `HerdrCore::client()` reaches the same `Client` at any time.

| Use case | What you can do | Run |
|---|---|---|
| [`connect`](connect/) | Reach a core on this machine, a named session, another machine over ssh, or through a transport you build; ask how it is doing; the same with the raw `Client` | `cargo run -p herdrify-hc --example connect` |
| [`terminals`](terminals/) | Run programs in terminals, wait for them, read their screens, type into them, interrupt, kill and remove them | `cargo run -p herdrify-hc --example terminals` |
| [`streams`](streams/) | Follow core events as they happen, and attach to a live terminal: its output as printed, your input as typed | `cargo run -p herdrify-hc --example streams` |
| [`files-and-exec`](files-and-exec/) | Run commands without a terminal, and write, read and edit files on the core's machine | `cargo run -p herdrify-hc --example files-and-exec` |

## Before you run them

They need `hc` on `PATH` (they were written against hc 0.1.21). Each acts on the core that
`hc connect` reaches, which is your everyday core unless you say otherwise. The examples only
touch terminals and files they create, and remove them again.

To run them on a throwaway core of their own instead:

```sh
dir=$(mktemp -d /tmp/hfy.XXXX)
mkdir -p "$dir/state" "$dir/config" "$dir/run"
export HC_SESSION=try-herdrify XDG_STATE_HOME=$dir/state XDG_CONFIG_HOME=$dir/config XDG_RUNTIME_DIR=$dir/run
cargo run -p herdrify-hc --example terminals
hc server stop        # stops the throwaway core only
rm -rf "$dir"
```

Inside a Herdr terminal, also `unset HC_SCOPE HC_TERMINAL_ID HC_TERMINAL_SOCKET` first: otherwise
`hc connect` inherits them from that terminal. Keep `$dir` short: macOS refuses a core socket path
longer than 103 bytes, which a `mktemp -d` under `/var/folders` exceeds.
