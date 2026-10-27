---
default: patch
---

# Encode record tags as SCID does, so game files are the same size

`Database::add_game` and `Database::update_game` now write a game record's tags as SCID does. SCID's ten common tags, such as `Annotator`, `Opening`, and `SetUp`, are stored as one-byte codes instead of by name, and the event date is stored only in the index entry instead of also in the record. A database imported from PGN now has the same game records as one SCID vs. PC's `pgnscid` writes from the same PGN, and its game file is about 8% smaller than before.

As in SCID, an event date the index can't hold is dropped: one with an unknown year, one more than 3 years from the game's year, or any event date when the game's year is unknown. Earlier versions kept it in the record. Records written by earlier versions still read the same.
