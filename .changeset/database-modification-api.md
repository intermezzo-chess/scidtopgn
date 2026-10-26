---
default: minor
---

# Replace, delete, and undelete games in a database

`Database::update_game` replaces a game with another and keeps its number. As in SCID, the new record is added to the end of the `.sg4` file and the old one stays there unused, the game keeps its user-settable flags, including the delete flag, and the names it no longer uses lose 1 from their frequency but stay in the name file.

`Database::delete_game` sets a game's logical delete flag, and `Database::undelete_game` clears it. The game's record, its number, and the name frequencies don't change. Like adding a game, these changes are written by `flush` or `close`.

`Database::add_game` no longer changes name frequencies when it fails.
