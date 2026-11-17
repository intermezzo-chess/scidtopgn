---
default: major
---

# Read PGN from a reader, holding only the game being read

`PgnParser::from_reader` reads games from any `Read + Send`, such as a file or `std::io::stdin()`, a block at a time. The parser holds the text of the game it is reading, not of the games before it, so its memory no longer grows with the size of the input. `PgnParser::new` and `from_bytes` read the same way, without copying their input, and give the same games, warnings, and errors as before, however the input is split into reads.

`PgnParser` now has a lifetime parameter, `PgnParser<'a>`, for the input it borrows. Code that names the type, such as a struct field, needs `PgnParser<'_>`, or `PgnParser<'static>` for a parser of an owned reader, and a parser of borrowed text can't outlive it. A read that fails, other than with `ErrorKind::Interrupted`, which is retried, yields `Error::Io` in place of the game being read, and ends the games.
