---
default: patch
---

# Read and write variations and null moves like SCID

Games with variations no longer fail to load from a SCID database, and each variation starts from the position before the move it replaces. A move with more than one variation is written the way SCID writes it. Null moves (`--`, `Z0`, or `null` in PGN) are imported and exported as SCID null moves. A null move while in check is an error.
