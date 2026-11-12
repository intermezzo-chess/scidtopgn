---
default: major
---

# `scidtopgn info` needs the database's game file to exist

`info` still reads only the index and name files, so a damaged or read-only `.sg4` file works as before. A missing one is now an error, as it is for export.
