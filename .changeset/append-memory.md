---
default: patch
---

# Append to a large database without holding its index twice

Adding games to a database that was opened, such as with `scidtopgn import` to an existing database, no longer copies the whole index when the first game is added. An append to a database of 4.9 million games now uses about as much memory as `info` on it, under 300 MB instead of over 500 MB.
