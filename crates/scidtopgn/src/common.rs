use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Piece {
    King,
    Queen,
    Rook,
    Bishop,
    Knight,
    Pawn,
    Empty,
}

impl Piece {
    pub fn from_char(c: char) -> Self {
        match c {
            'K' => Piece::King,
            'Q' => Piece::Queen,
            'R' => Piece::Rook,
            'B' => Piece::Bishop,
            'N' => Piece::Knight,
            'P' => Piece::Pawn,
            _ => Piece::Empty,
        }
    }

    pub fn to_char(self) -> char {
        match self {
            Piece::King => 'K',
            Piece::Queen => 'Q',
            Piece::Rook => 'R',
            Piece::Bishop => 'B',
            Piece::Knight => 'N',
            Piece::Pawn => 'P',
            Piece::Empty => ' ',
        }
    }

    pub fn to_byte(self) -> u8 {
        match self {
            Piece::King => 1,
            Piece::Queen => 2,
            Piece::Rook => 3,
            Piece::Bishop => 4,
            Piece::Knight => 5,
            Piece::Pawn => 6,
            Piece::Empty => 7,
        }
    }

    pub fn from_byte(b: u8) -> Self {
        match b & 7 {
            1 => Piece::King,
            2 => Piece::Queen,
            3 => Piece::Rook,
            4 => Piece::Bishop,
            5 => Piece::Knight,
            6 => Piece::Pawn,
            _ => Piece::Empty,
        }
    }

    pub fn is_slider(self) -> bool {
        matches!(self, Piece::Queen | Piece::Rook | Piece::Bishop)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn flip(self) -> Self {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }

    pub fn to_char(self) -> char {
        match self {
            Color::White => 'W',
            Color::Black => 'B',
        }
    }

    pub fn from_byte(b: u8) -> Self {
        if b & 8 != 0 {
            Color::Black
        } else {
            Color::White
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Square {
    A1,
    B1,
    C1,
    D1,
    E1,
    F1,
    G1,
    H1,
    A2,
    B2,
    C2,
    D2,
    E2,
    F2,
    G2,
    H2,
    A3,
    B3,
    C3,
    D3,
    E3,
    F3,
    G3,
    H3,
    A4,
    B4,
    C4,
    D4,
    E4,
    F4,
    G4,
    H4,
    A5,
    B5,
    C5,
    D5,
    E5,
    F5,
    G5,
    H5,
    A6,
    B6,
    C6,
    D6,
    E6,
    F6,
    G6,
    H6,
    A7,
    B7,
    C7,
    D7,
    E7,
    F7,
    G7,
    H7,
    A8,
    B8,
    C8,
    D8,
    E8,
    F8,
    G8,
    H8,
    Null,
}

impl Square {
    pub fn from_index(i: usize) -> Self {
        if i < 64 {
            unsafe { std::mem::transmute::<u8, Square>(i as u8) }
        } else {
            Square::Null
        }
    }

    pub fn to_index(self) -> usize {
        match self {
            Square::Null => 65,
            _ => self as usize,
        }
    }

    pub fn from_coords(file: u8, rank: u8) -> Result<Self> {
        if file > 7 || rank > 7 {
            return Err(Error::InvalidMove {
                move_str: format!("Invalid square: {}{}", (b'a' + file) as char, rank + 1),
            });
        }
        Ok(Square::from_index((rank as usize) * 8 + (file as usize)))
    }

    pub fn from_chars(file: char, rank: char) -> Result<Self> {
        let f = file as u8;
        let r = rank as u8;
        if !(b'a'..=b'h').contains(&f) || !(b'1'..=b'8').contains(&r) {
            return Err(Error::InvalidMove {
                move_str: format!("Invalid square: {}{}", file, rank),
            });
        }
        Ok(Square::from_index(
            ((r - b'1') as usize) * 8 + ((f - b'a') as usize),
        ))
    }

    pub fn file(self) -> u8 {
        self.to_index() as u8 & 7
    }

    pub fn rank(self) -> u8 {
        (self.to_index() as u8) >> 3
    }

    pub fn file_char(self) -> char {
        (b'a' + self.file()) as char
    }

    pub fn rank_char(self) -> char {
        (b'1' + self.rank()) as char
    }

    pub fn make(file: u8, rank: u8) -> Self {
        Square::from_index(((rank as usize) << 3) | (file as usize))
    }

    pub fn color(self) -> Color {
        let diag = self.rank() + self.file();
        if diag & 1 == 0 {
            Color::Black
        } else {
            Color::White
        }
    }

    pub fn is_valid(self) -> bool {
        self != Square::Null
    }

    pub fn distance(self, other: Square) -> u8 {
        let rd = (self.rank() as i8 - other.rank() as i8).unsigned_abs();
        let fd = (self.file() as i8 - other.file() as i8).unsigned_abs();
        rd.max(fd)
    }
}

impl std::fmt::Display for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if *self == Square::Null {
            write!(f, "NS")
        } else {
            write!(f, "{}{}", self.file_char(), self.rank_char())
        }
    }
}

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

pub fn piece_make(color: Color, piece: Piece) -> u8 {
    let c = match color {
        Color::White => 0,
        Color::Black => 8,
    };
    c | piece.to_byte()
}

pub fn piece_color(piece: u8) -> Color {
    Color::from_byte(piece)
}

pub fn piece_type(piece: u8) -> Piece {
    Piece::from_byte(piece)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_flip() {
        assert_eq!(Color::White.flip(), Color::Black);
        assert_eq!(Color::Black.flip(), Color::White);
    }

    #[test]
    fn test_square_coords() {
        let sq = Square::from_chars('e', '4').unwrap();
        assert_eq!(sq.file(), 4);
        assert_eq!(sq.rank(), 3);
        assert_eq!(sq.to_string(), "e4");
    }

    #[test]
    fn test_square_make() {
        assert_eq!(Square::make(0, 0), Square::A1);
        assert_eq!(Square::make(7, 7), Square::H8);
        assert_eq!(Square::make(4, 3), Square::E4);
    }

    #[test]
    fn test_game_result() {
        assert_eq!(GameResult::White.to_string(), "1-0");
        assert_eq!(GameResult::Black.to_string(), "0-1");
        assert_eq!(GameResult::Draw.to_string(), "1/2-1/2");
        assert_eq!(GameResult::None.to_string(), "*");
    }

    #[test]
    fn test_piece() {
        assert_eq!(Piece::from_char('K'), Piece::King);
        assert_eq!(Piece::King.to_char(), 'K');
        assert!(Piece::Queen.is_slider());
        assert!(!Piece::Knight.is_slider());
    }
}
