---
default: patch
---

# Set stored-line codes as SCID does

`Database::add_game` and `Database::update_game` now set each game's stored-line code as SCID vs. PC 4.26 does: the number of the longest of SCID's 254 stored opening lines that the game's main line starts with, or 0 if none matches or the game starts from a FEN. SCID vs. PC shows the line as the game list's opening moves, and its tree search uses the code to skip games without decoding them. Earlier versions stored 0, so the opening moves were blank and tree searches were slower. With this change, importing the PGN test files and two Caissabase chunks gives files byte-identical to those SCID vs. PC's `pgnscid` writes from the same PGN.

A database written by an earlier version keeps a code of 0 until its games are replaced with `update_game` or imported again.
