---
default: minor
---

# Import and export games that start from a FEN

PGN import reads the `FEN` tag and plays the game from that position, and export writes `SetUp` and `FEN` tags and numbers the moves from the position, such as `40... Kd7`. A malformed FEN or a position that can't be played is an error. Invalid castling rights and en passant squares in a FEN are dropped. Games whose `Variant` tag names a variant other than standard chess, such as Chess960, are rejected.
