# Changelog

All notable changes to `herdrify-hc` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-11-24

### Added

- Putting a file on a terminal in pieces of at most 1 MiB, up to 8 MiB per put, so a big file never
  holds up the connection. It needs a framed connection and a core of 0.1.22 or newer.
  - `Client::terminal_put_begin`, `terminal_put_chunk`, `terminal_put_finish` and
    `terminal_put_abort`, with their request types and the reply types `PutBeginResult`,
    `PutPieceResult` and `EmptyResult`.
  - `TransferId`, the identifier that ties the pieces of one transfer together: `TransferId::new`
    checks a value, `TransferId::random` makes one, and `protocol::InvalidTransferId` is the error
    of `new`.
  - `TerminalHandle::put_begin` opens a transfer; `TerminalHandle::put_in_pieces` sends a whole
    file in pieces and aborts the transfer if a piece fails.
  - `TerminalOps::put_chunk`, `put_finish` and `put_abort` send one piece, finish the transfer
    and cancel it.
- Document filter on event subscriptions: `EventsSubscribe::docs`, and the `docs` option of
  `HerdrCore::events`. `Error::DOCS_OPTION` names that option in `Error::NeedsNewerCore`.

### Changed

- **Breaking:** `Error` gains the variant `Error::NeedsNewerCore`. The `docs` option is refused
  with it, before anything is sent, on cores older than 0.1.22. A `match` on `Error` that has no
  wildcard arm must handle it.
- Tested against `hc` 0.1.22. On `hc` 0.1.21 every live test passes except the schema drift check,
  which pins the 0.1.22 schema; the put in pieces tests skip there.

## [0.1.0] - 2026-11-23

First release: an async client for herdr-core, tested against `hc` 0.1.21.

### Added

- `HerdrCore`, the entrypoint: one connection with every method reachable from plain values.
  - `HerdrCore::local`, and `HerdrCore::builder` with `.session`, `.ssh`, `.command`, `.env` and
    `.env_remove`; `HerdrCore::over` takes any transport, and `HerdrCore::client` reaches the
    `Client` underneath.
  - Methods grouped by what they act on: `terminals()`, `machines()`, `files()`, `docs()`,
    `server()`, plus `exec`, `exec_line`, `events`, `templates` and `discover`. A
    `TerminalHandle` holds every method about one terminal; `run` returns one in `Started`.
  - Every method returns a `Call`: chain its options, then `.await` it. `Call::with` applies an
    option of the `protocol` request type directly.
  - `HerdrCoreError`, whose `refusal()` is the core's refusal when there is one.
- `prelude`, the names most programs need in one import.
- `Client`: one framed connection to a core, running calls side by side.
  - `Client::connect` sends `hello` and switches the connection to frames.
  - The other 31 methods of the core's API are typed `Client` methods with typed parameters and
    results; `rpc_discover` returns the raw OpenRPC document.
- Streams, with flow-control credit granted as frames are read:
  - `Client::events_subscribe` returns a `Subscription` of core events.
  - `Client::terminal_attach` returns an `AttachStream` of terminal output, with `AttachEvent`
    and `CloseReason`.
- Transports:
  - `ProcessTransport` runs `hc connect`, or any command that speaks the protocol, such as
    `ssh box hc connect`.
  - `StreamTransport` carries the protocol over any async reader and writer.
  - The `Transport`, `FrameSink` and `FrameSource` traits, so a test can supply its own.
- Errors: `Error` for calls and `TransportError` for transports; `ConnectionEnd` says why a
  connection ended.
- Examples, one per use case, each with a README: `connect`, `terminals`, `streams` and
  `files-and-exec`. They use `HerdrCore`; `connect --client` shows the same with the `Client`.
- Minimum supported Rust version: 1.94.

[0.2.0]: https://github.com/rstlix0x0/herdrify/releases/tag/herdrify-hc-v0.2.0
[0.1.0]: https://github.com/rstlix0x0/herdrify/releases/tag/herdrify-hc-v0.1.0
