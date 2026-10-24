use shakmaty::{Chess, Color, Position};

use crate::bytebuf::ByteBuffer;
use crate::common::GameResult;
use crate::error::{Error, Result};
use crate::mov::{
    decode_move, GameMove, ENCODE_COMMENT, ENCODE_END_GAME, ENCODE_END_MARKER, ENCODE_NAG,
    ENCODE_START_MARKER,
};
use crate::pgn::PgnWarning;
use crate::position::{chess_from_fen, fen, ScidPosition};
use crate::Date;

pub const MAX_TAGS: usize = 40;
pub const MAX_TAG_LEN: u8 = 240;
pub const MAX_NAGS: usize = 8;
/// The deepest a variation can be nested, counting a variation of the main
/// line as 1. Reading and writing a game recurse once per level, so deeper
/// nesting could overflow the stack.
pub(crate) const MAX_VARIATION_DEPTH: u32 = 100;

/// SCID counts a move's variations in a byte, so it can't read back a move
/// with more than 255.
const TOO_MANY_VARIATIONS: &str = "More than 255 variations of one move";

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
    /// The move, or `None` for a marker node.
    pub move_data: Option<GameMove>,
    /// The move in SAN, generated from the position before it.
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
            move_data: None,
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
        self.move_data == Some(GameMove::Null)
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
    /// The position the game starts from, or `None` for the standard
    /// starting position.
    pub start_position: Option<Chess>,
    /// The number of half-moves in the main line.
    pub num_half_moves: u16,
    /// Problems found while reading the game from PGN that didn't stop it
    /// from being read. Empty for a game read from a SCID database.
    pub warnings: Vec<PgnWarning>,
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
            start_position: None,
            num_half_moves: 0,
            warnings: Vec::new(),
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
        self.start_position = None;
        self.num_half_moves = 0;
        self.warnings.clear();
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
        self.decode_start_position(buf)
    }

    /// Reads the game flags and, if the game doesn't start from the standard
    /// position, its starting FEN.
    fn decode_start_position(&mut self, buf: &mut ByteBuffer) -> Result<()> {
        let flags = buf.get_byte()?;
        self.start_position = if flags & 1 != 0 {
            Some(chess_from_fen(&buf.get_terminated_string()?)?)
        } else {
            None
        };
        Ok(())
    }

    /// The starting position with SCID's piece numbering.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidFen`] if `start_position` has more than 16
    /// pieces of one color.
    pub fn initial_position(&self) -> Result<ScidPosition> {
        match &self.start_position {
            Some(chess) => ScidPosition::from_setup_position(chess.clone()),
            None => Ok(ScidPosition::standard()),
        }
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

        self.decode_start_position(buf)?;

        let start_node_idx = self.add_marker_node(Marker::StartMarker, None);
        self.first_move = Some(start_node_idx);
        self.current_move = Some(start_node_idx);

        let position = self.initial_position()?;
        self.decode_variation(buf, flags, position, 0)?;

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

    /// Decodes a line of moves played from `position`, with its
    /// variations, up to the marker that ends it.
    fn decode_variation(
        &mut self,
        buf: &mut ByteBuffer,
        flags: u8,
        mut position: ScidPosition,
        level: u32,
    ) -> Result<()> {
        // The position before the last move, where its variations start.
        let mut before_last: Option<ScidPosition> = None;
        let mut b = buf.get_byte()?;

        while b != ENCODE_END_GAME && b != ENCODE_END_MARKER {
            match b {
                ENCODE_START_MARKER => {
                    let start = before_last
                        .clone()
                        .ok_or_else(|| Error::Decode("Variation before any move".to_string()))?;
                    if let Some(message) = self.variation_limit(level + 1) {
                        return Err(Error::Decode(message));
                    }
                    let parent = self.current_move;
                    self.add_variation()?;
                    self.decode_variation(buf, flags, start, level + 1)?;
                    self.current_move = parent;
                }
                ENCODE_NAG => {
                    let nag = buf.get_byte()?;
                    self.add_nag(nag);
                }
                // The comment's text follows the moves; mark the move it
                // belongs to.
                ENCODE_COMMENT => {
                    if flags & GAME_DECODE_COMMENTS != 0 {
                        let current_idx = self
                            .current_move
                            .ok_or(Error::Decode("No current move".to_string()))?;
                        self.moves[current_idx].comment = Some(String::new());
                    }
                }
                _ => {
                    let mv = decode_move(buf, &position, b)?;
                    before_last = Some(position.clone());
                    let san = position.play(mv)?;
                    self.append_move(mv, san.to_string(), level == 0);
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

    /// Adds `mv` after the current move and makes it the current move.
    pub(crate) fn append_move(&mut self, mv: GameMove, san: String, main_line: bool) {
        let new_idx = self.moves.len();
        let mut node = MoveNode::new();
        node.move_data = Some(mv);
        node.san = san;

        if let Some(current_idx) = self.current_move {
            node.prev = Some(current_idx);
            self.moves[current_idx].next = Some(new_idx);
        }

        self.moves.push(node);
        self.current_move = Some(new_idx);
        if main_line {
            self.num_half_moves += 1;
        }
    }

    /// Adds a variation to the current move and makes its start marker the
    /// current move.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Decode`] if there is no current move, or if it
    /// already has 255 variations, the most SCID can read back.
    pub fn add_variation(&mut self) -> Result<()> {
        let current_idx = self
            .current_move
            .ok_or(Error::Decode("No current move".to_string()))?;
        if self.moves[current_idx].num_variations == u8::MAX {
            return Err(Error::Decode(TOO_MANY_VARIATIONS.to_string()));
        }

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

        Ok(())
    }

    /// Why a variation at nesting `depth` can't be added to the current move,
    /// or `None` if it can.
    pub(crate) fn variation_limit(&self, depth: u32) -> Option<String> {
        if depth > MAX_VARIATION_DEPTH {
            return Some(format!(
                "Variation nested more than {MAX_VARIATION_DEPTH} levels deep"
            ));
        }
        let current = self.current_move.map(|idx| &self.moves[idx]);
        if current.is_some_and(|node| node.num_variations == u8::MAX) {
            return Some(TOO_MANY_VARIATIONS.to_string());
        }
        None
    }

    /// Adds `nag` to the current move, the move it follows.
    ///
    /// Like SCID, a NAG before the first move of the game or of a variation
    /// is dropped, as is NAG 0, which means no annotation.
    pub(crate) fn add_nag(&mut self, nag: u8) {
        let Some(node) = self.current_move.map(|idx| &mut self.moves[idx]) else {
            return;
        };
        if node.marker == Marker::NoMarker && nag != 0 && node.nags.len() < MAX_NAGS {
            node.nags.push(nag);
        }
    }

    /// Adds `comment` to the current move, the move it follows, or to the
    /// start of the game or variation before its first move.
    ///
    /// Like SCID, a second comment is appended to the first, with a space
    /// between them unless the first is empty or ends in whitespace.
    pub(crate) fn append_comment(&mut self, comment: &str) {
        let Some(idx) = self.current_move else {
            return;
        };
        match self.moves[idx].comment {
            Some(ref mut existing) => {
                if !existing.is_empty() && !existing.ends_with(|c: char| c.is_ascii_whitespace()) {
                    existing.push(' ');
                }
                existing.push_str(comment);
            }
            None => self.moves[idx].comment = Some(comment.to_string()),
        }
    }

    /// The start markers of the move at `idx`'s variations, in order. A
    /// start marker's `var_child` links to the next variation of the same
    /// move.
    pub(crate) fn variations(&self, idx: usize) -> impl Iterator<Item = usize> + '_ {
        let node = &self.moves[idx];
        let first = if node.marker == Marker::StartMarker {
            None
        } else {
            node.var_child
        };
        let valid = move |idx: &usize| *idx < self.moves.len();
        std::iter::successors(first.filter(valid), move |&start| {
            self.moves[start].var_child.filter(valid)
        })
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
            "SetUp",
            "FEN",
        ];

        for tag in &self.tags {
            if !excluded_tags.contains(&tag.name.as_str()) {
                pgn.push_str(&format!("[{} \"{}\"]\n", tag.name, tag.value));
            }
        }

        if let Some(ref start) = self.start_position {
            pgn.push_str("[SetUp \"1\"]\n");
            pgn.push_str(&format!("[FEN \"{}\"]\n", fen(start)));
        }

        pgn.push('\n');

        if let Some(start_idx) = self.first_move {
            // A FEN's move number can be u32::MAX, so count plies in u64.
            let ply = match self.start_position {
                Some(ref start) => {
                    2 * (u64::from(start.fullmoves().get()) - 1)
                        + u64::from(start.turn() == Color::Black)
                }
                None => 0,
            };
            let mut tokens = Vec::new();
            self.line_tokens(start_idx, ply, &mut tokens);

            let mut current_line = String::new();
            for token in tokens {
                if current_line.len() + token.len() + 1 > 80 && !current_line.is_empty() {
                    pgn.push_str(&current_line);
                    pgn.push('\n');
                    current_line.clear();
                }
                if !current_line.is_empty() {
                    current_line.push(' ');
                }
                current_line.push_str(&token);
            }
            if !current_line.is_empty() {
                pgn.push_str(&current_line);
                pgn.push('\n');
            }
        }

        pgn.push_str(&self.result.to_string());
        pgn.push('\n');

        pgn
    }

    /// Appends the movetext of the line that starts at the start marker
    /// `start_idx` to `tokens`, with its comments, NAGs, and variations.
    /// `ply` is the half-move number of its first move, counting from 0.
    ///
    /// Each token is a move with its number, NAGs, and comment, so they stay
    /// on one line. A Black move is numbered when it starts a line or
    /// follows a comment or a variation, as the PGN export format requires.
    fn line_tokens(&self, start_idx: usize, mut ply: u64, tokens: &mut Vec<String>) {
        if let Some(ref comment) = self.moves[start_idx].comment {
            tokens.push(format!("{{{comment}}}"));
        }

        let mut number_black_move = true;
        let mut idx = self.moves[start_idx].next;
        while let Some(i) = idx {
            let node = &self.moves[i];
            if matches!(node.marker, Marker::EndMarker | Marker::EndGame) {
                break;
            }

            let number = ply / 2 + 1;
            let mut token = if ply.is_multiple_of(2) {
                format!("{number}. ")
            } else if number_black_move {
                format!("{number}... ")
            } else {
                String::new()
            };
            token.push_str(&node.san);

            // NAG 0 means no annotation; SCID writes nothing for it.
            for &nag in node.nags.iter().filter(|&&nag| nag != 0) {
                token.push(' ');
                match nag_to_symbol(nag) {
                    "" => token.push_str(&format!("${nag}")),
                    symbol => token.push_str(symbol),
                }
            }

            if let Some(ref comment) = node.comment {
                token.push_str(&format!(" {{{comment}}}"));
            }
            number_black_move = node.comment.is_some();
            tokens.push(token);

            for var_start in self.variations(i) {
                let first = tokens.len();
                self.line_tokens(var_start, ply, tokens);
                if first == tokens.len() {
                    tokens.push("()".to_string());
                } else {
                    tokens[first].insert(0, '(');
                    tokens
                        .last_mut()
                        .expect("the variation added a token")
                        .push(')');
                }
                number_black_move = true;
            }

            ply += 1;
            idx = node.next;
        }
    }
}

/// The symbol PGN export writes for `nag`, or `""` if it is written as
/// `$n`.
///
/// These are the symbols SCID's converter writes (`evalNagsRegular` in
/// `scid-cpp/scid/nagtext.h`) that both SCID and this crate read back as
/// the same NAG. SCID writes other NAGs as symbols it can't read back, such
/// as `→` for $40, so this crate writes them as `$n`.
pub fn nag_to_symbol(nag: u8) -> &'static str {
    match nag {
        1 => "!",
        2 => "?",
        3 => "!!",
        4 => "??",
        5 => "!?",
        6 => "?!",
        10 => "=",
        13 => "~",
        14 => "+=",
        15 => "=+",
        16 => "+/-",
        17 => "-/+",
        18 => "+-",
        19 => "-+",
        20 => "+--",
        21 => "--+",
        44 => "=/&",
        144 => "R",
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
    fn test_every_nag_survives_export_and_import() {
        for nag in 1..=u8::MAX {
            let mut game =
                crate::pgn::parse_single_game("[Event \"x\"]\n\n1. e4 e5 2. Nf3 *\n").unwrap();
            // The node after the start marker and e4 is e5.
            game.moves[2].nags = vec![nag];
            let pgn = game.to_pgn();
            assert!(pgn.is_ascii(), "${nag}: {pgn}");
            let reread = crate::pgn::parse_single_game(&pgn).unwrap();
            assert!(reread.warnings.is_empty(), "${nag}: {pgn}");
            assert_eq!(reread.moves[2].nags, [nag], "${nag}: {pgn}");
            let nags = reread
                .moves
                .iter()
                .map(|node| node.nags.len())
                .sum::<usize>();
            assert_eq!(nags, 1, "${nag}: {pgn}");
        }
    }

    #[test]
    fn test_nags_without_a_readable_symbol_are_written_as_numbers() {
        let mut game =
            crate::pgn::parse_single_game("[Event \"x\"]\n\n1. e4 e5 2. Nf3 *\n").unwrap();
        game.moves[2].nags = vec![14, 20, 40, 41, 45, 145, 0];
        let pgn = game.to_pgn();
        assert!(
            pgn.contains("1. e4 e5 += +-- $40 $41 $45 $145 2. Nf3"),
            "{pgn}"
        );
    }

    #[test]
    fn test_nag_to_symbol() {
        assert_eq!(nag_to_symbol(1), "!");
        assert_eq!(nag_to_symbol(2), "?");
        assert_eq!(nag_to_symbol(3), "!!");
        assert_eq!(nag_to_symbol(4), "??");
    }
}
