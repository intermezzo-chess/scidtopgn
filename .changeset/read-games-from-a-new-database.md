---
default: patch
---

# Read back games from a new database before closing it

`GFile::create`, and so `Database::create`, opened the new `.sg4` game file for writing only. Once a 128 KiB block of game data had been written to the file, `get_game` for a game in that block failed with an I/O error, and so did `add_game` after a read of an earlier block. The file is now opened for reading and writing, as SCID does, so every game reads back before the database is closed.
