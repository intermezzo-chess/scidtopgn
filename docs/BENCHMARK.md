# Benchmark

`bench/run` measures how long scidtopgn's operations take, and how much memory they use, on a large set of PGN files such as [Caissabase](#caissabase). It checks that each operation works, and that its time and memory grow in proportion to the number of games. It also checks that SCID vs. PC 4.26 reads the databases it produces. It isn't run in CI.

Run it after changing how scidtopgn imports, appends, opens, or exports databases, and compare its results with the [last recorded results](#recorded-results).

## Running it

From the repository root:

```sh
bench/run <pgn-dir>
```

It reads every `*.pgn` file in `<pgn-dir>`, in the numeric order of their names (`1.pgn`, `2.pgn`, ... `488.pgn`), and needs at least two. It measures each operation twice: on a sample of the files, and on all of them. It prints one line per check, `PASS` or `FAIL`, and the number that passed and failed. It exits with 1 if any check failed, or if a tool doesn't build.

| Option | Default | What it sets |
| --- | --- | --- |
| `--sample N` | 10% of the files, rounded up | The number of files, from the first, in the sample |
| `--chunk FILE` | The last file | The PGN file appended to each database |
| `--memory-limit MB` | Half the machine's RAM | The most memory an operation may use. An operation that uses more is stopped |

- **Work directory.** Everything is written under `target/bench`, or `$BENCH_WORK` if set. Each run replaces `run/` there. It needs free disk space of about 4 times the size of the input, and it refuses to start without it. For Caissabase, that's about 14 GB.
- **SCID vs. PC.** `pgnscid` and `tcscid` are built by `compat/build-scidvspc` where `compat/run` keeps them, in `target/compat` or `$COMPAT_WORK`, so the two scripts share them. [SCID_COMPAT.md](SCID_COMPAT.md#prerequisites) lists what building them needs.
- **Toolchain.** The workspace's `rust-version` applies. Set `CARGO` to choose a toolchain, such as `CARGO="cargo +1.99.1" bench/run <pgn-dir>`.
- **Platforms.** It has been run on macOS. It has code for Linux, where the measurements need GNU time at `/usr/bin/time`, such as from `apt-get install time`, but it hasn't been run there yet. It doesn't run on Windows. #126 covers both.
- **Duration.** All of Caissabase takes about half an hour on an Apple M1 Max. A run on a few of its files, such as a directory with links to `1.pgn`, `2.pgn`, and `3.pgn`, takes under a minute, and checks the script itself.

### Caissabase

[Caissabase](http://caissabase.co.uk/) is a free database of chess games, distributed as a SCID database. The recorded results use its release of 2022-12-24, with about 4.9 million games, exported to 488 PGN files of up to 10,000 games each, `1.pgn` to `488.pgn`, 3.5 GB in all. The site may offer only a later release. Any similar set of files works, and the results give the checksum of the files used, so that a later run can tell whether it used the same.

## What it measures

For each size, the sample and all the files, it measures in order:

1. `pgnscid` writes a database from the files, concatenated into one PGN file. Its time and memory are the baseline, and aren't checked.
2. scidtopgn imports the files **one at a time**: `import` the first into a new database, then append each of the others in a run of its own. This is the scidtopgn database.
3. scidtopgn imports the concatenated PGN **all at once**, in one `import`. This database is then deleted.
4. On the scidtopgn database, then on the `pgnscid` database:
   - `info`;
   - `export` of every game to a file;
   - `validate`;
   - a header search, `export <db> --white carlsen -o <file>`;
   - an append of the chunk, with `import <chunk> -o <db>`. The chunk is one of the files, so its games are then stored twice.

Each operation's time is its wall time, and its memory its peak resident set size, both from `/usr/bin/time`. For the import one at a time, the time is the total of the runs, and the memory the largest of them. Then, on all the files, SCID vs. PC opens both databases and runs a tree search for the position after 1.e4.

## Checks

| Check | Passes when |
| --- | --- |
| Each operation, on all the files | It exits with 0, and it isn't stopped at the memory limit. Its time and its memory are each at most 1.5 times what growing in proportion to the number of games from the sample predicts. A time under 1 second or a memory under 64 MB is too small to compare, and passes |
| Import all at once | It stores as many games as the import one at a time |
| Game counts | `pgnscid` and scidtopgn store the same number of games, apart from the difference [explained](#explained-differences) for the input |
| SCID vs. PC | It opens both databases after the append, and reads as many games in each as scidtopgn reported |
| 1.e4 | SCID vs. PC's tree search finds as many games in both databases, apart from the explained difference |

`pgnscid` and scidtopgn import some PGN differently on purpose, as [PGN_IMPORT.md](PGN_IMPORT.md) describes, so their game counts can differ.

### Explained differences

The list `explained` in `bench/run` gives, for an input identified by its checksum, how many more games `pgnscid` stores than scidtopgn, how many more games its tree search for 1.e4 finds in `pgnscid`'s database, and why. Add an entry only once the reason is known.

## Results

`target/bench/run/` holds:

- `results.md`: the machine, toolchain, commit, and input, a table of every measurement, and the checks;
- `results.tsv`: the measurements, one per line, for other tools;
- `imports.tsv`: each run of the import one at a time, with the number of games in the database before it;
- `checks.txt`: the checks, as printed;
- `sample/` is deleted once measured, to save space. `full/` has the databases and each operation's output, in `<name>.out`, and its log with `/usr/bin/time`'s report, in `<name>.log`.

Times vary between runs and machines, so compare the growth of each operation, which the checks give, rather than its time. The inputs are read just before each size is measured, so they are in the OS file cache if memory allows.

## Recorded results

- [BENCHMARK_RESULTS_2026-11-11.md](BENCHMARK_RESULTS_2026-11-11.md): Caissabase 2022-12-24 on an Apple M1 Max (#109).
