---
default: patch
---

# Set the index fields SCID searches by, and keep half-move counts above 255

`Database::add_game` and `Database::update_game` now set the index fields SCID derives from a game's moves, as SCID does: the half-move count, the home pawn data, the final material signature, the promotion and underpromotion flags, and the variation, comment, and NAG counts. The promotion flags are also stored in the game record. Earlier versions left these fields at 0, so SCID's board search, tree, and novelty search couldn't find the games past the starting position.

`IndexEntry` now keeps half-move counts up to 1,023, as SCID stores them. Earlier versions lost everything above 255 when reading, setting, or writing a count, so any change to a database SCID wrote cut the counts of its games longer than 255 half-moves.

A database written by an earlier version keeps the empty fields until its games are replaced with `update_game` or imported again.
