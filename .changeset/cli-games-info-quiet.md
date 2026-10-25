---
default: minor
---

# Export selected games, show database info, and quiet the CLI

- `scidtopgn export --games 10-20` exports only the given games, numbered from 1. It takes numbers and ranges separated by commas, such as `1-5,8`, and still skips logically deleted games.
- `scidtopgn info` shows a database's total, deleted, and active games, its player, event, site, and round names, and the years of its active games. It reads only the index and name files.
- `--quiet` (`-q`) on export, import, and `info` writes nothing to stderr, not even errors. The exit status still shows whether the command failed.
- `--verbose` now reports progress at each 10% of the games, such as `Processed 50% (50/100 games)`, instead of every 100 games.
- The help and README now say that `--count` includes logically deleted games, so it can be more than the number of games exported.
