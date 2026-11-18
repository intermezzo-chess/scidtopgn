# Benchmark results, 2026-11-17, after #138

`bench/run` on Caissabase 2022-12-24, for #138. Its output follows unchanged, apart from the headings.

## Summary

- Every check passes, 18 of 18. In the [previous run](BENCHMARK_RESULTS_2026-11-17.md), three failed.
- An append no longer holds the database's index twice. On all the files, it peaks at 284 MB, against 507 MB before and 275 MB for `info` on the same database, so its memory grows 8.9 times for 9.9 times the games, instead of 15.8 times. Its time is the same. It writes byte for byte the same files as before, which was checked by appending the chunk to copies of the full database with the build before and after the change.
- The import one at a time peaks at 409 MB, against 580 MB before. 485 of its 488 runs grow steadily with the database, up to 283 MB with 4.86 million games, 3% more than `info` needs for the full database. Three runs use about one more copy of the index: 311.pgn (343 MB), 370.pgn (408 MB), and 371.pgn (409 MB). 369.pgn and 370.pgn add a name with a control character, `M\x01nster Open`. Reading the name file removes the control character, so the next append reads that name as a second copy of `Mnster Open`, which the database already has, and writing the database merges the two, which builds a renumbered copy of the index. The run of 311.pgn is most likely the same, after a name in 310.pgn that reads back differently, but which name wasn't found. Appending 311.pgn, 370.pgn, or 371.pgn to the full database, which has no such pair of names, peaks at 285 MB. These runs are left as they are: they come only after an append that adds such a name, and the check passes.
- The time of the import one at a time is no longer checked, as [BENCHMARK.md](BENCHMARK.md#the-import-one-at-a-time) explains. It took 465.7 s, 15.3 times the sample, as before.

## Results

| | |
| --- | --- |
| Commit | 4621d1cc312d202df2efb1e44d47294c0d5c2d45 |
| Run | 2026-11-17 20:13 UTC to 2026-11-17 20:42 UTC |
| Machine | MacBookPro18,4, macOS 26.6.2, arm64 |
| CPU | Apple M1 Max, 10 cores |
| RAM | 65536 MB; memory limit 32768 MB |
| Free disk | 237115 MB in the work directory at the start |
| Rust | rustc 1.99.1 (58cf8f9c3 2026-10-13) |
| SCID vs. PC | 4.26 |
| Input | 488 files, 3565 MB, in Caissabase_2022_12_24; sample: the first 49 files; chunk: 488.pgn |
| Input checksum | 4d550e51bec1155dafd899abebb330363b7fbc280eb09a624007b76fdbc16575 |
| Import | 5 games skipped, 5 warnings, 0 names that aren't UTF-8 |

The inputs are read just before each size is measured, so they are in the OS file cache if memory allows.

| Size | Database | Operation | Games | Time (s) | Peak memory (MB) | Status |
| --- | --- | --- | ---: | ---: | ---: | --- |
| sample | pgnscid | import (pgnscid) | 490000 | 9.81 | 17 | ok |
| sample | scidtopgn | import one at a time | 490000 | 30.47 | 32 | ok |
| sample | scidtopgn | import all at once | 490000 | 27.84 | 34 | ok |
| sample | scidtopgn | info | 490000 | 0.06 | 30 | ok |
| sample | scidtopgn | export | 490000 | 20.18 | 32 | ok |
| sample | scidtopgn | validate | 490000 | 15.00 | 31 | ok |
| sample | scidtopgn | header search | 490000 | 0.09 | 31 | ok |
| sample | scidtopgn | append | 490000 | 0.28 | 32 | ok |
| sample | pgnscid | info | 490000 | 0.06 | 30 | ok |
| sample | pgnscid | export | 490000 | 20.07 | 32 | ok |
| sample | pgnscid | validate | 490000 | 14.87 | 31 | ok |
| sample | pgnscid | header search | 490000 | 0.09 | 31 | ok |
| sample | pgnscid | append | 490000 | 0.28 | 32 | ok |
| full | pgnscid | import (pgnscid) | 4874120 | 91.93 | 165 | ok |
| full | scidtopgn | import one at a time | 4874115 | 465.72 | 409 | ok |
| full | scidtopgn | import all at once | 4874115 | 252.42 | 289 | ok |
| full | scidtopgn | info | 4874115 | 0.62 | 275 | ok |
| full | scidtopgn | export | 4874115 | 181.28 | 277 | ok |
| full | scidtopgn | validate | 4874115 | 139.26 | 277 | ok |
| full | scidtopgn | header search | 4874115 | 0.91 | 279 | ok |
| full | scidtopgn | append | 4874115 | 1.15 | 284 | ok |
| full | pgnscid | info | 4874120 | 0.66 | 275 | ok |
| full | pgnscid | export | 4874120 | 186.75 | 277 | ok |
| full | pgnscid | validate | 4874120 | 140.65 | 278 | ok |
| full | pgnscid | header search | 4874120 | 0.92 | 277 | ok |
| full | pgnscid | append | 4874120 | 1.21 | 284 | ok |

### Checks

- PASS  scidtopgn import one at a time: 465.72 s and 409 MB, 15.3x and 12.8x the sample, for 9.9x the games
- PASS  scidtopgn import all at once: 252.42 s and 289 MB, 9.1x and 8.5x the sample, for 9.9x the games
- PASS  info on the scidtopgn database: 0.62 s and 275 MB, 10.3x and 9.2x the sample, for 9.9x the games
- PASS  export on the scidtopgn database: 181.28 s and 277 MB, 9.0x and 8.7x the sample, for 9.9x the games
- PASS  validate on the scidtopgn database: 139.26 s and 277 MB, 9.3x and 8.9x the sample, for 9.9x the games
- PASS  header search on the scidtopgn database: 0.91 s and 279 MB, 10.1x and 9.0x the sample, for 9.9x the games
- PASS  append on the scidtopgn database: 1.15 s and 284 MB, 4.1x and 8.9x the sample, for 9.9x the games
- PASS  info on the pgnscid database: 0.66 s and 275 MB, 11.0x and 9.2x the sample, for 9.9x the games
- PASS  export on the pgnscid database: 186.75 s and 277 MB, 9.3x and 8.7x the sample, for 9.9x the games
- PASS  validate on the pgnscid database: 140.65 s and 278 MB, 9.5x and 9.0x the sample, for 9.9x the games
- PASS  header search on the pgnscid database: 0.92 s and 277 MB, 10.2x and 8.9x the sample, for 9.9x the games
- PASS  append on the pgnscid database: 1.21 s and 284 MB, 4.3x and 8.9x the sample, for 9.9x the games
- PASS  importing all at once stores as many games as one at a time: 4874115
- PASS  pgnscid stores 4874120 games, and scidtopgn 4874115: scidtopgn skips 5 Chess960 games, as PGN_IMPORT.md #32 says
- PASS  SCID vs. PC opens both databases after the append
- PASS  SCID vs. PC reads 4878235 games in the scidtopgn database after the append
- PASS  SCID vs. PC reads 4878240 games in the pgnscid database after the append
- PASS  SCID vs. PC's tree search finds 1.e4 in 2355300 games of the pgnscid database, and 2355300 of the scidtopgn database

18 passed, 0 failed
