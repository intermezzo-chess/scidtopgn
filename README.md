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
>
> With a subcommand, put options after it: `scidtopgn export mydb --games 1-5`, not `scidtopgn --games 1-5 export mydb`. Options before a subcommand are rejected with the corrected command.

## CLI usage

### Export (SCID → PGN)

```bash
scidtopgn /path/to/database              # stdout
scidtopgn /path/to/database -o out.pgn   # to file
scidtopgn /path/to/database --count      # just tell me how many games are stored
scidtopgn /path/to/database --games 10-20  # only games 10 to 20
scidtopgn export /path/to/database       # explicit subcommand, same thing
```

Export skips games that SCID has logically deleted. `--count` counts every stored game, deleted or not, so it can be more than the number of games exported. `scidtopgn info` shows both numbers.

`--games` takes game numbers, counting from 1, and ranges that include both ends, separated by commas, such as `1-5,8,12-14`. The games are exported in database order, each once, and deleted games in the selection are still skipped. A number past the last stored game is an error, and no output is written.

#### Export arguments

| Argument | Short | Required | Default | Description |
|----------|-------|----------|---------|-------------|
| `database` | - | Yes | - | Path to SCID database (without file extension) |
| `--output` | `-o` | No | stdout | Output file path |
| `--count` | - | No | - | Print the number of stored games, including logically deleted games, and exit |
| `--games` | - | No | all | Export only these games, such as `10-20` or `1-5,8`. Can't be combined with `--count` |
| `--overwrite` | - | No | - | Overwrite the output file if it already exists |
| `--verbose` | `-v` | No | - | Show progress at each 10% of the games, and a summary |
| `--quiet` | `-q` | No | - | Write nothing to stderr, not even errors. Can't be combined with `--verbose` |

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
| `--verbose` | `-v` | No | - | Show progress at each 10% of the games, and a summary |
| `--quiet` | `-q` | No | - | Write nothing to stderr, not even errors or skipped games. Can't be combined with `--verbose` |

With `--quiet` and the default `--on-error skip`, import doesn't report the games it skips, and it still exits with status 0. Use `--on-error abort` to make a bad game an error.

### Database info

```bash
scidtopgn info /path/to/database
```

```text
Total games: 5
Deleted games: 2
Active games: 3
Player names: 10
Event names: 3
Site names: 3
Round names: 5
Years of active games: 2022-2022
```

Total games counts every stored game, and Active games those export writes. The name counts are the names stored in the database's name file, which can include names used only by deleted games. The years come from active games with a known date, or are `unknown`. `info` reads only the index and name files, so it works even when some games are damaged. It accepts `--quiet`.

### Quiet mode

With `--quiet`, a command writes nothing to stderr: no warnings, progress, or error messages. Only the exit status shows whether it failed. Its results are still written, such as PGN, the `--count` line, and `info`'s output. Mistakes in the arguments themselves, such as an unknown option, are still reported.

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

`games()` and `get_game()` include games that SCID has logically deleted, unlike the CLI's export. The [API documentation](https://docs.rs/scidtopgn) shows how to skip them and covers the rest of the library. To build it locally, run `cargo doc -p scidtopgn --open`.

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
