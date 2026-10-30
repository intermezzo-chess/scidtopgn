---
default: minor
---

# Merge SCID databases

`scidtopgn merge db1 db2 -o merged` copies every game of the source databases, in order, into a new database, as SCID's `scmerge` does. Each game's record and index entry are copied as stored, without converting the game to PGN and back, so logically deleted games are copied and stay deleted. The new database holds each name its games use, stored as UTF-8, with frequencies that count its games, and has a new index header. The sources are opened read-only. `merge` refuses an output database that exists, unless `--overwrite` is given, and one that is also a source. It writes the new database beside the output path and moves it into place only once it is complete, so after an error no new database is left and an existing one is unchanged. It accepts `--verbose`, `--quiet`, and `--log`.

In the library, `Database::copy_game` copies a game from another database as stored.

When `import --overwrite` or `merge --overwrite` can't move one of the new database's files into place, the old database's files, and with `merge` its tree cache, are now put back. Before, the old database could be left with some of its files replaced. A directory at one of the output database's paths is now an error before any file is moved.
