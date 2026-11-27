---
default: patch
---

# End a comment at a NUL byte, as SCID does

A NUL byte in a comment no longer shifts the comments after it: the comment ends at the NUL, as in SCID, and the text after it is dropped, with a warning. The same applies to text before a game. No write stores a comment with a NUL, whether the game comes from PGN or from the library.
