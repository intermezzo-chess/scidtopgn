---
default: patch
---

# Report PGN games that fail to import instead of dropping the rest

`scidtopgn import` no longer stops at the first game it can't parse. With `--on-error skip`, the default, it warns with the game's number, line, and reason, counts the game as skipped, and imports the games after it. With `--on-error abort`, it exits with an error that names the game, before changing the database.

In the library, `PgnParser` is an iterator over each game or its error, and resumes at the next game after an error. `parse_pgn` returns the first failed game's error instead of returning the games before it.
