use crate::common::{Color, GameResult, Piece, Square};
use crate::error::{Error, Result};
use crate::game::Game;
use crate::mov::SimpleMove;
use crate::position::Board;
use crate::Date;

pub struct PgnParser {
    input: Vec<char>,
    pos: usize,
    line: u32,
}

impl PgnParser {
    pub fn new(input: &str) -> Self {
        PgnParser {
            input: input.chars().collect(),
            pos: 0,
            line: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        if self.pos < self.input.len() {
            Some(self.input[self.pos])
        } else {
            None
        }
    }

    fn advance(&mut self) -> Option<char> {
        if self.pos < self.input.len() {
            let c = self.input[self.pos];
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
            }
            Some(c)
        } else {
            None
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_to_end_of_line(&mut self) {
        while let Some(c) = self.peek() {
            self.advance();
            if c == '\n' {
                break;
            }
        }
    }

    fn read_string(&mut self) -> Result<String> {
        self.skip_whitespace();
        if self.peek() != Some('"') {
            return Err(Error::Parse {
                line: self.line,
                message: "Expected '\"'".to_string(),
            });
        }
        self.advance();

        let mut result = String::new();
        while let Some(c) = self.peek() {
            if c == '"' {
                self.advance();
                return Ok(result);
            }
            if c == '\\' {
                self.advance();
                if let Some(escaped) = self.advance() {
                    result.push(escaped);
                }
            } else {
                result.push(self.advance().unwrap());
            }
        }
        Err(Error::Parse {
            line: self.line,
            message: "Unterminated string".to_string(),
        })
    }

    pub fn parse_game(&mut self) -> Result<Game> {
        let mut game = Game::new();

        loop {
            self.skip_whitespace();

            match self.peek() {
                None => break,
                Some('[') => {
                    self.parse_tag(&mut game)?;
                }
                Some('%') => {
                    self.skip_to_end_of_line();
                }
                Some('{') | Some(';') => {
                    self.skip_comment();
                }
                Some('(') => {
                    if game.first_move.is_none() {
                        let start_idx = game.moves.len();
                        game.moves.push(crate::game::MoveNode::new());
                        game.first_move = Some(start_idx);
                        game.current_move = Some(start_idx);
                    }
                    break;
                }
                _ => {
                    if self
                        .peek()
                        .map(|c| c.is_ascii_alphanumeric() || c == '$' || c == '*')
                        .unwrap_or(false)
                    {
                        break;
                    }
                    self.advance();
                }
            }
        }

        if game.first_move.is_none() {
            let start_idx = game.moves.len();
            game.moves.push(crate::game::MoveNode::new());
            game.first_move = Some(start_idx);
            game.current_move = Some(start_idx);
        }

        self.parse_moves(&mut game)?;

        Ok(game)
    }

    fn parse_tag(&mut self, game: &mut Game) -> Result<()> {
        self.advance();

        self.skip_whitespace();
        let name = self.read_tag_name()?;

        self.skip_whitespace();
        let value = self.read_string()?;

        self.skip_whitespace();
        if self.peek() != Some(']') {
            return Err(Error::Parse {
                line: self.line,
                message: "Expected ']'".to_string(),
            });
        }
        self.advance();

        match name.as_str() {
            "Event" => game.event = value,
            "Site" => game.site = value,
            "Date" => game.date = Date::from_string(&value),
            "Round" => game.round = value,
            "White" => game.white = value,
            "Black" => game.black = value,
            "Result" => game.result = parse_result(&value),
            "WhiteElo" => game.white_elo = value.parse().ok(),
            "BlackElo" => game.black_elo = value.parse().ok(),
            "ECO" => game.eco = Some(value),
            "EventDate" => game.event_date = Some(Date::from_string(&value)),
            _ => game.add_tag(&name, &value),
        }

        Ok(())
    }

    fn read_tag_name(&mut self) -> Result<String> {
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c.is_whitespace() || c == ']' {
                break;
            }
            name.push(self.advance().unwrap());
        }
        Ok(name)
    }

    fn parse_moves(&mut self, game: &mut Game) -> Result<()> {
        let current_board = if game.non_standard_start {
            game.start_board.clone().unwrap_or_else(Board::std_start)
        } else {
            Board::std_start()
        };
        game.current_board = current_board.clone();

        loop {
            self.skip_whitespace();

            match self.peek() {
                None => break,
                Some(' ') | Some('\t') | Some('\n') | Some('\r') => {
                    self.advance();
                    continue;
                }
                Some(';') => {
                    self.skip_to_end_of_line();
                    continue;
                }
                Some('{') => {
                    let comment = self.read_brace_comment()?;
                    if let Some(current_idx) = game.current_move {
                        if let Some(prev_idx) = game.moves[current_idx].prev {
                            game.moves[prev_idx].comment = Some(comment);
                        }
                    }
                    continue;
                }
                Some('(') => {
                    self.advance();
                    self.parse_variation(game)?;
                    continue;
                }
                Some('$') => {
                    self.advance();
                    let nag = self.read_nag()?;
                    if let Some(current_idx) = game.current_move {
                        if let Some(prev_idx) = game.moves[current_idx].prev {
                            if game.moves[prev_idx].nags.len() < 8 {
                                game.moves[prev_idx].nags.push(nag);
                            }
                        }
                    }
                    continue;
                }
                Some('*') | Some('1') | Some('0') => {
                    let result_str = self.read_result_token()?;
                    if result_str == "*"
                        || result_str == "1-0"
                        || result_str == "0-1"
                        || result_str == "1/2-1/2"
                        || result_str == "½-½"
                    {
                        break;
                    }
                }
                Some('%') => {
                    self.skip_to_end_of_line();
                    continue;
                }
                _ => {}
            }

            let token = self.read_token()?;

            if token.is_empty() {
                continue;
            }

            if token == "1-0" || token == "0-1" || token == "1/2-1/2" || token == "*" {
                break;
            }

            if token == "½-½" {
                break;
            }

            if token.chars().all(|c| c.is_ascii_digit() || c == '.') {
                continue;
            }

            if is_move_token(&token) {
                let sm = parse_san_move(&token, &game.current_board)?;

                let san = token.clone();
                let new_idx = game.moves.len();
                let mut node = crate::game::MoveNode::new();
                node.move_data = sm.clone();
                node.san = san;

                if let Some(current_idx) = game.current_move {
                    node.prev = Some(current_idx);
                    game.moves[current_idx].next = Some(new_idx);
                }

                game.moves.push(node);
                game.current_move = Some(new_idx);

                game.make_move_on_board(&sm)?;
                game.num_half_moves += 1;
            }
        }

        Ok(())
    }

    fn parse_variation(&mut self, game: &mut Game) -> Result<()> {
        let saved_current = game.current_move;
        let saved_board = game.current_board.clone();
        let saved_depth = game.var_depth;

        game.add_variation()?;

        loop {
            self.skip_whitespace();

            match self.peek() {
                None => break,
                Some(')') => {
                    self.advance();
                    break;
                }
                Some('(') => {
                    self.advance();
                    self.parse_variation(game)?;
                }
                Some('{') => {
                    let _ = self.read_brace_comment();
                }
                Some('$') => {
                    self.advance();
                    let _ = self.read_nag();
                }
                Some(';') => {
                    self.skip_to_end_of_line();
                }
                _ => {
                    let token = self.read_token()?;

                    if token.is_empty() || token.chars().all(|c| c.is_ascii_digit() || c == '.') {
                        continue;
                    }

                    if is_move_token(&token) {
                        let sm = parse_san_move(&token, &game.current_board).ok();
                        if let Some(sm) = sm {
                            let new_idx = game.moves.len();
                            let mut node = crate::game::MoveNode::new();
                            node.move_data = sm.clone();
                            node.san = token;

                            if let Some(current_idx) = game.current_move {
                                node.prev = Some(current_idx);
                                game.moves[current_idx].next = Some(new_idx);
                            }

                            game.moves.push(node);
                            game.current_move = Some(new_idx);

                            game.make_move_on_board(&sm)?;
                        }
                    }
                }
            }
        }

        game.current_move = saved_current;
        game.current_board = saved_board;
        game.var_depth = saved_depth;

        Ok(())
    }

    fn skip_comment(&mut self) {
        match self.peek() {
            Some('{') => {
                self.advance();
                while let Some(c) = self.peek() {
                    self.advance();
                    if c == '}' {
                        break;
                    }
                }
            }
            Some(';') => {
                self.skip_to_end_of_line();
            }
            _ => {}
        }
    }

    fn read_brace_comment(&mut self) -> Result<String> {
        if self.peek() != Some('{') {
            return Err(Error::Parse {
                line: self.line,
                message: "Expected '{'".to_string(),
            });
        }
        self.advance();

        let mut comment = String::new();
        while let Some(c) = self.peek() {
            if c == '}' {
                self.advance();
                return Ok(comment);
            }
            comment.push(self.advance().unwrap());
        }

        Err(Error::Parse {
            line: self.line,
            message: "Unterminated comment".to_string(),
        })
    }

    fn read_nag(&mut self) -> Result<u8> {
        let mut num = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                num.push(self.advance().unwrap());
            } else {
                break;
            }
        }
        num.parse().map_err(|_| Error::Parse {
            line: self.line,
            message: "Invalid NAG".to_string(),
        })
    }

    fn read_result_token(&mut self) -> Result<String> {
        let mut result = String::new();
        while let Some(c) = self.peek() {
            if c.is_whitespace() || c == ')' || c == ']' {
                break;
            }
            result.push(self.advance().unwrap());
        }
        Ok(result)
    }

    fn read_token(&mut self) -> Result<String> {
        self.skip_whitespace();

        let mut token = String::new();
        while let Some(c) = self.peek() {
            if c.is_whitespace()
                || c == '('
                || c == ')'
                || c == '{'
                || c == '}'
                || c == '['
                || c == ']'
            {
                break;
            }
            token.push(self.advance().unwrap());
        }
        Ok(token)
    }
}

fn is_move_token(token: &str) -> bool {
    if token.is_empty() {
        return false;
    }

    let first = token.chars().next().unwrap();
    if first == 'O' {
        return true;
    }
    if "KQRBNP".contains(first) {
        return true;
    }
    if first >= 'a' && first <= 'h' {
        return true;
    }
    false
}

fn parse_result(s: &str) -> GameResult {
    match s.trim() {
        "1-0" => GameResult::White,
        "0-1" => GameResult::Black,
        "1/2-1/2" | "½-½" => GameResult::Draw,
        _ => GameResult::None,
    }
}

fn parse_san_move(san: &str, board: &Board) -> Result<SimpleMove> {
    let san = san.trim();

    if san == "O-O" || san == "0-0" {
        return parse_castling(board, true);
    }
    if san == "O-O-O" || san == "0-0-0" {
        return parse_castling(board, false);
    }

    let mut chars = san.chars().peekable();
    let mut piece = Piece::Pawn;
    let mut disambig_file: Option<u8> = None;
    let mut disambig_rank: Option<u8> = None;
    let mut to_file: Option<u8> = None;
    let mut to_rank: Option<u8> = None;
    let mut promotion: Option<Piece> = None;
    let mut is_capture = false;

    if let Some(&c) = chars.peek() {
        if "KQRBN".contains(c) {
            piece = Piece::from_char(chars.next().unwrap());
        }
    }

    let mut first_file_seen = false;
    let mut first_rank_seen = false;

    while let Some(&c) = chars.peek() {
        match c {
            'a'..='h' => {
                let f = (chars.next().unwrap() as u8) - b'a';
                if to_file.is_some() {
                    disambig_file = to_file;
                    to_file = Some(f);
                } else if first_file_seen && !first_rank_seen {
                    to_file = Some(f);
                } else if first_rank_seen {
                    to_file = Some(f);
                } else if !first_file_seen {
                    if piece == Piece::Pawn {
                        to_file = Some(f);
                    } else {
                        if to_file.is_none() {
                            to_file = Some(f);
                        } else {
                            disambig_file = Some(f);
                        }
                    }
                    first_file_seen = true;
                }
            }
            '1'..='8' => {
                let r = (chars.next().unwrap() as u8) - b'1';
                if to_file.is_some() && to_rank.is_none() {
                    to_rank = Some(r);
                } else if first_rank_seen {
                    disambig_rank = Some(r);
                } else if first_file_seen && !first_rank_seen {
                    to_rank = Some(r);
                    first_rank_seen = true;
                } else {
                    to_rank = Some(r);
                    first_rank_seen = true;
                }
            }
            'x' | 'X' => {
                chars.next();
                is_capture = true;
            }
            '=' => {
                chars.next();
                if let Some(prom_char) = chars.next() {
                    promotion = Some(Piece::from_char(prom_char));
                }
            }
            '+' | '#' | '!' | '?' | ' ' => {
                chars.next();
            }
            _ => {
                chars.next();
            }
        }
    }

    let to_file = to_file.ok_or_else(|| Error::Parse {
        line: 0,
        message: format!("No destination file in: {}", san),
    })?;
    let to_rank = to_rank.ok_or_else(|| Error::Parse {
        line: 0,
        message: format!("No destination rank in: {}", san),
    })?;
    let to_sq = Square::make(to_file, to_rank);

    let color = board.to_move;

    let mut sm = SimpleMove::new();
    sm.to = to_sq;
    sm.promote = promotion.unwrap_or(Piece::Empty);

    if piece == Piece::Pawn {
        if is_capture {
            let from_f = disambig_file.unwrap_or(to_file);
            let from_r = if let Some(r) = disambig_rank {
                r
            } else {
                if color == Color::White {
                    if to_rank > 0 {
                        to_rank - 1
                    } else {
                        0
                    }
                } else {
                    if to_rank < 7 {
                        to_rank + 1
                    } else {
                        7
                    }
                }
            };
            sm.from = Square::make(from_f, from_r);
        } else {
            let from_f = to_file;
            let from_r = if let Some(r) = disambig_rank {
                r
            } else if color == Color::White {
                if board.get_piece(Square::make(from_f, to_rank - 1)).is_some() {
                    to_rank - 1
                } else if to_rank >= 2
                    && board.get_piece(Square::make(from_f, to_rank - 2)).is_some()
                {
                    to_rank - 2
                } else {
                    to_rank - 1
                }
            } else {
                if board.get_piece(Square::make(from_f, to_rank + 1)).is_some() {
                    to_rank + 1
                } else if to_rank <= 5
                    && board.get_piece(Square::make(from_f, to_rank + 2)).is_some()
                {
                    to_rank + 2
                } else {
                    to_rank + 1
                }
            };
            sm.from = Square::make(from_f, from_r);
        }
    } else {
        let mut found_from: Option<Square> = None;

        for piece_num in 0..16 {
            let sq = board.piece_list.get_square(color, piece_num);
            if !sq.is_valid() {
                continue;
            }

            if let Some((p, c)) = board.get_piece(sq) {
                if c != color || p != piece {
                    continue;
                }

                if let Some(df) = disambig_file {
                    if sq.file() != df {
                        continue;
                    }
                }
                if let Some(dr) = disambig_rank {
                    if sq.rank() != dr {
                        continue;
                    }
                }

                if can_piece_reach(piece, sq, to_sq, board) {
                    if found_from.is_none() {
                        found_from = Some(sq);
                    } else {
                        return Err(Error::Parse {
                            line: 0,
                            message: format!("Ambiguous move: {}", san),
                        });
                    }
                }
            }
        }

        sm.from = found_from.ok_or_else(|| Error::Parse {
            line: 0,
            message: format!("No piece found for move: {}", san),
        })?;
    }

    sm.piece_num = board.piece_list.get_piece_num(sm.from);

    if let Some((cap_piece, cap_color)) = board.get_piece(sm.to) {
        if cap_color != color {
            sm.captured_piece = cap_piece;
            sm.captured_square = sm.to;
        }
    }

    if piece == Piece::Pawn && is_capture && sm.captured_piece == Piece::Empty {
        sm.captured_piece = Piece::Pawn;
        let ep_rank = if color == Color::White {
            to_rank - 1
        } else {
            to_rank + 1
        };
        sm.captured_square = Square::make(to_file, ep_rank);
    }

    Ok(sm)
}

fn parse_castling(board: &Board, kingside: bool) -> Result<SimpleMove> {
    let color = board.to_move;
    let king_sq = board.piece_list.get_king_square(color);

    let mut sm = SimpleMove::new();
    sm.from = king_sq;
    sm.piece_num = 0;

    if kingside {
        sm.to = Square::make(6, king_sq.rank());
    } else {
        sm.to = Square::make(2, king_sq.rank());
    }

    Ok(sm)
}

fn can_piece_reach(piece: Piece, from: Square, to: Square, board: &Board) -> bool {
    let rank_diff = (to.rank() as i8 - from.rank() as i8).abs();
    let file_diff = (to.file() as i8 - from.file() as i8).abs();

    let can_reach = match piece {
        Piece::Knight => (rank_diff == 2 && file_diff == 1) || (rank_diff == 1 && file_diff == 2),
        Piece::Bishop => rank_diff == file_diff && rank_diff > 0,
        Piece::Rook => rank_diff == 0 || file_diff == 0,
        Piece::Queen => rank_diff == file_diff || rank_diff == 0 || file_diff == 0,
        Piece::King => rank_diff <= 1 && file_diff <= 1,
        Piece::Pawn => false,
        Piece::Empty => false,
    };

    if !can_reach {
        return false;
    }

    if piece == Piece::Knight {
        return true;
    }

    let dr = (to.rank() as i8 - from.rank() as i8).signum();
    let df = (to.file() as i8 - from.file() as i8).signum();

    let mut check_sq = from;
    loop {
        let new_rank = check_sq.rank() as i8 + dr;
        let new_file = check_sq.file() as i8 + df;

        if new_rank < 0 || new_rank > 7 || new_file < 0 || new_file > 7 {
            return false;
        }

        check_sq = Square::make(new_file as u8, new_rank as u8);

        if check_sq == to {
            return true;
        }

        if board.get_piece(check_sq).is_some() {
            return false;
        }
    }
}

pub fn parse_pgn(pgn: &str) -> Result<Vec<Game>> {
    let mut parser = PgnParser::new(pgn);
    let mut games = Vec::new();

    loop {
        parser.skip_whitespace();
        if parser.peek().is_none() {
            break;
        }

        match parser.parse_game() {
            Ok(game) => games.push(game),
            Err(Error::Parse { .. }) => break,
            Err(e) => return Err(e),
        }
    }

    Ok(games)
}

pub fn parse_single_game(pgn: &str) -> Result<Game> {
    let mut parser = PgnParser::new(pgn);
    parser.parse_game()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_game() {
        let pgn = r#"[Event "Test"]
[Site "Test Site"]
[Date "2024.01.01"]
[Round "1"]
[White "Player1"]
[Black "Player2"]
[Result "1-0"]

1. e4 e5 2. Nf3 Nc6 1-0
"#;

        let game = parse_single_game(pgn).unwrap();
        assert_eq!(game.event, "Test");
        assert_eq!(game.white, "Player1");
        assert_eq!(game.black, "Player2");
        assert_eq!(game.result, GameResult::White);
    }

    #[test]
    fn test_parse_result() {
        assert_eq!(parse_result("1-0"), GameResult::White);
        assert_eq!(parse_result("0-1"), GameResult::Black);
        assert_eq!(parse_result("1/2-1/2"), GameResult::Draw);
        assert_eq!(parse_result("*"), GameResult::None);
    }

    #[test]
    fn test_is_move_token() {
        assert!(is_move_token("e4"));
        assert!(is_move_token("Nf3"));
        assert!(is_move_token("O-O"));
        assert!(!is_move_token(""));
        assert!(!is_move_token("1."));
    }
}
