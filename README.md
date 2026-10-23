![scidtopgn](https://raw.githubusercontent.com/intermezzo-chess/scidtopgn/main/assets/scidtopgn.png)

A Rust library and CLI for converting between SCID (.si4/.sn4/.sg4) and PGN formats. Fast, accurate, and functional on Windows, Mac, and Linux.

![animation showing scidtopgn in the terminal](https://raw.githubusercontent.com/intermezzo-chess/scidtopgn/main/assets/scidtopgn.gif)

## Install

Download an archive for your platform from [GitHub Releases](https://github.com/intermezzo-chess/scidtopgn/releases). Each release lists SHA-256 checksums in `SHA256SUMS`.

Or install from [crates.io](https://crates.io/crates/scidtopgn-cli) with Rust 1.97 or later:

```bash
cargo install scidtopgn-cli --locked
```

`--locked` uses the dependency versions the release was tested with. Without it, Cargo may pick newer dependencies that need a newer Rust.

## Quick start (CLI)

```bash
# dump a SCID database to PGN
scidtopgn /path/to/database

# save it to a file
scidtopgn /path/to/database -o output.pgn

# go the other direction — PGN into a fresh SCID database
scidtopgn import games.pgn -o mydb
```

> [!NOTE]
> Specify database paths *without* extensions — use `mydb`, not `mydb.si4`.

## CLI usage

### Export (SCID → PGN)

```bash
scidtopgn /path/to/database              # stdout
scidtopgn /path/to/database -o out.pgn   # to file
scidtopgn /path/to/database --count      # just tell me how many games
scidtopgn export /path/to/database       # explicit subcommand, same thing
```

#### Export arguments

| Argument | Short | Required | Default | Description |
|----------|-------|----------|---------|-------------|
| `database` | - | Yes | - | Path to SCID database (without file extension) |
| `--output` | `-o` | No | stdout | Output file path |
| `--count` | - | No | - | Just print the number of games and exit |
| `--overwrite` | - | No | - | Overwrite the output file if it already exists |
| `--verbose` | `-v` | No | - | Verbose output |

### Import (PGN → SCID)

```bash
scidtopgn import games.pgn -o mydb               # create or append
scidtopgn import games.pgn -o mydb --overwrite    # start fresh
scidtopgn import - -o mydb                        # read from stdin
scidtopgn import games.pgn -o mydb --on-error=abort  # strict mode
```

By default, import appends to an existing database. Bad games get skipped with a warning unless you pass `--on-error=abort`.

Import reads the same PGN notation as SCID, such as `0-0`, `ed` for `exd5`, `e2-e4`, or `N` for a novelty. Problems that don't stop a game from being imported, such as castling after the king has moved and returned, are printed as warnings. [docs/PGN_IMPORT.md](docs/PGN_IMPORT.md) lists each notation and where `scidtopgn` differs from SCID.

#### Import arguments

| Argument | Short | Required | Default | Description |
|----------|-------|----------|---------|-------------|
| `pgn_file` | - | Yes | - | PGN file to import (use `-` for stdin) |
| `--output` | `-o` | Yes | - | Output database path (without file extension) |
| `--overwrite` | - | No | - | Overwrite existing database instead of appending |
| `--on-error` | - | No | `skip` | Error handling strategy: `skip` or `abort` |
| `--verbose` | `-v` | No | - | Verbose output |

## Use it as a library

```rust
use scidtopgn::Database;

let mut db = Database::open("path/to/database")?;

for game in db.games() {
    let game = game?;
    println!("{}", game.to_pgn());
}

// or grab a specific game
let game = db.get_game(0)?;
println!("{} vs {}", game.white, game.black);
```

## Building & testing

```bash
cargo build              # debug
cargo build --release    # optimized
cargo test --workspace   # run the full suite
cargo run -p scidtopgn-cli -- /path/to/database  # run without installing
```

## SCID format docs

If you're the kind of person who wants to understand the binary format, see `SCID_DATABASE_FORMAT.md`.

## License

Licensed under the [GNU General Public License, version 3 or later](https://github.com/intermezzo-chess/scidtopgn/blob/main/LICENSE) (GPL-3.0-or-later). Parts of the library are derived from [Scid](https://scid.sourceforge.net/) and [Scid vs. PC](https://scidvspc.sourceforge.net/) by Shane Hudson and others.
