---
default: patch
---

# Open a database created without games

A database created without adding a game, such as by `scidtopgn import` of a PGN file with no game it could read, had no `.sn4` name file and couldn't be opened. `Database::create` now writes all three files at once, so the database opens with 0 games. Replacing an existing database, as `import --overwrite` does, now replaces its name file too; previously, if no game was added, `scidtopgn info` still reported the old database's names.
