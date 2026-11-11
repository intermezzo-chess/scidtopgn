# Benchmark results, 2026-11-11

`bench/run` on Caissabase 2022-12-24, for #109. Its output follows unchanged, apart from the headings.

## Summary

- Every operation works on the full 4.9 million games, and SCID vs. PC opens both databases after scidtopgn appends to them. Its tree search finds 1.e4 in the same 2,355,300 games in both.
- Opening a database holds its index and names: about 275 MB at this size. `info` and the header search take under a second, `validate` about 2.5 minutes, and exporting every game about 3 minutes.
- Importing the 488 files one at a time takes 8 minutes, with at most 800 MB for each run. Each run rewrites the whole index and name file, so a run takes longer as the database grows: 0.62 s for the second file, and 1.44 s for the 487th. Over all 488 files that is 14.9 times the sample's time for 9.9 times the games, just within the check's limit of 1.5 times.
- Importing all the files at once, as one PGN file, was stopped at 32 GB: import holds every parsed game in memory, about 25 KB each. This is #124.
- `pgnscid` stores 5 more games than scidtopgn: Chess960 games, which scidtopgn skips on purpose, as [PGN_IMPORT.md](PGN_IMPORT.md) #32 says.

The check messages for the explained difference were reworded after this run, without changing what they check.

## Results

| | |
| --- | --- |
| Commit | 475db30b27824b8be5dd508481fde5c931a83d23 |
| Run | 2026-11-11 16:27 UTC to 2026-11-11 16:52 UTC |
| Machine | MacBookPro18,4, macOS 26.6.2, arm64 |
| CPU | Apple M1 Max, 10 cores |
| RAM | 65536 MB; memory limit 32768 MB |
| Free disk | 239700 MB in the work directory at the start |
| Rust | rustc 1.99.1 (58cf8f9c3 2026-10-13) |
| SCID vs. PC | 4.26 |
| Input | 488 files, 3566 MB, in Caissabase_2022_12_24; sample: the first 49 files; chunk: 488.pgn |
| Input checksum | 4d550e51bec1155dafd899abebb330363b7fbc280eb09a624007b76fdbc16575 |
| Import | 5 games skipped, 5 warnings, 0 names that aren't UTF-8 |

The inputs are read just before each size is measured, so they are in the OS file cache if memory allows.

| Size | Database | Operation | Games | Time (s) | Peak memory (MB) | Status |
| --- | --- | --- | ---: | ---: | ---: | --- |
| sample | pgnscid | import (pgnscid) | 490000 | 10.22 | 18 | ok |
| sample | scidtopgn | import one at a time | 490000 | 32.16 | 287 | ok |
| sample | scidtopgn | import all at once | 490000 | 29.84 | 12391 | ok |
| sample | scidtopgn | info | 490000 | 0.06 | 30 | ok |
| sample | scidtopgn | export | 490000 | 20.55 | 31 | ok |
| sample | scidtopgn | validate | 490000 | 15.16 | 31 | ok |
| sample | scidtopgn | header search | 490000 | 0.09 | 31 | ok |
| sample | scidtopgn | append | 490000 | 0.30 | 115 | ok |
| sample | pgnscid | info | 490000 | 0.06 | 30 | ok |
| sample | pgnscid | export | 490000 | 20.42 | 32 | ok |
| sample | pgnscid | validate | 490000 | 15.23 | 31 | ok |
| sample | pgnscid | header search | 490000 | 0.10 | 31 | ok |
| sample | pgnscid | append | 490000 | 0.29 | 116 | ok |
| full | pgnscid | import (pgnscid) | 4874120 | 95.08 | 165 | ok |
| full | scidtopgn | import one at a time | 4874115 | 478.93 | 797 | ok |
| full | scidtopgn | import all at once |  | 41.32 | 32844 | stopped at the memory limit |
| full | scidtopgn | info | 4874115 | 0.62 | 275 | ok |
| full | scidtopgn | export | 4874115 | 183.55 | 277 | ok |
| full | scidtopgn | validate | 4874115 | 143.75 | 277 | ok |
| full | scidtopgn | header search | 4874115 | 0.88 | 279 | ok |
| full | scidtopgn | append | 4874115 | 1.19 | 592 | ok |
| full | pgnscid | info | 4874120 | 0.64 | 275 | ok |
| full | pgnscid | export | 4874120 | 186.06 | 278 | ok |
| full | pgnscid | validate | 4874120 | 141.00 | 277 | ok |
| full | pgnscid | header search | 4874120 | 0.88 | 278 | ok |
| full | pgnscid | append | 4874120 | 1.16 | 591 | ok |

### Checks

- PASS  scidtopgn import one at a time: 478.93 s and 797 MB, 14.9x and 2.8x the sample, for 9.9x the games
- FAIL  scidtopgn import all at once: stopped at the memory limit
- PASS  info on the scidtopgn database: 0.62 s and 275 MB, 10.3x and 9.2x the sample, for 9.9x the games
- PASS  export on the scidtopgn database: 183.55 s and 277 MB, 8.9x and 8.9x the sample, for 9.9x the games
- PASS  validate on the scidtopgn database: 143.75 s and 277 MB, 9.5x and 8.9x the sample, for 9.9x the games
- PASS  header search on the scidtopgn database: 0.88 s and 279 MB, 9.8x and 9.0x the sample, for 9.9x the games
- PASS  append on the scidtopgn database: 1.19 s and 592 MB, 4.0x and 5.1x the sample, for 9.9x the games
- PASS  info on the pgnscid database: 0.64 s and 275 MB, 10.7x and 9.2x the sample, for 9.9x the games
- PASS  export on the pgnscid database: 186.06 s and 278 MB, 9.1x and 8.7x the sample, for 9.9x the games
- PASS  validate on the pgnscid database: 141.00 s and 277 MB, 9.3x and 8.9x the sample, for 9.9x the games
- PASS  header search on the pgnscid database: 0.88 s and 278 MB, 8.8x and 9.0x the sample, for 9.9x the games
- PASS  append on the pgnscid database: 1.16 s and 591 MB, 4.0x and 5.1x the sample, for 9.9x the games
- PASS  pgnscid stores 4874120 games, and scidtopgn 4874115 (5 Chess960 games, which scidtopgn skips (PGN_IMPORT.md #32))
- PASS  SCID vs. PC opens both databases after the append
- PASS  SCID vs. PC reads 4878235 games in the scidtopgn database after the append
- PASS  SCID vs. PC reads 4878240 games in the pgnscid database after the append
- PASS  SCID vs. PC's tree search finds 1.e4 in 2355300 games of the pgnscid database, and 2355300 of the scidtopgn database (5 Chess960 games, which scidtopgn skips (PGN_IMPORT.md #32))

16 passed, 1 failed
