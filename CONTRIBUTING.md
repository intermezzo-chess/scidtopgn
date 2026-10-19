# Contributing

## Setup

Install stable Rust with [rustup](https://rustup.rs/), including the `rustfmt`
and `clippy` components. The workspace declares its minimum supported Rust
version in `rust-version` in the root `Cargo.toml`. Do not use language or
standard library features newer than that version unless the change raises it
deliberately. Run `cargo build` from the repository root to fetch dependencies
and build both crates.

> **AI assistance:** Agentic development is welcome in this repository if you follow solid practices and your agents produce quality code. Have your agent read [AGENTS.md](AGENTS.md) before making changes. It is the repository's instruction entry point and links to the [Rust](docs/RUST.md) conventions. If you use [bb](https://getbb.app/), [.bb/AGENTS.md](.bb/AGENTS.md) handles everything for you. Repository-specific instructions take precedence over the supporting conventions. Review AI-assisted changes and run the same checks as any other contribution.

## Before opening a PR

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Fill in the [pull request template](.github/pull_request_template.md) and
reference the issue the PR addresses.

## Architecture notes

- **Format logic belongs in the library.** `crates/scidtopgn` reads and writes
  SCID databases and parses PGN. `crates/scidtopgn-cli` is a thin `clap`
  wrapper around it. If a CLI change needs new format behavior, add it to the
  library and call it from the CLI.
- **Each SCID file has its own module.** `index.rs` handles the `.si4` index,
  `nfile.rs` and `namebase.rs` handle the `.sn4` name base, and `gfile.rs`
  handles the `.sg4` game file. Move and game decoding live in `game.rs`,
  `mov.rs`, `position.rs`, and `private/decoding.rs`. `database.rs` ties the
  three files together. See [SCID_DATABASE_FORMAT.md](SCID_DATABASE_FORMAT.md)
  for the binary layout.
- **The C++ converter in `scid-cpp/` is the behavioral reference.** It is built
  from the original SCID encoding and decoding sources. When there is a
  question about how SCID or PGN import and export should behave, such as
  which games to skip or how to encode a move, match `scid-cpp/src/main.cpp`.
  `scidvspc/` is a local reference checkout and is not tracked.
- **The library uses structured errors.** It returns `scidtopgn::Error`, which
  is defined with `thiserror` in `error.rs`. Only the CLI uses `anyhow`. Don't
  flatten library errors into strings.

## Test data

The fixtures in `tests/data/` (`one` and `five`, each with `.si4`, `.sn4`,
`.sg4`, and a matching `.pgn`) are shared by both crates' integration tests.
`.gitignore` excludes `*.si4`, `*.sn4`, and `*.sg4` everywhere, so new fixtures
must be added with `git add -f`.

Tests must not modify the tracked fixtures. To exercise an altered database,
such as one with a deleted game or a corrupted record, copy the fixture into a
`tempfile::TempDir` and change the copy.

## The C++ reference converter (`scid-cpp`)

Building the C++ converter is optional. It is useful when you need to compare
the Rust output with the reference output directly:

```sh
cd scid-cpp
mkdir -p build && cd build
cmake ..
make
```

To build the C++ unit tests, download `doctest.h` first. The steps are in
[scid-cpp/README.md](scid-cpp/README.md).

## Releasing

There is no release automation yet (see [#22](https://github.com/intermezzo-chess/scidtopgn/issues/22)).

## Commands

| Command                                                 | What it does                                                                                |
| ------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `cargo build` / `cargo build --release`                 | Builds the library and the `scidtopgn` binary.                                              |
| `cargo run -p scidtopgn-cli -- <database>`              | Exports a database to PGN on stdout without installing. Pass the database path without its extension. |
| `cargo run -p scidtopgn-cli -- import <pgn> -o <db>`    | Imports a PGN file into a SCID database.                                                    |
| `cargo test --workspace`                                | Runs the unit and integration tests for both crates.                                        |
| `cargo test -p scidtopgn` / `cargo test -p scidtopgn-cli` | Runs one crate's tests.                                                                   |
| `cargo fmt --all`                                       | Formats the workspace with rustfmt.                                                         |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lints the workspace and treats warnings as errors.                                          |
| `cargo doc --workspace --no-deps --open`                | Builds and opens the API documentation.                                                     |
