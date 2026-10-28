---
default: minor
---

# Compact a database's games and names

`Database::compact_games` removes logically deleted games and the records that no game uses, such as those that `update_game` replaced, as SCID's Compact games command does. The remaining games are renumbered from 0 in their original order, and their records and index entries are copied as stored. The index header is kept, the name frequencies are counted again, and SCID's `.stc` tree cache is deleted.

`Database::compact_names` removes the names that no game uses, as SCID's Compact names command does, and gives the other names new IDs. As in SCID, a name that only deleted games use is kept, so run `compact_games` first. If every name is used, no file changes.

Both write any unwritten changes first. They write the new files in a temporary directory beside the database, and replace the database's files only when the new files are complete. If a method fails, the database's files are left as they were. A game whose record can't be read stops `compact_games`, and a game that refers to a name the name file lacks stops `compact_names`, with the new `Error::NameNotFound`. `NameBase` now implements `Clone`.
