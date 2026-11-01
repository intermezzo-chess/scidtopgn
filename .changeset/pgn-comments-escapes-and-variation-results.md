---
default: minor
---

# Single-space PGN comments, escape tag values on export, and keep games with a result inside a variation

PGN import now treats four cases as [docs/PGN_IMPORT.md](https://github.com/intermezzo-chess/scidtopgn/blob/main/docs/PGN_IMPORT.md) describes:

- As in SCID, each run of whitespace in a comment, including line breaks, becomes one space, and whitespace at the ends is dropped.
- A result inside a variation is skipped with the new `PgnWarningKind::ResultInVariation` warning, and the game is imported. Before, the game was skipped with `Unterminated variation`. SCID ends the game at the result instead.
- A `}` or `]` outside a comment gives a `skipped unknown text` warning, as SCID also warns. Before, it was skipped without a warning.
- A `\` in a tag value before a character other than `"` or `\` is kept, as in SCID, so `C:\games` is no longer read as `C:games`.

PGN export writes `"` and `\` in tag values as `\"` and `\\`, as the PGN standard specifies, so `scidtopgn` reads its own export back unchanged. Before, a value with `"` gave PGN that couldn't be imported again. SCID imports the exported games, but reads `\\` as two backslashes and ends a value at `\"`.
