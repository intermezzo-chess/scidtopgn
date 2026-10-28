---
default: patch
---

# Keep games and databases readable when tags or names are too long to store

SCID stores the lengths of tag names, tag values, and player, event, site, and round names in one byte. Longer text used to make data that couldn't be read back: a game whose tag value was longer than 255 bytes was lost, and a name cut in the middle of a character, or two names that differ only after byte 255, made the database fail to open in `scidtopgn` or in SCID.

- PGN import drops a tag whose name or value is longer than 255 bytes, with a warning, and imports the rest of the game, as SCID does. This includes `White`, `Event`, and the other tags with their own fields, which then stay `?`.
- `Database::add_game` and `update_game` return the new `Error::TagTooLong` for a tag value longer than 255 bytes, and change nothing.
- A tag name longer than 240 bytes is cut to 240 when the game is stored, as in SCID. It used to be dropped.
- `NameBase::add_name` cuts a name longer than 255 bytes to 255 before looking it up, as SCID does, so names that start with the same 255 bytes share one name. `find_name` cuts the same way.
- A cut ends before a character it would split.
