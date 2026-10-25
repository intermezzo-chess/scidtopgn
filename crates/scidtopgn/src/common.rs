#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameResult {
    White,
    Black,
    Draw,
    None,
}

impl GameResult {
    pub fn from_char(c: char) -> Self {
        match c {
            '1' => GameResult::White,
            '0' => GameResult::Black,
            '=' | '2' => GameResult::Draw,
            _ => GameResult::None,
        }
    }

    pub fn to_char(self) -> char {
        match self {
            GameResult::White => '1',
            GameResult::Black => '0',
            GameResult::Draw => '=',
            GameResult::None => '*',
        }
    }

    pub fn from_byte(b: u8) -> Self {
        match b & 3 {
            1 => GameResult::White,
            2 => GameResult::Black,
            3 => GameResult::Draw,
            _ => GameResult::None,
        }
    }

    pub fn to_byte(self) -> u8 {
        match self {
            GameResult::None => 0,
            GameResult::White => 1,
            GameResult::Black => 2,
            GameResult::Draw => 3,
        }
    }
}

impl std::fmt::Display for GameResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            GameResult::White => "1-0",
            GameResult::Black => "0-1",
            GameResult::Draw => "1/2-1/2",
            GameResult::None => "*",
        };
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_result() {
        assert_eq!(GameResult::White.to_string(), "1-0");
        assert_eq!(GameResult::Black.to_string(), "0-1");
        assert_eq!(GameResult::Draw.to_string(), "1/2-1/2");
        assert_eq!(GameResult::None.to_string(), "*");
    }

    #[test]
    fn test_game_result_chars_and_bytes() {
        for (result, ch, byte) in [
            (GameResult::None, '*', 0),
            (GameResult::White, '1', 1),
            (GameResult::Black, '0', 2),
            (GameResult::Draw, '=', 3),
        ] {
            assert_eq!(result.to_char(), ch);
            assert_eq!(GameResult::from_char(ch), result);
            assert_eq!(result.to_byte(), byte);
            assert_eq!(GameResult::from_byte(byte), result);
            // Only the low two bits hold the result.
            assert_eq!(GameResult::from_byte(byte | 0xFC), result);
        }
        assert_eq!(GameResult::from_char('2'), GameResult::Draw);
        assert_eq!(GameResult::from_char('x'), GameResult::None);
    }
}
