# SCID compatibility check

`compat/run` checks that SCID vs. PC 4.26 reads the databases scidtopgn writes, and the SCID-written databases it changes. It compares scidtopgn's files with those SCID vs. PC writes from the same PGN, checks every database it produces with SCID's own code, and runs SCID vs. PC's board and tree searches on them. CI runs it on every pull request, and it is run on each release candidate (see [RELEASING.md](RELEASING.md#prepare-a-release)).

It doesn't test SCID vs. PC's GUI, or any other version of SCID.

## Running it

From the repository root:

```sh
compat/run
```

It builds what it needs, runs every scenario, and prints one line per scenario and input, `PASS` or `FAIL` with the reason and the scenario's log. It ends with the number that passed and failed, and exits with 1 if any failed, or if a tool doesn't build. A full run takes under a minute once the tools are built.

- **More inputs.** `compat/run <pgn-file>...` also runs every scenario on each UTF-8 PGN file given, such as a Caissabase extract kept outside the repository. Each file needs a different name.
- **Work directory.** Everything is written under `target/compat`, or `$COMPAT_WORK` if set. Each run replaces `run/` there. The SCID vs. PC download and the built tools are kept for the next run.
- **Toolchain.** The workspace's `rust-version` applies. Set `CARGO` to choose a toolchain, such as `CARGO="cargo +1.99.1" compat/run`.

Inspect a failure in the scenario's directory, `target/compat/run/<input>/`. Each database there has its three files and, once checked, `<db>.pgn` (the reference converter's export), `<db>.dump` (scid-dump's output), and the `.diff` of the comparison that failed.

### Prerequisites

macOS, on Apple silicon or Intel:

- Xcode Command Line Tools, for the C++ compiler, zlib, and Tcl 8.5
- CMake 3.15 or later, such as `brew install cmake`
- Rust, as in [CONTRIBUTING.md](../CONTRIBUTING.md#setup)

Linux on x86_64, as on Debian or Ubuntu:

```sh
sudo apt-get install build-essential cmake zlib1g-dev tcl-dev pkg-config curl
```

Plus Rust. Set `TCL_CFLAGS` and `TCL_LIBS` to use a Tcl that `pkg-config` doesn't find.

## The tools

| Tool | Source | What it does |
| --- | --- | --- |
| `scidtopgn` | `crates/scidtopgn-cli` | Imports, appends, and merges |
| `compat_driver` | `crates/scidtopgn/examples/compat_driver.rs` | Deletes, undeletes, and updates games with the library, which the CLI doesn't offer |
| Reference converter | `scid-cpp/src/main.cpp` | Reads every game of a database back to PGN with SCID's decoder |
| `scid-dump` | `scid-cpp/tools/scid_dump.cpp` | Prints every index entry and name, and checks them with SCID's code (below) |
| `pgnscid` | SCID vs. PC 4.26 | Writes the SCID databases scidtopgn's are compared with |
| `tcscid` | SCID vs. PC 4.26 | SCID vs. PC without its GUI. Runs the Tcl scripts in `compat/tcl` |

`compat/run` builds them all. To build them on their own:

- `cargo build --release --locked -p scidtopgn-cli -p scidtopgn --bin scidtopgn --example compat_driver`
- The reference converter and `scid-dump`: as in [CONTRIBUTING.md](../CONTRIBUTING.md#the-c-reference-converter-scid-cpp). Both are written to `scid-cpp/build/`.
- `compat/build-scidvspc [<work>]` downloads [`scid_vs_pc-4.26.tgz`](https://sourceforge.net/projects/scidvspc/files/source/) from SourceForge, checks its SHA-256, and builds `pgnscid` and `tcscid` into `<work>/scidvspc/bin`. It compiles the objects SCID vs. PC's `Makefile.conf` lists, without its `configure`, which doesn't work on macOS. It skips the download and the build when the binaries match the tarball, the script, the compiler, and Tcl.

`scid-dump` checks that:

- each game record reads and decodes;
- SCID's `Game::Encode` of each decoded game gives the same record bytes, and the same derived index fields: half-moves, start, promotion flags, counts of variations, comments and NAGs, material signature, result, ECO code, and home-pawn data;
- each entry's name IDs are in range;
- each name's frequency equals the number of entries that use it, deleted games included;
- `FindExactName` finds each name at its own ID.

### Signed `char`

SCID compares and hashes names as C `char`s, which are signed on x86 and Apple arm64, where SCID databases are written. Every SCID tool here is built with `-fsigned-char`, so the check gives the same result on hosts where `char` is unsigned, such as Linux on arm64. Without it, SCID's code would reject name files with names that aren't ASCII.

### Tcl

`tcscid` links the system Tcl: 8.5 on macOS and 8.6 on Linux. The searches the check runs don't depend on the Tcl version.

## Inputs

The inputs are the PGN files of the fixtures in `tests/data`, `compat/data/names.pgn`, and an empty PGN made at run time. `names.pgn` has names that aren't ASCII as players, events, sites, and rounds, ordered so that signed and unsigned bytes sort them differently. Its second game has a `;` comment line and a `%` escape line between its tags, and comments over several lines, with lines that start with `[`. None of these may end its tags or start a game when the check finds where games start. It is also the PGN every scenario appends, merges, and takes new games from.

Two fixtures are left out, because scidtopgn imports them differently from SCID on purpose, as [PGN_IMPORT.md](PGN_IMPORT.md) describes: `notation.pgn` holds those cases, and `search.pgn` has the player `van der Wiel, John`, whose prefix SCID capitalizes.

`latin1` and `mixed`, whose names aren't UTF-8, are run from their committed databases instead of their PGN, because `pgnscid` would convert their names. The reference converter that wrote them counts no names, so each is first repaired by SCID vs. PC (`sc_base fixCorrupted`), which recounts them and keeps their bytes.

## Scenarios

`scid` below is the database `pgnscid` writes from the input, and `scid+names` the one it writes from the input followed by `names.pgn`. Every database a scenario produces must also be read by the reference converter with no warnings, pass `scid-dump`, and open in `tcscid`. "Same searches" means that SCID vs. PC's board search and tree search find the same number of games as in the given SCID-written database, for the position after 2, 6, and 12 half-moves of each of its games, and that at least one search finds a game.

| Scenario | What scidtopgn does | Passes when |
| --- | --- | --- |
| `reference` | Nothing | `scid` and `scid+names` pass the checks themselves |
| `import` | Imports the input into a new database | Its files are byte-identical to `scid`'s, and it has the same searches |
| `overwrite` | Imports `names.pgn`, then the input with `--overwrite` | Its files are byte-identical to `scid`'s |
| `append` | Appends `names.pgn` to a copy of `scid` on which SCID set a description, type, auto-load game, custom flag names, and flags on the first three games | The original index entries are byte-identical, SCID reads the same header fields and flags, the appended entries and the names match `scid+names`, and it has `scid+names`'s searches |
| `merge` | Merges `scid` and `names.pgn`'s SCID database | Its entries, names, and games read back match `scid+names`, and it has the same searches |
| `delete` | Deletes the first, middle, and last games of a copy of `scid` | Its files are byte-identical to a copy in which SCID deleted the same games, and SCID reads the rest |
| `undelete` | Deletes those games, then undeletes them | Its files are byte-identical to `scid`'s |
| `update-same-names` | Replaces the middle game with other moves, keeping its tags | Its games read back, entries, and names match `pgnscid`'s database from the PGN with that game replaced, apart from game-file offsets and names left unused. Every name it had before the update is still there, and it has the same searches. Before the update, SCID writes its tree cache for the old and new positions, which must not give the old results |
| `update-new-names` | The same, with `names.pgn`'s first game | As above |
| `update-long` | The same, with the 72 KB second game of `long.pgn` | As above |
| `update-twice` | Replaces the same game twice | As above, for the second game |
| `append`, `delete`, `update` on `latin1` and `mixed` | As above, on the repaired database | SCID reads it, every game keeps its names as [README.md](../README.md#names-that-arent-utf-8) describes, and the appended entries match `names.pgn`'s SCID database |
| `self-check` | Nothing | The checks fail for a database whose half-move count and home-pawn data are zeroed, as before #55 |

The empty input runs only `reference` through `merge`. An update must leave the names it no longer uses in the name file, with a frequency of 0, until the database is compacted, as SCID does (#72). For `update-twice`, that includes the names the first update added.

## CI

The **SCID compatibility** job in `ci.yml` runs `compat/run` on `ubuntu-latest`, and `ci-success` requires it. It caches the SCID vs. PC tarball, keyed on its SHA-256, and the built `pgnscid` and `tcscid`, keyed on the runner's architecture and image and on `compat/build-scidvspc`. Caches saved by runs on `main` are restored by pull requests.

`compat/build-scidvspc` downloads nothing when the tools are up to date, so SourceForge is contacted only when the tools must be built and the tarball isn't cached: when the job hasn't run for 7 days, when the tarball's hash changes, or when GitHub evicts the cache. If SourceForge is unavailable then, the job fails with `could not download`, and passes when rerun once SourceForge is back. That risk is accepted (#105).

## Changing the check

- To support a new version of SCID vs. PC, change `version`, `url`, and `sha256` in `compat/build-scidvspc`, and the key of the source cache in `ci.yml`.
- When scidtopgn imports a fixture differently from SCID on purpose, document it in [PGN_IMPORT.md](PGN_IMPORT.md) and leave the fixture out of `inputs` in `compat/run`, saying why.

## History

The check repeats two manual checks made before 1.0. [SCID_COMPAT_RESULTS_2026-10-26.md](SCID_COMPAT_RESULTS_2026-10-26.md) records the first, on `main` at `73e532e`, which found #55, #75, and #76. The scripts and paths it names were never committed.
