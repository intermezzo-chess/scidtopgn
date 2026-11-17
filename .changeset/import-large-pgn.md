---
default: patch
---

# Import a large PGN file without holding every game in memory

`scidtopgn import` stores each game as it reads it, instead of reading the whole PGN file and every game first, so its memory no longer grows with the number of games in the file, from a file or from stdin. `--on-error=abort` still leaves an existing database as it was, and creates no new one. A PGN file or stdin that can't be read part way stops the import the same way, whichever `--on-error` is passed.

With `--verbose`, import reports progress at each 10% of the PGN file read, with the number of games read so far, or, from stdin, whose size isn't known, the number of games read once a second. It no longer reports `Parsed N games from PGN`, and warnings about games that can't be read now come in order with the rest, as each game is read.
