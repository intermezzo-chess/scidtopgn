---
default: patch
---

# Reject index files that aren't SCID format version 400, as SCID vs. PC does

Opening a database whose index file is of a SCID format version other than 400 now fails with the new `Error::UnsupportedVersion`, which holds the version found, instead of reading the file as version 400. `IndexHeader::read`, `Index::open`, `Database::open`, and `Database::open_read_only` all return it, the last three wrapped in `Error::File` for `DatabaseFile::Index`. No file is changed.

The CLI's `export`, `info`, `validate`, `merge`, and `import` (appending to an existing database) report the database, the index file, the version found, and that only SCID vs. PC 4 databases (version 400) are supported.
