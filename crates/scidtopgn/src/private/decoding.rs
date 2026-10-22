use crate::bytebuf::ByteBuffer;
use crate::error::Result;
use crate::mov::{parse_move_byte, ENCODE_END_GAME, ENCODE_NAG};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mov::{ENCODE_COMMENT, ENCODE_END_MARKER, ENCODE_START_MARKER};

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
