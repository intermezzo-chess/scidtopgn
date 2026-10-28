---
default: minor
---

# Export a database that can't be written to

`scidtopgn` export and `--count` failed with `Permission denied` for a database whose `.sg4` game file couldn't be written, such as one marked read-only or on read-only media, though they only read it. They now open the database with the new `Database::open_read_only`, which needs no write permission on any of its files.

A database opened with `Database::open_read_only` can't be changed: `add_game`, `update_game`, `delete_game`, and `undelete_game` return `Error::ReadOnly` and change nothing, and `flush` and `close` write nothing. `GFile::open_read_only` opens a game file the same way. `Error::ReadOnly` now reads "Database is open read-only".
