---
default: minor
---

# Report PGN games that import can't store, and leave the database unchanged when aborting

`scidtopgn import` now always warns about a game it parsed but can't add to the database, such as one whose record is longer than 131,071 bytes. The warning names the game by its number and first line, as for a game that fails to parse, and the game is counted as skipped. Before, the warning appeared only with `--verbose` and didn't name the game.

With `--on-error abort`, the error now names the game, and the database is left as it was. Before, the game file kept the records of games added before the failure, and `--overwrite` had already replaced the existing database. A new database, with or without `--overwrite`, is now written beside its path and moved into place when the import succeeds.

In the library, `PgnParser::game_number` and `PgnParser::game_line` give the number and first line of the game the parser last read. `Database::discard` closes a database without writing its changes, and cuts the game file back so that it holds no records that its index doesn't point to.
