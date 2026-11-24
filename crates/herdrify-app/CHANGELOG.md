# Changelog

All notable changes to `herdrify-app` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows
[Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-11-24

First release: an async client for herdr-app and models of its layout, theme and keys files, tested
against `herdr-app` 0.1.17.

### Added

- `HerdrApp`, the entrypoint: it runs the `herdr-app` program, checks its version once and
  reaches every command from plain values.
  - `HerdrApp::local`, and `HerdrApp::builder` with `.program`, `.home`, `.env` and `.env_remove`;
    `HerdrApp::from_client` takes a `Client` over any transport, and `HerdrApp::client` reaches the
    `Client` underneath.
  - Operations grouped by what they act on: `spaces()`, `tabs()`, `panes()`, `terminals()`,
    `agents()`, `layouts()`, `mods()`, `overlays()` and `window()`, plus `ls`, `status` and
    `commands`.
  - Every operation returns a `Call`: chain its options, then `.await` it. A value a command
    refuses becomes `Error::InvalidArgument` at `.await`, and nothing runs.
- `prelude`, the names most programs need in one import.
- `Client`: one connection to the `herdr-app` program, with typed commands.
  - `Client::version`, `Client::check_version`, `Client::commands`, `Client::ls` and
    `Client::status`, and `Client::run` for any typed command of `protocol`.
  - `Client::raw` sends any invocation and returns the reply as JSON.
- `protocol`: the 77 commands of `herdr-app commands` as typed values, with their replies, the
  window listing and status, and the version of the program.
- `Transport`, the seam to whatever runs the program, and `ProcessTransport`, which runs
  `herdr-app`, so a test can supply its own transport.
- `files`: the layout, theme and keys files as plain data, with no I/O. `Layout`, `Theme` and
  `Keys` each parse from text and write back to JSON, and keep every key they do not name.
- Errors: `Error` for calls, `TransportError` for transports and `FileError` for files.
- Live tests that open a `herdr-app` window of their own, with its own home and `hc` session, and
  skip when no window can open.
- Examples, one per use case, each with a README: `inspect`, `drive` and `files`.
- Minimum supported Rust version: 1.94.

[0.1.0]: https://github.com/rstlix0x0/herdrify/releases/tag/herdrify-app-v0.1.0
