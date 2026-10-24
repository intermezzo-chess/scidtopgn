---
default: major
---

# Keep text before a PGN game as its first comment instead of reading it as a game

As in SCID, a PGN game now starts at its first tag line. Text before it, such as notes at the start of a file or between games, is kept as the comment before the game's first move. Text after the last game is dropped. Before, the text was read as a game, which failed: `scidtopgn import` reported an extra skipped game and numbered every later game one too high, and with `--on-error abort` it couldn't import the file.

Moves with no tags are now text too. Before the next game they become its first comment, and at the end of the input they are dropped, so a file of games without tags imports no games. `PgnParser::parse_game` and `parse_single_game` still read moves without tags.
