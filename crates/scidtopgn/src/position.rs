//! Chess positions with SCID's piece numbering.
//!
//! shakmaty provides the chess rules. SCID's game format also names the moving
//! piece by a number from 0 to 15 for each side, so [`ScidPosition`] keeps
//! that numbering next to the position.

use shakmaty::fen::Fen;
use shakmaty::san::{San, SanPlus, Suffix};
use shakmaty::{
    Bitboard, Board, ByColor, CastlingMode, CastlingSide, Chess, Color, EnPassantMode, File,
    FromSetup, Move, Position, PositionError, Rank, Setup, Square,
};

use crate::error::{Error, Result};
use crate::mov::GameMove;

/// The most pieces one side can have in SCID's numbering.
const MAX_PIECES: usize = 16;

/// SCID's numbers for each side's pieces.
///
/// The king is always number 0. In the standard starting position, the other
/// pieces on the back rank are numbered from the a-file to the h-file, then
/// the pawns from 8 to 15. A position set up from a FEN numbers the pieces in
/// FEN order, from a8 to h1, then gives the king number 0 by moving the piece
/// that had it to the end. A capture gives the captured piece's number to its
/// side's highest-numbered piece. A promoted pawn keeps its number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PieceList {
    squares: ByColor<[Option<Square>; MAX_PIECES]>,
    numbers: [Option<u8>; 64],
    counts: ByColor<u8>,
}

impl PieceList {
    fn empty() -> Self {
        PieceList {
            squares: ByColor::default(),
            numbers: [None; 64],
            counts: ByColor::default(),
        }
    }

    /// The numbering of the standard starting position.
    pub fn standard() -> Self {
        let mut list = PieceList::empty();
        for color in Color::ALL {
            let back = color.backrank();
            let pawns = color.relative_rank(Rank::Second);
            let king = Square::from_coords(File::E, back);
            list.push(color, king);
            for file in File::ALL.into_iter().filter(|&f| f != File::E) {
                list.push(color, Square::from_coords(file, back));
            }
            for file in File::ALL {
                list.push(color, Square::from_coords(file, pawns));
            }
        }
        list
    }

    /// Numbers the pieces on `board` in FEN order, as SCID does for a game
    /// that starts from a FEN.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidFen`] if a side has more than 16 pieces or
    /// no king.
    pub fn from_board(board: &Board) -> Result<Self> {
        let mut list = PieceList::empty();
        for rank in Rank::ALL.into_iter().rev() {
            for file in File::ALL {
                let square = Square::from_coords(file, rank);
                let Some(piece) = board.piece_at(square) else {
                    continue;
                };
                if usize::from(*list.counts.get(piece.color)) == MAX_PIECES {
                    return Err(invalid_setup(
                        board,
                        "SCID allows at most 16 pieces per side",
                    ));
                }
                list.push(piece.color, square);
                if piece.role == shakmaty::Role::King {
                    let last = usize::from(*list.counts.get(piece.color)) - 1;
                    list.squares.get_mut(piece.color).swap(0, last);
                    list.renumber(piece.color, 0);
                    list.renumber(piece.color, last);
                }
            }
        }
        for color in Color::ALL {
            if list.square(color, 0).and_then(|sq| board.role_at(sq)) != Some(shakmaty::Role::King)
            {
                return Err(invalid_setup(board, "each side needs one king"));
            }
        }
        Ok(list)
    }

    /// The number of pieces `color` has.
    pub fn count(&self, color: Color) -> u8 {
        *self.counts.get(color)
    }

    /// The square of `color`'s piece number `number`, if it has one.
    pub fn square(&self, color: Color, number: u8) -> Option<Square> {
        self.squares
            .get(color)
            .get(usize::from(number))
            .copied()
            .flatten()
    }

    /// The number of the piece on `square`, if there is one.
    pub fn number(&self, square: Square) -> Option<u8> {
        self.numbers[square.to_usize()]
    }

    fn push(&mut self, color: Color, square: Square) {
        let number = *self.counts.get(color);
        self.squares.get_mut(color)[usize::from(number)] = Some(square);
        self.numbers[square.to_usize()] = Some(number);
        *self.counts.get_mut(color) += 1;
    }

    /// Points the square of `color`'s piece `index` back at its number.
    fn renumber(&mut self, color: Color, index: usize) {
        if let Some(square) = self.squares.get(color)[index] {
            // `index` is below MAX_PIECES, so it fits in a u8.
            self.numbers[square.to_usize()] = Some(index as u8);
        }
    }

    fn relocate(&mut self, color: Color, from: Square, to: Square) {
        if let Some(number) = self.numbers[from.to_usize()].take() {
            self.squares.get_mut(color)[usize::from(number)] = Some(to);
            self.numbers[to.to_usize()] = Some(number);
        }
    }

    fn remove(&mut self, color: Color, square: Square) {
        let Some(number) = self.numbers[square.to_usize()].take() else {
            return;
        };
        let count = self.counts.get_mut(color);
        *count -= 1;
        let last = usize::from(*count);
        let squares = self.squares.get_mut(color);
        squares[usize::from(number)] = squares[last];
        squares[last] = None;
        self.renumber(color, usize::from(number));
    }

    /// Updates the numbering for `m`, a legal move by `color`.
    fn play(&mut self, color: Color, m: Move) {
        match m {
            Move::Normal {
                from, to, capture, ..
            } => {
                if capture.is_some() {
                    self.remove(!color, to);
                }
                self.relocate(color, from, to);
            }
            Move::EnPassant { from, to } => {
                self.remove(!color, Square::from_coords(to.file(), from.rank()));
                self.relocate(color, from, to);
            }
            Move::Castle { king, rook } => {
                let Some(side) = m.castling_side() else {
                    return;
                };
                // Take both numbers first: the king's and the rook's
                // destinations may be each other's origins.
                let king_number = self.numbers[king.to_usize()].take();
                let rook_number = self.numbers[rook.to_usize()].take();
                for (number, to) in [
                    (king_number, side.king_to(color)),
                    (rook_number, side.rook_to(color)),
                ] {
                    if let Some(number) = number {
                        self.squares.get_mut(color)[usize::from(number)] = Some(to);
                        self.numbers[to.to_usize()] = Some(number);
                    }
                }
            }
            // Drops only exist in Crazyhouse.
            Move::Put { .. } => {}
        }
    }
}

/// A chess position together with SCID's numbering of its pieces.
#[derive(Debug, Clone)]
pub struct ScidPosition {
    chess: Chess,
    pieces: PieceList,
}

impl Default for ScidPosition {
    fn default() -> Self {
        ScidPosition {
            chess: Chess::default(),
            pieces: PieceList::standard(),
        }
    }
}

impl ScidPosition {
    /// The standard starting position.
    pub fn standard() -> Self {
        Self::default()
    }

    /// A position set up from a FEN, numbered in FEN order.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidFen`] if a side has more than 16 pieces.
    pub fn from_setup_position(chess: Chess) -> Result<Self> {
        let pieces = PieceList::from_board(chess.board())?;
        Ok(ScidPosition { chess, pieces })
    }

    /// Returns the chess position.
    pub fn chess(&self) -> &Chess {
        &self.chess
    }

    /// Returns SCID's numbering of the pieces.
    pub fn pieces(&self) -> &PieceList {
        &self.pieces
    }

    /// Whether `m` can be played: it is legal, or it is castling that would
    /// be legal with the castling right.
    ///
    /// SCID plays castling after the king or rook has moved and returned,
    /// so databases it writes can contain such a move.
    pub fn can_play(&self, m: Move) -> bool {
        self.chess.is_legal(m) || with_castling_right_for(&self.chess, m).is_some()
    }

    /// The castling move on `side`, if the side to move has lost the
    /// castling right but castling would otherwise be legal: the king and
    /// rook are on their original squares, the squares between them are
    /// empty, and the king isn't in or passing through check.
    pub fn castle_without_right(&self, side: CastlingSide) -> Option<Move> {
        castle_without_right(&self.chess, side).map(|(m, _)| m)
    }

    /// Plays `mv` and returns its SAN in the position before it.
    ///
    /// Castling without the castling right is played if
    /// [`castle_without_right`](Self::castle_without_right) allows it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidMove`] if the move is illegal, or if it is a
    /// null move while the side to move is in check.
    pub fn play(&mut self, mv: GameMove) -> Result<SanPlus> {
        let color = self.chess.turn();
        match mv {
            GameMove::Move(m) => {
                if !self.chess.is_legal(m) {
                    match with_castling_right_for(&self.chess, m) {
                        Some(chess) => self.chess = chess,
                        None => {
                            return Err(Error::InvalidMove {
                                move_str: format!(
                                    "{} is illegal in {}",
                                    m.to_uci(CastlingMode::Standard),
                                    fen(&self.chess)
                                ),
                            })
                        }
                    }
                }
                let san = SanPlus::from_move_and_play_unchecked(&mut self.chess, m);
                self.pieces.play(color, m);
                Ok(san)
            }
            GameMove::Null => {
                self.chess = play_null(&self.chess)?;
                Ok(SanPlus {
                    san: San::Null,
                    suffix: Suffix::from_position(&self.chess),
                })
            }
        }
    }
}

/// The castling move on `side` for the side to move, and `chess` with the
/// castling right that makes it legal, if the side to move has lost that
/// right but castling would otherwise be legal.
fn castle_without_right(chess: &Chess, side: CastlingSide) -> Option<(Move, Chess)> {
    let color = chess.turn();
    if chess.castles().has(color, side) {
        return None;
    }
    let rook_file = match side {
        CastlingSide::KingSide => File::H,
        CastlingSide::QueenSide => File::A,
    };
    let mut setup = chess.to_setup(EnPassantMode::Legal);
    setup
        .castling_rights
        .add(Square::from_coords(rook_file, color.backrank()));
    // An invalid castling right is dropped, so the right only survives if
    // the king and rook are on their original squares.
    let with_right = chess_from_setup(setup).ok()?;
    if !with_right.castles().has(color, side) {
        return None;
    }
    let m = with_right
        .legal_moves()
        .into_iter()
        .find(|m| m.castling_side() == Some(side))?;
    Some((m, with_right))
}

/// `chess` with the castling right that makes `m` legal, if `m` is castling
/// without the castling right.
fn with_castling_right_for(chess: &Chess, m: Move) -> Option<Chess> {
    let (castle, with_right) = castle_without_right(chess, m.castling_side()?)?;
    (castle == m).then_some(with_right)
}

/// Swaps the side to move like SCID's null move: the en passant square is
/// cleared, the halfmove clock advances, and the fullmove number advances
/// after Black's turn.
fn play_null(chess: &Chess) -> Result<Chess> {
    let null_in_check = || Error::InvalidMove {
        move_str: format!("a null move is illegal in check in {}", fen(chess)),
    };
    if chess.is_check() {
        return Err(null_in_check());
    }
    let mut setup = chess.to_setup(EnPassantMode::Legal);
    setup.swap_turn();
    setup.halfmoves = setup.halfmoves.saturating_add(1);
    if chess.turn() == Color::Black {
        setup.fullmoves = setup.fullmoves.saturating_add(1);
    }
    // The side to move isn't in check, so the side that moves next can't
    // capture a king.
    chess_from_setup(setup).map_err(|_| null_in_check())
}

/// Sets up a standard chess position like SCID does. Invalid castling rights
/// and en passant squares are dropped. Checks that couldn't arise in a game
/// and more material than a game allows are accepted. Missing kings, pawns
/// on the back rank, and a side not to move in check are rejected.
fn chess_from_setup(setup: Setup) -> std::result::Result<Chess, Box<PositionError<Chess>>> {
    Chess::from_setup(setup, CastlingMode::Standard)
        .or_else(PositionError::ignore_invalid_castling_rights)
        .or_else(PositionError::ignore_invalid_ep_square)
        .or_else(PositionError::ignore_impossible_check)
        .or_else(PositionError::ignore_too_much_material)
        .map_err(Box::new)
}

/// Parses a standard chess position from a FEN.
///
/// # Errors
///
/// Returns [`Error::InvalidFen`] if the FEN is malformed or the position
/// can't be played. See [`chess_from_setup`] for what is accepted.
pub(crate) fn chess_from_fen(fen: &str) -> Result<Chess> {
    chess_from_fen_reporting_drops(fen).map(|(chess, _)| chess)
}

/// What a FEN set but the position couldn't keep.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct DroppedFromFen {
    /// The rook squares of castling rights that don't fit the position.
    pub castling_rights: Bitboard,
    /// An en passant square that no pawn could have just moved past.
    pub ep_square: Option<Square>,
}

/// Parses a standard chess position from a FEN, like [`chess_from_fen`],
/// and reports the castling rights and en passant square it dropped.
pub(crate) fn chess_from_fen_reporting_drops(fen: &str) -> Result<(Chess, DroppedFromFen)> {
    let invalid = |source: Box<dyn std::error::Error + Send + Sync>| Error::InvalidFen {
        fen: fen.to_string(),
        source,
    };
    let setup = fen
        .trim()
        .parse::<Fen>()
        .map_err(|e| invalid(Box::new(e)))?
        .into_setup();
    let (castling_rights, ep_square) = (setup.castling_rights, setup.ep_square);
    let chess = chess_from_setup(setup).map_err(|e| invalid(e))?;
    let dropped = DroppedFromFen {
        castling_rights: castling_rights & !chess.castles().castling_rights(),
        ep_square: ep_square.filter(|_| chess.ep_square(EnPassantMode::Always).is_none()),
    };
    Ok((chess, dropped))
}

/// The FEN of `chess`, with an en passant square only when an en passant
/// capture is legal.
pub(crate) fn fen(chess: &Chess) -> String {
    Fen::from_position(chess, EnPassantMode::Legal).to_string()
}

fn invalid_setup(board: &Board, reason: &str) -> Error {
    Error::InvalidFen {
        fen: board.board_fen().to_string(),
        source: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_numbering() {
        let list = PieceList::standard();
        for color in Color::ALL {
            assert_eq!(list.count(color), 16);
        }
        assert_eq!(list.number(Square::E1), Some(0));
        assert_eq!(list.number(Square::A1), Some(1));
        assert_eq!(list.number(Square::D1), Some(4));
        assert_eq!(list.number(Square::F1), Some(5));
        assert_eq!(list.number(Square::H1), Some(7));
        assert_eq!(list.number(Square::A2), Some(8));
        assert_eq!(list.number(Square::H2), Some(15));
        assert_eq!(list.number(Square::E8), Some(0));
        assert_eq!(list.number(Square::H7), Some(15));
        assert_eq!(list.square(Color::Black, 1), Some(Square::A8));
        assert_eq!(list.number(Square::E4), None);
    }

    #[test]
    fn fen_numbering_moves_the_king_to_zero() {
        let chess = chess_from_fen("4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1").unwrap();
        let list = PieceList::from_board(chess.board()).unwrap();
        assert_eq!(list.number(Square::E1), Some(0));
        assert_eq!(list.number(Square::A1), Some(1));
        assert_eq!(list.number(Square::H1), Some(2));
        assert_eq!(list.number(Square::E8), Some(0));
        assert_eq!(list.count(Color::White), 3);
    }

    #[test]
    fn capture_gives_the_number_to_the_last_piece() {
        let mut list = PieceList::standard();
        list.remove(Color::Black, Square::D7);
        assert_eq!(list.count(Color::Black), 15);
        assert_eq!(list.number(Square::H7), Some(11));
        assert_eq!(list.square(Color::Black, 11), Some(Square::H7));
        assert_eq!(list.square(Color::Black, 15), None);
        assert_eq!(list.number(Square::D7), None);
    }

    #[test]
    fn castling_moves_king_and_rook() {
        let chess = chess_from_fen("4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1").unwrap();
        let mut pos = ScidPosition::from_setup_position(chess).unwrap();
        let castle = Move::Castle {
            king: Square::E1,
            rook: Square::A1,
        };
        let san = pos.play(GameMove::Move(castle)).unwrap();
        assert_eq!(san.to_string(), "O-O-O");
        assert_eq!(pos.pieces().number(Square::C1), Some(0));
        assert_eq!(pos.pieces().number(Square::D1), Some(1));
        assert_eq!(pos.pieces().number(Square::E1), None);
        assert_eq!(pos.pieces().number(Square::A1), None);
    }

    #[test]
    fn castling_without_the_castling_right() {
        // The king and rook are on their original squares, but the FEN gives
        // White no castling rights.
        let chess = chess_from_fen("r3k2r/8/8/8/8/8/8/R3K2R w kq - 0 1").unwrap();
        let mut pos = ScidPosition::from_setup_position(chess).unwrap();
        let castle = pos.castle_without_right(CastlingSide::KingSide).unwrap();
        assert!(!pos.chess().is_legal(castle));
        assert!(pos.can_play(castle));
        let san = pos.play(GameMove::Move(castle)).unwrap();
        assert_eq!(san.to_string(), "O-O");
        assert_eq!(fen(pos.chess()), "r3k2r/8/8/8/8/8/8/R4RK1 b kq - 1 1");
        assert_eq!(pos.pieces().number(Square::G1), Some(0));
    }

    #[test]
    fn castling_without_the_castling_right_needs_the_rest_of_the_rules() {
        for (fen, side) in [
            // No rook.
            ("4k3/8/8/8/8/8/8/4K3 w - - 0 1", CastlingSide::KingSide),
            ("4k3/8/8/8/8/8/8/4K3 w - - 0 1", CastlingSide::QueenSide),
            // The king isn't on its original square.
            ("4k3/8/8/8/8/8/8/R2K3R w - - 0 1", CastlingSide::KingSide),
            // A piece between the king and rook.
            ("4k3/8/8/8/8/8/8/R3KB1R w - - 0 1", CastlingSide::KingSide),
            // Through check.
            ("4k1r1/8/8/8/8/8/8/R3K2R w - - 0 1", CastlingSide::KingSide),
            // In check.
            ("4r1k1/8/8/8/8/8/8/R3K2R w - - 0 1", CastlingSide::QueenSide),
            // The castling right is still there, so the move is simply legal.
            ("4k3/8/8/8/8/8/8/R3K2R w K - 0 1", CastlingSide::KingSide),
        ] {
            let chess = chess_from_fen(fen).unwrap();
            let pos = ScidPosition::from_setup_position(chess).unwrap();
            assert_eq!(pos.castle_without_right(side), None, "{fen} {side:?}");
        }
    }

    #[test]
    fn reports_what_a_fen_drops() {
        let (_, dropped) =
            chess_from_fen_reporting_drops("4k3/8/8/8/8/8/8/R3K3 w KQkq e3 0 1").unwrap();
        assert_eq!(
            dropped.castling_rights,
            Bitboard::from(Square::H1) | Square::A8 | Square::H8
        );
        assert_eq!(dropped.ep_square, Some(Square::E3));

        // An en passant square that a pawn just moved past is kept, even if
        // no capture is possible.
        let (chess, dropped) =
            chess_from_fen_reporting_drops("4k3/8/8/8/4P3/8/8/4K3 b - e3 0 1").unwrap();
        assert_eq!(dropped, DroppedFromFen::default());
        assert_eq!(fen(&chess), "4k3/8/8/8/4P3/8/8/4K3 b - - 0 1");
    }

    #[test]
    fn too_many_pieces() {
        let chess = chess_from_fen("QQQQQQQQ/QQQQQQQQ/8/8/8/8/8/K6k b - - 0 1").unwrap();
        assert!(matches!(
            ScidPosition::from_setup_position(chess),
            Err(Error::InvalidFen { .. })
        ));
    }

    #[test]
    fn null_move_swaps_turn_and_advances_clocks() {
        let chess = chess_from_fen("4k3/8/8/8/4P3/8/8/4K3 b - e3 3 20").unwrap();
        let mut pos = ScidPosition::from_setup_position(chess).unwrap();
        let san = pos.play(GameMove::Null).unwrap();
        assert_eq!(san.to_string(), "--");
        assert_eq!(fen(pos.chess()), "4k3/8/8/8/4P3/8/8/4K3 w - - 4 21");
    }

    #[test]
    fn null_move_in_check_is_illegal() {
        let chess = chess_from_fen("4k3/8/8/8/8/8/8/4R1K1 b - - 0 1").unwrap();
        let mut pos = ScidPosition::from_setup_position(chess).unwrap();
        assert!(matches!(
            pos.play(GameMove::Null),
            Err(Error::InvalidMove { .. })
        ));
    }

    #[test]
    fn invalid_fens() {
        for fen in [
            "",
            "not a fen",
            "8/8/8/8/8/8/8/8 w - - 0 1",
            "4k3/8/8/8/8/8/8/4K2P w - - 0 1",
            "4k3/8/8/8/8/8/8/4R1K1 w - - 0 1",
        ] {
            assert!(
                matches!(chess_from_fen(fen), Err(Error::InvalidFen { .. })),
                "{fen:?} should be rejected"
            );
        }
    }

    #[test]
    fn lenient_fens() {
        // Castling rights without the rook, and an en passant square without
        // a pawn that could have moved there, are dropped.
        let chess = chess_from_fen("4k3/8/8/8/8/8/8/4K3 w KQkq e6 0 1").unwrap();
        assert_eq!(fen(&chess), "4k3/8/8/8/8/8/8/4K3 w - - 0 1");
    }
}
