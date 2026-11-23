---
default: patch
---

# Merge names stored two ways without holding the index twice

Writing a database whose name file holds the same name in two encodings, such as Latin-1 and UTF-8 from different versions of SCID, merges the two names and renumbers the games that use them. The index is now renumbered as it is written, instead of copied first. On a database of 4.9 million games, that write, such as the first `scidtopgn import` to it, now uses about 330 MB instead of 550 MB; another append uses about 285 MB. If writing fails, the database and the `Database` are left as before.
