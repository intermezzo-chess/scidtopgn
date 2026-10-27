---
default: patch
---

# Write names with non-ASCII bytes in the order SCID requires

The name file now lists names in SCID's order: by first byte, then by bytes compared as signed, as SCID does on x86 and Apple arm64. Earlier versions sorted names with a byte of 0x80 or above, such as accented names, in a different order. SCID then couldn't open the database ("Error opening name file") or couldn't look its names up, even after only deleting a game in a database SCID wrote.

To repair a database written by an earlier version, make any change to it with this version, then `close` it, which rewrites the name file.
