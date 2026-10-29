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

Run the same checks as CI. `--locked` fails if `Cargo.lock` would change.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p scidtopgn --no-deps --locked
cargo test --workspace --locked
```

### Record the change

If users of the library or the CLI can notice the change, add a changeset. Install [Knope](https://knope.tech) 0.23.0 with `cargo install knope --version 0.23.0 --locked`, then run:

```sh
knope document-change
```

Choose `major` for a breaking change, `minor` for a backward-compatible addition, or `patch` for a fix, and enter a one-line summary. Commit the new `.changeset/*.md` file with the change. Changes to tests, CI, or contributor documentation don't need a changeset. [docs/RELEASING.md](docs/RELEASING.md#record-a-change) explains the file format and how each type bumps a 0.x version.

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
  `mov.rs`, and `position.rs`. `database.rs` ties the
  three files together. See [SCID_DATABASE_FORMAT.md](SCID_DATABASE_FORMAT.md)
  for the binary layout.
- **Chess rules come from [shakmaty](https://crates.io/crates/shakmaty).**
  Legality, SAN, check and mate, and FEN all use it. `position.rs` adds only
  SCID's piece numbering to a shakmaty position, and `mov.rs` only SCID's move
  encoding. Don't add chess rules to the crate.
- **The C++ converter in `scid-cpp/` is the behavioral reference.** It is built
  from the original SCID encoding and decoding sources. When there is a
  question about how SCID or PGN import and export should behave, such as
  which games to skip or how to encode a move, match `scid-cpp/src/main.cpp`.
  [docs/PGN_IMPORT.md](docs/PGN_IMPORT.md) records where PGN import
  deliberately differs, and why. Update it when you change how PGN is read.
  `scidvspc/` is a local reference checkout and is not tracked.
- **The library uses structured errors.** It returns `scidtopgn::Error`, which
  is defined with `thiserror` in `error.rs`. Only the CLI uses `anyhow`. Don't
  flatten library errors into strings.

## Test data

The fixtures in `tests/data/` (`one`, `five`, `edge`, `castling`, `annotated`, `long`, `accents`, `derived`, `latin1`, and `mixed`, each
with `.si4`, `.sn4`, `.sg4`, and a matching `.pgn`) are shared by both crates'
integration tests. `castling` and `annotated` were written by the C++ reference
converter from `castling.pgn` and `annotated.pgn`, like `edge`. `annotated` has
NAGs and comments in the main line and in variations. `long` was written from
`long.pgn` by `pgnscid -f long.pgn long`, SCID vs. PC's PGN converter built from
`scidvspc/`, because the C++ reference converter can't encode a record over
4,096 bytes. Its second game's record is 72,015 bytes, more than a 16-bit length
holds. SCID keeps at most 16,000 characters of each PGN comment, so its text is
split over several comments. `accents` was written from `accents.pgn` by
`pgnscid -f accents.pgn accents`. Its names sort differently as signed and as
unsigned bytes, to check that the name file keeps SCID's order. `derived` was
written from `derived.pgn` by `pgnscid -f derived.pgn derived`. It covers the
index fields SCID derives from the moves: games longer than 255 and 1,023
half-moves, promotions in the main line and in a variation, and more than 10
variations, comments, and NAGs. `latin1` and `mixed` were written by the C++
reference converter from `latin1.pgn` and `mixed.pgn`, which aren't valid UTF-8.
The converter stores their names' bytes as they are, and counts no names, so
every frequency in their name files is 0. `latin1.pgn` has Latin-1 and
Windows-1252 names. `mixed.pgn` has `Müller, Hans` and `Événement` both as
Latin-1 and as UTF-8, so `mixed.sn4` holds each of them twice.
`notation.pgn` has one game per case in
[docs/PGN_IMPORT.md](docs/PGN_IMPORT.md). `.gitignore` excludes `*.si4`, `*.sn4`, and `*.sg4` everywhere, so new
fixtures must be added with `git add -f`.

`edge` covers variations, null moves, promotions, en passant, castling, and
games that start from a FEN. The C++ reference converter wrote it from
`edge.pgn`, so the tests compare the Rust reader and writer with SCID's own
encoding. To change it:

1. Edit `edge.pgn`.
2. Delete `edge.si4`, `edge.sn4`, and `edge.sg4`.
3. From the repository root, run the converter (see below) as
   `scidtopgn tests/data/edge tests/data/edge.pgn`.
4. Update the expected movetext in `crates/scidtopgn/tests/test_interop.rs`.
   It is python-chess's export of `edge.pgn`, with variations and without
   line wrapping.

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

See [docs/RELEASING.md](docs/RELEASING.md).

## Commands

| Command                                                 | What it does                                                                                |
| ------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `cargo build` / `cargo build --release`                 | Builds the library and the `scidtopgn` binary.                                              |
| `cargo run -p scidtopgn-cli -- <database>`              | Exports a database to PGN on stdout without installing. Pass the database path without its extension. |
| `cargo run -p scidtopgn-cli -- import <pgn> -o <db>`    | Imports a PGN file into a SCID database.                                                    |
| `cargo test --workspace --locked`                       | Runs the unit and integration tests for both crates.                                        |
| `cargo test -p scidtopgn` / `cargo test -p scidtopgn-cli` | Runs one crate's tests.                                                                   |
| `cargo fmt --all`                                       | Formats the workspace with rustfmt.                                                         |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Lints the workspace and treats warnings as errors.                                 |
| `cargo doc --workspace --no-deps --open`                | Builds and opens the API documentation.                                                     |
| `knope document-change`                                 | Adds a changeset describing a change.                                                       |
| `.github/scripts/run-tests`                             | Tests the release workflow's scripts in `.github/scripts/`.                                 |
