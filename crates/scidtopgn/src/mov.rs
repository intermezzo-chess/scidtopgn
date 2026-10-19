use crate::bytebuf::ByteBuffer;
use crate::common::{Color, Piece, Square};
use crate::error::{Error, Result};

pub const ENCODE_NAG: u8 = 11;
pub const ENCODE_COMMENT: u8 = 12;
pub const ENCODE_START_MARKER: u8 = 13;
pub const ENCODE_END_MARKER: u8 = 14;
pub const ENCODE_END_GAME: u8 = 15;
pub const ENCODE_FIRST: u8 = 11;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleMove {
    pub piece_num: u8,
    pub from: Square,
    pub to: Square,
    pub promote: Piece,
    pub captured_piece: Piece,
    pub captured_num: u8,
    pub captured_square: Square,
}

impl Default for SimpleMove {
    fn default() -> Self {
        Self::new()
    }
}

impl SimpleMove {
    pub fn new() -> Self {
        SimpleMove {
            piece_num: 0,
            from: Square::Null,
            to: Square::Null,
            promote: Piece::Empty,
            captured_piece: Piece::Empty,
            captured_num: 255,
            captured_square: Square::Null,
        }
    }
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
const PAWN_PROMO: [Piece; 16] = [
    Piece::Empty,
    Piece::Empty,
    Piece::Empty,
    Piece::Queen,
    Piece::Queen,
    Piece::Queen,
    Piece::Rook,
    Piece::Rook,
    Piece::Rook,
    Piece::Bishop,
    Piece::Bishop,
    Piece::Bishop,
    Piece::Knight,
    Piece::Knight,
    Piece::Knight,
    Piece::Empty,
];

const KING_DIFF_TO_VAL: [u8; 19] = [1, 2, 3, 0, 0, 0, 0, 9, 4, 0, 5, 10, 0, 0, 0, 0, 6, 7, 8];

const KNIGHT_DIFF_TO_VAL: [u8; 35] = [
    1, 0, 2, 0, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 6, 0, 0, 0, 0,
    7, 0, 8,
];

pub fn encode_king(buf: &mut ByteBuffer, sm: &SimpleMove) -> Result<()> {
    assert!(sm.piece_num == 0, "Kings MUST be piece number zero");

    if sm.to == sm.from {
        buf.put_byte(make_move_byte(0, 0))?;
        return Ok(());
    }

    let diff = sm.to.to_index() as i32 - sm.from.to_index() as i32;
    assert!(diff >= -9 && diff <= 9, "Invalid king move difference");
    let val = KING_DIFF_TO_VAL[(diff + 9) as usize];
    assert!(val != 0, "Invalid king move");
    buf.put_byte(make_move_byte(0, val))?;
    Ok(())
}

pub fn decode_king(val: u8, sm: &mut SimpleMove) -> Result<()> {
    if val == 0 {
        sm.to = sm.from;
        return Ok(());
    }

    if val < 1 || val > 10 {
        return Err(Error::Decode(format!("Invalid king move value: {}", val)));
    }

    let new_idx = sm.from.to_index() as i32 + KING_SQDIFF[val as usize];
    if new_idx < 0 || new_idx > 63 {
        return Err(Error::Decode(format!(
            "King move out of bounds: {}",
            new_idx
        )));
    }
    sm.to = Square::from_index(new_idx as usize);
    Ok(())
}

pub fn encode_knight(buf: &mut ByteBuffer, sm: &SimpleMove) -> Result<()> {
    let diff = sm.to.to_index() as i32 - sm.from.to_index() as i32;
    assert!(diff >= -17 && diff <= 17, "Invalid knight move difference");
    let val = KNIGHT_DIFF_TO_VAL[(diff + 17) as usize];
    assert!(val != 0, "Invalid knight move");
    buf.put_byte(make_move_byte(sm.piece_num, val))?;
    Ok(())
}

pub fn decode_knight(val: u8, sm: &mut SimpleMove) -> Result<()> {
    if val < 1 || val > 8 {
        return Err(Error::Decode(format!("Invalid knight move value: {}", val)));
    }

    let new_idx = sm.from.to_index() as i32 + KNIGHT_SQDIFF[val as usize];
    if new_idx < 0 || new_idx > 63 {
        return Err(Error::Decode(format!(
            "Knight move out of bounds: {}",
            new_idx
        )));
    }
    sm.to = Square::from_index(new_idx as usize);
    Ok(())
}

pub fn encode_rook(buf: &mut ByteBuffer, sm: &SimpleMove) -> Result<()> {
    let val = if sm.from.rank() == sm.to.rank() {
        sm.to.file()
    } else {
        8 + sm.to.rank()
    };
    buf.put_byte(make_move_byte(sm.piece_num, val))?;
    Ok(())
}

pub fn decode_rook(val: u8, sm: &mut SimpleMove) -> Result<()> {
    if val >= 8 {
        sm.to = Square::make(sm.from.file(), val - 8);
    } else {
        sm.to = Square::make(val, sm.from.rank());
    }
    Ok(())
}

pub fn encode_bishop(buf: &mut ByteBuffer, sm: &SimpleMove) -> Result<()> {
    let mut val = sm.to.file();
    let rank_diff = sm.to.rank() as i32 - sm.from.rank() as i32;
    let fyle_diff = sm.to.file() as i32 - sm.from.file() as i32;

    if rank_diff * fyle_diff < 0 {
        val += 8;
    }

    buf.put_byte(make_move_byte(sm.piece_num, val))?;
    Ok(())
}

pub fn decode_bishop(val: u8, sm: &mut SimpleMove) -> Result<()> {
    let fyle = val & 7;
    let fyle_diff = fyle as i32 - sm.from.file() as i32;

    let to_idx = if val >= 8 {
        sm.from.to_index() as i32 - 7 * fyle_diff
    } else {
        sm.from.to_index() as i32 + 9 * fyle_diff
    };

    if to_idx < 0 || to_idx > 63 {
        return Err(Error::Decode(format!(
            "Bishop move out of bounds: {}",
            to_idx
        )));
    }
    sm.to = Square::from_index(to_idx as usize);
    Ok(())
}

pub fn encode_queen(buf: &mut ByteBuffer, sm: &SimpleMove) -> Result<()> {
    if sm.from.rank() == sm.to.rank() {
        let val = sm.to.file();
        buf.put_byte(make_move_byte(sm.piece_num, val))?;
    } else if sm.from.file() == sm.to.file() {
        let val = 8 + sm.to.rank();
        buf.put_byte(make_move_byte(sm.piece_num, val))?;
    } else {
        let val = sm.from.file();
        buf.put_byte(make_move_byte(sm.piece_num, val))?;
        buf.put_byte(sm.to.to_index() as u8 + 64)?;
    }
    Ok(())
}

pub fn decode_queen(buf: &mut ByteBuffer, val: u8, sm: &mut SimpleMove) -> Result<()> {
    if val >= 8 {
        sm.to = Square::make(sm.from.file(), val - 8);
    } else if val != sm.from.file() {
        sm.to = Square::make(val, sm.from.rank());
    } else {
        let second_byte = buf.get_byte()?;
        if second_byte < 64 || second_byte > 127 {
            return Err(Error::Decode(format!(
                "Invalid queen diagonal second byte: {}",
                second_byte
            )));
        }
        sm.to = Square::from_index((second_byte - 64) as usize);
    }
    Ok(())
}

pub fn encode_pawn(buf: &mut ByteBuffer, sm: &SimpleMove) -> Result<()> {
    let diff = (sm.to.to_index() as i32 - sm.from.to_index() as i32).abs();

    let val = if diff == 16 {
        15
    } else {
        let base_val = match diff {
            7 => 0,
            8 => 1,
            9 => 2,
            _ => panic!("Invalid pawn move difference: {}", diff),
        };

        match sm.promote {
            Piece::Queen => base_val + 3,
            Piece::Rook => base_val + 6,
            Piece::Bishop => base_val + 9,
            Piece::Knight => base_val + 12,
            Piece::Empty => base_val,
            _ => panic!("Invalid promotion piece"),
        }
    };

    buf.put_byte(make_move_byte(sm.piece_num, val))?;
    Ok(())
}

pub fn decode_pawn(val: u8, sm: &mut SimpleMove, to_move: Color) -> Result<()> {
    let diff = PAWN_SQDIFF[val as usize];

    let to_idx = if to_move == Color::White {
        sm.from.to_index() as i32 + diff
    } else {
        sm.from.to_index() as i32 - diff
    };

    if to_idx < 0 || to_idx > 63 {
        return Err(Error::Decode(format!(
            "Pawn move out of bounds: {}",
            to_idx
        )));
    }
    sm.to = Square::from_index(to_idx as usize);
    sm.promote = PAWN_PROMO[val as usize];

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_parse_move_byte() {
        let byte = make_move_byte(5, 7);
        assert_eq!(byte, 0x57);
        let (piece, val) = parse_move_byte(byte);
        assert_eq!(piece, 5);
        assert_eq!(val, 7);
    }

    #[test]
    fn test_king_decode_null_move() {
        let mut sm = SimpleMove::new();
        sm.from = Square::E1;
        decode_king(0, &mut sm).unwrap();
        assert_eq!(sm.to, Square::E1);
    }

    #[test]
    fn test_king_decode_castling() {
        let mut sm = SimpleMove::new();
        sm.from = Square::E1;
        decode_king(10, &mut sm).unwrap();
        assert_eq!(sm.to, Square::G1);

        let mut sm = SimpleMove::new();
        sm.from = Square::E1;
        decode_king(9, &mut sm).unwrap();
        assert_eq!(sm.to, Square::C1);
    }

    #[test]
    fn test_knight_decode() {
        let mut sm = SimpleMove::new();
        sm.from = Square::B1;
        decode_knight(8, &mut sm).unwrap();
        assert_eq!(sm.to, Square::C3);

        let mut sm = SimpleMove::new();
        sm.from = Square::G1;
        decode_knight(5, &mut sm).unwrap();
        assert_eq!(sm.to, Square::E2);
    }

    #[test]
    fn test_rook_decode() {
        let mut sm = SimpleMove::new();
        sm.from = Square::A1;
        decode_rook(4, &mut sm).unwrap();
        assert_eq!(sm.to, Square::E1);

        let mut sm = SimpleMove::new();
        sm.from = Square::A1;
        decode_rook(8 + 4, &mut sm).unwrap();
        assert_eq!(sm.to, Square::A5);
    }

    #[test]
    fn test_bishop_decode() {
        let mut sm = SimpleMove::new();
        sm.from = Square::C1;
        decode_bishop(5, &mut sm).unwrap();
        assert_eq!(sm.to, Square::F4);

        let mut sm = SimpleMove::new();
        sm.from = Square::C1;
        decode_bishop(8 + 0, &mut sm).unwrap();
        assert_eq!(sm.to, Square::A3);

        let mut sm = SimpleMove::new();
        sm.from = Square::C1;
        decode_bishop(8 + 1, &mut sm).unwrap();
        assert_eq!(sm.to, Square::B2);
    }

    #[test]
    fn test_pawn_decode_white() {
        let mut sm = SimpleMove::new();
        sm.from = Square::E2;
        decode_pawn(1, &mut sm, Color::White).unwrap();
        assert_eq!(sm.to, Square::E3);
        assert_eq!(sm.promote, Piece::Empty);

        let mut sm = SimpleMove::new();
        sm.from = Square::E2;
        decode_pawn(15, &mut sm, Color::White).unwrap();
        assert_eq!(sm.to, Square::E4);
        assert_eq!(sm.promote, Piece::Empty);

        let mut sm = SimpleMove::new();
        sm.from = Square::E7;
        decode_pawn(4, &mut sm, Color::White).unwrap();
        assert_eq!(sm.to, Square::E8);
        assert_eq!(sm.promote, Piece::Queen);
    }

    #[test]
    fn test_pawn_decode_black() {
        let mut sm = SimpleMove::new();
        sm.from = Square::E7;
        decode_pawn(1, &mut sm, Color::Black).unwrap();
        assert_eq!(sm.to, Square::E6);
        assert_eq!(sm.promote, Piece::Empty);

        let mut sm = SimpleMove::new();
        sm.from = Square::E7;
        decode_pawn(15, &mut sm, Color::Black).unwrap();
        assert_eq!(sm.to, Square::E5);
        assert_eq!(sm.promote, Piece::Empty);
    }

    #[test]
    fn test_queen_decode_horizontal() {
        let mut sm = SimpleMove::new();
        sm.from = Square::D1;
        let mut buf = ByteBuffer::new();
        decode_queen(&mut buf, 4, &mut sm).unwrap();
        assert_eq!(sm.to, Square::E1);
    }

    #[test]
    fn test_queen_decode_vertical() {
        let mut sm = SimpleMove::new();
        sm.from = Square::D1;
        let mut buf = ByteBuffer::new();
        decode_queen(&mut buf, 8 + 4, &mut sm).unwrap();
        assert_eq!(sm.to, Square::D5);
    }

    #[test]
    fn test_queen_decode_diagonal() {
        let mut sm = SimpleMove::new();
        sm.from = Square::D1;
        let mut buf = ByteBuffer::from_slice(&[Square::H5.to_index() as u8 + 64]);
        decode_queen(&mut buf, 3, &mut sm).unwrap();
        assert_eq!(sm.to, Square::H5);
    }

    #[test]
    fn test_encode_king() {
        let mut buf = ByteBuffer::new();
        let mut sm = SimpleMove::new();
        sm.piece_num = 0;
        sm.from = Square::E1;
        sm.to = Square::E1;
        encode_king(&mut buf, &sm).unwrap();
        assert_eq!(buf.as_slice(), &[0x00]);

        let mut buf = ByteBuffer::new();
        let mut sm = SimpleMove::new();
        sm.piece_num = 0;
        sm.from = Square::E1;
        sm.to = Square::G1;
        encode_king(&mut buf, &sm).unwrap();
        assert_eq!(buf.as_slice(), &[0x0A]);

        let mut buf = ByteBuffer::new();
        let mut sm = SimpleMove::new();
        sm.piece_num = 0;
        sm.from = Square::E1;
        sm.to = Square::C1;
        encode_king(&mut buf, &sm).unwrap();
        assert_eq!(buf.as_slice(), &[0x09]);
    }

    #[test]
    fn test_encode_knight() {
        let mut buf = ByteBuffer::new();
        let mut sm = SimpleMove::new();
        sm.piece_num = 2;
        sm.from = Square::B1;
        sm.to = Square::C3;
        encode_knight(&mut buf, &sm).unwrap();
        assert_eq!(buf.as_slice(), &[0x28]);

        let mut buf = ByteBuffer::new();
        let mut sm = SimpleMove::new();
        sm.piece_num = 2;
        sm.from = Square::G1;
        sm.to = Square::E2;
        encode_knight(&mut buf, &sm).unwrap();
        assert_eq!(buf.as_slice(), &[0x25]);
    }

    #[test]
    fn test_encode_rook() {
        let mut buf = ByteBuffer::new();
        let mut sm = SimpleMove::new();
        sm.piece_num = 1;
        sm.from = Square::A1;
        sm.to = Square::H1;
        encode_rook(&mut buf, &sm).unwrap();
        assert_eq!(buf.as_slice(), &[0x17]);

        let mut buf = ByteBuffer::new();
        let mut sm = SimpleMove::new();
        sm.piece_num = 1;
        sm.from = Square::A1;
        sm.to = Square::A8;
        encode_rook(&mut buf, &sm).unwrap();
        assert_eq!(buf.as_slice(), &[0x1F]);
    }

    #[test]
    fn test_encode_pawn() {
        let mut buf = ByteBuffer::new();
        let mut sm = SimpleMove::new();
        sm.piece_num = 12;
        sm.from = Square::E2;
        sm.to = Square::E4;
        sm.promote = Piece::Empty;
        encode_pawn(&mut buf, &sm).unwrap();
        assert_eq!(buf.as_slice(), &[0xCF]);

        let mut buf = ByteBuffer::new();
        let mut sm = SimpleMove::new();
        sm.piece_num = 12;
        sm.from = Square::E7;
        sm.to = Square::E8;
        sm.promote = Piece::Queen;
        encode_pawn(&mut buf, &sm).unwrap();
        assert_eq!(buf.as_slice(), &[0xC4]);
    }
}
