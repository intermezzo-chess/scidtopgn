---
default: patch
---

# Keep at most 7 NAGs on a move, as SCID does

SCID keeps a move's first 7 NAGs and drops the rest. PGN import kept 8, so SCID dropped the 8th when it read the stored game. Import now keeps 7 and warns about each NAG it drops, with the new `PgnWarningKind::TooManyNags`. Reading a SCID database also keeps the first 7, without a warning, as SCID does.

`MAX_NAGS` is still 8, the size of SCID's array of a move's NAGs, so a move keeps `MAX_NAGS - 1`. A move whose `nags` are set directly to more than 7 NAGs, or to NAG 0, is now stored and exported with only the first 7 that aren't 0, and the index's NAG count counts only those, so SCID reads back each NAG stored.
