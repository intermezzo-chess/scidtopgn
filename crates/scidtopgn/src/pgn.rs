use std::fmt;

use shakmaty::san::{San, SanError};
use shakmaty::uci::UciMove;
use shakmaty::{Bitboard, CastlingSide, Chess, File, Move, Position, Rank, Role, Square};

use crate::common::GameResult;
use crate::error::{Error, Result};
use crate::game::{Game, Marker, MoveNode, MAX_NAGS};
use crate::mov::GameMove;
use crate::position::{chess_from_fen_reporting_drops, ScidPosition};
use crate::Date;

/// A problem in a PGN game that didn't stop the game from being read.
///
/// `docs/PGN_IMPORT.md` lists the warnings and compares them with SCID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgnWarning {
    /// The game's number in the input, counting from 1.
    pub game: usize,
    /// The line of the input the warning is about.
    pub line: u32,
    pub kind: PgnWarningKind,
}

impl fmt::Display for PgnWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PGN game {}, line {}: {}",
            self.game, self.line, self.kind
        )
    }
}

/// What a [`PgnWarning`] is about.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PgnWarningKind {
    /// Text in the moves that isn't a move, an annotation, or a result, such
    /// as `zz` or `nf3`. It is skipped.
    UnknownText { text: String },
    /// Castling after the king or rook has moved and returned. SCID plays
    /// it, so this crate does too.
    CastlingWithoutRight { text: String },
    /// `O-O` played as `O-O-O` because only queenside castling is legal, as
    /// SCID does.
    KingsideReadAsQueenside { text: String },
    /// Castling rights in the `FEN` tag that don't fit the position. They
    /// are dropped.
    FenCastlingRights { rights: String },
    /// An en passant square in the `FEN` tag that no pawn could have just
    /// moved past. It is dropped.
    FenEnPassant { square: Square },
    /// The result at the end of the moves differs from the `Result` tag.
    /// The result at the end of the moves is used.
    ResultMismatch { tag: GameResult, moves: GameResult },
    /// The moves don't end with a result, so the `Result` tag, if any, is
    /// used.
    MissingResult { tag: Option<GameResult> },
}

impl fmt::Display for PgnWarningKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownText { text } => write!(f, "skipped unknown text {text:?}"),
            Self::CastlingWithoutRight { text } => {
                write!(f, "{text:?} castles without the castling right")
            }
            Self::KingsideReadAsQueenside { text } => write!(
                f,
                "read {text:?} as O-O-O because only queenside castling is legal"
            ),
            Self::FenCastlingRights { rights } => write!(
                f,
                "dropped castling rights {rights:?} from the FEN because they don't fit the position"
            ),
            Self::FenEnPassant { square } => write!(
                f,
                "dropped en passant square {square} from the FEN because no pawn could have just moved past it"
            ),
            Self::ResultMismatch { tag, moves } => write!(
                f,
                "the result {moves} after the moves replaces the Result tag {tag}"
            ),
            Self::MissingResult { tag: Some(tag) } => write!(
                f,
                "the moves don't end with a result, so the Result tag {tag} is used"
            ),
            Self::MissingResult { tag: None } => write!(
                f,
                "the moves don't end with a result, and there is no Result tag"
            ),
        }
    }
}

/// How a line of moves ended.
enum LineEnd {
    /// The game's result, on `line`.
    Result { result: GameResult, line: u32 },
    /// The end of the input, or the next game's tags, after `line`.
    Missing { line: u32 },
    /// The `)` that ends a variation.
    Variation,
}

/// A token in the moves, after any move number.
#[derive(Debug, PartialEq)]
enum MoveToken {
    /// A move, with the annotation symbols written directly after it.
    Move {
        text: String,
        suffix: String,
    },
    /// Annotation symbols on their own, as NAGs.
    Nags(Vec<u8>),
    Result(GameResult),
    /// A move number, dots, a check sign, or an en passant marker.
    Ignored,
    /// Anything else. It is skipped with a warning.
    Unknown(String),
}

type MoveError = Box<dyn std::error::Error + Send + Sync>;

/// A move read from PGN, and the warning reading it raised, if any.
#[derive(Debug)]
struct ParsedMove {
    mv: GameMove,
    warning: Option<PgnWarningKind>,
}

impl From<GameMove> for ParsedMove {
    fn from(mv: GameMove) -> Self {
        ParsedMove { mv, warning: None }
    }
}

impl From<Move> for ParsedMove {
    fn from(m: Move) -> Self {
        GameMove::Move(m).into()
    }
}

/// Reads games from PGN text.
///
/// As an iterator, it yields each game or the error that stopped it. After an
/// error it resumes at the next line that starts with `[`, the next game's
/// tags.
pub struct PgnParser {
    input: Vec<char>,
    pos: usize,
    line: u32,
    games: usize,
    in_movetext: bool,
}

impl PgnParser {
    pub fn new(input: &str) -> Self {
        PgnParser {
            input: input.chars().collect(),
            pos: 0,
            line: 1,
            games: 0,
            in_movetext: false,
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

    fn at_line_start(&self) -> bool {
        self.pos == 0 || self.input[self.pos - 1] == '\n'
    }

    /// Whether the next character starts a tag line: a `[` with only spaces
    /// or tabs before it on its line.
    fn at_tag_line(&self) -> bool {
        self.peek() == Some('[')
            && self.input[..self.pos]
                .iter()
                .rev()
                .find(|&&c| c != ' ' && c != '\t')
                .is_none_or(|&c| c == '\n')
    }

    fn error(&self, message: &str) -> Error {
        Error::Parse {
            line: self.line,
            message: message.to_string(),
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

    /// Skips spaces and tabs, but not line breaks.
    fn skip_line_whitespace(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.advance();
        }
    }

    /// Moves back to `pos`, on line `line`.
    fn rewind(&mut self, pos: usize, line: u32) {
        self.pos = pos;
        self.line = line;
    }

    fn skip_to_end_of_line(&mut self) {
        while let Some(c) = self.peek() {
            self.advance();
            if c == '\n' {
                break;
            }
        }
    }

    /// Skips whitespace, a byte order mark, and line comments before a game.
    fn skip_between_games(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() || c == '\u{feff}' {
                self.advance();
            } else if c == '%' || c == ';' {
                self.skip_to_end_of_line();
            } else {
                break;
            }
        }
    }

    /// Skips the rest of a game that failed to parse, up to the next tag
    /// line outside a comment. If the game failed in its tags, its remaining
    /// tag lines are skipped first; a failed tag leaves the parser just after
    /// its `[`.
    fn skip_to_next_game(&mut self) {
        if !self.in_movetext {
            if !self.at_line_start() {
                self.skip_to_end_of_line();
            }
            loop {
                self.skip_line_whitespace();
                if self.peek() != Some('[') {
                    break;
                }
                self.skip_to_end_of_line();
            }
        }
        // Comments are read as the movetext parser reads them, so a brace in
        // a line comment doesn't hide the following games.
        while let Some(c) = self.peek() {
            if self.at_tag_line() {
                return;
            }
            match c {
                '{' => self.skip_comment(),
                ';' | '%' => self.skip_to_end_of_line(),
                _ => {
                    self.advance();
                }
            }
        }
    }

    /// Reads a tag value, which must end on its line.
    fn read_string(&mut self) -> Result<String> {
        self.skip_whitespace();
        if self.peek() != Some('"') {
            return Err(self.error("Expected '\"'"));
        }
        self.advance();

        let mut result = String::new();
        while let Some(c) = self.peek() {
            if c == '\n' {
                break;
            }
            self.advance();
            match c {
                '"' => return Ok(result),
                '\\' => match self.peek() {
                    Some(escaped) if escaped != '\n' => {
                        result.push(escaped);
                        self.advance();
                    }
                    _ => break,
                },
                _ => result.push(c),
            }
        }
        Err(self.error("Unterminated string"))
    }

    /// Parses the next game.
    ///
    /// Problems that don't stop the game from being read are recorded in
    /// [`Game::warnings`].
    ///
    /// # Errors
    ///
    /// Returns an error if a tag, the `FEN`, or a move is invalid, or if the
    /// `Variant` tag names a variant other than standard chess.
    pub fn parse_game(&mut self) -> Result<Game> {
        self.games += 1;
        let mut game = Game::new();
        let mut fen = None;
        let mut variant = None;
        let mut result_tag = None;
        self.in_movetext = false;

        loop {
            self.skip_whitespace();

            match self.peek() {
                None => break,
                Some('[') => {
                    let (pos, line) = (self.pos, self.line);
                    let (name, value) = self.read_tag().inspect_err(|_| {
                        // A malformed tag may have read past its line, into
                        // the next game. Recovery starts from the tag.
                        self.rewind(pos, line);
                        self.advance();
                    })?;
                    if name == "FEN" {
                        fen = Some((value, line));
                    } else {
                        if name == "Variant" {
                            variant = Some(value.clone());
                        } else if name == "Result" {
                            result_tag = Some(parse_result(&value));
                        }
                        apply_tag(&mut game, &name, value);
                    }
                }
                Some('%') => {
                    self.skip_to_end_of_line();
                }
                Some('{') | Some(';') => {
                    self.skip_comment();
                }
                Some(c) if c.is_ascii_alphanumeric() || matches!(c, '$' | '*' | '(' | '-') => {
                    break;
                }
                Some(_) => {
                    self.advance();
                }
            }
        }

        // Checked here rather than in `game.tags`, which keeps at most
        // MAX_TAGS tags.
        if let Some(variant) = variant.filter(|v| !is_standard_chess(v)) {
            return Err(Error::UnsupportedVariant { variant });
        }
        // Like SCID, a FEN tag sets the start position whether or not the
        // game also has a SetUp tag.
        if let Some((fen, line)) = fen {
            let (chess, dropped) = chess_from_fen_reporting_drops(&fen)?;
            if dropped.castling_rights.any() {
                let rights = castling_rights_text(dropped.castling_rights);
                self.warn(
                    &mut game,
                    line,
                    PgnWarningKind::FenCastlingRights { rights },
                );
            }
            if let Some(square) = dropped.ep_square {
                self.warn(&mut game, line, PgnWarningKind::FenEnPassant { square });
            }
            game.start_position = Some(chess);
        }

        let start_idx = game.moves.len();
        let mut start = MoveNode::new();
        start.marker = Marker::StartMarker;
        game.moves.push(start);
        game.first_move = Some(start_idx);
        game.current_move = Some(start_idx);

        let position = game.initial_position()?;
        self.in_movetext = true;
        // Like SCID, the result after the moves wins over the Result tag.
        match self.parse_line(&mut game, position, 0)? {
            LineEnd::Result { result, line } => {
                if let Some(tag) = result_tag.filter(|&tag| tag != result) {
                    let kind = PgnWarningKind::ResultMismatch { tag, moves: result };
                    self.warn(&mut game, line, kind);
                }
                game.result = result;
            }
            LineEnd::Missing { line } => {
                let kind = PgnWarningKind::MissingResult { tag: result_tag };
                self.warn(&mut game, line, kind);
            }
            LineEnd::Variation => {}
        }

        Ok(game)
    }

    /// Records a warning about the current game.
    fn warn(&self, game: &mut Game, line: u32, kind: PgnWarningKind) {
        game.warnings.push(PgnWarning {
            game: self.games,
            line,
            kind,
        });
    }

    /// Reads a tag pair. Its tokens may be on separate lines.
    fn read_tag(&mut self) -> Result<(String, String)> {
        self.advance();

        self.skip_whitespace();
        let name = self.read_tag_name();

        let value = self.read_string()?;

        self.skip_whitespace();
        if self.peek() != Some(']') {
            return Err(self.error("Expected ']'"));
        }
        self.advance();

        Ok((name, value))
    }

    fn read_tag_name(&mut self) -> String {
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c.is_whitespace() || c == ']' {
                break;
            }
            name.push(c);
            self.advance();
        }
        name
    }

    /// Parses a line of moves played from `position`, with its variations,
    /// up to the `)` that ends a variation (`depth` > 0) or the game's
    /// result.
    fn parse_line(
        &mut self,
        game: &mut Game,
        mut position: ScidPosition,
        depth: u32,
    ) -> Result<LineEnd> {
        // The position before the last move, where its variations start.
        let mut before_last: Option<ScidPosition> = None;

        loop {
            let line_before = self.line;
            self.skip_whitespace();

            let Some(c) = self.peek() else {
                return if depth == 0 {
                    Ok(LineEnd::Missing { line: line_before })
                } else {
                    Err(self.error("Unterminated variation"))
                };
            };

            match c {
                ';' | '%' => {
                    self.skip_to_end_of_line();
                }
                '*' => {
                    self.advance();
                    if depth > 0 {
                        return Err(self.error("Unterminated variation"));
                    }
                    return Ok(LineEnd::Result {
                        result: GameResult::None,
                        line: self.line,
                    });
                }
                '{' => {
                    let comment = self.read_brace_comment()?;
                    if depth == 0 {
                        if let Some(current_idx) = game.current_move {
                            if let Some(prev_idx) = game.moves[current_idx].prev {
                                game.moves[prev_idx].comment = Some(comment);
                            }
                        }
                    }
                }
                '$' => {
                    self.advance();
                    let nag = self.read_nag()?;
                    add_nag(game, nag, depth);
                }
                '(' => {
                    self.advance();
                    let start = before_last
                        .clone()
                        .ok_or_else(|| self.error("Variation before any move"))?;
                    let parent = game.current_move;
                    game.add_variation()?;
                    self.parse_line(game, start, depth + 1)?;
                    game.current_move = parent;
                }
                ')' if depth == 0 => {
                    return Err(self.error("Unexpected ')'"));
                }
                ')' => {
                    self.advance();
                    return Ok(LineEnd::Variation);
                }
                // The next game's tags: this game has no result.
                '[' if depth == 0 && self.at_tag_line() => {
                    return Ok(LineEnd::Missing { line: line_before });
                }
                '[' => {
                    return Err(self.error("Unexpected '['"));
                }
                // A stray closing brace or bracket.
                '}' | ']' => {
                    self.advance();
                }
                _ => {
                    let line = self.line;
                    let token = self.read_token();

                    match classify_token(&token) {
                        MoveToken::Result(result) => {
                            if depth > 0 {
                                return Err(self.error("Unterminated variation"));
                            }
                            return Ok(LineEnd::Result { result, line });
                        }
                        MoveToken::Ignored => {}
                        MoveToken::Unknown(text) => {
                            self.warn(game, line, PgnWarningKind::UnknownText { text });
                        }
                        MoveToken::Nags(nags) => {
                            for nag in nags {
                                add_nag(game, nag, depth);
                            }
                        }
                        MoveToken::Move { text, suffix } => {
                            let invalid = |source: MoveError| Error::PgnMove {
                                line,
                                san: strip_move_number(&token).to_string(),
                                source,
                            };
                            let parsed = parse_move(&text, &position).map_err(invalid)?;
                            if let Some(kind) = parsed.warning {
                                self.warn(game, line, kind);
                            }
                            before_last = Some(position.clone());
                            let played =
                                position.play(parsed.mv).map_err(|e| invalid(Box::new(e)))?;
                            game.append_move(parsed.mv, played.to_string(), depth == 0);

                            let (nags, rest) = parse_symbols(&suffix);
                            for nag in nags {
                                add_nag(game, nag, depth);
                            }
                            if !rest.is_empty() {
                                let text = rest.to_string();
                                self.warn(game, line, PgnWarningKind::UnknownText { text });
                            }
                        }
                    }
                }
            }
        }
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
            return Err(self.error("Expected '{'"));
        }
        self.advance();

        let mut comment = String::new();
        while let Some(c) = self.advance() {
            if c == '}' {
                return Ok(comment);
            }
            comment.push(c);
        }

        Err(self.error("Unterminated comment"))
    }

    fn read_nag(&mut self) -> Result<u8> {
        let mut num = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                num.push(c);
                self.advance();
            } else {
                break;
            }
        }
        num.parse().map_err(|_| self.error("Invalid NAG"))
    }

    fn read_token(&mut self) -> String {
        let mut token = String::new();
        while let Some(c) = self.peek() {
            // A comment, NAG, or `*` may follow a move without a space.
            if c.is_whitespace() || matches!(c, '(' | ')' | '{' | '}' | '[' | ']' | ';' | '$' | '*')
            {
                break;
            }
            token.push(c);
            self.advance();
        }
        token
    }
}

impl Iterator for PgnParser {
    type Item = Result<Game>;

    /// Parses the next game. An error is wrapped in [`Error::PgnGame`], which
    /// gives the game's number and first line.
    fn next(&mut self) -> Option<Result<Game>> {
        self.skip_between_games();
        self.peek()?;

        let line = self.line;
        match self.parse_game() {
            Ok(game) => Some(Ok(game)),
            Err(source) => {
                self.skip_to_next_game();
                Some(Err(Error::PgnGame {
                    number: self.games,
                    line,
                    source: Box::new(source),
                }))
            }
        }
    }
}

fn apply_tag(game: &mut Game, name: &str, value: String) {
    match name {
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
        _ => game.add_tag(name, &value),
    }
}

/// Whether a `Variant` tag names standard chess. Lichess uses
/// "From Position" for standard chess from a FEN.
fn is_standard_chess(variant: &str) -> bool {
    matches!(
        variant.trim().to_ascii_lowercase().as_str(),
        "" | "standard" | "chess" | "normal" | "from position"
    )
}

/// Adds `nag` where a `$n` NAG goes, so an annotation symbol and its NAG
/// read the same. That is the move before the last one, and NAGs in
/// variations are dropped; #38 tracks both.
fn add_nag(game: &mut Game, nag: u8, depth: u32) {
    if depth > 0 {
        return;
    }
    if let Some(current_idx) = game.current_move {
        if let Some(prev_idx) = game.moves[current_idx].prev {
            if game.moves[prev_idx].nags.len() < MAX_NAGS {
                game.moves[prev_idx].nags.push(nag);
            }
        }
    }
}

/// The castling rights on `rooks`, the rooks' squares, in FEN notation.
fn castling_rights_text(rooks: Bitboard) -> String {
    let letter = |square: Square| match square {
        Square::H1 => 'K',
        Square::A1 => 'Q',
        Square::H8 => 'k',
        Square::A8 => 'q',
        _ if square.rank() == Rank::First => square.file().upper_char(),
        _ => square.file().char(),
    };
    let order = [Square::H1, Square::A1, Square::H8, Square::A8];
    order
        .into_iter()
        .filter(|&square| rooks.contains(square))
        .chain(rooks.into_iter().filter(|square| !order.contains(square)))
        .map(letter)
        .collect()
}

/// A result after the moves. SCID also accepts `:` for `-`, and `1/2` for a
/// draw.
fn parse_result_token(token: &str) -> Option<GameResult> {
    match token {
        "1-0" | "1:0" => Some(GameResult::White),
        "0-1" | "0:1" => Some(GameResult::Black),
        "1/2-1/2" | "1/2:1/2" | "1/2" | "½-½" => Some(GameResult::Draw),
        "*" => Some(GameResult::None),
        _ => None,
    }
}

/// Removes a move number such as `12.` or `12...` from the start of a token.
fn strip_move_number(token: &str) -> &str {
    let rest = token.trim_start_matches(|c: char| c.is_ascii_digit());
    if rest.len() < token.len() && rest.starts_with('.') {
        rest.trim_start_matches('.')
    } else {
        token
    }
}

/// SCID reads `--`, `Z0`, and `null` as a null move.
fn is_null_move(token: &str) -> bool {
    matches!(token, "--" | "Z0" | "null")
}

/// SCID ignores an en passant marker after a move, or on its own.
fn is_en_passant_marker(token: &str) -> bool {
    matches!(token, "ep" | "e.p." | "e.p")
}

/// Removes an en passant marker written directly after a move's
/// destination, as in `exd6ep`.
fn remove_en_passant_marker(token: &str) -> String {
    for marker in ["e.p.", "e.p", "ep"] {
        let found = token
            .match_indices(marker)
            .find(|&(i, _)| token[..i].ends_with(|c: char| c.is_ascii_digit()));
        if let Some((i, _)) = found {
            return format!("{}{}", &token[..i], &token[i + marker.len()..]);
        }
    }
    token.to_string()
}

/// Classifies a token in the moves.
fn classify_token(token: &str) -> MoveToken {
    let token = strip_move_number(token);
    if token.is_empty() || token.chars().all(|c| c == '.') || is_en_passant_marker(token) {
        return MoveToken::Ignored;
    }
    if let Some(result) = parse_result_token(token) {
        return MoveToken::Result(result);
    }
    // Before null moves, since `--+` is a symbol.
    if let Some(nags) = symbols_token(token) {
        return if nags.is_empty() {
            // Only check or mate signs.
            MoveToken::Ignored
        } else {
            MoveToken::Nags(nags)
        };
    }
    // A null move may have symbols attached, as in `Z0!`, but a word such as
    // `nullify` isn't a null move.
    if let Some(null) = ["--", "Z0", "null"].into_iter().find(|&n| {
        token
            .strip_prefix(n)
            .is_some_and(|rest| rest.is_empty() || starts_with_symbol(rest))
    }) {
        return MoveToken::Move {
            text: null.to_string(),
            suffix: token[null.len()..].to_string(),
        };
    }
    // A move number without a dot. `00` and `000` are castling.
    if token.bytes().all(|b| b.is_ascii_digit()) && !matches!(token, "00" | "000") {
        return MoveToken::Ignored;
    }
    // A single letter, such as a piece letter on its own, isn't a move.
    if token.len() < 2 || !token.starts_with(|c: char| "abcdefghKQRBNPOo0".contains(c)) {
        return MoveToken::Unknown(token.to_string());
    }
    let token = remove_en_passant_marker(token);
    let (text, suffix) = split_suffix(&token);
    MoveToken::Move {
        text: text.to_string(),
        suffix: suffix.to_string(),
    }
}

/// Whether `text` starts with an annotation symbol, a check sign, or a
/// mate sign.
fn starts_with_symbol(text: &str) -> bool {
    text.starts_with(['!', '?', '+', '#', '=', '-', '~'])
}

/// Splits a move from the annotation symbols written directly after it,
/// as in `e4!?`, `Nf3=`, `e4-+`, or `e4--+`.
fn split_suffix(token: &str) -> (&str, &str) {
    let bytes = token.as_bytes();
    let end = (0..bytes.len())
        .find(|&i| {
            let next = bytes.get(i + 1).copied();
            let in_move = match bytes[i] {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b':' => true,
                // `-` separates squares, as in `e2-e4` or `O-O`, and SCID
                // ignores one at the end of a move. Otherwise it starts a
                // symbol, as in `-+`, `-/+`, `--+`, or `->`.
                b'-' => next.is_none_or(|b| b.is_ascii_alphanumeric()),
                // `=` marks a promotion, as in `e8=Q`, but is also a symbol.
                b'=' => next.is_some_and(is_promotion_letter),
                _ => false,
            };
            !in_move
        })
        .unwrap_or(bytes.len());
    // `end` is at an ASCII byte or the end, so it is a char boundary.
    token.split_at(end)
}

fn is_promotion_letter(b: u8) -> bool {
    matches!(b, b'Q' | b'R' | b'B' | b'N' | b'q' | b'r' | b'b' | b'n')
}

/// SCID's annotation symbols and their NAGs, from `game_parseNag` in
/// `scid-cpp/scid/game.cpp`. `--` is a null move and `o-o` is castling, so
/// SCID's symbols for those aren't included.
const SYMBOL_NAGS: [(&str, u8); 22] = [
    ("!", 1),
    ("?", 2),
    ("!!", 3),
    ("??", 4),
    ("!?", 5),
    ("?!", 6),
    ("=", 10),
    ("~", 13),
    ("+=", 14),
    ("+/=", 14),
    ("=+", 15),
    ("=/+", 15),
    ("+/-", 16),
    ("-/+", 17),
    ("+-", 18),
    ("-+", 19),
    ("+--", 20),
    ("--+", 21),
    ("+>", 40),
    ("->", 41),
    ("=/&", 44),
    ("~=", 44),
];

/// Reads annotation symbols, check signs, and mate signs from the start of
/// `text`. Returns the symbols' NAGs and the text after the last one read.
/// Each symbol is the longest one that matches.
fn parse_symbols(mut text: &str) -> (Vec<u8>, &str) {
    let mut nags = Vec::new();
    loop {
        let symbol = SYMBOL_NAGS
            .iter()
            .filter(|(symbol, _)| text.starts_with(symbol))
            .max_by_key(|(symbol, _)| symbol.len());
        if let Some(&(symbol, nag)) = symbol {
            nags.push(nag);
            text = &text[symbol.len()..];
        } else if let Some(rest) = text.strip_prefix(['+', '#']) {
            text = rest;
        } else {
            return (nags, text);
        }
    }
}

/// The NAGs of a token made only of annotation symbols, check signs, and
/// mate signs, or of SCID's letter symbols `N` (novelty), `D` (diagram),
/// and `R` (various moves).
fn symbols_token(token: &str) -> Option<Vec<u8>> {
    match token {
        "N" => return Some(vec![146]),
        "D" => return Some(vec![201]),
        "R" => return Some(vec![144]),
        _ => {}
    }
    match parse_symbols(token) {
        (nags, "") => Some(nags),
        _ => None,
    }
}

/// The castling side of a castling token, written with `O`, `o`, or `0`,
/// with or without dashes.
fn castling_side(text: &str) -> Option<std::result::Result<CastlingSide, MoveError>> {
    if !text.starts_with(['O', 'o', '0']) || !text.chars().all(|c| "Oo0-".contains(c)) {
        return None;
    }
    Some(match text.chars().filter(|&c| c != '-').count() {
        2 => Ok(CastlingSide::KingSide),
        3 => Ok(CastlingSide::QueenSide),
        _ => Err(SanError::IllegalSan.into()),
    })
}

/// Parses a move in SAN, or in one of the other notations SCID reads.
///
/// Besides standard SAN this accepts:
///
/// - castling written with `0` or `o`, or without dashes (`0-0`, `OO`, `000`)
/// - castling after the king or rook has moved and returned, with a warning
/// - `O-O` when only queenside castling is legal, as `O-O-O`, with a warning
/// - a king moving two squares as castling (`Kg1`)
/// - coordinates (`e2e4`, `g1f3`, `e1g1`, `a7a8q`)
/// - a pawn move with a `P` (`Pe4`)
/// - `:` for a capture, and `-` between squares (`e:d5`, `Ng1-f3`)
/// - a missing or extra capture `x` when that leaves only one legal move
/// - a pawn capture without a rank when only one capture fits (`ed`, `ab=Q`)
/// - a promotion without `=` or with a lowercase piece (`a8Q`, `a8q`), or
///   without a piece, as a queen (`a8`)
/// - SCID's null moves
fn parse_move(text: &str, position: &ScidPosition) -> std::result::Result<ParsedMove, MoveError> {
    let chess = position.chess();
    if is_null_move(text) {
        return Ok(GameMove::Null.into());
    }
    if let Some(side) = castling_side(text) {
        return parse_castling(text, side?, position);
    }

    let text = match text.strip_prefix('P') {
        Some(rest) if rest.starts_with(|c: char| ('a'..='h').contains(&c)) => rest,
        _ => text,
    };
    let text: String = text
        .chars()
        .filter(|&c| c != '-')
        .map(|c| if c == ':' { 'x' } else { c })
        .collect();

    if let Some(m) = coordinate_move(&text, chess) {
        return Ok(m?.into());
    }
    if let Some(m) = pawn_capture_without_rank(&text, chess) {
        return Ok(m?.into());
    }

    let text = normalize_promotion(&text);
    let mut san = San::from_ascii(text.as_bytes())?;
    // SCID promotes to a queen when no piece is given.
    if let San::Normal {
        role: Role::Pawn,
        to,
        promotion: promotion @ None,
        ..
    } = &mut san
    {
        if matches!(to.rank(), Rank::First | Rank::Eighth) {
            *promotion = Some(Role::Queen);
        }
    }
    match san.to_move(chess) {
        Ok(m) => Ok(m.into()),
        Err(SanError::IllegalSan) => alternative_move(&san, chess)
            .map(ParsedMove::from)
            .ok_or_else(|| SanError::IllegalSan.into()),
        Err(e) => Err(e.into()),
    }
}

/// Castling on `side`, falling back as SCID does: `O-O` is played as
/// `O-O-O` if only queenside castling is legal, and castling without the
/// castling right is played if it would otherwise be legal.
fn parse_castling(
    text: &str,
    side: CastlingSide,
    position: &ScidPosition,
) -> std::result::Result<ParsedMove, MoveError> {
    let castle = |side| San::Castle(side).to_move(position.chess()).ok();
    if let Some(m) = castle(side) {
        return Ok(m.into());
    }
    let text = text.to_string();
    let (mv, warning) = if let Some(m) =
        castle(CastlingSide::QueenSide).filter(|_| side == CastlingSide::KingSide)
    {
        (m, PgnWarningKind::KingsideReadAsQueenside { text })
    } else if let Some(m) = position.castle_without_right(side) {
        (m, PgnWarningKind::CastlingWithoutRight { text })
    } else {
        return Err(SanError::IllegalSan.into());
    };
    Ok(ParsedMove {
        mv: GameMove::Move(mv),
        warning: Some(warning),
    })
}

/// A move in coordinates, such as `e2e4`, `g1f3`, `e1g1`, `exd5` written as
/// `e4xd5`, or `a7a8q`. Returns `None` if `text` isn't in coordinates.
fn coordinate_move(text: &str, chess: &Chess) -> Option<std::result::Result<Move, MoveError>> {
    let text = text.replace('x', "");
    let bytes = text.as_bytes();
    let (squares, promotion) = match bytes {
        [squares @ .., b'=', piece] | [squares @ .., piece] if is_promotion_letter(*piece) => {
            (squares, Some(*piece))
        }
        squares => (squares, None),
    };
    let [from_file, from_rank, to_file, to_rank] = squares else {
        return None;
    };
    let square = |file: u8, rank: u8| {
        Some(Square::from_coords(
            File::from_char(char::from(file))?,
            Rank::from_char(char::from(rank))?,
        ))
    };
    let from = square(*from_file, *from_rank)?;
    let to = square(*to_file, *to_rank)?;
    let promotion = match promotion {
        Some(piece) => Some(Role::from_char(char::from(piece.to_ascii_lowercase()))?),
        None => None,
    };
    let uci = UciMove::Normal {
        from,
        to,
        promotion,
    };
    Some(uci.to_move(chess).map_err(Into::into))
}

/// A pawn capture without the destination's rank, such as `ed`, `exd`, or
/// `ab=Q`. SCID picks the lowest rank when more than one capture fits; this
/// crate rejects that as ambiguous. Returns `None` if `text` isn't in this
/// form.
fn pawn_capture_without_rank(
    text: &str,
    chess: &Chess,
) -> Option<std::result::Result<Move, MoveError>> {
    let bytes = text.as_bytes();
    // `ab` is a capture on the b-file, not a promotion to a bishop.
    let (files, promotion) = match bytes {
        [_, _] | [_, b'x', _] => (bytes, None),
        [files @ .., b'=', piece] | [files @ .., piece] if is_promotion_letter(*piece) => {
            (files, Some(*piece))
        }
        _ => return None,
    };
    let ([from, to] | [from, b'x', to]) = files else {
        return None;
    };
    let from = File::from_char(char::from(*from))?;
    let to = File::from_char(char::from(*to))?;
    let promotion = match promotion {
        Some(piece) => Some(Role::from_char(char::from(piece.to_ascii_lowercase()))?),
        None => None,
    };
    let candidates: Vec<Move> = chess
        .legal_moves()
        .into_iter()
        .filter(|m| {
            m.role() == Role::Pawn
                && m.is_capture()
                && m.from().map(Square::file) == Some(from)
                && m.to().file() == to
                // SCID promotes to a queen when no piece is given.
                && m.promotion().is_none_or(|role| role == promotion.unwrap_or(Role::Queen))
                && (promotion.is_none() || m.promotion().is_some())
        })
        .collect();
    Some(match candidates[..] {
        [m] => Ok(m),
        [] => Err(SanError::IllegalSan.into()),
        _ => Err(SanError::AmbiguousSan.into()),
    })
}

/// Writes a pawn's promotion as SAN does, as in `a8=Q`: SCID also accepts
/// it without `=` and with a lowercase piece.
fn normalize_promotion(text: &str) -> String {
    let is_pawn_move = text.starts_with(|c: char| ('a'..='h').contains(&c));
    match text.as_bytes() {
        [square @ .., rank @ (b'1' | b'8'), b'=', piece]
        | [square @ .., rank @ (b'1' | b'8'), piece]
            if is_pawn_move && is_promotion_letter(*piece) =>
        {
            let square = &text[..square.len()];
            let rank = char::from(*rank);
            let piece = char::from(piece.to_ascii_uppercase());
            format!("{square}{rank}={piece}")
        }
        _ => text.to_string(),
    }
}

/// Another reading of a SAN move that isn't legal as written: SCID ignores
/// the capture sign, and reads a king moving two squares from its original
/// square as castling.
fn alternative_move(san: &San, chess: &Chess) -> Option<Move> {
    let San::Normal {
        role,
        file,
        rank,
        capture,
        to,
        promotion,
    } = *san
    else {
        return None;
    };
    let other_capture = San::Normal {
        role,
        file,
        rank,
        capture: !capture,
        to,
        promotion,
    };
    if let Ok(m) = other_capture.to_move(chess) {
        return Some(m);
    }
    if role != Role::King {
        return None;
    }
    let home = Square::from_coords(File::E, chess.turn().backrank());
    let from_home = chess.board().king_of(chess.turn()) == Some(home)
        && file.is_none_or(|f| f == File::E)
        && rank.is_none_or(|r| r == home.rank());
    let side = match to {
        _ if !from_home || to.rank() != home.rank() => return None,
        _ if to.file() == File::G => CastlingSide::KingSide,
        _ if to.file() == File::C => CastlingSide::QueenSide,
        _ => return None,
    };
    San::Castle(side).to_move(chess).ok()
}

fn parse_result(s: &str) -> GameResult {
    match s.trim() {
        "1-0" => GameResult::White,
        "0-1" => GameResult::Black,
        "1/2-1/2" | "½-½" => GameResult::Draw,
        _ => GameResult::None,
    }
}

/// Parses every game in `pgn`.
///
/// # Errors
///
/// Returns [`Error::PgnGame`] for the first game that fails to parse. To
/// skip failed games and continue, iterate over a [`PgnParser`] instead.
pub fn parse_pgn(pgn: &str) -> Result<Vec<Game>> {
    PgnParser::new(pgn).collect()
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
        assert_eq!(game.num_half_moves, 4);
    }

    #[test]
    fn test_parse_result() {
        assert_eq!(parse_result("1-0"), GameResult::White);
        assert_eq!(parse_result("0-1"), GameResult::Black);
        assert_eq!(parse_result("1/2-1/2"), GameResult::Draw);
        assert_eq!(parse_result("*"), GameResult::None);
    }

    fn position(fen: &str) -> ScidPosition {
        ScidPosition::from_setup_position(crate::position::chess_from_fen(fen).unwrap()).unwrap()
    }

    /// The SAN of `text` read in `position`, and its warning, if any.
    fn read(text: &str, position: &ScidPosition) -> (String, Option<PgnWarningKind>) {
        let parsed = parse_move(text, position).unwrap_or_else(|e| panic!("{text}: {e}"));
        // A castle without the castling right isn't legal in `position`
        // itself, so play it to get its SAN.
        let mut position = position.clone();
        let san = position.play(parsed.mv).unwrap().san.to_string();
        (san, parsed.warning)
    }

    fn san(text: &str, position: &ScidPosition) -> String {
        let (san, warning) = read(text, position);
        assert_eq!(warning, None, "{text}");
        san
    }

    #[test]
    fn test_promotion_notations() {
        let pos = position("1n2k3/P7/8/8/8/8/8/4K3 w - - 0 1");
        for (token, expected) in [
            ("a8Q", "a8=Q"),
            ("axb8N", "axb8=N"),
            ("a8=R", "a8=R"),
            ("a8q", "a8=Q"),
            ("a8=n", "a8=N"),
            ("a8b", "a8=B"),
            ("a8", "a8=Q"),
            ("axb8", "axb8=Q"),
            ("a7a8q", "a8=Q"),
            ("a7a8b", "a8=B"),
            ("a7b8=N", "axb8=N"),
            ("ab=Q", "axb8=Q"),
            ("abQ", "axb8=Q"),
            ("abn", "axb8=N"),
            ("ab", "axb8=Q"),
            ("axb", "axb8=Q"),
        ] {
            assert_eq!(san(token, &pos), expected, "{token}");
        }
        // SCID requires the piece in coordinates too.
        assert!(parse_move("a7a8", &pos).is_err());
    }

    #[test]
    fn test_pawn_capture_without_rank() {
        let pos = position("4k3/8/8/3p4/4P3/8/8/4K3 w - - 0 1");
        assert_eq!(san("ed", &pos), "exd5");
        assert_eq!(san("exd", &pos), "exd5");
        assert_eq!(san("e:d", &pos), "exd5");
        assert!(parse_move("ef", &pos).is_err());
        // En passant.
        let pos = position("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 2");
        assert_eq!(san("ed", &pos), "exd6");
        // Two captures fit. SCID would play exd4.
        let pos = position("4k3/8/3n4/4P3/3n4/4P3/8/4K3 w - - 0 1");
        let err = parse_move("ed", &pos).unwrap_err();
        assert_eq!(err.to_string(), SanError::AmbiguousSan.to_string());
    }

    #[test]
    fn test_coordinates_and_separators() {
        let pos = ScidPosition::standard();
        assert_eq!(san("e2e4", &pos), "e4");
        assert_eq!(san("g1f3", &pos), "Nf3");
        assert_eq!(san("e2-e4", &pos), "e4");
        assert_eq!(san("Ng1-f3", &pos), "Nf3");
        assert_eq!(san("Ng1f3", &pos), "Nf3");
        assert!(parse_move("e2e5", &pos).is_err());
        let pos = position("4k3/8/8/3p4/4P3/5N2/8/4K3 w - - 0 1");
        assert_eq!(san("e:d5", &pos), "exd5");
        assert_eq!(san("e4xd5", &pos), "exd5");
        assert_eq!(san("e4:d5", &pos), "exd5");
    }

    #[test]
    fn test_castling_notations() {
        let pos = position("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
        for (token, expected) in [
            ("O-O", "O-O"),
            ("0-0", "O-O"),
            ("o-o", "O-O"),
            ("OO", "O-O"),
            ("00", "O-O"),
            ("O-O-O", "O-O-O"),
            ("0-0-0", "O-O-O"),
            ("OOO", "O-O-O"),
            ("000", "O-O-O"),
            ("Kg1", "O-O"),
            ("Kc1", "O-O-O"),
            ("Ke1g1", "O-O"),
            ("e1g1", "O-O"),
            ("e1c1", "O-O-O"),
        ] {
            assert_eq!(san(token, &pos), expected, "{token}");
        }
        assert!(parse_move("O", &pos).is_err());
        assert!(parse_move("O-O-O-O", &pos).is_err());
    }

    #[test]
    fn test_castling_without_the_castling_right() {
        let pos = position("r3k2r/8/8/8/8/8/8/R3K2R w - - 0 1");
        for (token, expected) in [("O-O", "O-O"), ("0-0-0", "O-O-O")] {
            let (san, warning) = read(token, &pos);
            assert_eq!(san, expected);
            assert_eq!(
                warning,
                Some(PgnWarningKind::CastlingWithoutRight {
                    text: token.to_string()
                })
            );
        }
        // SCID plays only castling tokens without the castling right.
        assert!(parse_move("Kg1", &pos).is_err());
        assert!(parse_move("e1g1", &pos).is_err());
        // No rook.
        let pos = position("4k3/8/8/8/8/8/8/4K3 w - - 0 1");
        assert!(parse_move("O-O", &pos).is_err());
        assert!(parse_move("O-O-O", &pos).is_err());
    }

    #[test]
    fn test_kingside_read_as_queenside() {
        let pos = position("r3k2r/8/8/8/8/8/8/R3K1NR w KQkq - 0 1");
        let (san, warning) = read("O-O", &pos);
        assert_eq!(san, "O-O-O");
        assert_eq!(
            warning,
            Some(PgnWarningKind::KingsideReadAsQueenside {
                text: "O-O".to_string()
            })
        );
        // SCID doesn't read `O-O-O` as `O-O`.
        let pos = position("r3k2r/8/8/8/8/8/8/RN2K2R w KQkq - 0 1");
        assert!(parse_move("O-O-O", &pos).is_err());
    }

    #[test]
    fn test_classify_token() {
        let mv = |text: &str, suffix: &str| MoveToken::Move {
            text: text.to_string(),
            suffix: suffix.to_string(),
        };
        assert_eq!(classify_token("12."), MoveToken::Ignored);
        assert_eq!(classify_token("12"), MoveToken::Ignored);
        assert_eq!(classify_token("..."), MoveToken::Ignored);
        assert_eq!(classify_token("+"), MoveToken::Ignored);
        assert_eq!(classify_token("e.p."), MoveToken::Ignored);
        assert_eq!(classify_token("ep"), MoveToken::Ignored);
        assert_eq!(classify_token("12.e4"), mv("e4", ""));
        assert_eq!(classify_token("3...Nf6!?"), mv("Nf6", "!?"));
        assert_eq!(classify_token("00"), mv("00", ""));
        assert_eq!(classify_token("O-O-+"), mv("O-O", "-+"));
        assert_eq!(classify_token("e4=+"), mv("e4", "=+"));
        assert_eq!(classify_token("e8=Q+"), mv("e8=Q", "+"));
        assert_eq!(classify_token("Nf3="), mv("Nf3", "="));
        assert_eq!(classify_token("e2-e4"), mv("e2-e4", ""));
        assert_eq!(classify_token("e4-/+"), mv("e4", "-/+"));
        assert_eq!(classify_token("e4--+"), mv("e4", "--+"));
        assert_eq!(classify_token("e4->"), mv("e4", "->"));
        assert_eq!(classify_token("e4-"), mv("e4-", ""));
        assert_eq!(classify_token("O-O-O"), mv("O-O-O", ""));
        assert_eq!(classify_token("e4~"), mv("e4", "~"));
        assert_eq!(classify_token("exd6ep"), mv("exd6", ""));
        assert_eq!(classify_token("exd6e.p.+"), mv("exd6", "+"));
        assert_eq!(classify_token("--"), mv("--", ""));
        assert_eq!(classify_token("Z0!"), mv("Z0", "!"));
        assert_eq!(classify_token("null?"), mv("null", "?"));
        assert_eq!(classify_token("--+"), MoveToken::Nags(vec![21]));
        assert_eq!(classify_token("--!"), mv("--", "!"));
        for word in ["nullify", "Z0x", "--x"] {
            assert_eq!(
                classify_token(word),
                MoveToken::Unknown(word.to_string()),
                "{word}"
            );
        }
        assert_eq!(classify_token("1-0"), MoveToken::Result(GameResult::White));
        assert_eq!(classify_token("0:1"), MoveToken::Result(GameResult::Black));
        assert_eq!(classify_token("1/2"), MoveToken::Result(GameResult::Draw));
        assert_eq!(
            classify_token("1/2:1/2"),
            MoveToken::Result(GameResult::Draw)
        );
        assert_eq!(classify_token("N"), MoveToken::Nags(vec![146]));
        assert_eq!(classify_token("D"), MoveToken::Nags(vec![201]));
        assert_eq!(classify_token("R"), MoveToken::Nags(vec![144]));
        assert_eq!(classify_token("+-"), MoveToken::Nags(vec![18]));
        assert_eq!(classify_token("!!+-"), MoveToken::Nags(vec![3, 18]));
        for unknown in ["zz", "/\\", "Diagram", "B", "nf3", "-", "\u{b1}"] {
            assert_eq!(
                classify_token(unknown),
                MoveToken::Unknown(unknown.to_string()),
                "{unknown}"
            );
        }
    }

    /// The moves and each node's NAGs of a game with `movetext`.
    fn moves_and_nags(movetext: &str) -> (Vec<String>, Vec<Vec<u8>>) {
        let game = parse_single_game(&format!("[Event \"x\"]\n\n{movetext}\n")).unwrap();
        assert!(game.warnings.is_empty(), "{movetext}: {:?}", game.warnings);
        let sans = game.moves.iter().map(|node| node.san.clone()).collect();
        let nags = game.moves.iter().map(|node| node.nags.clone()).collect();
        (sans, nags)
    }

    #[test]
    fn test_every_symbol_matches_its_nag_attached_or_separate() {
        for (symbol, nag) in SYMBOL_NAGS {
            let expected = moves_and_nags(&format!("1. e4 ${nag} e5 2. Nf3 ${nag} *"));
            for movetext in [
                format!("1. e4{symbol} e5 2. Nf3{symbol} *"),
                format!("1. e4 {symbol} e5 2. Nf3 {symbol} *"),
            ] {
                assert_eq!(moves_and_nags(&movetext), expected, "{movetext}");
            }
        }
        for (symbol, nag) in [("N", 146), ("D", 201), ("R", 144)] {
            assert_eq!(
                moves_and_nags(&format!("1. e4 {symbol} e5 *")),
                moves_and_nags(&format!("1. e4 ${nag} e5 *")),
                "{symbol}"
            );
        }
    }

    #[test]
    fn test_parse_symbols() {
        assert_eq!(parse_symbols(""), (vec![], ""));
        assert_eq!(parse_symbols("!?"), (vec![5], ""));
        assert_eq!(parse_symbols("+!"), (vec![1], ""));
        assert_eq!(parse_symbols("#!"), (vec![1], ""));
        assert_eq!(parse_symbols("++"), (vec![], ""));
        assert_eq!(parse_symbols("+="), (vec![14], ""));
        assert_eq!(parse_symbols("+/-"), (vec![16], ""));
        assert_eq!(parse_symbols("+--"), (vec![20], ""));
        assert_eq!(parse_symbols("--+"), (vec![21], ""));
        assert_eq!(parse_symbols("!x"), (vec![1], "x"));
        for (symbol, nag) in SYMBOL_NAGS {
            assert_eq!(parse_symbols(symbol), (vec![nag], ""), "{symbol}");
        }
    }

    #[test]
    fn test_strip_move_number() {
        assert_eq!(strip_move_number("12."), "");
        assert_eq!(strip_move_number("12..."), "");
        assert_eq!(strip_move_number("12.e4"), "e4");
        assert_eq!(strip_move_number("3...Nf6"), "Nf6");
        assert_eq!(strip_move_number("0-0"), "0-0");
        assert_eq!(strip_move_number("e4"), "e4");
    }

    #[test]
    fn test_parse_move_normalization() {
        let pos = position("r3k2r/pppq1ppp/2n2n2/3pp3/1b1PP3/2N2N2/PPPQ1PPP/R3K2R w KQkq - 0 1");
        assert_eq!(san("O-O", &pos), "O-O");
        assert_eq!(san("Pa3", &pos), "a3");
        assert_eq!(san("Nxe5", &pos), "Nxe5");
        assert_eq!(san("Ne5", &pos), "Nxe5");
        assert_eq!(san("dxe5", &pos), "dxe5");
        assert_eq!(san("Nxg5", &pos), "Ng5");
        assert_eq!(san("Z0", &pos), "--");
        assert_eq!(san("null", &pos), "--");
        assert!(parse_move("Nd6", &pos).is_err());
        assert!(parse_move("Qe2e3", &pos).is_err());
        assert!(parse_move("Xe4", &pos).is_err());
    }

    #[test]
    fn test_castling_rights_text() {
        assert_eq!(
            castling_rights_text(Bitboard::from(Square::A8) | Square::H1 | Square::A1),
            "KQq"
        );
        assert_eq!(castling_rights_text(Bitboard::from(Square::B1)), "B");
    }

    #[test]
    fn test_warning_display() {
        let warning = PgnWarning {
            game: 3,
            line: 12,
            kind: PgnWarningKind::CastlingWithoutRight {
                text: "O-O".to_string(),
            },
        };
        assert_eq!(
            warning.to_string(),
            "PGN game 3, line 12: \"O-O\" castles without the castling right"
        );
    }
}
