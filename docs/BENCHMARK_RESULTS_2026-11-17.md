# Benchmark results, 2026-11-17

`bench/run` on Caissabase 2022-12-24, for #124. Its output follows unchanged, apart from the headings, and is followed by a measurement of importing from stdin.

## Summary

- Importing all the files at once, as one 3.5 GB PGN file, now works: 260 s, at most 338 MB, and the same 4,874,115 games as the import one at a time. In #109 it was stopped at the 32 GB memory limit. Import now stores each game as it reads it, so its memory is the database's index and names, not the games of the PGN file. On the sample its time is about the same as before, 28.5 s against 29.8 s, with 33 MB instead of 12,391 MB.
- Importing the same file from stdin takes the same time and memory as from the file, and writes the same database (see [Import from stdin](#import-from-stdin)).
- Three checks fail, on appends, which this change doesn't touch. Before it, each import also held its 10,000 parsed games, about 85 MB, which made the sample's memory 115 MB instead of 32 MB and hid how the memory of an append grows with the database: 59 MB with 1 million games, 206 MB with 2 million, and at most 580 MB, with 3.7 million. The import one at a time takes 478 s, as in #109, but its sample got faster, so its time grows 15.3 times, against the limit of 14.9. Each append rewrites the whole index and name file, which #124 leaves as it is.
- `pgnscid` stores 5 more games than scidtopgn: Chess960 games, which scidtopgn skips on purpose, as [PGN_IMPORT.md](PGN_IMPORT.md) #32 says.

## Results

| | |
| --- | --- |
| Commit | 3574fe2f22fe601f779b10df9296e2e9e8b79284 |
| Run | 2026-11-17 18:24 UTC to 2026-11-17 18:53 UTC |
| Machine | MacBookPro18,4, macOS 26.6.2, arm64 |
| CPU | Apple M1 Max, 10 cores |
| RAM | 65536 MB; memory limit 32768 MB |
| Free disk | 246187 MB in the work directory at the start |
| Rust | rustc 1.99.1 (58cf8f9c3 2026-10-13) |
| SCID vs. PC | 4.26 |
| Input | 488 files, 3565 MB, in Caissabase_2022_12_24; sample: the first 49 files; chunk: 488.pgn |
| Input checksum | 4d550e51bec1155dafd899abebb330363b7fbc280eb09a624007b76fdbc16575 |
| Import | 5 games skipped, 5 warnings, 0 names that aren't UTF-8 |

The inputs are read just before each size is measured, so they are in the OS file cache if memory allows.

| Size | Database | Operation | Games | Time (s) | Peak memory (MB) | Status |
| --- | --- | --- | ---: | ---: | ---: | --- |
| sample | pgnscid | import (pgnscid) | 490000 | 10.35 | 17 | ok |
| sample | scidtopgn | import one at a time | 490000 | 31.22 | 32 | ok |
| sample | scidtopgn | import all at once | 490000 | 28.53 | 33 | ok |
| sample | scidtopgn | info | 490000 | 0.06 | 30 | ok |
| sample | scidtopgn | export | 490000 | 20.70 | 32 | ok |
| sample | scidtopgn | validate | 490000 | 15.31 | 31 | ok |
| sample | scidtopgn | header search | 490000 | 0.10 | 31 | ok |
| sample | scidtopgn | append | 490000 | 0.29 | 32 | ok |
| sample | pgnscid | info | 490000 | 0.06 | 30 | ok |
| sample | pgnscid | export | 490000 | 20.67 | 32 | ok |
| sample | pgnscid | validate | 490000 | 15.34 | 31 | ok |
| sample | pgnscid | header search | 490000 | 0.10 | 31 | ok |
| sample | pgnscid | append | 490000 | 0.29 | 32 | ok |
| full | pgnscid | import (pgnscid) | 4874120 | 95.96 | 170 | ok |
| full | scidtopgn | import one at a time | 4874115 | 477.79 | 580 | ok |
| full | scidtopgn | import all at once | 4874115 | 259.56 | 338 | ok |
| full | scidtopgn | info | 4874115 | 0.70 | 275 | ok |
| full | scidtopgn | export | 4874115 | 188.63 | 277 | ok |
| full | scidtopgn | validate | 4874115 | 138.54 | 277 | ok |
| full | scidtopgn | header search | 4874115 | 0.88 | 278 | ok |
| full | scidtopgn | append | 4874115 | 1.16 | 507 | ok |
| full | pgnscid | info | 4874120 | 0.63 | 275 | ok |
| full | pgnscid | export | 4874120 | 182.79 | 277 | ok |
| full | pgnscid | validate | 4874120 | 138.21 | 277 | ok |
| full | pgnscid | header search | 4874120 | 0.91 | 278 | ok |
| full | pgnscid | append | 4874120 | 1.22 | 507 | ok |

### Checks

- FAIL  scidtopgn import one at a time: time grows 15.3x; memory grows 18.1x: 477.79 s and 580 MB, 15.3x and 18.1x the sample, for 9.9x the games
- PASS  scidtopgn import all at once: 259.56 s and 338 MB, 9.1x and 10.2x the sample, for 9.9x the games
- PASS  info on the scidtopgn database: 0.70 s and 275 MB, 11.7x and 9.2x the sample, for 9.9x the games
- PASS  export on the scidtopgn database: 188.63 s and 277 MB, 9.1x and 8.7x the sample, for 9.9x the games
- PASS  validate on the scidtopgn database: 138.54 s and 277 MB, 9.0x and 8.9x the sample, for 9.9x the games
- PASS  header search on the scidtopgn database: 0.88 s and 278 MB, 8.8x and 9.0x the sample, for 9.9x the games
- FAIL  append on the scidtopgn database: memory grows 15.8x: 1.16 s and 507 MB, 4.0x and 15.8x the sample, for 9.9x the games
- PASS  info on the pgnscid database: 0.63 s and 275 MB, 10.5x and 9.2x the sample, for 9.9x the games
- PASS  export on the pgnscid database: 182.79 s and 277 MB, 8.8x and 8.7x the sample, for 9.9x the games
- PASS  validate on the pgnscid database: 138.21 s and 277 MB, 9.0x and 8.9x the sample, for 9.9x the games
- PASS  header search on the pgnscid database: 0.91 s and 278 MB, 9.1x and 9.0x the sample, for 9.9x the games
- FAIL  append on the pgnscid database: memory grows 15.8x: 1.22 s and 507 MB, 4.2x and 15.8x the sample, for 9.9x the games
- PASS  importing all at once stores as many games as one at a time: 4874115
- PASS  pgnscid stores 4874120 games, and scidtopgn 4874115: scidtopgn skips 5 Chess960 games, as PGN_IMPORT.md #32 says
- PASS  SCID vs. PC opens both databases after the append
- PASS  SCID vs. PC reads 4878235 games in the scidtopgn database after the append
- PASS  SCID vs. PC reads 4878240 games in the pgnscid database after the append
- PASS  SCID vs. PC's tree search finds 1.e4 in 2355300 games of the pgnscid database, and 2355300 of the scidtopgn database

15 passed, 3 failed

## Import from stdin

The 488 files concatenated in order, 3,738,822,950 bytes, imported once from the file and once piped to stdin with `cat`, each into a new database, with the release build of the same commit and `/usr/bin/time -l`:

| Input | Time (s) | Peak memory (MB) |
| --- | ---: | ---: |
| File | 256.81 | 290 |
| stdin | 255.13 | 283 |

The two databases are byte for byte the same.
