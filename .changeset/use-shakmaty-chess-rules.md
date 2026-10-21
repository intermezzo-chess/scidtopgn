---
default: major
---

# Use shakmaty for chess rules

The library now uses [shakmaty](https://crates.io/crates/shakmaty) 0.30 for legality, SAN, check and mate detection, and FEN, and keeps only SCID's move encoding and piece numbering.

- PGN export disambiguates moves correctly (for example `Nbd2`, `R1a3`, `Qh4e1`, and `Nd2` when the other knight is pinned) and writes `#` only for checkmate.
- Every move read from a SCID database or PGN file is checked for legality. An illegal move is an error instead of a corrupted game.
- PGN import stores the SAN it generates, so check and mate signs, unneeded disambiguation, and castling written with zeros are corrected.

Breaking changes to the library API:

- `Board`, `SimpleMove`, `Square`, `Piece`, `Color`, `piece_make`, `piece_color`, `piece_type`, the castling flag constants, and the per-piece `encode_*` and `decode_*` functions are removed. Use the re-exported `scidtopgn::shakmaty` types, `ScidPosition`, `PieceList`, `GameMove`, `encode_move`, and `decode_move`.
- `Game::start_board`, `current_board`, `non_standard_start`, and `var_depth` are replaced by `Game::start_position: Option<Chess>`. `Game::make_move_on_board` is removed.
- `MoveNode::move_data` is an `Option<GameMove>`, `None` for marker nodes.
- `Error::InvalidFen` has a `source`, and `Error::PgnMove`, `Error::PgnGame`, and `Error::UnsupportedVariant` are new.
- `private::MoveDecoder`, `private::DecodeElement`, and `private::decode_move` are removed.
