---
default: minor
---

# Import PGN files that aren't valid UTF-8

`scidtopgn import` now reads a Latin-1 or Windows-1252 PGN file instead of failing for the whole file with `stream did not contain valid UTF-8`. Each byte that isn't valid UTF-8 is read as Windows-1252 and stored as UTF-8, and each game with such bytes gets the warning `read text that isn't UTF-8 as Windows-1252`. Valid UTF-8 is read as before. In the library, `PgnParser::from_bytes` reads such input, and `PgnWarningKind::NotUtf8` is the new warning.
