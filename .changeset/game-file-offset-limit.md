---
default: patch
---

# Refuse a game that would start past 4 GiB in the game file

An index entry stores where its game's record starts in the game file in 32 bits. Adding a game whose record would start past that, about 4 GiB into the game file, now fails with the new `Error::GameFileFull`. Before, the offset wrapped around, and the index pointed at the wrong record.
