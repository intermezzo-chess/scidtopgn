use shakmaty::san::{San, SanError};
use shakmaty::Chess;

use crate::common::GameResult;
use crate::error::{Error, Result};
use crate::game::{Game, Marker, MoveNode};
use crate::mov::GameMove;
use crate::position::{chess_from_fen, ScidPosition};
use crate::Date;

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
    /// # Errors
    ///
    /// Returns an error if a tag, the `FEN`, or a move is invalid, or if the
    /// `Variant` tag names a variant other than standard chess.
    pub fn parse_game(&mut self) -> Result<Game> {
        let mut game = Game::new();
        let mut fen = None;
        let mut variant = None;
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
                        fen = Some(value);
                    } else {
                        if name == "Variant" {
                            variant = Some(value.clone());
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
        if let Some(fen) = fen {
            game.start_position = Some(chess_from_fen(&fen)?);
        }

        let start_idx = game.moves.len();
        let mut start = MoveNode::new();
        start.marker = Marker::StartMarker;
        game.moves.push(start);
        game.first_move = Some(start_idx);
        game.current_move = Some(start_idx);

        let position = game.initial_position()?;
        self.in_movetext = true;
        self.parse_line(&mut game, position, 0)?;

        Ok(game)
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
    ) -> Result<()> {
        // The position before the last move, where its variations start.
        let mut before_last: Option<ScidPosition> = None;

        loop {
            self.skip_whitespace();

            let Some(c) = self.peek() else {
                return if depth == 0 {
                    Ok(())
                } else {
                    Err(self.error("Unterminated variation"))
                };
            };

            match c {
                ';' | '%' => {
                    self.skip_to_end_of_line();
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
                    if depth == 0 {
                        if let Some(current_idx) = game.current_move {
                            if let Some(prev_idx) = game.moves[current_idx].prev {
                                if game.moves[prev_idx].nags.len() < 8 {
                                    game.moves[prev_idx].nags.push(nag);
                                }
                            }
                        }
                    }
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
                    return Ok(());
                }
                // The next game's tags: this game has no result.
                '[' if depth == 0 && self.at_tag_line() => {
                    return Ok(());
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

                    if is_result(&token) {
                        if depth > 0 {
                            return Err(self.error("Unterminated variation"));
                        }
                        return Ok(());
                    }

                    let san = strip_move_number(&token);
                    if !is_move_token(san.trim_end_matches(['+', '#', '!', '?'])) {
                        continue;
                    }

                    let invalid =
                        |source: Box<dyn std::error::Error + Send + Sync>| Error::PgnMove {
                            line,
                            san: san.to_string(),
                            source,
                        };
                    let mv = parse_move(san, position.chess()).map_err(invalid)?;
                    before_last = Some(position.clone());
                    let played = position.play(mv).map_err(|e| invalid(Box::new(e)))?;
                    game.append_move(mv, played.to_string(), depth == 0);
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
            if c.is_whitespace() || matches!(c, '(' | ')' | '{' | '}' | '[' | ']') {
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

        self.games += 1;
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

fn is_result(token: &str) -> bool {
    matches!(token, "1-0" | "0-1" | "1/2-1/2" | "½-½" | "*")
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

fn is_move_token(token: &str) -> bool {
    is_null_move(token)
        || token.starts_with("0-0")
        || token.starts_with("o-o")
        || token.starts_with(|c: char| "abcdefghKQRBNPO".contains(c))
}

/// SCID reads `--`, `Z0`, and `null` as a null move.
fn is_null_move(token: &str) -> bool {
    matches!(token, "--" | "Z0" | "null")
}

/// Parses a move in SAN.
///
/// Besides standard SAN this accepts a check, mate, or annotation suffix
/// (ignored, since the SAN is regenerated), castling written with zeros or
/// lowercase letters, a pawn move with a `P`, a promotion without `=`, a
/// missing or extra capture `x` when that leaves only one legal move, and
/// SCID's null moves.
fn parse_move(
    token: &str,
    chess: &Chess,
) -> std::result::Result<GameMove, Box<dyn std::error::Error + Send + Sync>> {
    let token = token.trim_end_matches(['+', '#', '!', '?']);
    if is_null_move(token) {
        return Ok(GameMove::Null);
    }
    let token = match token
        .strip_prefix("0-0")
        .or_else(|| token.strip_prefix("o-o"))
    {
        Some(rest) => format!("O-O{}", rest.replace(['0', 'o'], "O")),
        None => token.to_string(),
    };
    let token = match token.strip_prefix('P') {
        Some(rest) if rest.starts_with(|c: char| ('a'..='h').contains(&c)) => rest,
        _ => &token,
    };
    let is_pawn_move = token.starts_with(|c: char| ('a'..='h').contains(&c));
    let token = match token.as_bytes() {
        [.., b'1' | b'8', piece @ (b'Q' | b'R' | b'B' | b'N')] if is_pawn_move => {
            format!("{}={}", &token[..token.len() - 1], char::from(*piece))
        }
        _ => token.to_string(),
    };

    let san = San::from_ascii(token.as_bytes())?;
    match san.to_move(chess) {
        Ok(m) => Ok(GameMove::Move(m)),
        // SCID ignores the capture sign, so retry without it.
        Err(SanError::IllegalSan) => match san {
            San::Normal {
                role,
                file,
                rank,
                capture,
                to,
                promotion,
            } => San::Normal {
                role,
                file,
                rank,
                capture: !capture,
                to,
                promotion,
            }
            .to_move(chess)
            .map(GameMove::Move)
            .map_err(|_| SanError::IllegalSan.into()),
            _ => Err(SanError::IllegalSan.into()),
        },
        Err(e) => Err(e.into()),
    }
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

    #[test]
    fn test_promotion_without_equals_sign() {
        let chess = crate::position::chess_from_fen("1n2k3/P7/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        for (token, expected) in [("a8Q", "a8=Q"), ("axb8N", "axb8=N"), ("a8=R", "a8=R")] {
            let GameMove::Move(m) = parse_move(token, &chess).unwrap() else {
                panic!("{token} is not a null move");
            };
            assert_eq!(San::from_move(&chess, m).to_string(), expected);
        }
    }

    #[test]
    fn test_is_move_token() {
        assert!(is_move_token("e4"));
        assert!(is_move_token("Nf3"));
        assert!(is_move_token("O-O"));
        assert!(is_move_token("0-0-0"));
        assert!(is_move_token("o-o"));
        assert!(is_move_token("--"));
        assert!(!is_move_token(""));
        assert!(!is_move_token("1."));
        assert!(!is_move_token("+="));
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
        let chess = crate::position::chess_from_fen(
            "r3k2r/pppq1ppp/2n2n2/3pp3/1b1PP3/2N2N2/PPPQ1PPP/R3K2R w KQkq - 0 1",
        )
        .unwrap();
        let san = |token: &str| match parse_move(token, &chess).unwrap() {
            GameMove::Move(m) => San::from_move(&chess, m).to_string(),
            GameMove::Null => "--".to_string(),
        };
        assert_eq!(san("0-0"), "O-O");
        assert_eq!(san("0-0-0"), "O-O-O");
        assert_eq!(san("o-o"), "O-O");
        assert_eq!(san("o-o-o"), "O-O-O");
        assert_eq!(san("O-O+!?"), "O-O");
        assert_eq!(san("Pa3"), "a3");
        assert_eq!(san("Nxe5"), "Nxe5");
        assert_eq!(san("Ne5"), "Nxe5");
        assert_eq!(san("dxe5"), "dxe5");
        assert_eq!(san("Nxg5"), "Ng5");
        assert_eq!(san("Z0"), "--");
        assert_eq!(san("null"), "--");
        assert!(parse_move("Nd6", &chess).is_err());
        assert!(parse_move("Qe2e3", &chess).is_err());
        assert!(parse_move("Xe4", &chess).is_err());
    }
}
