---
default: patch
---

# Keep NAGs and comments on the move they follow, and export variations

PGN import and SCID database reading now attach each NAG and comment to the move it follows, as SCID does, in the main line and in variations. Previously they attached to the move before, moved one move earlier each time a database was read back, and were dropped inside variations. A comment before the first move of the game or of a variation is kept, and several comments after one move are joined. A NAG before the first move is dropped, as in SCID. PGN export now writes variations.
