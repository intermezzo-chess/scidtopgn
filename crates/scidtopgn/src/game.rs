use crate::bytebuf::ByteBuffer;
use crate::common::{Color, GameResult, Piece, Square};
use crate::error::{Error, Result};
use crate::mov::{
    decode_bishop, decode_king, decode_knight, decode_pawn, decode_queen, decode_rook,
    parse_move_byte, SimpleMove, ENCODE_COMMENT, ENCODE_END_GAME, ENCODE_END_MARKER, ENCODE_NAG,
    ENCODE_START_MARKER,
};
use crate::position::Board;
use crate::Date;

pub const MAX_TAGS: usize = 40;
pub const MAX_TAG_LEN: u8 = 240;
pub const MAX_NAGS: usize = 8;

pub const GAME_DECODE_NONE: u8 = 0;
pub const GAME_DECODE_TAGS: u8 = 1;
pub const GAME_DECODE_COMMENTS: u8 = 2;
pub const GAME_DECODE_ALL: u8 = 3;

static COMMON_TAGS: [&str; 14] = [
    "WhiteCountry",
    "BlackCountry",
    "Annotator",
    "PlyCount",
    "EventDate",
    "Opening",
    "Variation",
    "Setup",
    "Source",
    "SetUp",
    "",
    "",
    "",
    "",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Marker {
    #[default]
    NoMarker,
    StartMarker,
    EndMarker,
    EndGame,
}

#[derive(Debug, Clone)]
pub struct MoveNode {
    pub move_data: SimpleMove,
    pub san: String,
    pub comment: Option<String>,
    pub nags: Vec<u8>,
    pub prev: Option<usize>,
    pub next: Option<usize>,
    pub var_child: Option<usize>,
    pub var_parent: Option<usize>,
    pub marker: Marker,
    pub num_variations: u8,
}

impl Default for MoveNode {
    fn default() -> Self {
        Self::new()
    }
}

impl MoveNode {
    pub fn new() -> Self {
        MoveNode {
            move_data: SimpleMove::new(),
            san: String::new(),
            comment: None,
            nags: Vec::new(),
            prev: None,
            next: None,
            var_child: None,
            var_parent: None,
            marker: Marker::NoMarker,
            num_variations: 0,
        }
    }

    pub fn is_null_move(&self) -> bool {
        self.move_data.from == self.move_data.to
    }
}

#[derive(Debug, Clone)]
pub struct Tag {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct Game {
    pub white: String,
    pub black: String,
    pub event: String,
    pub site: String,
    pub round: String,
    pub date: Date,
    pub event_date: Option<Date>,
    pub result: GameResult,
    pub white_elo: Option<u16>,
    pub black_elo: Option<u16>,
    pub eco: Option<String>,
    pub moves: Vec<MoveNode>,
    pub first_move: Option<usize>,
    pub current_move: Option<usize>,
    pub tags: Vec<Tag>,
    pub start_board: Option<Board>,
    pub current_board: Board,
    pub var_depth: u32,
    pub num_half_moves: u16,
    pub non_standard_start: bool,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    pub fn new() -> Self {
        Game {
            white: "?".to_string(),
            black: "?".to_string(),
            event: "?".to_string(),
            site: "?".to_string(),
            round: "?".to_string(),
            date: Date::ZERO,
            event_date: None,
            result: GameResult::None,
            white_elo: None,
            black_elo: None,
            eco: None,
            moves: Vec::new(),
            first_move: None,
            current_move: None,
            tags: Vec::new(),
            start_board: None,
            current_board: Board::std_start(),
            var_depth: 0,
            num_half_moves: 0,
            non_standard_start: false,
        }
    }

    pub fn clear(&mut self) {
        self.white = "?".to_string();
        self.black = "?".to_string();
        self.event = "?".to_string();
        self.site = "?".to_string();
        self.round = "?".to_string();
        self.date = Date::ZERO;
        self.event_date = None;
        self.result = GameResult::None;
        self.white_elo = None;
        self.black_elo = None;
        self.eco = None;
        self.moves.clear();
        self.first_move = None;
        self.current_move = None;
        self.tags.clear();
        self.start_board = None;
        self.current_board = Board::std_start();
        self.var_depth = 0;
        self.num_half_moves = 0;
        self.non_standard_start = false;
    }

    pub fn add_tag(&mut self, name: &str, value: &str) {
        for tag in &mut self.tags {
            if tag.name == name {
                tag.value = value.to_string();
                return;
            }
        }
        if self.tags.len() < MAX_TAGS {
            self.tags.push(Tag {
                name: name.to_string(),
                value: value.to_string(),
            });
        }
    }

    pub fn find_tag(&self, name: &str) -> Option<&String> {
        self.tags.iter().find(|t| t.name == name).map(|t| &t.value)
    }

    pub fn decode_start(&mut self, buf: &mut ByteBuffer) -> Result<()> {
        self.decode_tags(buf, false)?;

        let flags = buf.get_byte()?;

        self.non_standard_start = (flags & 1) != 0;

        if self.non_standard_start {
            let fen = buf.get_terminated_string()?;
            self.start_board = Some(Board::from_fen(&fen)?);
            self.current_board = self.start_board.clone().unwrap();
        } else {
            self.current_board = Board::std_start();
        }

        Ok(())
    }

    pub fn decode_tags(&mut self, buf: &mut ByteBuffer, store_tags: bool) -> Result<()> {
        let mut b = buf.get_byte()?;

        while b != 0 {
            if b == 255 {
                let mut date_bytes = [0u8; 3];
                date_bytes[0] = buf.get_byte()?;
                date_bytes[1] = buf.get_byte()?;
                date_bytes[2] = buf.get_byte()?;
                let date_val = ((date_bytes[0] as u32) << 16)
                    | ((date_bytes[1] as u32) << 8)
                    | (date_bytes[2] as u32);
                self.event_date = Some(Date::from_raw(date_val));
            } else if b > MAX_TAG_LEN {
                let tag_idx = (b - MAX_TAG_LEN - 1) as usize;
                if tag_idx < COMMON_TAGS.len() {
                    let tag_name = COMMON_TAGS[tag_idx];
                    if !tag_name.is_empty() {
                        let val_len = buf.get_byte()?;
                        let value = buf.get_fixed_string(val_len as usize)?;
                        if store_tags {
                            self.add_tag(tag_name, &value);
                        }
                        if tag_name == "EventDate" {
                            self.event_date = Some(Date::from_string(&value));
                        }
                    }
                }
            } else {
                let tag_name = buf.get_fixed_string(b as usize)?;
                let val_len = buf.get_byte()?;
                let value = buf.get_fixed_string(val_len as usize)?;
                if store_tags {
                    self.add_tag(&tag_name, &value);
                }
            }
            b = buf.get_byte()?;
        }

        Ok(())
    }

    pub fn decode(&mut self, buf: &mut ByteBuffer, flags: u8) -> Result<()> {
        self.clear();

        if flags & GAME_DECODE_TAGS != 0 {
            self.decode_tags(buf, true)?;
        } else {
            self.skip_tags(buf)?;
        }

        let game_flags = buf.get_byte()?;
        self.non_standard_start = (game_flags & 1) != 0;

        if self.non_standard_start {
            let fen = buf.get_terminated_string()?;
            self.start_board = Some(Board::from_fen(&fen)?);
            self.current_board = self.start_board.clone().unwrap();
        }

        let start_node_idx = self.add_marker_node(Marker::StartMarker, None);
        self.first_move = Some(start_node_idx);
        self.current_move = Some(start_node_idx);

        self.decode_variation(buf, flags, 0)?;

        if flags & GAME_DECODE_COMMENTS != 0 {
            self.decode_comments(buf)?;
        }

        Ok(())
    }

    fn skip_tags(&mut self, buf: &mut ByteBuffer) -> Result<()> {
        let mut b = buf.get_byte()?;

        while b != 0 {
            if b == 255 {
                buf.skip(3)?;
            } else if b > MAX_TAG_LEN {
                let val_len = buf.get_byte()?;
                buf.skip(val_len as usize)?;
            } else {
                buf.skip(b as usize)?;
                let val_len = buf.get_byte()?;
                buf.skip(val_len as usize)?;
            }
            b = buf.get_byte()?;
        }

        Ok(())
    }

    fn add_marker_node(&mut self, marker: Marker, var_parent: Option<usize>) -> usize {
        let idx = self.moves.len();
        let mut node = MoveNode::new();
        node.marker = marker;
        node.var_parent = var_parent;
        self.moves.push(node);
        idx
    }

    fn decode_variation(&mut self, buf: &mut ByteBuffer, flags: u8, level: u32) -> Result<()> {
        let mut b = buf.get_byte()?;

        while b != ENCODE_END_GAME && b != ENCODE_END_MARKER {
            match b {
                ENCODE_START_MARKER => {
                    self.add_variation()?;
                    self.decode_variation(buf, flags, level + 1)?;
                    self.exit_variation()?;
                    self.move_forward()?;
                }
                ENCODE_NAG => {
                    let nag = buf.get_byte()?;
                    self.add_nag(nag);
                }
                ENCODE_COMMENT => {
                    if flags & GAME_DECODE_COMMENTS != 0 {
                        let current_idx = self
                            .current_move
                            .ok_or(Error::Decode("No current move".to_string()))?;
                        if let Some(prev_idx) = self.moves[current_idx].prev {
                            self.moves[prev_idx].comment = Some(String::new());
                        }
                    }
                }
                _ => {
                    let mut sm = SimpleMove::new();
                    self.decode_move(buf, &mut sm, b)?;
                    self.add_move_from_simple(&sm)?;
                }
            }

            b = buf.get_byte()?;
        }

        if level == 0 && b != ENCODE_END_GAME {
            return Err(Error::Decode("Expected ENCODE_END_GAME".to_string()));
        }
        if level > 0 && b != ENCODE_END_MARKER {
            return Err(Error::Decode("Expected ENCODE_END_MARKER".to_string()));
        }

        Ok(())
    }

    fn decode_move(&mut self, buf: &mut ByteBuffer, sm: &mut SimpleMove, b: u8) -> Result<()> {
        let (piece_num, move_val) = parse_move_byte(b);

        sm.piece_num = piece_num;
        sm.from = self
            .current_board
            .piece_list
            .get_square(self.current_board.to_move, piece_num);

        if !sm.from.is_valid() {
            return Err(Error::Decode(format!(
                "Invalid piece number: {}",
                piece_num
            )));
        }

        let board_piece = self.current_board.get_piece(sm.from);
        let piece = match board_piece {
            Some((p, _)) => p,
            None => return Err(Error::Decode("No piece on square".to_string())),
        };

        sm.promote = Piece::Empty;
        sm.captured_piece = Piece::Empty;
        sm.captured_square = Square::Null;

        match piece {
            Piece::King => decode_king(move_val, sm)?,
            Piece::Queen => decode_queen(buf, move_val, sm)?,
            Piece::Rook => decode_rook(move_val, sm)?,
            Piece::Bishop => decode_bishop(move_val, sm)?,
            Piece::Knight => decode_knight(move_val, sm)?,
            Piece::Pawn => decode_pawn(move_val, sm, self.current_board.to_move)?,
            Piece::Empty => return Err(Error::Decode("Empty piece".to_string())),
        }

        if sm.to != sm.from {
            if let Some((cap_piece, cap_color)) = self.current_board.get_piece(sm.to) {
                if cap_color != self.current_board.to_move {
                    sm.captured_piece = cap_piece;
                    sm.captured_square = sm.to;
                }
            }

            if piece == Piece::Pawn && sm.captured_piece == Piece::Empty {
                let ep_target = self.current_board.ep_target;
                if sm.to == ep_target.unwrap_or(Square::Null) {
                    sm.captured_piece = Piece::Pawn;
                    let ep_rank = sm.to.rank();
                    let ep_file = sm.to.file();
                    let captured_rank = if self.current_board.to_move == Color::White {
                        ep_rank - 1
                    } else {
                        ep_rank + 1
                    };
                    sm.captured_square = Square::make(ep_file, captured_rank);
                }
            }
        }

        Ok(())
    }

    fn add_move_from_simple(&mut self, sm: &SimpleMove) -> Result<()> {
        let san = self.generate_san(sm)?;

        let new_idx = self.moves.len();
        let mut node = MoveNode::new();
        node.move_data = sm.clone();
        node.san = san.clone();

        if let Some(current_idx) = self.current_move {
            node.prev = Some(current_idx);
            self.moves[current_idx].next = Some(new_idx);
        }

        self.moves.push(node);
        self.current_move = Some(new_idx);

        self.make_move_on_board(sm)?;
        self.num_half_moves += 1;

        Ok(())
    }

    pub fn make_move_on_board(&mut self, sm: &SimpleMove) -> Result<()> {
        if sm.from == sm.to {
            return Ok(());
        }

        let board = &mut self.current_board;
        let color = board.to_move;

        if let Some((cap_piece, cap_color)) = board.get_piece(sm.captured_square) {
            if cap_color != color {
                board.remove_from_board(sm.captured_square);
                board.piece_list.remove_piece(sm.captured_square, cap_color);
                board.material[crate::common::piece_make(cap_color, cap_piece) as usize] -= 1;
            }
        }

        if let Some((piece, _)) = board.get_piece(sm.from) {
            board.remove_from_board(sm.from);

            if sm.promote != Piece::Empty {
                board.material[crate::common::piece_make(color, piece) as usize] -= 1;
                board.material[crate::common::piece_make(color, sm.promote) as usize] += 1;
                board.squares[sm.to.to_index()] = Some((sm.promote, color));
            } else {
                board.add_to_board(piece, color, sm.to);
            }

            board.piece_list.move_piece(sm.from, sm.to, color);

            if piece == Piece::King {
                let diff = sm.to.to_index() as i32 - sm.from.to_index() as i32;
                if diff == 2 {
                    let rook_from = Square::make(7, sm.from.rank());
                    let rook_to = Square::make(5, sm.from.rank());
                    board.remove_from_board(rook_from);
                    board.add_to_board(Piece::Rook, color, rook_to);
                    board.piece_list.move_piece(rook_from, rook_to, color);
                } else if diff == -2 {
                    let rook_from = Square::make(0, sm.from.rank());
                    let rook_to = Square::make(3, sm.from.rank());
                    board.remove_from_board(rook_from);
                    board.add_to_board(Piece::Rook, color, rook_to);
                    board.piece_list.move_piece(rook_from, rook_to, color);
                }
            }

            if piece == Piece::Pawn
                && sm.captured_piece == Piece::Empty
                && sm.captured_square != sm.to
            {
                let ep_rank = sm.to.rank();
                let from_rank = sm.from.rank();
                if (ep_rank as i8 - from_rank as i8).abs() == 2 {
                    let ep_rank = if color == Color::White {
                        ep_rank - 1
                    } else {
                        ep_rank + 1
                    };
                    board.ep_target = Some(Square::make(sm.to.file(), ep_rank));
                } else {
                    board.ep_target = None;
                }
            } else {
                board.ep_target = None;
            }
        }

        board.to_move = board.to_move.flip();
        if color == Color::White {
            board.fullmove_number += 1;
        }

        Ok(())
    }

    fn generate_san(&self, sm: &SimpleMove) -> Result<String> {
        let piece_opt = self.current_board.get_piece(sm.from);
        let piece = match piece_opt {
            Some((p, _)) => p,
            None => return Err(Error::Decode("No piece".to_string())),
        };

        let mut san = String::new();

        if piece == Piece::King {
            let diff = sm.to.to_index() as i32 - sm.from.to_index() as i32;
            if diff == 2 {
                let mut result = "O-O".to_string();
                let mut test_board = self.current_board.clone();
                self.apply_move_to_board(&mut test_board, sm)?;
                if test_board.is_in_check(test_board.to_move) {
                    if self.is_checkmate(&test_board) {
                        result.push('#');
                    } else {
                        result.push('+');
                    }
                }
                return Ok(result);
            } else if diff == -2 {
                let mut result = "O-O-O".to_string();
                let mut test_board = self.current_board.clone();
                self.apply_move_to_board(&mut test_board, sm)?;
                if test_board.is_in_check(test_board.to_move) {
                    if self.is_checkmate(&test_board) {
                        result.push('#');
                    } else {
                        result.push('+');
                    }
                }
                return Ok(result);
            }
        }

        if piece != Piece::Pawn {
            san.push(piece.to_char());

            let mut ambiguous_file = false;
            let mut ambiguous_rank = false;
            let mut ambiguous_other = false;

            for i in
                0..self.current_board.piece_list.count[self.current_board.to_move as usize] as u8
            {
                let other_sq = self
                    .current_board
                    .piece_list
                    .get_square(self.current_board.to_move, i);
                if other_sq == sm.from || other_sq == Square::Null {
                    continue;
                }

                if let Some((other_piece, other_color)) = self.current_board.get_piece(other_sq) {
                    if other_color != self.current_board.to_move || other_piece != piece {
                        continue;
                    }

                    if self.can_piece_reach(other_piece, other_sq, sm.to)
                        && self.current_board.is_path_clear(other_sq, sm.to)
                    {
                        if other_sq.file() != sm.from.file() {
                            ambiguous_file = true;
                        } else if other_sq.rank() != sm.from.rank() {
                            ambiguous_rank = true;
                        } else {
                            ambiguous_other = true;
                        }
                    }
                }
            }

            if ambiguous_other || (ambiguous_file && ambiguous_rank) {
                san.push(sm.from.file_char());
                san.push(sm.from.rank_char());
            } else if ambiguous_file {
                san.push(sm.from.file_char());
            } else if ambiguous_rank {
                san.push(sm.from.rank_char());
            }
        }

        let is_capture = sm.captured_piece != Piece::Empty
            || (piece == Piece::Pawn && sm.from.file() != sm.to.file());

        if is_capture {
            if piece == Piece::Pawn {
                san.push(sm.from.file_char());
            }
            san.push('x');
        }

        if piece == Piece::Pawn && sm.promote != Piece::Empty {
            san.push_str(&sm.to.to_string());
            san.push('=');
            san.push(sm.promote.to_char());
        } else {
            san.push_str(&sm.to.to_string());
        }

        let mut test_board = self.current_board.clone();
        self.apply_move_to_board(&mut test_board, sm)?;
        if test_board.is_in_check(test_board.to_move) {
            if self.is_checkmate(&test_board) {
                san.push('#');
            } else {
                san.push('+');
            }
        }

        Ok(san)
    }

    fn apply_move_to_board(&self, board: &mut Board, sm: &SimpleMove) -> Result<()> {
        if sm.from == sm.to {
            return Ok(());
        }

        let color = board.to_move;

        if let Some((cap_piece, cap_color)) = board.get_piece(sm.captured_square) {
            if cap_color != color {
                board.remove_from_board(sm.captured_square);
                board.piece_list.remove_piece(sm.captured_square, cap_color);
                board.material[crate::common::piece_make(cap_color, cap_piece) as usize] -= 1;
            }
        }

        if let Some((piece, _)) = board.get_piece(sm.from) {
            board.remove_from_board(sm.from);

            if sm.promote != Piece::Empty {
                board.material[crate::common::piece_make(color, piece) as usize] -= 1;
                board.material[crate::common::piece_make(color, sm.promote) as usize] += 1;
                board.squares[sm.to.to_index()] = Some((sm.promote, color));
            } else {
                board.add_to_board(piece, color, sm.to);
            }

            board.piece_list.move_piece(sm.from, sm.to, color);

            if piece == Piece::King {
                let diff = sm.to.to_index() as i32 - sm.from.to_index() as i32;
                if diff == 2 {
                    let rook_from = Square::make(7, sm.from.rank());
                    let rook_to = Square::make(5, sm.from.rank());
                    board.remove_from_board(rook_from);
                    board.add_to_board(Piece::Rook, color, rook_to);
                    board.piece_list.move_piece(rook_from, rook_to, color);
                } else if diff == -2 {
                    let rook_from = Square::make(0, sm.from.rank());
                    let rook_to = Square::make(3, sm.from.rank());
                    board.remove_from_board(rook_from);
                    board.add_to_board(Piece::Rook, color, rook_to);
                    board.piece_list.move_piece(rook_from, rook_to, color);
                }
            }
        }

        board.to_move = board.to_move.flip();

        Ok(())
    }

    fn is_checkmate(&self, board: &Board) -> bool {
        if !board.is_in_check(board.to_move) {
            return false;
        }

        let color = board.to_move;
        let king_sq = board.get_king_square(color);

        for king_file_delta in -1i8..=1 {
            for king_rank_delta in -1i8..=1 {
                if king_file_delta == 0 && king_rank_delta == 0 {
                    continue;
                }

                let new_file = king_sq.file() as i8 + king_file_delta;
                let new_rank = king_sq.rank() as i8 + king_rank_delta;

                if !(0..=7).contains(&new_file) || !(0..=7).contains(&new_rank) {
                    continue;
                }

                let to_sq = Square::make(new_file as u8, new_rank as u8);

                if let Some((_, sq_color)) = board.get_piece(to_sq) {
                    if sq_color == color {
                        continue;
                    }
                }

                let mut test_board = board.clone();
                test_board.remove_from_board(king_sq);
                test_board.add_to_board(Piece::King, color, to_sq);
                test_board.piece_list.move_piece(king_sq, to_sq, color);

                if !test_board.is_attacked(to_sq, color.flip()) {
                    return false;
                }
            }
        }

        true
    }

    fn can_piece_reach(&self, piece: Piece, from: Square, to: Square) -> bool {
        let rank_diff = (to.rank() as i8 - from.rank() as i8).abs();
        let file_diff = (to.file() as i8 - from.file() as i8).abs();

        match piece {
            Piece::Knight => rank_diff == 2 && file_diff == 1 || rank_diff == 1 && file_diff == 2,
            Piece::Bishop => rank_diff == file_diff && rank_diff > 0,
            Piece::Rook => rank_diff == 0 || file_diff == 0,
            Piece::Queen => rank_diff == file_diff || rank_diff == 0 || file_diff == 0,
            Piece::King => rank_diff <= 1 && file_diff <= 1,
            Piece::Pawn => false,
            Piece::Empty => false,
        }
    }

    pub fn add_variation(&mut self) -> Result<()> {
        let current_idx = self
            .current_move
            .ok_or(Error::Decode("No current move".to_string()))?;

        let var_parent = Some(current_idx);
        let start_idx = self.add_marker_node(Marker::StartMarker, var_parent);

        let var_child = self.moves[current_idx].var_child;
        self.moves[start_idx].var_parent = Some(current_idx);

        if let Some(mut last_var) = var_child {
            while let Some(next_var) = self.moves[last_var].var_child {
                last_var = next_var;
            }
            self.moves[last_var].var_child = Some(start_idx);
        } else {
            self.moves[current_idx].var_child = Some(start_idx);
        }

        self.moves[current_idx].num_variations += 1;
        self.current_move = Some(start_idx);
        self.var_depth += 1;

        Ok(())
    }

    fn exit_variation(&mut self) -> Result<()> {
        let current_idx = self
            .current_move
            .ok_or(Error::Decode("No current move".to_string()))?;

        let var_start = loop {
            let node = &self.moves[current_idx];
            if node.marker == Marker::StartMarker && node.var_parent.is_some() {
                break current_idx;
            }
            let prev = node
                .prev
                .ok_or(Error::Decode("No prev in variation".to_string()))?;
            self.current_move = Some(prev);
        };

        let var_parent = self.moves[var_start]
            .var_parent
            .ok_or(Error::Decode("No var parent".to_string()))?;
        self.current_move = Some(var_parent);
        self.var_depth -= 1;

        Ok(())
    }

    fn move_forward(&mut self) -> Result<()> {
        let current_idx = self
            .current_move
            .ok_or(Error::Decode("No current move".to_string()))?;
        let next = self.moves[current_idx]
            .next
            .ok_or(Error::Decode("No next move".to_string()))?;
        self.current_move = Some(next);
        Ok(())
    }

    fn add_nag(&mut self, nag: u8) {
        let current_idx = match self.current_move {
            Some(idx) => idx,
            None => return,
        };

        let prev_idx = match self.moves[current_idx].prev {
            Some(idx) => idx,
            None => return,
        };

        if self.moves[prev_idx].nags.len() < MAX_NAGS && nag != 0 {
            self.moves[prev_idx].nags.push(nag);
        }
    }

    fn decode_comments(&mut self, buf: &mut ByteBuffer) -> Result<()> {
        let start_idx = self
            .first_move
            .ok_or(Error::Decode("No first move".to_string()))?;
        self.decode_comments_recursive(buf, start_idx)
    }

    fn decode_comments_recursive(&mut self, buf: &mut ByteBuffer, start_idx: usize) -> Result<()> {
        let mut current_idx = start_idx;

        loop {
            if current_idx >= self.moves.len() {
                break;
            }

            let marker = self.moves[current_idx].marker;
            if marker == Marker::EndMarker || marker == Marker::EndGame {
                break;
            }

            if self.moves[current_idx].comment.is_some() {
                let comment = buf.get_terminated_string()?;
                self.moves[current_idx].comment = Some(comment);
            }

            if self.moves[current_idx].num_variations > 0 {
                let mut var_idx = self.moves[current_idx].var_child;
                while let Some(vi) = var_idx {
                    self.decode_comments_recursive(buf, vi)?;
                    var_idx = self.moves[vi].var_child;
                }
            }

            current_idx = match self.moves[current_idx].next {
                Some(idx) => idx,
                None => break,
            };
        }

        Ok(())
    }

    // One parameter per Seven Tag Roster tag; grouping them would change the
    // public API.
    #[allow(clippy::too_many_arguments)]
    pub fn load_standard_tags(
        &mut self,
        white: &str,
        black: &str,
        event: &str,
        site: &str,
        round: &str,
        date: Date,
        result: GameResult,
    ) {
        self.white = white.to_string();
        self.black = black.to_string();
        self.event = event.to_string();
        self.site = site.to_string();
        self.round = round.to_string();
        self.date = date;
        self.result = result;
    }

    pub fn to_pgn(&self) -> String {
        let mut pgn = String::new();

        pgn.push_str(&format!("[Event \"{}\"]\n", self.event));
        pgn.push_str(&format!("[Site \"{}\"]\n", self.site));
        pgn.push_str(&format!("[Date \"{}\"]\n", self.date));
        pgn.push_str(&format!("[Round \"{}\"]\n", self.round));
        pgn.push_str(&format!("[White \"{}\"]\n", self.white));
        pgn.push_str(&format!("[Black \"{}\"]\n", self.black));
        pgn.push_str(&format!("[Result \"{}\"]\n", self.result));

        if let Some(ref title) = self.find_tag("WhiteTitle") {
            pgn.push_str(&format!("[WhiteTitle \"{}\"]\n", title));
        }
        if let Some(ref title) = self.find_tag("BlackTitle") {
            pgn.push_str(&format!("[BlackTitle \"{}\"]\n", title));
        }

        if let Some(elo) = self.white_elo {
            pgn.push_str(&format!("[WhiteElo \"{}\"]\n", elo));
        }
        if let Some(elo) = self.black_elo {
            pgn.push_str(&format!("[BlackElo \"{}\"]\n", elo));
        }

        if let Some(ref eco) = self.eco {
            pgn.push_str(&format!("[ECO \"{}\"]\n", eco));
        }

        if let Some(ref opening) = self.find_tag("Opening") {
            pgn.push_str(&format!("[Opening \"{}\"]\n", opening));
        }
        if let Some(ref variation) = self.find_tag("Variation") {
            pgn.push_str(&format!("[Variation \"{}\"]\n", variation));
        }

        if let Some(event_date) = self.event_date {
            pgn.push_str(&format!("[EventDate \"{}\"]\n", event_date));
        }

        let excluded_tags = [
            "WhiteTitle",
            "BlackTitle",
            "WhiteElo",
            "BlackElo",
            "ECO",
            "Opening",
            "Variation",
            "EventDate",
        ];

        for tag in &self.tags {
            if !excluded_tags.contains(&tag.name.as_str()) {
                pgn.push_str(&format!("[{} \"{}\"]\n", tag.name, tag.value));
            }
        }

        pgn.push('\n');

        if let Some(start_idx) = self.first_move {
            let mut lines: Vec<String> = Vec::new();
            let mut current_line = String::new();
            let mut move_num = 1;
            let mut is_white_move = true;

            let mut current_idx = start_idx;
            while let Some(next_idx) = self.moves[current_idx].next {
                current_idx = next_idx;

                if self.moves[current_idx].marker == Marker::EndMarker
                    || self.moves[current_idx].marker == Marker::EndGame
                {
                    break;
                }

                let mut token = String::new();

                if is_white_move {
                    token.push_str(&format!("{}. ", move_num));
                }

                token.push_str(&self.moves[current_idx].san);

                if !self.moves[current_idx].nags.is_empty() {
                    for &nag in &self.moves[current_idx].nags {
                        token.push(' ');
                        token.push_str(nag_to_symbol(nag));
                    }
                }

                if let Some(ref comment) = self.moves[current_idx].comment {
                    token.push(' ');
                    token.push_str(&format!("{{{}}}", comment));
                }

                if current_line.len() + token.len() + 1 > 80 && !current_line.is_empty() {
                    lines.push(current_line.trim().to_string());
                    current_line = String::new();
                }

                if !current_line.is_empty() {
                    current_line.push(' ');
                }
                current_line.push_str(&token);

                is_white_move = !is_white_move;
                if is_white_move {
                    move_num += 1;
                }
            }

            if !current_line.is_empty() {
                lines.push(current_line.trim().to_string());
            }

            for line in &lines {
                pgn.push_str(line);
                pgn.push('\n');
            }
        }

        pgn.push_str(&self.result.to_string());
        pgn.push('\n');

        pgn
    }
}

pub fn nag_to_symbol(nag: u8) -> &'static str {
    match nag {
        1 => "!",
        2 => "?",
        3 => "!!",
        4 => "??",
        5 => "!?",
        6 => "?!",
        10 => "=",
        13 => "∞",
        14 => "⩲",
        15 => "⩱",
        16 => "±",
        17 => "∓",
        18 => "+−",
        19 => "−+",
        22 => "⨀",
        26 => "⟳",
        36 => "↑",
        40 => "⇗",
        44 => "↮",
        132 => "⇆",
        136 => "⨁",
        140 => "∆",
        142 => "□",
        145 => "RR",
        146 => "N",
        201 => "D",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_new() {
        let game = Game::new();
        assert_eq!(game.white, "?");
        assert_eq!(game.black, "?");
        assert_eq!(game.result, GameResult::None);
    }

    #[test]
    fn test_game_clear() {
        let mut game = Game::new();
        game.white = "Test".to_string();
        game.clear();
        assert_eq!(game.white, "?");
    }

    #[test]
    fn test_add_tag() {
        let mut game = Game::new();
        game.add_tag("Annotator", "Test");
        assert_eq!(game.tags.len(), 1);
        assert_eq!(game.find_tag("Annotator"), Some(&"Test".to_string()));
    }

    #[test]
    fn test_nag_to_symbol() {
        assert_eq!(nag_to_symbol(1), "!");
        assert_eq!(nag_to_symbol(2), "?");
        assert_eq!(nag_to_symbol(3), "!!");
        assert_eq!(nag_to_symbol(4), "??");
    }
}
