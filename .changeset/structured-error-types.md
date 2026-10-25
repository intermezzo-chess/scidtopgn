---
default: major
---

# Report damaged SCID databases with structured errors instead of panicking or using messages

A damaged SCID database, or a game too large to store, no longer panics. These panicked before: a name in the `.sn4` file that claims to share more bytes with the previous name than it has, an index entry with zero length, and adding a game whose record is longer than a game file block. A game with more than 65,535 half-moves in its main line, such as a long run of null moves, no longer panics either; `Game::num_half_moves` stops at 65,535. A comment or starting FEN that ends without its terminator is now an error instead of being read as if it were complete.

In the library, `Error` is `#[non_exhaustive]`, and SCID errors are variants that callers can match on, such as `CorruptGame`, `GameOutOfRange`, `InvalidMagic`, `Truncated`, `NamePrefixTooLong`, `VariationTooDeep`, `TooManyVariations`, `IndexFull`, `NameBaseFull`, and `GameTooLarge`. `CorruptGame` gives the byte offset in the game record and a `CorruptGameKind`. This includes a null move in check and a starting position that can't be read, which returned `InvalidMove` and `InvalidFen` without an offset before. `Database::get_game` wraps its errors in `Error::Game`, which numbers games from 1, and returns `GameNotFound` for a game past the end. Opening or creating a database file wraps its errors in `Error::File`, which names the file as a `DatabaseFile`. `Error::Io` now displays only the I/O error.

The `Decode`, `Database`, `CorruptData`, `Encoding`, `Buffer`, `BufferRead`, `NameNotFound`, and `EndOfMoveList` variants are removed. `Database::add_game` rejects a game whose record is longer than 131,071 bytes, the most the index can record.

`scidtopgn export` names the database it can't open and the game it can't read, for example `SCID game 3` followed by `Damaged game data at byte 17: …`.
