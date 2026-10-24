# PGN import compared with SCID

This document lists how `scidtopgn` reads PGN notation that isn't standard, and compares each case with SCID. It uses three PGN terms: SAN (standard algebraic notation) is how moves are written, such as `Nf3` or `exd5`. FEN is the notation for a position, used by the `FEN` tag to set a game's start position. A NAG (numeric annotation glyph) is an annotation written as `$n`, such as `$1` for a good move, or as a symbol, such as `!`. "SCID" means the original SCID sources in `scid-cpp/scid` (`pgnparse.cpp`, `Position::ReadMove` in `position.cpp`, and `game_parseNag` in `game.cpp`), together with the C++ reference converter in `scid-cpp/src/main.cpp`.

Each case is marked **Same** or **Differs** in its **Comparison** column. "Same" means `scidtopgn` reads the input and stores the game the way SCID does, apart from the [general differences](#general-differences). **It doesn't mean that the game comes back unchanged when `scidtopgn` exports it to PGN.** Export writes moves in canonical SAN and can drop parts of the game, as [Export and round trips](#export-and-round-trips) explains. Each difference also says what changes for someone moving from SCID, and why `scidtopgn` behaves that way.

The case numbers match [#37](https://github.com/intermezzo-chess/scidtopgn/issues/37). Most cases are games in [`tests/data/notation.pgn`](../tests/data/notation.pgn), where each game's `Event` tag starts with the case number. Both products' behavior was checked by running them on that file (see [How SCID's behavior was checked](#how-scids-behavior-was-checked)).

## Contents

- [Summary of differences](#summary-of-differences)
- [General differences](#general-differences)
- [Warnings](#warnings)
- [Already accepted by both](#already-accepted-by-both)
- [Move notation](#move-notation)
- [Castling](#castling)
- [Annotation symbols](#annotation-symbols)
- [Results and the start position](#results-and-the-start-position)
- [Known differences outside these cases](#known-differences-outside-these-cases)
- [Export and round trips](#export-and-round-trips)
- [How SCID's behavior was checked](#how-scids-behavior-was-checked)
- [Tests](#tests)

## Summary of differences

`scidtopgn` reads input that SCID reads incorrectly or rejects:

- **#8d** `a8b`, a lowercase bishop promotion without `=`, is read as `a8=B`. SCID rejects the move.
- **#9** `a8=n` promotes to a knight. SCID promotes to a queen and warns.
- **#15** `00` and `000` are castling. SCID drops them as move numbers, without a message.
- **#22** A piece move with `-+` or `-/+` attached, such as `Bb5-+`, is read with its annotation. SCID drops the move, and usually fails at the next one.
- **#22c** `e4--+` and `e4->` are read with the symbol written, $21 and $41. SCID reads `e4--+` as $19 and fails at `e4->`.
- **#23** `e4~` is read with its annotation, `$13`. SCID fails at the move.
- **#28** A game whose moves don't end with a result is imported with a warning, with the `Result` tag's result, or `*` without one. SCID skips it.

`scidtopgn` rejects or changes input where SCID guesses or produces a corrupt game:

- **A5b** A null move while in check is rejected. SCID plays it.
- **#2** A pawn capture without a rank that fits more than one capture is rejected as ambiguous. SCID plays the capture on the lowest rank.
- **#18** Castling with no rook on the board is rejected. SCID moves the king as if castling and produces a corrupt game.
- **#29** A game with a move that can't be read is skipped and reported. SCID keeps the game up to that move and adds an `{Error reading move: ...}` comment.
- **#31** Castling rights and en passant squares in a `FEN` that don't fit the position are dropped, with a warning. SCID keeps them. An en passant square where no capture is possible is dropped from the exported `FEN`, without a warning.
- **#32** A game with a `Variant` tag other than standard chess is rejected. SCID reads its moves as standard chess.

`scidtopgn` reports more:

- **#20** Playing `O-O` as `O-O-O` gives a warning. SCID doesn't warn.
- **#25** Every unknown symbol gives a warning. SCID warns about most, but not about a word that starts with `D`, such as `Diagram`.
- **#27c** A result after the moves gives no warning when the game has no `Result` tag. SCID warns.
- **#27d** An unrecognized `Result` tag, such as `1-0 (time)`, gives a warning only when the result after the moves isn't `*`. SCID reads the tag as `1-0` and warns when the results differ.
- **#33** The CLI prints every warning. The SCID converter never prints them.

PGN export:

- **#34** Each NAG is exported as the symbol SCID writes when both SCID and `scidtopgn` read it back as the same NAG, and as `$n` otherwise. SCID writes some NAGs as symbols it can't read back, such as `→` for $40.
- **#38** Variations, NAGs, and comments are exported where SCID exports them, in the PGN standard's export format, such as `1. e4 ! (1. d4)`. SCID writes `1.e4! ( 1.d4 )`.

Other known differences, which these cases don't cover, are listed in [Known differences outside these cases](#known-differences-outside-these-cases).

## General differences

These apply to every case below.

- **A move that can't be read.** SCID keeps the game up to that move and adds the comment `{Error reading move: ...}`. `scidtopgn` skips the game and reports which game and move failed (#29). When this document says that SCID rejects a move, SCID still keeps the game up to that move.
- **Warnings.** SCID's parser logs warnings, but the SCID converter never prints them. `scidtopgn` prints each warning (#33). See [Warnings](#warnings).
- **Canonical SAN.** `scidtopgn` stores and exports the SAN it generates, with `O-O`, `=` before a promotion piece, `x` for a capture, and check and mate signs only when they are correct. SCID does the same.
- **Where annotations attach (#38).** As in SCID, a NAG or comment attaches to the move it follows, in the main line and in variations. An annotation symbol attaches the same way as the `$n` NAG it stands for. Both products also handle these cases the same way:
  - A NAG or comment after a variation's `)` attaches to the move the variation replaces. In `1. e4 (1. d4) $1`, `$1` is on `1. e4`.
  - A comment before the first move of the game, or right after a variation's `(`, is kept at the start of the game or variation, and exported there.
  - Several comments after one move are joined into one, with a space between them unless the first is empty or ends in whitespace. `1. e4 {a} {b}` stores `a b`.
  - A comment between tags is ignored.
  - **A NAG before the first move of the game or of a variation is dropped, without a warning.** In `$1 1. e4 ($2 1. d4)`, neither NAG is kept. SCID keeps such a NAG in memory, but it neither saves it to the database nor exports it, so the result is the same.
  - `$0` means no annotation, and isn't stored.
- **Variation limits ([issue 4](https://github.com/intermezzo-chess/scidtopgn/issues/4)).** A game with variations nested more than 100 levels deep, or with more than 255 variations of one move, is rejected with an error, whether it is read from PGN or from a SCID database. Reading and writing a game recurse once per level of nesting, so deeper nesting could overflow the stack. SCID has no limit on nesting. It counts a move's variations in a byte, which wraps, so it loses variations after the 255th.

## Warnings

A warning doesn't stop a game from being imported. The CLI prints each warning to stderr, in both `--on-error` modes, as `Warning: PGN game <number>, line <line>: <message>`. A game with warnings isn't counted as skipped, and warnings don't change the exit status. In the library, a game's warnings are in `Game::warnings`, as `PgnWarning` values with the game number, the line, and a `PgnWarningKind`.

A game that fails to parse is reported by its error only, without its warnings.

| Warning | When | Case |
| --- | --- | --- |
| `skipped unknown text "zz"` | Text in the moves that isn't a move, an annotation symbol, or a result, such as `zz`, `Diagram`, `B` on its own, or `nf3`. Also text after the annotation symbols attached to a move, as in `e4!x`. | #12, #25 |
| `"O-O" castles without the castling right` | Castling after the king or rook has moved and returned, or when the `FEN` gives no castling right. | #16 |
| `read "O-O" as O-O-O because only queenside castling is legal` | `O-O` when only queenside castling is legal. | #20 |
| `dropped castling rights "KQkq" from the FEN because they don't fit the position` | Castling rights in the `FEN` without the king or rook on its original square. | #31 |
| `dropped en passant square e3 from the FEN because no pawn could have just moved past it` | An en passant square in the `FEN` that no pawn could have just moved past. | #31 |
| `the result 0-1 after the moves replaces the Result tag 1-0` | The result after the moves differs from the `Result` tag. A tag `scidtopgn` doesn't recognize, such as `1-0 (time)`, counts as `*`, so it gives this warning only when the moves end with another result. | #27 |
| `the moves don't end with a result, so the Result tag 1-0 is used` | The moves end at the next game's tags, or at the end of the input, without a result. | #28a, #28b |
| `the moves don't end with a result, and there is no Result tag` | The same, in a game with no `Result` tag. The game's result is `*`. | #28c |

## Already accepted by both

`scidtopgn` already read these the same way as SCID before #37, except for a null move while in check (A5b).

| # | Case | Example | SCID | scidtopgn | Comparison |
| --- | --- | --- | --- | --- | --- |
| A1 | Castling with zeros or lowercase letters | `0-0`, `o-o` | `O-O` | `O-O` | Same |
| A2 | `P` before a pawn move | `Pe4` | `e4` | `e4` | Same |
| A3 | A missing or extra capture sign | `ed5`, `Nxc3` | `exd5`, `Nc3` | `exd5`, `Nc3` when only one move fits | Same |
| A4 | Pawn moves in coordinates | `e2e4` | `e4` | `e4` | Same |
| A5 | Null moves, when the side to move isn't in check | `--`, `Z0`, `null` | Null move | Null move | Same |
| A5b | A null move while in check | `1. --` from `k7/8/8/8/8/8/4r3/4K3 w - -` | Plays it | Rejects the move | **Differs** |
| A6 | Wrong check or mate signs | `Qh5+` | Corrected | Corrected | Same |
| A7 | Move numbers without dots | `1 e4 e5 2 Nf3` | Ignored | Ignored | Same |
| A8 | `++` for check | `Qh5++` | Corrected | Corrected | Same |

**A5b: a null move while in check.** *What changes:* a game that SCID imports is skipped and reported. *Why:* after a null move in check, the side that moves next could capture the king, so the position can't continue as a game of chess. shakmaty can't represent it, and [#20](https://github.com/intermezzo-chess/scidtopgn/issues/20) chose to reject it.

## Move notation

| # | Case | Example | SCID | scidtopgn | Comparison |
| --- | --- | --- | --- | --- | --- |
| 1a, 1b | Pawn capture without a rank | `ed`, `exd` | `exd5` | `exd5` | Same |
| 1c | Pawn capture without a rank, en passant | `ed` | `exd6` | `exd6` | Same |
| 2 | Pawn capture without a rank that fits two captures | `ed` with pawns on e3 and e5 and pieces on d4 and d6 | Plays `exd4`, the lowest rank, without a warning | Rejects the move as ambiguous | **Differs** |
| 3 | `:` for a capture | `e:d5`, `Q:d5` | `exd5`, `Qxd5` | `exd5`, `Qxd5` | Same |
| 4 | Hyphen between squares | `e2-e4`, `Ng1-f3` | `e4`, `Nf3` | `e4`, `Nf3` | Same |
| 5 | Piece moves in coordinates | `g1f3` | `Nf3` | `Nf3` | Same |
| 6a, 6b | King moves two squares | `Kg1`, `e1g1` | `O-O` | `O-O` | Same |
| 6c | King moves two squares without the castling right | `Kg1` after `Ke2` and `Ke1` | Rejects the move | Rejects the move | Same |
| 7a, 7b | Promotion with no piece | `a8`, `axb8` | `a8=Q`, `axb8=Q` | `a8=Q`, `axb8=Q` | Same |
| 7c | Coordinate promotion with no piece | `a7a8` | Rejects the move | Rejects the move | Same |
| 8a, 8b, 8c, 8e | Lowercase promotion piece | `a8q`, `a7a8q`, `abn`, `a7a8b` | `a8=Q`, `a8=Q`, `axb8=N`, `a8=B` | Same moves | Same |
| 8d | Lowercase bishop promotion without `=` | `a8b` | Rejects the move | `a8=B` | **Differs** |
| 9 | Lowercase piece after `=` | `a8=n` | `a8=Q`, and warns `Invalid annotation symbol: =n` | `a8=N` | **Differs** |
| 10a, 10b | Capture-promotion without a rank | `ab=Q`, `abQ` | `axb8=Q` | `axb8=Q` | Same |
| 11a, 11b | En passant marker | `exd6ep`, `exd6 e.p.` | Ignores the marker | Ignores the marker | Same |
| 12 | Lowercase piece letter | `nf3` | Skips it and logs `Unexpected symbol: nf` | Skips it with a warning | Same |
| 13 | Lowercase `b` for a bishop | `bc4` | Rejects the move | Rejects the move | Same |

**#2: ambiguous pawn capture without a rank.** *What changes:* a game that SCID imports with SCID's guess is skipped and reported. *Why:* SCID tries each rank from 1 to 8 and plays the first capture that fits, which can be the wrong move. `scidtopgn` already rejects other ambiguous moves, such as a missing capture sign that fits two moves.

**#8d: `a8b`.** *What changes:* a game that SCID truncates at `a8b` is imported in full. *Why:* SCID reads the `b` as a file, since its reader allows a second file after a pawn's first. A pawn on the last rank must promote, so `a8b` can only mean `a8=B`. SCID already reads `a7a8b` as `a8=B`.

**#9: `a8=n`.** *What changes:* the pawn promotes to the piece written instead of a queen. *Why:* SCID only reads an uppercase letter after `=`. It reads `=n` as an unknown annotation, then promotes to a queen by default. `scidtopgn` reads lowercase promotion letters everywhere else, so it reads them after `=` too.

## Castling

| # | Case | Example | SCID | scidtopgn | Comparison |
| --- | --- | --- | --- | --- | --- |
| 14 | No dashes | `OO`, `OOO` | `O-O`, `O-O-O` | `O-O`, `O-O-O` | Same |
| 15 | Zeros and no dashes | `00`, `000` | Dropped as move numbers, without a message | `O-O`, `O-O-O` | **Differs** |
| 16a, 16b | After the king or rook has moved and returned | `4. Ke2 Ke7 5. Ke1 Ke8 6. O-O` | Castles and logs `illegal castling (O-O)` | Castles with a warning | Same |
| 16c | With no castling right in the `FEN` | `O-O-O` from `r3k2r/8/8/8/8/8/8/R3K2R w - -` | Castles and logs `illegal castling` | Castles with a warning | Same |
| 16d | Without the castling right, in a variation | `6. d3 (6. O-O)` | Castles and logs `illegal castling` | Castles with a warning | Same |
| 17 | Exporting a SCID database that contains #16 | | Exports `O-O` | Exports `O-O` | Same |
| 18a, 18b | No rook on the board | `O-O` from `4k3/8/8/8/8/8/8/4K3 w - -` | Moves the king to g1 as if castling and logs `illegal castling`. The game is corrupt. | Rejects the move | **Differs** |
| 19 | Through check | `O-O` with a rook on g8 | Rejects the move | Rejects the move | Same |
| 20a | `O-O` when only queenside castling is legal | `O-O` with a knight on g1 | Plays `O-O-O`, without a warning | Plays `O-O-O`, with a warning | **Differs** |
| 20b | `O-O-O` when only kingside castling is legal | `O-O-O` with a knight on b1 | Rejects the move | Rejects the move | Same |

Castling without the castling right (#16) still needs the king and rook on their original squares, the squares between them empty, and the king not in check or passing through an attacked square. Afterward, that side has no castling rights left, as after any castle. Like SCID, `scidtopgn` reads castling in this order:

- `O-O`: kingside, then queenside (#20), then kingside without the castling right (#16).
- `O-O-O`: queenside, then queenside without the castling right.

Only castling written with `O`, `o`, or `0` can castle without the castling right. Like SCID, `Kg1` and `e1g1` castle only when castling is legal (#6c).

Before #37, `scidtopgn` couldn't export a SCID database containing such a castle (#17). It failed with `invalid move byte 0x0a (e1g1 is illegal)`.

**#15: `00` and `000`.** *What changes:* a game written with `00` keeps its castling move. With SCID, the castle disappears, so the game usually fails at the next move. If the next move happens to be legal for the other side, the game is silently wrong. *Why:* SCID reads any token made only of digits as a move number before it checks for castling. `scidtopgn` reads `0-0`, `0-0-0`, `OO`, and `OOO` as castling, so it treats `00` and `000` the same way.

**#18: castling with no rook.** *What changes:* a game that SCID imports in a corrupt state is skipped and reported. *Why:* when SCID retries castling without the castling right, it checks the squares between the king and rook, but not that the rook is there. It moves the king, and the game can't continue correctly. `scidtopgn` lets shakmaty check castling with the right restored, which requires the rook.

**#20: `O-O` played as `O-O-O`.** *What changes:* a warning names the move. *Why:* SCID guesses that the writer meant queenside castling. `scidtopgn` follows SCID, but says so, because the guess changes the move that was written.

## Annotation symbols

`scidtopgn` reads the symbols SCID reads, with the NAGs from SCID's `game_parseNag`, whether the symbol is attached to the move or written separately.

| Symbols | NAG |
| --- | --- |
| `!` `?` `!!` `??` `!?` `?!` | 1, 2, 3, 4, 5, 6 |
| `=` | 10 |
| `~` | 13 |
| `+=` `+/=` | 14 |
| `=+` `=/+` | 15 |
| `+/-` `-/+` | 16, 17 |
| `+-` `-+` | 18, 19 |
| `+--` `--+` | 20, 21 |
| `+>` `->` | 40, 41 |
| `=/&` `~=` | 44 |
| `N` on its own | 146 (novelty) |
| `D` on its own | 201 (diagram) |
| `R` on its own | 144 (various moves) |

`--` is a null move and `o-o` is castling, as in SCID, although `game_parseNag` also lists them as symbols. `--+` is NAG 21, as in SCID. Before #37, `scidtopgn` read `--+` as a null move with a check sign. A null move can have symbols attached, as in `Z0!`, but a word that only starts like one, such as `nullify`, is skipped with a warning (#25c). Several symbols can follow a move, as in `e4!!+-`, which gives NAGs 3 and 18. Check and mate signs among them are ignored. Before #37, `scidtopgn` dropped these symbols without a message, or rejected the game when the symbol started with `=` or `-`.

| # | Case | Example | SCID | scidtopgn | Comparison |
| --- | --- | --- | --- | --- | --- |
| 21a | Attached to the move | `e4! e5? Nf3!! Nc6?? Bb5!? a6?!` | $1 to $6 | $1 to $6 | Same |
| 21b, 21c | After the move | `e4 += e5 =+`, `Bb5 +> a6 ->` | $14, $15, $40, $41 | $14, $15, $40, $41 | Same |
| 21d | After a mate sign | `Qxf7#!` | $1 | $1 | Same |
| 22a | Attached by `=` or `-` to a pawn move | `e4=+`, `Nf3=` | $15, $10 | $15, $10 | Same |
| 22a | Attached by `-` to a piece move | `Bb5-+`, `Ba4-/+` | Logs `Unexpected symbol: Bb5` and drops the move. The next move then usually fails. | `Bb5` with $19, `Ba4` with $17 | **Differs** |
| 22b | Several attached | `e4!!+-` | $3, $18 | $3, $18 | Same |
| 22c | `--+` or `->` attached | `e4--+`, `Nf3->` | `e4` with $19, the symbol after the first `-`. `Nf3->` logs `Unexpected symbol: Nf3` and drops the move. | `e4` with $21, `Nf3` with $41 | **Differs** |
| 23 | `~` attached | `e4~` | Logs `Unexpected symbol: e4`, drops the move, and then fails at the next move | `e4` with $13 | **Differs** |
| 24 | Letters on their own | `N`, `D`, `R` | $146, $201, $144 | $146, $201, $144 | Same |
| 25a | Unknown symbols | `zz`, `/\`, `Diagram` | Skips them; logs `Unexpected symbol` for `zz` and `/\` | Skips them with a warning for each | **Differs** (warning) |
| 25b | A piece letter on its own | `B` | Skips it and logs `Invalid annotation symbol: B` | Skips it with a warning | Same |
| 25c | A word that starts like a null move | `nullify` | Skips it and logs `Unexpected symbol: nullify` | Skips it with a warning | Same |

**#22: a piece move with `-+` attached.** *What changes:* the move and its annotation are kept, instead of a game that breaks after the move. *Why:* SCID splits `-+` and `-/+` from a pawn move but not from a piece move, where its reader rejects the whole token. `scidtopgn` splits them the same way for every move.

**#22c: `--+` or `->` attached.** *What changes:* `e4--+` keeps the symbol written, $21, instead of $19, and `Nf3->` keeps the move and $41 instead of breaking the game. *Why:* SCID ignores a `-` inside a move unless `+` or `/` follows it, so it drops the first `-` of `--+`, and its reader rejects the `>` of `->`. `scidtopgn` treats a `-` as part of the move only between squares, as in `e2-e4`, or at its end, so every symbol reads the same attached or separate.

**#23: `e4~`.** *What changes:* the game is imported with the annotation, instead of being cut at the next move. *Why:* SCID's reader doesn't allow `~` directly after a move. `scidtopgn` reads every symbol either attached or separate.

**#25: `Diagram`.** *What changes:* a warning names the skipped word. *Why:* SCID reads a word that starts with `D` as an annotation, finds no NAG for it, and drops it without a message. `scidtopgn` warns about any text it skips.

## Results and the start position

| # | Case | Example | SCID | scidtopgn | Comparison |
| --- | --- | --- | --- | --- | --- |
| 26a to 26d | Other result notation | `1:0`, `0:1`, `1/2`, `1/2:1/2` | Results | Results | Same |
| 27a | Result after the moves differs from the `Result` tag | `[Result "1-0"]`, moves end `0-1` | Uses `0-1` and logs `Result did not match the header result` | Uses `0-1`, with a warning | Same |
| 27b | `*` after the moves with a `Result` tag | `[Result "1-0"]`, moves end `*` | Uses `*` and logs the mismatch | Uses `*`, with a warning | Same |
| 27c | Result after the moves with no `Result` tag | moves end `1-0` | Uses `1-0` and logs the mismatch | Uses `1-0`, without a warning | **Differs** (warning) |
| 27d | `*` after the moves with an unrecognized `Result` tag | `[Result "1-0 (time)"]`, moves end `*` | Reads the tag as `1-0`, uses `*`, and logs the mismatch | Uses `*`, without a warning, since it reads the tag as `*` | **Differs** (warning) |
| 28a | No result before the next game | `1. e4 e5` followed by `[Event ...]` | Logs `PGN header '[' seen inside game (result missing ?)` and skips the game | Imports the game with the `Result` tag's result, with a warning | **Differs** |
| 28b | No result at the end of the input | `1. e4 e5` at the end of the file | Logs `End of input reached in game (result missing ?)` and skips the game | Imports the game with the `Result` tag's result, with a warning | **Differs** |
| 28c | No result after the moves and no `Result` tag | `1. e4 e5` followed by `[Event ...]` | Skips the game, as in #28a | Imports the game with the result `*`, with a warning | **Differs** |
| 29 | A move that can't be read | `2. Nf6` | Keeps `1. e4 e5 {Error reading move: Nf6}` | Skips the game and reports it | **Differs** |
| 30 | `[SetUp "1"]` without `FEN` | | Standard start, without a warning | Standard start, without a warning | Same |
| 31a | `FEN` castling rights that don't fit | `4k3/8/8/8/8/8/8/4K3 w KQkq -` | Keeps `KQkq` | Drops them, with a warning | **Differs** |
| 31b | `FEN` en passant square that doesn't fit | `4k3/8/8/8/8/8/8/4K3 w - e3` | Keeps `e3` | Drops it, with a warning | **Differs** |
| 31c | `FEN` en passant square with no capture possible | `4k3/8/8/8/4P3/8/8/4K3 b - e3` | Keeps `e3` | Keeps it while reading, but exports the `FEN` without it, without a warning | **Differs** |
| 32 | `Variant` tag other than standard chess | `[Variant "Chess960"]` | Reads the moves as standard chess | Rejects the game | **Differs** |

A `FEN` tag sets the start position with or without `SetUp`, in both products.

**#27c: no `Result` tag.** *What changes:* no warning. *Why:* without a `Result` tag there is nothing to contradict, so the result after the moves is simply the game's result.

**#27d: an unrecognized `Result` tag.** *What changes:* no warning when the moves end with `*`. *Why:* SCID reads a `Result` tag by its first characters, so `1-0 (time)` is `1-0`. `scidtopgn` reads only the four PGN results, so it treats the tag as `*`, which agrees with the moves. Reading tags like SCID does is listed in [Known differences outside these cases](#known-differences-outside-these-cases).

**#28: no result after the moves.** *What changes:* games that SCID skips are imported, with a warning naming the game. The game's result is the `Result` tag's, or `*` if there is no tag. *Why:* the moves were read without error. Skipping the whole game loses them, and the `Result` tag, if present, still gives the result.

**#29: a move that can't be read.** *What changes:* SCID imports the start of the game and marks the bad move with a comment. `scidtopgn` imports none of the game, and reports the game number, its first line, and the move. With `--on-error abort`, the import stops. *Why:* [#26](https://github.com/intermezzo-chess/scidtopgn/issues/26) chose to report such games rather than store part of a game that looks complete.

**#31a, #31b: castling rights and en passant squares that don't fit.** *What changes:* the stored and exported `FEN` doesn't have them, and a warning says what was dropped. *Why:* shakmaty can't represent a position with castling rights for a rook that isn't there, or an en passant square no pawn just moved past. Keeping them could also make an illegal castle or en passant capture legal.

**#31c: en passant square with no capture possible.** *What changes:* the exported `FEN` has `-` instead of the en passant square. *Why:* `scidtopgn` writes an en passant square only when an en passant capture is legal, so the same position always has the same `FEN`. Nothing is lost: a square no capture can use doesn't affect the game. There is no warning, for that reason.

**#32: `Variant` tag.** *What changes:* games in other variants, such as Chess960, are skipped and reported instead of being read as standard chess. *Why:* reading them as standard chess gives wrong moves or fails partway through. Chess960 is [#14](https://github.com/intermezzo-chess/scidtopgn/issues/14). `Variant` values that mean standard chess are accepted: `Standard`, `Chess`, `Normal`, and Lichess's `From Position`.

## Known differences outside these cases

#37 doesn't change these. Each was checked against SCID's sources in `scid-cpp/scid` and `scidtopgn` as of #37.

| Case | Example | SCID | scidtopgn |
| --- | --- | --- | --- |
| Text before the first game | `This text comes before the first game.` above `[Event ...]` | Keeps it as a comment before the first move | Reads it as a game, fails, and reports `PGN game 1`. The following games are numbered from 2. |
| Rating in a player name | `[White "Carlsen, Magnus (2850)"]` | Removes the rating from the name and uses it as the Elo | Keeps the name as written and has no Elo |
| Rating tags | `[WhiteUSCF "2400"]` | Uses the rating as the player's rating, with its type, such as USCF | Keeps it as an ordinary tag, with no rating in the index |
| `Result` tag with more text | `[Result "1-0 (time)"]` | Reads `1-0` | Reads it as `*`. The result after the moves is used, with a warning only when that result isn't `*` (#27d). |
| Text that isn't UTF-8 | A Latin-1 `é` in a tag | Stores the bytes unchanged | The CLI fails for the whole file: `stream did not contain valid UTF-8` |
| A `}` outside a comment | `1. e4 } e5` | Skips it and logs `"}" seen outside a comment` | Skips it, without a warning |

## Export and round trips

The cases above compare how a game is imported and stored. Exporting the stored game to PGN is a separate step, and it doesn't always give back what was imported. Variations, NAGs, and comments are exported where they were imported (#38). These are the losses:

- **A NAG before the first move** of the game or of a variation is dropped on import, as in SCID. See [Where annotations attach](#general-differences).
- **The en passant square** of a `FEN` is exported only when an en passant capture is legal (#31c).

The moves themselves are exported as canonical SAN, so `ed` comes back as `exd5` and `0-0` as `O-O`. This is a change in notation, not a loss. The movetext follows the PGN standard's export format: a space after each move number, no space inside a variation's parentheses, and a move number before a Black move that starts a variation or follows a comment or a variation.

For example, importing and exporting

```text
1. e4 e5 (1... c5 2. Nf3) 2. Nf3 $14 {Develops} Nc6 +-- 3. Bb5 *
```

gives this from `scidtopgn`:

```text
1. e4 e5 (1... c5 2. Nf3) 2. Nf3 += {Develops} 2... Nc6 +-- 3. Bb5
*
```

The SCID converter gives this:

```text
1.e4 e5 ( 1...c5 2.Nf3 ) 2.Nf3 += {Develops} 2...Nc6 +-- 3.Bb5 *
```

### How NAGs are exported (#34)

| | SCID | scidtopgn | Comparison |
| --- | --- | --- | --- |
| A NAG with a symbol both read back | The symbol | The symbol | Same |
| Any other NAG | A symbol from `evalNagsRegular` in `scid-cpp/scid/nagtext.h`, such as `→` for $40, `forced` for $7, or `RR` for $145, or `$n` where it has none | `$n` | **Differs** |

The symbols both read back are `!` `?` `!!` `??` `!?` `?!` (NAGs 1 to 6), `=` (10), `~` (13), `+=` (14), `=+` (15), `+/-` (16), `-/+` (17), `+-` (18), `-+` (19), `+--` (20), `--+` (21), `=/&` (44), `R` (144), `N` (146), and `D` (201). Every NAG from 1 to 255 can be imported again from `scidtopgn`'s export.

**#34: NAGs without a symbol both read back.** *What changes:* `scidtopgn` writes `$40` where SCID writes `→`, for example. *Why:* many of SCID's symbols are Unicode or words that neither SCID nor `scidtopgn` reads, so a game exported with them loses those annotations when it is imported again. `$n` is the standard PGN form, and every PGN reader reads it. Before #37, `scidtopgn` exported most NAGs, such as $20 and $41, as nothing, and others as Unicode symbols, such as `⩲`, that it couldn't import again.

## How SCID's behavior was checked

[`scid-cpp/tools/pgn_probe.cpp`](../scid-cpp/tools/pgn_probe.cpp) runs SCID's PGN parser from `scid-cpp/scid` on a PGN file. For each game, it prints:

- whether SCID would skip the game
- the game as SCID would store it, without tags whose value is `?`
- the messages SCID logged for the game

Build and run it from the repository root with a C++17 compiler and zlib:

```sh
c++ -std=c++17 -Wno-register -w -Iscid-cpp/scid \
    scid-cpp/tools/pgn_probe.cpp \
    scid-cpp/scid/{bytebuf,date,dstring,game,gfile,index,matsig,mfile,misc,movelist,myassert,namebase,pgnparse,position,stralloc,textbuf}.cpp \
    -lz -o pgn_probe
./pgn_probe tests/data/notation.pgn
```

`scid-cpp/CMakeLists.txt` names `scid-cpp/scid/src/main.cpp` as the converter's source, but the file is `scid-cpp/src/main.cpp`. These commands call the compiler directly instead. To build the reference converter itself, for example to write a test database such as `tests/data/castling`, use the same command with `-Iscid-cpp/src scid-cpp/src/main.cpp` in place of `pgn_probe.cpp`. Then run it as `<converter> <database> <pgn-file>`.

To compare with `scidtopgn`, run `cargo test -p scidtopgn --test test_pgn test_notation_fixture`, which checks every game in `tests/data/notation.pgn`. To check another file, run `scidtopgn import <file> -o <database>`, which prints each warning and skipped game. The cases in [Known differences outside these cases](#known-differences-outside-these-cases) were checked the same way, with the example inputs shown.

<details>
<summary>SCID's output for <code>tests/data/notation.pgn</code></summary>

```text
game 1: OK | [Event "A1 castling with zeros or lowercase"] [Result "*"] 1.e4 e5 2.Nf3 Nf6 3.Bc4 Bc5 4.O-O O-O *
game 2: OK | [Event "A2 pawn prefix"] [Result "*"] 1.e4 e5 *
game 3: OK | [Event "A3 missing or extra capture sign"] [Result "*"] 1.e4 d5 2.exd5 Qxd5 3.Nc3 *
game 4: OK | [Event "A4 pawn coordinates"] [Result "*"] 1.e4 e5 *
game 5: OK | [Event "A5 null moves"] [Result "*"] 1.e4 -- 2.-- -- 3.d4 *
game 6: OK | [Event "A5b null move in check"] [Result "*"] [FEN "k7/8/8/8/8/8/4r3/4K3 w - - 0 1"] 1.-- *
game 7: OK | [Event "A6 check and mate signs"] [Result "*"] 1.e4 e5 2.Bc4 Nc6 3.Qh5 Nf6 4.Qxf7# *
game 8: OK | [Event "A7 move numbers without dots"] [Result "*"] 1.e4 e5 2.Nf3 *
game 9: OK | [Event "A8 double check sign"] [Result "*"] 1.e4 f6 2.d4 g5 3.Qh5# *
game 10: OK | [Event "1a pawn capture without a rank"] [Result "*"] 1.e4 d5 2.exd5 *
game 11: OK | [Event "1b pawn capture without a rank, with x"] [Result "*"] 1.e4 d5 2.exd5 *
game 12: OK | [Event "1c pawn capture without a rank, en passant"] [Result "*"] [FEN "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 2"] 2.exd6 *
game 13: OK | [Event "2 ambiguous pawn capture without a rank"] [Result "*"] [FEN "4k3/8/3n4/4P3/3n4/4P3/8/4K3 w - - 0 1"] 1.exd4 *
game 14: OK | [Event "3 colon for a capture"] [Result "*"] 1.e4 d5 2.exd5 Qxd5 *
game 15: OK | [Event "4 hyphen between squares"] [Result "*"] 1.e4 e5 2.Nf3 Nc6 *
game 16: OK | [Event "5 coordinate piece moves"] [Result "*"] 1.Nf3 Nf6 *
game 17: OK | [Event "6a king moves two squares"] [Result "*"] 1.e4 e5 2.Nf3 Nf6 3.Bc4 Bc5 4.O-O O-O *
game 18: OK | [Event "6b king coordinate castling"] [Result "*"] 1.e4 e5 2.Nf3 Nf6 3.Bc4 Bc5 4.O-O O-O *
game 19: OK | [Event "6c king moves two squares without the castling right"] [Result "*"] 1.e4 e5 2.Nf3 Nf6 3.Bc4 Bc5 4.Ke2 Ke7 5.Ke1 Ke8 {Error reading move: Kg1} *
    logged: (game 19, line 77) Error reading move in game ? - ?, 0: Kg1
game 20: OK | [Event "7a promotion with no piece"] [Result "*"] [FEN "4k3/P7/8/8/8/8/8/4K3 w - - 0 1"] 1.a8=Q+ *
game 21: OK | [Event "7b capture promotion with no piece"] [Result "*"] [FEN "1n2k3/P7/8/8/8/8/8/4K3 w - - 0 1"] 1.axb8=Q+ *
game 22: OK | [Event "7c coordinate promotion with no piece"] [Result "*"] [FEN "4k3/P7/8/8/8/8/8/4K3 w - - 0 1"] {Error reading move: a7a8}  *
    logged: (game 22, line 92) Error reading move in game ? - ?, 0: a7a8
game 23: OK | [Event "8a lowercase promotion piece"] [Result "*"] [FEN "4k3/P7/8/8/8/8/8/4K3 w - - 0 1"] 1.a8=Q+ *
game 24: OK | [Event "8b lowercase coordinate promotion"] [Result "*"] [FEN "4k3/P7/8/8/8/8/8/4K3 w - - 0 1"] 1.a8=Q+ *
game 25: OK | [Event "8c lowercase capture promotion without a rank"] [Result "*"] [FEN "1n2k3/P7/8/8/8/8/8/4K3 w - - 0 1"] 1.axb8=N *
game 26: OK | [Event "8d lowercase bishop promotion"] [Result "*"] [FEN "4k3/P7/8/8/8/8/8/4K3 w - - 0 1"] {Error reading move: a8b}  *
    logged: (game 26, line 112) Error reading move in game ? - ?, 0: a8b
game 27: OK | [Event "8e lowercase coordinate bishop promotion"] [Result "*"] [FEN "4k3/P7/8/8/8/8/8/4K3 w - - 0 1"] 1.a8=B *
game 28: OK | [Event "9 lowercase piece after equals sign"] [Result "*"] [FEN "4k3/P7/8/8/8/8/8/4K3 w - - 0 1"] 1.a8=Q+ *
    logged: (game 28, line 122) Warning: Invalid annotation symbol: =n
game 29: OK | [Event "10a capture promotion without a rank"] [Result "*"] [FEN "1n2k3/P7/8/8/8/8/8/4K3 w - - 0 1"] 1.axb8=Q+ *
game 30: OK | [Event "10b capture promotion without a rank or equals sign"] [Result "*"] [FEN "1n2k3/P7/8/8/8/8/8/4K3 w - - 0 1"] 1.axb8=Q+ *
game 31: OK | [Event "11a en passant marker attached"] [Result "*"] [FEN "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 2"] 2.exd6 *
game 32: OK | [Event "11b en passant marker after the move"] [Result "*"] [FEN "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 2"] 2.exd6 *
game 33: OK | [Event "12 lowercase piece letter"] [Result "*"] 1.e4 e5 *
    logged: (game 33, line 146) Error: Unexpected symbol: nf
game 34: OK | [Event "13 lowercase b for a bishop"] [Result "*"] 1.e4 e5 {Error reading move: bc4} *
    logged: (game 34, line 150) Error reading move in game ? - ?, 0: bc4
game 35: OK | [Event "14 castling without dashes"] [Result "*"] [FEN "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1"] 1.O-O-O O-O *
game 36: OK | [Event "15 castling with zeros and no dashes"] [Result "*"] [FEN "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1"] *
game 37: OK | [Event "16a castling after the king moved"] [Result "*"] 1.e4 e5 2.Nf3 Nf6 3.Bc4 Bc5 4.Ke2 Ke7 5.Ke1 Ke8 6.O-O O-O *
    logged: (game 37, line 164) Warning: illegal castling (O-O) in game ? - ?, 0
    logged: (game 37, line 164) Warning: illegal castling (O-O) in game ? - ?, 0
game 38: OK | [Event "16b castling after the rook moved"] [Result "*"] 1.e4 e5 2.Nf3 Nf6 3.Bc4 Bc5 4.Rg1 Rg8 5.Rh1 Rh8 6.O-O *
    logged: (game 38, line 168) Warning: illegal castling (O-O) in game ? - ?, 0
game 39: OK | [Event "16c queenside castling without the right in the FEN"] [Result "*"] [FEN "r3k2r/8/8/8/8/8/8/R3K2R w - - 0 1"] 1.O-O-O *
    logged: (game 39, line 173) Warning: illegal castling (O-O-O) in game ? - ?, 0
game 40: OK | [Event "16d castling without the right in a variation"] [Result "*"] 1.e4 e5 2.Nf3 Nf6 3.Bc4 Bc5 4.Ke2 Ke7 5.Ke1 Ke8 6.d3 ( 6.O-O ) 6...d6 *
    logged: (game 40, line 177) Warning: illegal castling (O-O) in game ? - ?, 0
game 41: OK | [Event "18a castling with no rook"] [Result "*"] [FEN "4k3/8/8/8/8/8/8/4K3 w - - 0 1"] 1.O-O *
    logged: (game 41, line 182) Warning: illegal castling (O-O) in game ? - ?, 0
game 42: OK | [Event "18b queenside castling with no rook"] [Result "*"] [FEN "4k3/8/8/8/8/8/8/4K3 w - - 0 1"] 1.O-O-O *
    logged: (game 42, line 187) Warning: illegal castling (O-O-O) in game ? - ?, 0
game 43: OK | [Event "19 castling through check"] [Result "*"] [FEN "4k1r1/8/8/8/8/8/8/4K2R w K - 0 1"] {Error reading move: O-O}  *
    logged: (game 43, line 192) Error reading move in game ? - ?, 0: O-O
game 44: OK | [Event "20a kingside castling when only queenside is legal"] [Result "*"] [FEN "r3k2r/8/8/8/8/8/8/R3K1NR w KQkq - 0 1"] 1.O-O-O *
game 45: OK | [Event "20b queenside castling when only kingside is legal"] [Result "*"] [FEN "r3k2r/8/8/8/8/8/8/RN2K2R w KQkq - 0 1"] {Error reading move: O-O-O}  *
    logged: (game 45, line 202) Error reading move in game ? - ?, 0: O-O-O
game 46: OK | [Event "21a annotation symbols attached"] [Result "*"] 1.e4 $1 e5 $2 2.Nf3 $3 Nc6 $4 3.Bb5 $5 a6 $6 *
game 47: OK | [Event "21b annotation symbols after the move"] [Result "*"] 1.e4 $1 e5 $2 2.Nf3 $14 Nc6 $15 3.Bb5 $16 a6 $17 4.Ba4 $18 Nf6 $19 5.O-O $10  Be7 $13 *
game 48: OK | [Event "21c other annotation symbols"] [Result "*"] 1.e4 $14 e5 $15 2.Nf3 $20 Nc6 $21 3.Bb5 $40 a6 $41 4.Ba4 $44 Nf6 $44 *
game 49: OK | [Event "21d annotation symbols after check"] [Result "*"] 1.e4 e5 2.Bc4 Nc6 3.Qh5 Nf6 4.Qxf7# $1 *
game 50: OK | [Event "22a annotation symbols attached by equals or hyphen"] [Result "*"] 1.e4 $15 e5 2.Nf3 $10 Nc6 $19 {Error reading move: a6} *
    logged: (game 50, line 222) Error: Unexpected symbol: Bb5
    logged: (game 50, line 222) Error reading move in game ? - ?, 0: a6
    logged: (game 50, line 222) Error: Unexpected symbol: Ba4
game 51: OK | [Event "22b several annotation symbols attached"] [Result "*"] 1.e4 $3 $18 e5 *
game 52: OK | [Event "22c symbols starting with two hyphens or an arrow attached"] [Result "*"] 1.e4 $19 e5 {Error reading move: Nc6} *
    logged: (game 52, line 230) Error: Unexpected symbol: Nf3
    logged: (game 52, line 230) Error reading move in game ? - ?, 0: Nc6
game 53: OK | [Event "23 tilde attached"] [Result "*"] {Error reading move: e5}  *
    logged: (game 53, line 234) Error: Unexpected symbol: e4
    logged: (game 53, line 234) Error reading move in game ? - ?, 0: e5
game 54: OK | [Event "24 letter symbols"] [Result "*"] 1.e4 $146 e5 $201 2.Nf3 $144 Nc6 *
game 55: OK | [Event "25a unknown symbols"] [Result "*"] 1.e4 e5 2.Nf3 Nc6 *
    logged: (game 55, line 242) Error: Unexpected symbol: zz
    logged: (game 55, line 242) Error: Unexpected symbol: /\
game 56: OK | [Event "25b piece letter on its own"] [Result "*"] 1.e4 e5 *
    logged: (game 56, line 246) Warning: Invalid annotation symbol: B
game 57: OK | [Event "25c word that starts like a null move"] [Result "*"] 1.e4 e5 *
    logged: (game 57, line 250) Error: Unexpected symbol: nullify
game 58: OK | [Event "26a result 1:0"] [Result "1-0"] 1.e4 e5 1-0
game 59: OK | [Event "26b result 0:1"] [Result "0-1"] 1.e4 e5 0-1
game 60: OK | [Event "26c result 1/2"] [Result "1/2-1/2"] 1.e4 e5 1/2-1/2
game 61: OK | [Event "26d result 1/2:1/2"] [Result "1/2-1/2"] 1.e4 e5 1/2-1/2
game 62: OK | [Event "27a result differs from the tag"] [Result "0-1"] 1.e4 e5 0-1
    logged: (game 62, line 275) Result did not match the header result
game 63: OK | [Event "27b unknown result with a result tag"] [Result "*"] 1.e4 e5 *
    logged: (game 63, line 280) Result did not match the header result
game 64: OK | [Event "27c result with no result tag"] [Result "1-0"] 1.e4 e5 1-0
    logged: (game 64, line 284) Result did not match the header result
game 65: OK | [Event "27d unrecognized result tag and unknown result"] [Result "*"] 1.e4 e5 *
    logged: (game 65, line 289) Result did not match the header result
game 66: ERROR (SCID skips this game) | [Event "28a no result before the next game"] [Result "1-0"] 1.e4 e5 1-0
    logged: (game 66, line 296) PGN header '[' seen inside game (result missing ?)
game 67: ERROR (SCID skips this game) | [Event "28c no result and no result tag"] [Result "*"] 1.e4 e5 *
    logged: (game 67, line 300) PGN header '[' seen inside game (result missing ?)
game 68: OK | [Event "29 move that can't be read"] [Result "*"] 1.e4 e5 {Error reading move: Nf6} *
    logged: (game 68, line 302) Error reading move in game ? - ?, 0: Nf6
game 69: OK | [Event "30 SetUp without FEN"] [Result "*"] [SetUp "1"] 1.e4 *
game 70: OK | [Event "31a FEN castling rights that don't fit"] [Result "*"] [FEN "4k3/8/8/8/8/8/8/4K3 w KQkq - 0 1"] 1.Kd1 *
game 71: OK | [Event "31b FEN en passant square that doesn't fit"] [Result "*"] [FEN "4k3/8/8/8/8/8/8/4K3 w - e3 0 1"] 1.Kd1 *
game 72: OK | [Event "31c FEN en passant square with no capture"] [Result "*"] [FEN "4k3/8/8/8/4P3/8/8/4K3 b - e3 0 1"] 1...Kd7 *
game 73: OK | [Event "32 Variant tag"] [Result "*"] [Variant "Chess960"] 1.e4 *
game 74: ERROR (SCID skips this game) | [Event "28b no result at the end of the input"] [Result "0-1"] 1.e4 e5 0-1
    logged: (game 74, line 333) End of input reached in game (result missing ?)
```

</details>

## Tests

- **`test_notation_fixture`** in `crates/scidtopgn/tests/test_pgn.rs` checks how `scidtopgn` reads each game in `tests/data/notation.pgn`: the moves, NAGs, result, warnings, or error.
- **`test_annotation_symbols_match_nags`** checks that each annotation symbol gives the same game as its `$n` NAG.
- **The other tests in `test_pgn.rs`** check that warnings name the game and line, and that a rejected game reports only its error.
- **Unit tests in `crates/scidtopgn/src/pgn.rs`** cover each notation, the symbol table, and how tokens are split.
- **`tests/data/castling`** was written by the reference converter from `tests/data/castling.pgn`. `crates/scidtopgn/tests/test_interop.rs` checks that `scidtopgn` reads it (#17) and writes the same moves for `castling.pgn`. `position.rs` and `mov.rs` test castling without the castling right on its own.
- **`tests/data/annotated`** was written by the reference converter from `tests/data/annotated.pgn`, which has NAGs and comments in the main line and in variations. `test_interop.rs` checks that `scidtopgn` reads and exports it with each annotation where SCID stores it, and writes the same moves and annotations for `annotated.pgn` (#38).
- **`crates/scidtopgn/tests/test_variations.rs`** checks where annotations attach in variations nested six levels deep, after PGN import and after round trips through SCID, and checks both variation limits from PGN and from SCID ([issue 4](https://github.com/intermezzo-chess/scidtopgn/issues/4)).
- **`test_every_nag_survives_export_and_import`** in `crates/scidtopgn/src/game.rs` exports and imports each NAG from 1 to 255 (#34).
- **`test_cli_import_prints_warnings_and_keeps_the_game`** in `crates/scidtopgn-cli/tests/integration.rs` checks how the CLI prints warnings in both `--on-error` modes (#33).
