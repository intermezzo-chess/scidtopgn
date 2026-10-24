---
default: patch
---

# Reject games with variations nested too deeply or too many variations of one move

A game whose variations are nested more than 100 levels deep, or that has more than 255 variations of one move, is now rejected with an error, whether it is read from PGN or from a SCID database. Deeper nesting overflowed the stack and aborted the whole process, even with `--on-error skip`. More than 255 variations of one move panicked in debug builds, and in release builds lost the game's comments when it was read back.
