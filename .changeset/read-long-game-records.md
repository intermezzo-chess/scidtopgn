---
default: patch
---

# Read games whose SCID records are longer than 64 KiB

Reading a game whose record is between 65,536 and 131,071 bytes from a SCID database, such as a game with long comments, read only the first `length % 65536` bytes. Usually its comments were cut short without an error; otherwise reading it failed. This affected databases written by SCID too. A record that crosses a 128 KiB block boundary of the `.sg4` file is now read, as SCID reads it, instead of failing.

In the library, `GFile::read_game_data` takes its length as a `u32`, and `GFile::add_game` rejects records longer than 131,071 bytes, the most the index can record, instead of 131,072.
