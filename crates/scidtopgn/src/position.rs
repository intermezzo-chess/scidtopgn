use crate::common::{piece_make, Color, Piece, Square};
use crate::error::{Error, Result};

pub const WQ_CASTLE: u8 = 1;
pub const WK_CASTLE: u8 = 2;
pub const BQ_CASTLE: u8 = 4;
pub const BK_CASTLE: u8 = 8;

#[derive(Debug, Clone)]
pub struct PieceList {
    pub list: [[Square; 16]; 2],
    pub list_pos: [u8; 64],
    pub count: [u32; 2],
}

impl Default for PieceList {
    fn default() -> Self {
        Self::new()
    }
}

impl PieceList {
    pub fn new() -> Self {
        PieceList {
            list: [[Square::Null; 16]; 2],
            list_pos: [255; 64],
            count: [0; 2],
        }
    }

    pub fn get_king_square(&self, color: Color) -> Square {
        self.list[color as usize][0]
    }

    pub fn get_piece_num(&self, square: Square) -> u8 {
        if square == Square::Null || square.to_index() >= 64 {
            return 255;
        }
        self.list_pos[square.to_index()]
    }

    pub fn get_square(&self, color: Color, piece_num: u8) -> Square {
        if (piece_num as usize) < 16 {
            self.list[color as usize][piece_num as usize]
        } else {
            Square::Null
        }
    }

    pub fn add_piece(&mut self, square: Square, piece: Piece, color: Color) {
        let color_idx = color as usize;
        let count = self.count[color_idx] as usize;

        if piece == Piece::King {
            if count > 0 {
                let old_sq = self.list[color_idx][0];
                self.list[color_idx][count] = old_sq;
                if old_sq != Square::Null && old_sq.to_index() < 64 {
                    self.list_pos[old_sq.to_index()] = count as u8;
                }
            }
            self.list[color_idx][0] = square;
            if square != Square::Null && square.to_index() < 64 {
                self.list_pos[square.to_index()] = 0;
            }
        } else {
            if square != Square::Null && square.to_index() < 64 {
                self.list_pos[square.to_index()] = count as u8;
            }
            self.list[color_idx][count] = square;
        }
        self.count[color_idx] += 1;
    }

    pub fn remove_piece(&mut self, square: Square, color: Color) -> u8 {
        let color_idx = color as usize;
        let captured_num = if square != Square::Null && square.to_index() < 64 {
            self.list_pos[square.to_index()]
        } else {
            255
        };

        self.count[color_idx] -= 1;
        let last_idx = self.count[color_idx] as usize;
        let last_piece_square = self.list[color_idx][last_idx];

        if last_piece_square != Square::Null && last_piece_square.to_index() < 64 {
            self.list_pos[last_piece_square.to_index()] = captured_num;
        }
        self.list[color_idx][captured_num as usize] = last_piece_square;

        captured_num
    }

    pub fn move_piece(&mut self, from: Square, to: Square, color: Color) {
        let piece_num = self.list_pos[from.to_index()];
        self.list[color as usize][piece_num as usize] = to;
        self.list_pos[to.to_index()] = piece_num;
        self.list_pos[from.to_index()] = 255;
    }
}

#[derive(Debug, Clone)]
pub struct Board {
    pub squares: [Option<(Piece, Color)>; 64],
    pub to_move: Color,
    pub castling: u8,
    pub ep_target: Option<Square>,
    pub halfmove_clock: u16,
    pub fullmove_number: u16,
    pub piece_list: PieceList,
    pub material: [u8; 16],
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

impl Board {
    pub fn new() -> Self {
        Board {
            squares: [None; 64],
            to_move: Color::White,
            castling: 0,
            ep_target: None,
            halfmove_clock: 0,
            fullmove_number: 1,
            piece_list: PieceList::new(),
            material: [0; 16],
        }
    }

    pub fn clear(&mut self) {
        self.squares = [None; 64];
        self.to_move = Color::White;
        self.castling = 0;
        self.ep_target = None;
        self.halfmove_clock = 0;
        self.fullmove_number = 1;
        self.piece_list = PieceList::new();
        self.material = [0; 16];
    }

    pub fn std_start() -> Self {
        let mut board = Board::new();
        board.clear();

        board.material[piece_make(Color::White, Piece::King) as usize] = 1;
        board.material[piece_make(Color::Black, Piece::King) as usize] = 1;
        board.material[piece_make(Color::White, Piece::Queen) as usize] = 1;
        board.material[piece_make(Color::Black, Piece::Queen) as usize] = 1;
        board.material[piece_make(Color::White, Piece::Rook) as usize] = 2;
        board.material[piece_make(Color::Black, Piece::Rook) as usize] = 2;
        board.material[piece_make(Color::White, Piece::Bishop) as usize] = 2;
        board.material[piece_make(Color::Black, Piece::Bishop) as usize] = 2;
        board.material[piece_make(Color::White, Piece::Knight) as usize] = 2;
        board.material[piece_make(Color::Black, Piece::Knight) as usize] = 2;
        board.material[piece_make(Color::White, Piece::Pawn) as usize] = 8;
        board.material[piece_make(Color::Black, Piece::Pawn) as usize] = 8;

        board.add_to_board(Piece::King, Color::White, Square::E1);
        board.piece_list.list[0][0] = Square::E1;
        board.piece_list.list_pos[Square::E1.to_index()] = 0;

        board.add_to_board(Piece::King, Color::Black, Square::E8);
        board.piece_list.list[1][0] = Square::E8;
        board.piece_list.list_pos[Square::E8.to_index()] = 0;

        board.add_to_board(Piece::Rook, Color::White, Square::A1);
        board.piece_list.list[0][1] = Square::A1;
        board.piece_list.list_pos[Square::A1.to_index()] = 1;

        board.add_to_board(Piece::Rook, Color::Black, Square::A8);
        board.piece_list.list[1][1] = Square::A8;
        board.piece_list.list_pos[Square::A8.to_index()] = 1;

        board.add_to_board(Piece::Knight, Color::White, Square::B1);
        board.piece_list.list[0][2] = Square::B1;
        board.piece_list.list_pos[Square::B1.to_index()] = 2;

        board.add_to_board(Piece::Knight, Color::Black, Square::B8);
        board.piece_list.list[1][2] = Square::B8;
        board.piece_list.list_pos[Square::B8.to_index()] = 2;

        board.add_to_board(Piece::Bishop, Color::White, Square::C1);
        board.piece_list.list[0][3] = Square::C1;
        board.piece_list.list_pos[Square::C1.to_index()] = 3;

        board.add_to_board(Piece::Bishop, Color::Black, Square::C8);
        board.piece_list.list[1][3] = Square::C8;
        board.piece_list.list_pos[Square::C8.to_index()] = 3;

        board.add_to_board(Piece::Queen, Color::White, Square::D1);
        board.piece_list.list[0][4] = Square::D1;
        board.piece_list.list_pos[Square::D1.to_index()] = 4;

        board.add_to_board(Piece::Queen, Color::Black, Square::D8);
        board.piece_list.list[1][4] = Square::D8;
        board.piece_list.list_pos[Square::D8.to_index()] = 4;

        board.add_to_board(Piece::Bishop, Color::White, Square::F1);
        board.piece_list.list[0][5] = Square::F1;
        board.piece_list.list_pos[Square::F1.to_index()] = 5;

        board.add_to_board(Piece::Bishop, Color::Black, Square::F8);
        board.piece_list.list[1][5] = Square::F8;
        board.piece_list.list_pos[Square::F8.to_index()] = 5;

        board.add_to_board(Piece::Knight, Color::White, Square::G1);
        board.piece_list.list[0][6] = Square::G1;
        board.piece_list.list_pos[Square::G1.to_index()] = 6;

        board.add_to_board(Piece::Knight, Color::Black, Square::G8);
        board.piece_list.list[1][6] = Square::G8;
        board.piece_list.list_pos[Square::G8.to_index()] = 6;

        board.add_to_board(Piece::Rook, Color::White, Square::H1);
        board.piece_list.list[0][7] = Square::H1;
        board.piece_list.list_pos[Square::H1.to_index()] = 7;

        board.add_to_board(Piece::Rook, Color::Black, Square::H8);
        board.piece_list.list[1][7] = Square::H8;
        board.piece_list.list_pos[Square::H8.to_index()] = 7;

        for i in 0..8u8 {
            let white_sq = Square::from_index((8 + i) as usize);
            let black_sq = Square::from_index((48 + i) as usize);

            board.add_to_board(Piece::Pawn, Color::White, white_sq);
            board.piece_list.list[0][(8 + i) as usize] = white_sq;
            board.piece_list.list_pos[white_sq.to_index()] = 8 + i;

            board.add_to_board(Piece::Pawn, Color::Black, black_sq);
            board.piece_list.list[1][(8 + i) as usize] = black_sq;
            board.piece_list.list_pos[black_sq.to_index()] = 8 + i;
        }

        board.piece_list.count[0] = 16;
        board.piece_list.count[1] = 16;

        board.castling = WQ_CASTLE | WK_CASTLE | BQ_CASTLE | BK_CASTLE;
        board.to_move = Color::White;
        board.fullmove_number = 1;
        board.halfmove_clock = 0;

        board
    }

    pub fn add_to_board(&mut self, piece: Piece, color: Color, square: Square) {
        self.squares[square.to_index()] = Some((piece, color));
    }

    pub fn remove_from_board(&mut self, square: Square) {
        self.squares[square.to_index()] = None;
    }

    pub fn get_piece(&self, square: Square) -> Option<(Piece, Color)> {
        if square.to_index() >= 64 {
            return None;
        }
        self.squares[square.to_index()]
    }

    pub fn add_piece(&mut self, piece: Piece, color: Color, square: Square) -> Result<()> {
        let color_idx = color as usize;

        if self.piece_list.count[color_idx] >= 16 {
            return Err(Error::InvalidMove {
                move_str: "Too many pieces for one side".to_string(),
            });
        }

        if piece == Piece::King && self.material[piece_make(color, piece) as usize] > 0 {
            return Err(Error::InvalidMove {
                move_str: "Cannot have more than one king per side".to_string(),
            });
        }

        self.material[piece_make(color, piece) as usize] += 1;
        self.piece_list.add_piece(square, piece, color);
        self.add_to_board(piece, color, square);

        Ok(())
    }

    pub fn from_fen(fen: &str) -> Result<Self> {
        let mut board = Board::new();
        board.clear();

        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.is_empty() {
            return Err(Error::InvalidMove {
                move_str: "Empty FEN string".to_string(),
            });
        }

        let ranks: Vec<&str> = parts[0].split('/').collect();
        if ranks.len() != 8 {
            return Err(Error::InvalidMove {
                move_str: format!("Invalid FEN: expected 8 ranks, got {}", ranks.len()),
            });
        }

        for (rank_idx, rank_str) in ranks.iter().enumerate() {
            let rank = 7 - rank_idx;
            let mut file = 0;

            for c in rank_str.chars() {
                if file >= 8 {
                    return Err(Error::InvalidMove {
                        move_str: format!("Invalid FEN: too many squares in rank {}", rank + 1),
                    });
                }

                if c.is_ascii_digit() {
                    let skip = c.to_digit(10).unwrap() as u8;
                    file += skip;
                } else {
                    let color = if c.is_uppercase() {
                        Color::White
                    } else {
                        Color::Black
                    };
                    let piece = Piece::from_char(c.to_ascii_uppercase());

                    if piece == Piece::Empty {
                        return Err(Error::InvalidMove {
                            move_str: format!("Invalid FEN piece character: {}", c),
                        });
                    }

                    let square = Square::make(file as u8, rank as u8);
                    board.add_piece(piece, color, square)?;
                    file += 1;
                }
            }
        }

        if parts.len() > 1 {
            board.to_move = match parts[1] {
                "w" => Color::White,
                "b" => Color::Black,
                _ => {
                    return Err(Error::InvalidMove {
                        move_str: format!("Invalid side to move: {}", parts[1]),
                    })
                }
            };
        }

        if parts.len() > 2 {
            let castling = parts[2];
            if castling != "-" {
                if castling.contains('K') {
                    board.castling |= WK_CASTLE;
                }
                if castling.contains('Q') {
                    board.castling |= WQ_CASTLE;
                }
                if castling.contains('k') {
                    board.castling |= BK_CASTLE;
                }
                if castling.contains('q') {
                    board.castling |= BQ_CASTLE;
                }
            }
        }

        if parts.len() > 3 && parts[3] != "-" {
            let ep = parts[3];
            if ep.len() == 2 {
                board.ep_target = Some(Square::from_chars(
                    ep.chars().next().unwrap(),
                    ep.chars().nth(1).unwrap(),
                )?);
            }
        }

        if parts.len() > 4 {
            board.halfmove_clock = parts[4].parse().unwrap_or(0);
        }

        if parts.len() > 5 {
            board.fullmove_number = parts[5].parse().unwrap_or(1);
        }

        Ok(board)
    }

    pub fn to_fen(&self) -> String {
        let mut fen = String::new();

        for rank in (0..8).rev() {
            let mut empty_count = 0;
            for file in 0..8 {
                let sq = Square::make(file, rank);
                match self.get_piece(sq) {
                    Some((piece, color)) => {
                        if empty_count > 0 {
                            fen.push_str(&empty_count.to_string());
                            empty_count = 0;
                        }
                        let c = piece.to_char();
                        fen.push(if color == Color::White {
                            c
                        } else {
                            c.to_ascii_lowercase()
                        });
                    }
                    None => {
                        empty_count += 1;
                    }
                }
            }
            if empty_count > 0 {
                fen.push_str(&empty_count.to_string());
            }
            if rank > 0 {
                fen.push('/');
            }
        }

        fen.push(' ');
        fen.push(if self.to_move == Color::White {
            'w'
        } else {
            'b'
        });

        fen.push(' ');
        let mut castling = String::new();
        if self.castling & WK_CASTLE != 0 {
            castling.push('K');
        }
        if self.castling & WQ_CASTLE != 0 {
            castling.push('Q');
        }
        if self.castling & BK_CASTLE != 0 {
            castling.push('k');
        }
        if self.castling & BQ_CASTLE != 0 {
            castling.push('q');
        }
        if castling.is_empty() {
            castling.push('-');
        }
        fen.push_str(&castling);

        fen.push(' ');
        match self.ep_target {
            Some(sq) => fen.push_str(&sq.to_string()),
            None => fen.push('-'),
        }

        fen.push(' ');
        fen.push_str(&self.halfmove_clock.to_string());

        fen.push(' ');
        fen.push_str(&self.fullmove_number.to_string());

        fen
    }

    pub fn get_king_square(&self, color: Color) -> Square {
        self.piece_list.get_king_square(color)
    }

    pub fn get_piece_count(&self, color: Color) -> u32 {
        self.piece_list.count[color as usize]
    }

    pub fn total_material(&self) -> u32 {
        self.piece_list.count[0] + self.piece_list.count[1]
    }

    pub fn set_castling(&mut self, color: Color, kingside: bool, value: bool) {
        let flag: u8 = match (color, kingside) {
            (Color::White, true) => WK_CASTLE,
            (Color::White, false) => WQ_CASTLE,
            (Color::Black, true) => BK_CASTLE,
            (Color::Black, false) => BQ_CASTLE,
        };
        if value {
            self.castling |= flag;
        } else {
            self.castling &= !flag;
        }
    }

    pub fn get_castling(&self, color: Color, kingside: bool) -> bool {
        let flag: u8 = match (color, kingside) {
            (Color::White, true) => WK_CASTLE,
            (Color::White, false) => WQ_CASTLE,
            (Color::Black, true) => BK_CASTLE,
            (Color::Black, false) => BQ_CASTLE,
        };
        self.castling & flag != 0
    }

    pub fn is_attacked(&self, square: Square, by_color: Color) -> bool {
        let file = square.file() as i8;
        let rank = square.rank() as i8;

        for i in 0..self.piece_list.count[by_color as usize] as u8 {
            let from_sq = self.piece_list.get_square(by_color, i);
            if from_sq == Square::Null {
                continue;
            }

            if let Some((piece, color)) = self.get_piece(from_sq) {
                if color != by_color {
                    continue;
                }

                let from_file = from_sq.file() as i8;
                let from_rank = from_sq.rank() as i8;
                let file_diff = file - from_file;
                let rank_diff = rank - from_rank;

                match piece {
                    Piece::Pawn => {
                        let direction = if by_color == Color::White { 1 } else { -1 };
                        if rank_diff == direction && file_diff.abs() == 1 {
                            return true;
                        }
                    }
                    Piece::Knight => {
                        if (file_diff.abs() == 2 && rank_diff.abs() == 1)
                            || (file_diff.abs() == 1 && rank_diff.abs() == 2)
                        {
                            return true;
                        }
                    }
                    Piece::Bishop => {
                        if file_diff.abs() == rank_diff.abs() && file_diff != 0 {
                            if self.is_path_clear(from_sq, square) {
                                return true;
                            }
                        }
                    }
                    Piece::Rook => {
                        if (file_diff == 0 || rank_diff == 0) && (file_diff != 0 || rank_diff != 0)
                        {
                            if self.is_path_clear(from_sq, square) {
                                return true;
                            }
                        }
                    }
                    Piece::Queen => {
                        if (file_diff.abs() == rank_diff.abs() || file_diff == 0 || rank_diff == 0)
                            && (file_diff != 0 || rank_diff != 0)
                        {
                            if self.is_path_clear(from_sq, square) {
                                return true;
                            }
                        }
                    }
                    Piece::King => {
                        if file_diff.abs() <= 1
                            && rank_diff.abs() <= 1
                            && (file_diff != 0 || rank_diff != 0)
                        {
                            return true;
                        }
                    }
                    Piece::Empty => {}
                }
            }
        }

        false
    }

    pub fn is_path_clear(&self, from: Square, to: Square) -> bool {
        let from_file = from.file() as i8;
        let from_rank = from.rank() as i8;
        let to_file = to.file() as i8;
        let to_rank = to.rank() as i8;

        let file_step = (to_file - from_file).signum();
        let rank_step = (to_rank - from_rank).signum();

        let mut cur_file = from_file + file_step;
        let mut cur_rank = from_rank + rank_step;

        while cur_file != to_file || cur_rank != to_rank {
            let sq = Square::make(cur_file as u8, cur_rank as u8);
            if self.get_piece(sq).is_some() {
                return false;
            }
            cur_file += file_step;
            cur_rank += rank_step;
        }

        true
    }

    pub fn is_in_check(&self, color: Color) -> bool {
        let king_sq = self.get_king_square(color);
        if king_sq == Square::Null {
            return false;
        }
        let opponent = color.flip();
        self.is_attacked(king_sq, opponent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_piece_list_new() {
        let pl = PieceList::new();
        assert_eq!(pl.count[0], 0);
        assert_eq!(pl.count[1], 0);
    }

    #[test]
    fn test_piece_list_add_piece() {
        let mut pl = PieceList::new();

        pl.add_piece(Square::E1, Piece::King, Color::White);
        assert_eq!(pl.count[0], 1);
        assert_eq!(pl.get_king_square(Color::White), Square::E1);
        assert_eq!(pl.get_piece_num(Square::E1), 0);
    }

    #[test]
    fn test_piece_list_king_swap() {
        let mut pl = PieceList::new();

        pl.add_piece(Square::A1, Piece::Rook, Color::White);
        assert_eq!(pl.list[0][0], Square::A1);
        assert_eq!(pl.list_pos[Square::A1.to_index()], 0);
        assert_eq!(pl.count[0], 1);

        pl.add_piece(Square::E1, Piece::King, Color::White);
        assert_eq!(pl.get_king_square(Color::White), Square::E1);
        assert_eq!(pl.get_piece_num(Square::E1), 0);
        assert_eq!(pl.list[0][1], Square::A1);
        assert_eq!(pl.list_pos[Square::A1.to_index()], 1);
        assert_eq!(pl.count[0], 2);
    }

    #[test]
    fn test_piece_list_remove_piece() {
        let mut pl = PieceList::new();

        pl.add_piece(Square::A1, Piece::Rook, Color::White);
        pl.add_piece(Square::B1, Piece::Knight, Color::White);
        pl.add_piece(Square::C1, Piece::Bishop, Color::White);

        assert_eq!(pl.count[0], 3);

        let captured_num = pl.remove_piece(Square::B1, Color::White);

        assert_eq!(captured_num, 1);
        assert_eq!(pl.count[0], 2);
        assert_eq!(pl.list[0][1], Square::C1);
        assert_eq!(pl.list_pos[Square::C1.to_index()], 1);
    }

    #[test]
    fn test_board_std_start() {
        let board = Board::std_start();

        assert_eq!(board.to_move, Color::White);
        assert_eq!(board.get_piece_count(Color::White), 16);
        assert_eq!(board.get_piece_count(Color::Black), 16);
        assert_eq!(board.get_king_square(Color::White), Square::E1);
        assert_eq!(board.get_king_square(Color::Black), Square::E8);

        assert!(board.get_castling(Color::White, true));
        assert!(board.get_castling(Color::White, false));
        assert!(board.get_castling(Color::Black, true));
        assert!(board.get_castling(Color::Black, false));

        assert_eq!(board.piece_list.get_piece_num(Square::A1), 1);
        assert_eq!(board.piece_list.get_piece_num(Square::H1), 7);
        assert_eq!(board.piece_list.get_piece_num(Square::A2), 8);
        assert_eq!(board.piece_list.get_piece_num(Square::H2), 15);
    }

    #[test]
    fn test_board_from_fen_std_start() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let board = Board::from_fen(fen).unwrap();

        assert_eq!(board.to_move, Color::White);
        assert_eq!(board.get_piece_count(Color::White), 16);
        assert_eq!(board.get_piece_count(Color::Black), 16);
        assert_eq!(board.get_king_square(Color::White), Square::E1);
        assert_eq!(board.get_king_square(Color::Black), Square::E8);
    }

    #[test]
    fn test_board_fen_roundtrip() {
        let board = Board::std_start();
        let fen = board.to_fen();
        let board2 = Board::from_fen(&fen).unwrap();
        let fen2 = board2.to_fen();

        assert_eq!(fen, fen2);
    }

    #[test]
    fn test_capture_swap_algorithm() {
        let mut board = Board::std_start();

        assert_eq!(board.piece_list.get_piece_num(Square::D7), 11);
        assert_eq!(board.piece_list.get_piece_num(Square::H7), 15);
        assert_eq!(board.get_piece_count(Color::Black), 16);

        board.remove_from_board(Square::D7);
        board.material[piece_make(Color::Black, Piece::Pawn) as usize] -= 1;
        board.piece_list.remove_piece(Square::D7, Color::Black);

        assert_eq!(board.get_piece_count(Color::Black), 15);
        assert_eq!(board.piece_list.get_piece_num(Square::H7), 11);
        assert_eq!(board.piece_list.get_square(Color::Black, 11), Square::H7);
    }

    #[test]
    fn test_fen_king_swap() {
        let fen = "8/8/8/8/8/8/8/R3K2R w KQ - 0 1";
        let board = Board::from_fen(fen).unwrap();

        assert_eq!(board.get_king_square(Color::White), Square::E1);
        assert_eq!(board.piece_list.get_piece_num(Square::E1), 0);

        assert_eq!(board.piece_list.get_piece_num(Square::A1), 1);
        assert_eq!(board.piece_list.get_piece_num(Square::H1), 2);
    }
}
