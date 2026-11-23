# Changelog

All notable changes to `herdrify-hc` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

First release: an async client for herdr-core, tested against `hc` 0.1.21.

### Added

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
- Minimum supported Rust version: 1.94.

[0.1.0]: https://github.com/rstlix0x0/herdrify/tree/main/crates/herdrify-hc
