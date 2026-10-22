//! SCID's move encoding.
//!
//! A move is one byte, or two for a diagonal queen move. The high nibble is
//! the moving piece's number (see [`PieceList`](crate::position::PieceList))
//! and the low nibble a value whose meaning depends on the piece. Bytes 11 to
//! 15, which no king move uses, mark NAGs, comments, and the ends of
//! variations and the game.

use shakmaty::{CastlingMode, Color, Move, Position, Role, Square};

use crate::bytebuf::ByteBuffer;
use crate::error::{Error, Result};
use crate::position::{fen, ScidPosition};

pub const ENCODE_NAG: u8 = 11;
pub const ENCODE_COMMENT: u8 = 12;
pub const ENCODE_START_MARKER: u8 = 13;
pub const ENCODE_END_MARKER: u8 = 14;
pub const ENCODE_END_GAME: u8 = 15;
pub const ENCODE_FIRST: u8 = 11;

/// A move in a SCID game: a chess move, or the null move that passes the
/// turn, which SCID allows in analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameMove {
    Move(Move),
    Null,
}

pub fn make_move_byte(piece_num: u8, move_value: u8) -> u8 {
    (piece_num << 4) | (move_value & 0x0F)
}

pub fn parse_move_byte(byte: u8) -> (u8, u8) {
    (byte >> 4, byte & 0x0F)
}

const KING_SQDIFF: [i32; 11] = [0, -9, -8, -7, -1, 1, 7, 8, 9, -2, 2];
const KNIGHT_SQDIFF: [i32; 9] = [0, -17, -15, -10, -6, 6, 10, 15, 17];
const PAWN_SQDIFF: [i32; 16] = [7, 8, 9, 7, 8, 9, 7, 8, 9, 7, 8, 9, 7, 8, 9, 16];
const PAWN_PROMO: [Option<Role>; 16] = [
    None,
    None,
    None,
    Some(Role::Queen),
    Some(Role::Queen),
    Some(Role::Queen),
    Some(Role::Rook),
    Some(Role::Rook),
    Some(Role::Rook),
    Some(Role::Bishop),
    Some(Role::Bishop),
    Some(Role::Bishop),
    Some(Role::Knight),
    Some(Role::Knight),
    Some(Role::Knight),
    None,
];

const KING_DIFF_TO_VAL: [u8; 19] = [1, 2, 3, 0, 0, 0, 0, 9, 4, 0, 5, 10, 0, 0, 0, 0, 6, 7, 8];

const KNIGHT_DIFF_TO_VAL: [u8; 35] = [
    1, 0, 2, 0, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 6, 0, 0, 0, 0,
    7, 0, 8,
];

/// The square SCID records as a move's destination: the king's destination
/// for castling.
fn destination(m: Move, color: Color) -> Square {
    match m.castling_side() {
        Some(side) => side.king_to(color),
        None => m.to(),
    }
}

fn square_diff(from: Square, to: Square) -> i32 {
    to.to_u32() as i32 - from.to_u32() as i32
}

/// Decodes the move whose first byte is `byte`, reading a diagonal queen
/// move's second byte from `buf`.
///
/// # Errors
///
/// Returns [`Error::Decode`] if the piece number or value is invalid, or if
/// the move isn't legal in `pos`. A null move is returned without checking;
/// [`ScidPosition::play`] rejects it in check.
pub fn decode_move(buf: &mut ByteBuffer, pos: &ScidPosition, byte: u8) -> Result<GameMove> {
    let (number, value) = parse_move_byte(byte);
    let chess = pos.chess();
    let color = chess.turn();
    let invalid = |reason: &str| {
        Error::Decode(format!(
            "invalid move byte {byte:#04x} ({reason}) in {}",
            fen(chess)
        ))
    };
    let from = pos
        .pieces()
        .square(color, number)
        .ok_or_else(|| invalid("no such piece"))?;
    let role = chess
        .board()
        .role_at(from)
        .ok_or_else(|| invalid("no piece on its square"))?;

    let offset = |diff: i32| from.offset(diff).ok_or_else(|| invalid("off the board"));
    let (to, promotion) = match role {
        Role::King if value == 0 => return Ok(GameMove::Null),
        Role::King => match KING_SQDIFF.get(usize::from(value)) {
            Some(&diff) => (offset(diff)?, None),
            None => return Err(invalid("not a king move")),
        },
        Role::Knight => match KNIGHT_SQDIFF.get(usize::from(value)) {
            Some(&diff) if value != 0 => (offset(diff)?, None),
            _ => return Err(invalid("not a knight move")),
        },
        Role::Rook => (rook_destination(from, value), None),
        Role::Bishop => {
            let file_diff = i32::from(value & 7) - from.file().to_u32() as i32;
            let diff = if value >= 8 {
                -7 * file_diff
            } else {
                9 * file_diff
            };
            (offset(diff)?, None)
        }
        Role::Queen if value >= 8 || value != from.file().to_u32() as u8 => {
            (rook_destination(from, value), None)
        }
        Role::Queen => {
            let second = buf.get_byte()?;
            if !(64..=127).contains(&second) {
                return Err(invalid("bad diagonal queen move"));
            }
            (Square::new(u32::from(second - 64)), None)
        }
        Role::Pawn => {
            let diff = PAWN_SQDIFF[usize::from(value)];
            let diff = if color == Color::White { diff } else { -diff };
            (offset(diff)?, PAWN_PROMO[usize::from(value)])
        }
    };

    chess
        .legal_moves()
        .iter()
        .copied()
        .find(|&m| {
            m.from() == Some(from) && destination(m, color) == to && m.promotion() == promotion
        })
        .map(GameMove::Move)
        .ok_or_else(|| invalid(&format!("{from}{to} is illegal")))
}

/// The destination of a rook move, or a queen move along a rank or file.
fn rook_destination(from: Square, value: u8) -> Square {
    if value >= 8 {
        Square::from_coords(from.file(), shakmaty::Rank::new(u32::from(value - 8)))
    } else {
        Square::from_coords(shakmaty::File::new(u32::from(value)), from.rank())
    }
}

/// Encodes `mv`, which must be legal in `pos`.
///
/// # Errors
///
/// Returns [`Error::InvalidMove`] if the move isn't legal in `pos`.
pub fn encode_move(buf: &mut ByteBuffer, pos: &ScidPosition, mv: GameMove) -> Result<()> {
    let chess = pos.chess();
    let illegal = |m: &str| Error::InvalidMove {
        move_str: format!("{m} is illegal in {}", fen(chess)),
    };
    let m = match mv {
        GameMove::Null if chess.is_check() => return Err(illegal("a null move")),
        // A null move is a king move to its own square.
        GameMove::Null => return buf.put_byte(make_move_byte(0, 0)),
        GameMove::Move(m) => m,
    };
    let (Some(from), true) = (m.from(), chess.is_legal(m)) else {
        return Err(illegal(&m.to_uci(CastlingMode::Standard).to_string()));
    };
    let color = chess.turn();
    let to = destination(m, color);
    let number = pos
        .pieces()
        .number(from)
        .ok_or_else(|| Error::Encoding(format!("no piece number for the piece on {from}")))?;
    let diff = square_diff(from, to);
    let same_rank = from.rank() == to.rank();
    let same_file = from.file() == to.file();

    let value = match m.role() {
        Role::King if number != 0 => {
            return Err(Error::Encoding(format!(
                "the king on {from} has piece number {number}, not 0"
            )))
        }
        Role::King => KING_DIFF_TO_VAL[(diff + 9) as usize],
        Role::Knight => KNIGHT_DIFF_TO_VAL[(diff + 17) as usize],
        Role::Rook | Role::Queen if same_rank => to.file().to_u32() as u8,
        Role::Rook | Role::Queen if same_file => 8 + to.rank().to_u32() as u8,
        Role::Rook => unreachable!("a legal rook move stays on its rank or file"),
        Role::Queen => {
            buf.put_byte(make_move_byte(number, from.file().to_u32() as u8))?;
            return buf.put_byte(to.to_u32() as u8 + 64);
        }
        Role::Bishop => {
            let file_diff = to.file().to_u32() as i32 - from.file().to_u32() as i32;
            let rank_diff = to.rank().to_u32() as i32 - from.rank().to_u32() as i32;
            let value = to.file().to_u32() as u8;
            if file_diff * rank_diff < 0 {
                value + 8
            } else {
                value
            }
        }
        Role::Pawn if diff.abs() == 16 => 15,
        Role::Pawn => {
            let promotion = match m.promotion() {
                None => 0,
                Some(Role::Queen) => 3,
                Some(Role::Rook) => 6,
                Some(Role::Bishop) => 9,
                Some(_) => 12,
            };
            (diff.unsigned_abs() - 7) as u8 + promotion
        }
    };
    buf.put_byte(make_move_byte(number, value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::chess_from_fen;

    fn position(fen: &str) -> ScidPosition {
        ScidPosition::from_setup_position(chess_from_fen(fen).unwrap()).unwrap()
    }

    /// Encodes `uci` in `pos`, checks the bytes, and decodes them back.
    fn round_trip(pos: &ScidPosition, uci: &str, expected: &[u8]) {
        let m = uci
            .parse::<shakmaty::uci::UciMove>()
            .unwrap()
            .to_move(pos.chess())
            .unwrap();
        let mut buf = ByteBuffer::new();
        encode_move(&mut buf, pos, GameMove::Move(m)).unwrap();
        assert_eq!(buf.as_slice(), expected, "encoding {uci}");

        let mut read = ByteBuffer::from_slice(&expected[1..]);
        let decoded = decode_move(&mut read, pos, expected[0]).unwrap();
        assert_eq!(decoded, GameMove::Move(m), "decoding {uci}");
    }

    #[test]
    fn test_make_parse_move_byte() {
        let byte = make_move_byte(5, 7);
        assert_eq!(byte, 0x57);
        assert_eq!(parse_move_byte(byte), (5, 7));
    }

    #[test]
    fn standard_start_moves() {
        let pos = ScidPosition::standard();
        round_trip(&pos, "e2e4", &[0xCF]);
        round_trip(&pos, "e2e3", &[0xC1]);
        round_trip(&pos, "b1c3", &[0x28]);
        round_trip(&pos, "g1f3", &[0x67]);
        round_trip(&pos, "a2a4", &[0x8F]);
    }

    #[test]
    fn castling() {
        let pos = position("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
        round_trip(&pos, "e1g1", &[0x0A]);
        round_trip(&pos, "e1c1", &[0x09]);
        let pos = position("r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1");
        round_trip(&pos, "e8g8", &[0x0A]);
        round_trip(&pos, "e8c8", &[0x09]);
    }

    #[test]
    fn king_moves() {
        let pos = position("4k3/8/8/8/4K3/8/8/8 w - - 0 1");
        for (uci, value) in [
            ("e4d3", 1),
            ("e4e3", 2),
            ("e4f3", 3),
            ("e4d4", 4),
            ("e4f4", 5),
            ("e4d5", 6),
            ("e4e5", 7),
            ("e4f5", 8),
        ] {
            round_trip(&pos, uci, &[value]);
        }
    }

    #[test]
    fn sliders() {
        // FEN order numbers Bc4 0 and Ka1 1, then the king takes 0: Ka1 0,
        // Bc4 1, Qd1 2, Rh1 3.
        let pos = position("4k3/8/8/8/2B5/8/8/K2Q3R w - - 0 1");
        round_trip(&pos, "d1d8", &[0x2F]);
        round_trip(&pos, "d1g1", &[0x26]);
        round_trip(&pos, "d1h5", &[0x23, 39 + 64]);
        round_trip(&pos, "d1a4", &[0x23, 24 + 64]);
        round_trip(&pos, "h1h8", &[0x3F]);
        round_trip(&pos, "h1e1", &[0x34]);
        round_trip(&pos, "c4f7", &[0x15]);
        round_trip(&pos, "c4a6", &[0x18]);
        round_trip(&pos, "c4f1", &[0x1D]);
        round_trip(&pos, "c4a2", &[0x10]);
    }

    #[test]
    fn pawn_captures_promotions_and_en_passant() {
        // Ke1 0, e5 1, a7 2: a7 had 0 until the king took it.
        let pos = position("1n2k3/P7/8/3pP3/8/8/8/4K3 w - d6 0 1");
        round_trip(&pos, "e5d6", &[0x10]);
        round_trip(&pos, "a7a8q", &[0x24]);
        round_trip(&pos, "a7a8r", &[0x27]);
        round_trip(&pos, "a7a8b", &[0x2A]);
        round_trip(&pos, "a7a8n", &[0x2D]);
        round_trip(&pos, "a7b8n", &[0x2E]);
        let pos = position("4k3/8/8/8/3pP3/8/8/4K3 b - e3 0 1");
        round_trip(&pos, "d4e3", &[0x10]);
        round_trip(&pos, "d4d3", &[0x11]);
    }

    #[test]
    fn null_move() {
        let pos = ScidPosition::standard();
        let mut buf = ByteBuffer::new();
        encode_move(&mut buf, &pos, GameMove::Null).unwrap();
        assert_eq!(buf.as_slice(), &[0x00]);
        let decoded = decode_move(&mut ByteBuffer::new(), &pos, 0x00).unwrap();
        assert_eq!(decoded, GameMove::Null);
    }

    #[test]
    fn rejects_illegal_bytes() {
        let pos = ScidPosition::standard();
        // The e-pawn can't capture on f3, and the knight on b1 can't move to
        // d2.
        for byte in [0xC2, 0x26] {
            assert!(
                matches!(
                    decode_move(&mut ByteBuffer::new(), &pos, byte),
                    Err(Error::Decode(_))
                ),
                "{byte:#04x} should be rejected"
            );
        }
        // Lost castling rights.
        let pos = position("r3k2r/8/8/8/8/8/8/R3K2R w kq - 0 1");
        assert!(decode_move(&mut ByteBuffer::new(), &pos, 0x0A).is_err());
        // A piece number past the end of the list.
        let pos = position("4k3/8/8/8/8/8/8/4K3 w - - 0 1");
        assert!(decode_move(&mut ByteBuffer::new(), &pos, 0x17).is_err());
    }

    #[test]
    fn rejects_illegal_moves() {
        let pos = ScidPosition::standard();
        let m = Move::Normal {
            role: Role::Pawn,
            from: Square::E2,
            capture: None,
            to: Square::E5,
            promotion: None,
        };
        let mut buf = ByteBuffer::new();
        assert!(matches!(
            encode_move(&mut buf, &pos, GameMove::Move(m)),
            Err(Error::InvalidMove { .. })
        ));
        assert!(buf.as_slice().is_empty());
    }
}
