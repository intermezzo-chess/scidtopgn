---
default: patch
---

# Delete SCID's tree cache when a game is replaced or a database is overwritten

SCID uses a cached tree as long as the database has the same number of games, so a replaced game could show the old game's tree statistics. Writing the changes of `update_game` now deletes the database's `.stc` file, as SCID does when it saves a game, just before the new name and index files replace the old ones. A cache that exists but can't be deleted is reported as an error, and the changes stay unwritten so that `flush` can try again.

`Database::create` and `import --overwrite` now delete the `.stc` file of the database they replace too, because its new games could reach the old count. Because of this, `import` refuses a `--log` path that is the output's `.stc` file, as `merge` does.

Adding, deleting, and undeleting games, deleting twins, and compacting names still keep the cache.
