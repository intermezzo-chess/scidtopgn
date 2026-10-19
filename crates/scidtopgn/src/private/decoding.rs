use crate::bytebuf::ByteBuffer;
use crate::common::{Color, Piece};
use crate::error::{Error, Result};
use crate::mov::{
    decode_bishop, decode_king, decode_knight, decode_pawn, decode_queen, decode_rook,
    parse_move_byte, SimpleMove, ENCODE_COMMENT, ENCODE_END_GAME, ENCODE_END_MARKER, ENCODE_NAG,
    ENCODE_START_MARKER,
};

pub fn decode_move(
    buf: &mut ByteBuffer,
    sm: &mut SimpleMove,
    to_move: Color,
    piece: Piece,
) -> Result<()> {
    let byte = buf.get_byte()?;
    let (piece_num, move_val) = parse_move_byte(byte);

    if move_val >= ENCODE_NAG {
        return Err(Error::Decode(format!(
            "Unexpected special byte {} in move position",
            move_val
        )));
    }

    sm.piece_num = piece_num;

    match piece {
        Piece::King => decode_king(move_val, sm),
        Piece::Queen => decode_queen(buf, move_val, sm),
        Piece::Rook => decode_rook(move_val, sm),
        Piece::Bishop => decode_bishop(move_val, sm),
        Piece::Knight => decode_knight(move_val, sm),
        Piece::Pawn => decode_pawn(move_val, sm, to_move),
        Piece::Empty => Err(Error::Decode(
            "Cannot decode move for empty piece".to_string(),
        )),
    }
}

pub fn is_special_byte(byte: u8) -> bool {
    (ENCODE_NAG..=ENCODE_END_GAME).contains(&byte)
}

pub fn is_move_byte(byte: u8) -> bool {
    let (_, move_val) = parse_move_byte(byte);
    move_val < ENCODE_NAG
}

pub fn skip_tags(buf: &mut ByteBuffer) -> Result<()> {
    loop {
        let byte = buf.get_byte()?;
        if byte == 0 {
            break;
        }

        if byte == 255 {
            buf.skip(3)?;
        } else if byte > 240 {
            let len = buf.get_byte()?;
            buf.skip(len as usize)?;
        } else {
            buf.skip(byte as usize)?;
            let val_len = buf.get_byte()?;
            buf.skip(val_len as usize)?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeElement {
    Move(SimpleMove),
    Nag(u8),
    Comment(String),
    StartVariation,
    EndVariation,
    EndGame,
}

pub struct MoveDecoder<'a> {
    buf: &'a mut ByteBuffer,
    to_move: Color,
    current_piece: Piece,
}

impl<'a> MoveDecoder<'a> {
    pub fn new(buf: &'a mut ByteBuffer) -> Self {
        MoveDecoder {
            buf,
            to_move: Color::White,
            current_piece: Piece::Empty,
        }
    }

    pub fn skip_tags(&mut self) -> Result<()> {
        skip_tags(self.buf)
    }

    pub fn read_game_flags(&mut self) -> Result<u8> {
        self.buf.get_byte()
    }

    pub fn set_to_move(&mut self, color: Color) {
        self.to_move = color;
    }

    pub fn flip_to_move(&mut self) {
        self.to_move = self.to_move.flip();
    }

    pub fn set_current_piece(&mut self, piece: Piece) {
        self.current_piece = piece;
    }

    pub fn next_element(&mut self) -> Result<Option<DecodeElement>> {
        let byte = match self.buf.get_byte() {
            Ok(b) => b,
            Err(Error::BufferRead) => return Ok(None),
            Err(e) => return Err(e),
        };

        match byte {
            ENCODE_NAG => {
                let nag_val = self.buf.get_byte()?;
                Ok(Some(DecodeElement::Nag(nag_val)))
            }
            ENCODE_COMMENT => {
                let comment = self.buf.get_terminated_string()?;
                Ok(Some(DecodeElement::Comment(comment)))
            }
            ENCODE_START_MARKER => Ok(Some(DecodeElement::StartVariation)),
            ENCODE_END_MARKER => Ok(Some(DecodeElement::EndVariation)),
            ENCODE_END_GAME => Ok(Some(DecodeElement::EndGame)),
            _ => {
                let (piece_num, move_val) = parse_move_byte(byte);

                if move_val >= ENCODE_NAG {
                    return Err(Error::Decode(format!("Invalid move value: {}", move_val)));
                }

                let mut sm = SimpleMove::new();
                sm.piece_num = piece_num;

                let piece = self.current_piece;

                match piece {
                    Piece::King => decode_king(move_val, &mut sm)?,
                    Piece::Queen => decode_queen(self.buf, move_val, &mut sm)?,
                    Piece::Rook => decode_rook(move_val, &mut sm)?,
                    Piece::Bishop => decode_bishop(move_val, &mut sm)?,
                    Piece::Knight => decode_knight(move_val, &mut sm)?,
                    Piece::Pawn => decode_pawn(move_val, &mut sm, self.to_move)?,
                    Piece::Empty => return Err(Error::Decode("No piece type set".to_string())),
                }

                Ok(Some(DecodeElement::Move(sm)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_special_byte() {
        assert!(is_special_byte(ENCODE_NAG));
        assert!(is_special_byte(ENCODE_COMMENT));
        assert!(is_special_byte(ENCODE_START_MARKER));
        assert!(is_special_byte(ENCODE_END_MARKER));
        assert!(is_special_byte(ENCODE_END_GAME));
        assert!(!is_special_byte(0));
        assert!(!is_special_byte(10));
    }

    #[test]
    fn test_is_move_byte() {
        assert!(is_move_byte(0x00));
        assert!(is_move_byte(0x10));
        assert!(is_move_byte(0xA0));
        assert!(!is_move_byte(0x0B));
        assert!(!is_move_byte(0x0F));
        assert!(!is_move_byte(0xAF));
    }
}
