---
default: patch
---

# Write index and name changes safely, so a failed write can't damage the database

`Database::flush` and `close`, and so appending with `scidtopgn import`, used to truncate the database's `.si4` and `.sn4` files and write them again, so a failure part way left a database that couldn't be opened. They now write new files in a temporary directory beside the database, and replace the originals only once both are complete. After a failed `flush`, the `Database` keeps its changes, so calling `flush` again writes them.

Writing changes now needs a writable directory and disk space for the new files alongside the old ones. It also fails if the `.si4` or `.sn4` file is read-only or a symbolic link, instead of replacing it. `scidtopgn import` names the database when writing it fails.
