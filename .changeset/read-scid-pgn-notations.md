---
default: major
---

# Read the PGN notations SCID reads, and report what import changes

PGN import reads the notations SCID reads: a pawn capture without a rank (`ed`), `:` for a capture, `-` between squares, coordinates for any piece (`g1f3`, `e1g1`), a king moving two squares (`Kg1`), castling without dashes (`OO`, `00`), lowercase or missing promotion pieces (`a8q`, `a8`), en passant markers (`exd6 e.p.`), annotation symbols such as `!?`, `+=`, `N`, and `D` as NAGs, and results written `1:0`, `0:1`, or `1/2`. Like SCID, castling after the king or rook has moved and returned is played, and export reads SCID databases that contain such a castle. The result after the moves replaces the `Result` tag. `--+` is NAG 21, not a null move with a check sign.

Problems that don't stop a game from being imported are reported as warnings, which the CLI prints to stderr: castling without the castling right, `O-O` played as `O-O-O`, unknown text in the moves, castling rights and en passant squares dropped from a `FEN`, a result that replaces the `Result` tag, and moves that don't end with a result. Text that isn't a move, an annotation, or a result is skipped with a warning instead of being dropped silently or failing the game.

PGN export writes each NAG as the symbol SCID writes when SCID and `scidtopgn` both read it back as the same NAG, such as `!`, `+=`, or `N`, and as `$n` otherwise. Previously most NAGs, such as $20 and $41, were dropped from the export, and others were written as Unicode symbols, such as `⩲`, that couldn't be imported again. `nag_to_symbol` returns these symbols, or `""` for a NAG written as `$n`.

[docs/PGN_IMPORT.md](https://github.com/intermezzo-chess/scidtopgn/blob/main/docs/PGN_IMPORT.md) lists each notation and where `scidtopgn` differs from SCID.

Breaking changes to the library API:

- `Game` has a new public field, `warnings: Vec<PgnWarning>`.
- `ScidPosition::play`, `encode_move`, and `decode_move` accept castling without the castling right when castling would otherwise be legal. `ScidPosition::can_play` and `ScidPosition::castle_without_right` are new.
- `PgnWarning` and `PgnWarningKind` are new.
