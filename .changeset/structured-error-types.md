---
default: major
---

# Report damaged SCID databases with structured errors instead of panicking or using messages

A damaged SCID database, or a game too large to store, no longer panics. These panicked before: a name in the `.sn4` file that claims to share more bytes with the previous name than it has, an index entry with zero length, and adding a game whose record is longer than a game file block.

In the library, `Error` is `#[non_exhaustive]`, and SCID errors are variants that callers can match on, such as `CorruptGame`, `GameOutOfRange`, `InvalidMagic`, `Truncated`, `NamePrefixTooLong`, `VariationTooDeep`, `TooManyVariations`, `IndexFull`, `NameBaseFull`, and `GameTooLarge`. `CorruptGame` gives the byte offset in the game record and a `CorruptGameKind`. `Database::get_game` wraps its errors in `Error::Game`, which numbers games from 1, and returns `GameNotFound` for a game past the end. Opening or creating a database file wraps its errors in `Error::File`, which names the file as a `DatabaseFile`. `Error::Io` now displays only the I/O error.

The `Decode`, `Database`, `CorruptData`, `Encoding`, `Buffer`, `BufferRead`, `NameNotFound`, and `EndOfMoveList` variants are removed. `Database::add_game` rejects a game whose record is longer than 131,071 bytes, the most the index can record.

`scidtopgn export` names the database it can't open and the game it can't read, for example `SCID game 3` followed by `Damaged game data at byte 17: …`.
