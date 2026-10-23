use scidtopgn::{parse_pgn, parse_single_game, Game, GameResult};

#[test]
fn test_parse_simple_pgn() {
    let pgn = r#"[Event "Test"]
[Site "Test Site"]
[Date "2024.01.01"]
[Round "1"]
[White "Player1"]
[Black "Player2"]
[Result "1-0"]

1. e4 e5 2. Nf3 Nc6 1-0
"#;

    let game = parse_single_game(pgn).expect("Failed to parse PGN");
    assert_eq!(game.event, "Test");
    assert_eq!(game.white, "Player1");
    assert_eq!(game.black, "Player2");
    assert_eq!(game.result, GameResult::White);
}

#[test]
fn test_parse_multiple_games() {
    let pgn = r#"[Event "Game 1"]
[Site "?"]
[Date "2024.01.01"]
[Round "1"]
[White "White1"]
[Black "Black1"]
[Result "1-0"]

1. e4 e5 2. Nf3 Nc6 1-0

[Event "Game 2"]
[Site "?"]
[Date "2024.01.02"]
[Round "2"]
[White "White2"]
[Black "Black2"]
[Result "0-1"]

1. d4 d5 0-1
"#;

    let games = parse_pgn(pgn).expect("Failed to parse PGN");
    assert_eq!(games.len(), 2);
    assert_eq!(games[0].event, "Game 1");
    assert_eq!(games[1].event, "Game 2");
}

#[test]
fn test_parse_with_comments() {
    let pgn = r#"[Event "Test"]
[Site "?"]
[Date "2024.01.01"]
[Round "1"]
[White "Player1"]
[Black "Player2"]
[Result "*"]

1. e4 {This is a comment} e5 *
"#;

    let game = parse_single_game(pgn).expect("Failed to parse PGN");
    assert_eq!(game.event, "Test");
}

#[test]
fn test_parse_castling() {
    let pgn = r#"[Event "Test"]
[Site "?"]
[Date "2024.01.01"]
[Round "1"]
[White "Player1"]
[Black "Player2"]
[Result "*"]

1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 4. c3 Nf6 5. d4 exd4 6. cxd4 Bb4+ 7. Bd2 Bxd2+ 8. Nbxd2 d5 9. exd5 Nxd5 10. O-O *
"#;

    let game = parse_single_game(pgn).expect("Failed to parse PGN");
    assert_eq!(game.event, "Test");
}

#[test]
fn test_game_roundtrip() {
    let mut game = Game::new();
    game.event = "Test Event".to_string();
    game.site = "Test Site".to_string();
    game.white = "White Player".to_string();
    game.black = "Black Player".to_string();
    game.result = GameResult::Draw;

    let pgn = game.to_pgn();

    assert!(pgn.contains("[Event \"Test Event\"]"));
    assert!(pgn.contains("[White \"White Player\"]"));
    assert!(pgn.contains("[Black \"Black Player\"]"));
    assert!(pgn.contains("1/2-1/2"));
}

use scidtopgn::shakmaty::{Color, Position};
use scidtopgn::{Error, PgnParser, PgnWarningKind};

fn pgn_game(tags: &str, movetext: &str) -> String {
    format!("[Event \"Test\"]\n{tags}\n\n{movetext}\n")
}

/// The main line's SAN.
fn sans(game: &Game) -> Vec<String> {
    let mut sans = Vec::new();
    let mut idx = game.first_move;
    while let Some(i) = idx {
        if game.moves[i].move_data.is_some() {
            sans.push(game.moves[i].san.clone());
        }
        idx = game.moves[i].next;
    }
    sans
}

fn warning_kinds(game: &Game) -> Vec<PgnWarningKind> {
    game.warnings.iter().map(|w| w.kind.clone()).collect()
}

fn parse_sans(movetext: &str) -> Vec<String> {
    sans(&parse_single_game(&pgn_game("", movetext)).unwrap())
}

/// Asserts that the movetext fails at the move `san`.
fn assert_illegal(tags: &str, movetext: &str, san: &str) {
    match parse_single_game(&pgn_game(tags, movetext)) {
        Err(Error::PgnMove { san: bad, .. }) => assert_eq!(bad, san, "{movetext}"),
        other => panic!("{movetext}: expected an invalid move, got {other:?}"),
    }
}

#[test]
fn test_canonical_san_is_stored() {
    // Wrong check and mate signs, an unneeded disambiguation, castling with
    // zeros, a missing capture sign, and annotation glyphs.
    assert_eq!(
        parse_sans("1. f3 e5 2. g4+ Qh4 *"),
        ["f3", "e5", "g4", "Qh4#"]
    );
    assert_eq!(
        parse_sans("1. e4 e5 2. Ngf3 Nc6 3. Bc4!? Nf6 4. 0-0 Nxe4 5. Re1 d5 6. Rxe4 de4 *"),
        ["e4", "e5", "Nf3", "Nc6", "Bc4", "Nf6", "O-O", "Nxe4", "Re1", "d5", "Rxe4", "dxe4"]
    );
}

#[test]
fn test_castling_rights() {
    assert_eq!(
        parse_sans("1. e4 e5 2. Nf3 Nf6 3. Bc4 Bc5 4. O-O O-O *"),
        ["e4", "e5", "Nf3", "Nf6", "Bc4", "Bc5", "O-O", "O-O"]
    );
    // Like SCID, castling after the king or rook has moved and returned is
    // played, with a warning.
    for movetext in [
        "1. e4 e5 2. Nf3 Nf6 3. Bc4 Bc5 4. Ke2 Ke7 5. Ke1 Ke8 6. O-O *",
        "1. e4 e5 2. Nf3 Nf6 3. Bc4 Bc5 4. Rg1 Rg8 5. Rh1 Rh8 6. O-O *",
    ] {
        let game = parse_single_game(&pgn_game("", movetext)).unwrap();
        assert_eq!(sans(&game).last().map(String::as_str), Some("O-O"));
        assert_eq!(
            warning_kinds(&game),
            [PgnWarningKind::CastlingWithoutRight {
                text: "O-O".to_string()
            }]
        );
    }
    // Castling through an attacked square.
    assert_illegal(
        "[FEN \"4k3/8/8/8/8/8/5r2/4K2R w K - 0 1\"]",
        "1. O-O *",
        "O-O",
    );
}

#[test]
fn test_en_passant() {
    assert_eq!(
        parse_sans("1. e4 a6 2. e5 d5 3. exd6 *"),
        ["e4", "a6", "e5", "d5", "exd6"]
    );
    // The right to capture en passant expires after one move.
    assert_illegal("", "1. e4 a6 2. e5 d5 3. a3 h6 4. exd6 *", "exd6");
    // Capturing would expose the king to the rook on h5.
    let tags = "[FEN \"4k3/2p5/8/KP5r/8/8/8/8 b - - 0 1\"]";
    assert_illegal(tags, "1... c5 2. bxc6 *", "bxc6");
    let tags = "[FEN \"4k3/2p5/8/KP6/8/8/8/8 b - - 0 1\"]";
    assert_eq!(
        sans(&parse_single_game(&pgn_game(tags, "1... c5 2. bxc6 *")).unwrap()),
        ["c5", "bxc6"]
    );
}

#[test]
fn test_illegal_and_ambiguous_moves() {
    // Moves that expose the king, pass through a piece, or can't reach
    // their square.
    assert_illegal("", "1. e4 e5 2. d3 Bb4+ 3. Nc3 Nf6 4. Nb5 *", "Nb5");
    assert_illegal("", "1. e4 e5 2. Ke2 Qh4 3. Kf3 d6 4. Kg4 *", "Kg4");
    assert_illegal("", "1. Ra3 *", "Ra3");
    assert_illegal("", "1. Nd4 *", "Nd4");
    // Both knights can reach d2.
    assert_illegal("", "1. Nf3 Nf6 2. d3 d6 3. Nd2 *", "Nd2");
}

#[test]
fn test_null_moves() {
    assert_eq!(parse_sans("1. e4 -- 2. d4 Z0 3. Nf3 null *").len(), 6);
    // A null move can't leave the king in check.
    assert_illegal("", "1. e4 e5 2. Qh5 Nc6 3. Qxf7+ -- *", "--");
}

#[test]
fn test_fen_setup() {
    let tags = "[SetUp \"1\"]\n[FEN \"4k3/8/8/8/8/8/4p3/4K3 b - - 3 40\"]";
    let game = parse_single_game(&pgn_game(tags, "40... Kd7 41. Kxe2 *")).unwrap();
    let start = game.start_position.as_ref().unwrap();
    assert_eq!(start.turn(), Color::Black);
    assert_eq!(start.fullmoves().get(), 40);
    assert_eq!(sans(&game), ["Kd7", "Kxe2"]);
    let pgn = game.to_pgn();
    assert!(pgn.contains("[SetUp \"1\"]\n[FEN \"4k3/8/8/8/8/8/4p3/4K3 b - - 3 40\"]\n"));
    assert_eq!(pgn.matches("[SetUp").count(), 1);
    assert!(pgn.contains("\n40... Kd7 41. Kxe2\n"), "{pgn}");

    // A FEN without SetUp still sets the position, as in SCID.
    let game = parse_single_game(&pgn_game(
        "[FEN \"4k3/8/8/8/8/8/8/R3K3 w Q - 0 1\"]",
        "1. O-O-O *",
    ))
    .unwrap();
    assert_eq!(sans(&game), ["O-O-O"]);

    // Lichess marks standard games from a FEN as "From Position".
    let tags = "[Variant \"From Position\"]\n[FEN \"4k3/8/8/8/8/8/8/4K3 w - - 0 1\"]";
    assert!(parse_single_game(&pgn_game(tags, "1. Kd2 *")).is_ok());
}

#[test]
fn test_invalid_setups() {
    for fen in [
        "not a fen",
        "8/8/8/8/8/8/8/4K3 w - - 0 1",
        "4k3/8/8/8/8/8/8/4K2P w - - 0 1",
    ] {
        let tags = format!("[SetUp \"1\"]\n[FEN \"{fen}\"]");
        assert!(
            matches!(
                parse_single_game(&pgn_game(&tags, "*")),
                Err(Error::InvalidFen { .. })
            ),
            "{fen}"
        );
    }
    for variant in ["Chess960", "Crazyhouse"] {
        let tags = format!("[Variant \"{variant}\"]");
        assert!(matches!(
            parse_single_game(&pgn_game(&tags, "1. e4 *")),
            Err(Error::UnsupportedVariant { .. })
        ));
    }
}

#[test]
fn test_variations_start_before_the_varied_move() {
    let game = parse_single_game(&pgn_game(
        "",
        "1. e4 e5 (1... c5 2. Nf3 (2. c3 d5) d6) (1... e6 2. d4) 2. Nf3 *",
    ))
    .unwrap();
    assert_eq!(sans(&game), ["e4", "e5", "Nf3"]);
    let e5 = game.moves[game.first_move.unwrap()].next.unwrap();
    let e5 = game.moves[e5].next.unwrap();
    assert_eq!(game.moves[e5].san, "e5");
    assert_eq!(game.moves[e5].num_variations, 2);

    for movetext in [
        "(1. d4) 1. e4 *",
        "1. e4 (1... e5) *",
        "1. e4 (1. d4 *",
        "1. e4 ) *",
    ] {
        assert!(
            parse_single_game(&pgn_game("", movetext)).is_err(),
            "{movetext}"
        );
    }
}

const VALID: &str = "[Event \"Valid\"]\n\n1. e4 e5 1-0\n";
const ILLEGAL_MOVE: &str = "[Event \"Illegal\"]\n\n1. e4 e5 2. Ke3 1-0\n";
const BAD_TAG: &str = "[Event \"Bad tag]\n[Site \"?\"]\n\n1. d4 d5 1-0\n";

#[test]
fn test_parser_skips_failed_games() {
    for bad in [ILLEGAL_MOVE, BAD_TAG] {
        let pgn = [VALID, bad, VALID].join("\n");
        let results: Vec<_> = PgnParser::new(&pgn).collect();
        assert_eq!(results.len(), 3, "{bad}");
        assert!(results[0].is_ok());
        assert!(matches!(results[1], Err(Error::PgnGame { number: 2, .. })));
        assert_eq!(results[2].as_ref().unwrap().event, "Valid");

        // parse_pgn reports the failed game instead of dropping it.
        assert!(matches!(
            parse_pgn(&pgn),
            Err(Error::PgnGame { number: 2, .. })
        ));
    }

    // A failed first or last game.
    let results: Vec<_> = PgnParser::new(&[ILLEGAL_MOVE, VALID].join("\n")).collect();
    assert!(results[0].is_err() && results[1].is_ok() && results.len() == 2);
    let results: Vec<_> = PgnParser::new(&[VALID, ILLEGAL_MOVE].join("\n")).collect();
    assert!(results[0].is_ok() && results[1].is_err() && results.len() == 2);
}

#[test]
fn test_parser_handles_crlf_and_missing_results() {
    let pgn = [VALID, ILLEGAL_MOVE, VALID]
        .join("\n")
        .replace('\n', "\r\n");
    let results: Vec<_> = PgnParser::new(&pgn).collect();
    assert_eq!(results.len(), 3);
    assert!(results[0].is_ok() && results[1].is_err() && results[2].is_ok());

    // A game without a result ends at the next game's tags.
    let pgn = "[Event \"A\"]\n\n1. e4 e5\n[Event \"B\"]\n\n1. d4 *\n";
    let games = parse_pgn(pgn).unwrap();
    assert_eq!(games.len(), 2);
    assert_eq!(sans(&games[0]), ["e4", "e5"]);
}

#[test]
fn test_malformed_input_does_not_panic() {
    for input in [
        "",
        "\u{feff}",
        "[",
        "[Event",
        "[Event \"x",
        "{",
        "(",
        ")",
        "1.",
        "1. e4 (",
        "1. e4 {",
        "$",
        "$999",
        "e9",
        "Qxz1",
        "O-O-O-O",
        "[FEN \"\"]\n\n*",
        "]]]] }}}} [[[[",
        "1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 ((((",
    ] {
        let _ = PgnParser::new(input).collect::<Vec<_>>();
        let _ = parse_single_game(input);
    }
}

/// The events of the games the parser yields, or "error" for a failed game.
fn events(pgn: &str) -> Vec<String> {
    PgnParser::new(pgn)
        .map(|result| match result {
            Ok(game) => game.event,
            Err(_) => "error".to_string(),
        })
        .collect()
}

#[test]
fn test_broken_last_tag_keeps_the_next_game() {
    let good = "[Event \"Good\"]\n[Result \"1-0\"]\n\n1. e4 e5 1-0\n";
    for broken in [
        // A missing closing quote.
        "[Event \"Broken]\n\n1. d4 d5 *\n",
        // A missing closing bracket.
        "[Event \"Broken\"\n\n1. d4 d5 *\n",
        // A missing value.
        "[Event\n\n1. d4 d5 *\n",
    ] {
        let pgn = [broken, good].join("\n");
        assert_eq!(events(&pgn), ["error", "Good"], "{broken}");
        assert_eq!(events(&pgn.replace('\n', "\r\n")), ["error", "Good"]);
    }
    // A tag value can't continue onto the next line.
    assert!(matches!(
        parse_single_game("[Event \"Broken\n\"]\n\n*\n"),
        Err(Error::Parse { line: 1, .. })
    ));
}

#[test]
fn test_recovery_finds_indented_tags() {
    for indent in ["  ", "\t", " \t "] {
        let pgn = [
            format!("{indent}[Event \"First\"]\n\n1. e4 e5 1-0\n"),
            format!("{indent}[Event \"Illegal\"]\n\n1. Ke3 *\n"),
            format!("{indent}[Event \"Last\"]\n{indent}[Result \"1-0\"]\n\n1. d4 d5 1-0\n"),
        ]
        .join("\n");
        assert_eq!(events(&pgn), ["First", "error", "Last"], "{indent:?}");

        // A game without a result ends at indented tags too.
        let pgn = format!("{indent}[Event \"A\"]\n\n1. e4 e5\n{indent}[Event \"B\"]\n\n1. d4 *\n");
        assert_eq!(events(&pgn), ["A", "B"], "{indent:?}");
    }
}

#[test]
fn test_recovery_skips_line_comments() {
    let good = "[Event \"Good\"]\n[Result \"1-0\"]\n\n1. e4 e5 1-0\n";
    for comment in ["; this brace is ordinary line-comment text: {", "% note {"] {
        let pgn = format!("[Event \"Bad\"]\n\n1. Ke3 *\n{comment}\n\n{good}");
        assert_eq!(events(&pgn), ["error", "Good"], "{comment}");
        assert_eq!(events(&pgn.replace('\n', "\r\n")), ["error", "Good"]);
    }
}

#[test]
fn test_tag_pairs_may_span_lines() {
    for event in [
        "[Event\n \"Good\"]",
        "[Event \"Good\"\n]",
        "[\nEvent\n\"Good\"\n]",
    ] {
        let pgn = format!("{event}\n[Result \"1-0\"]\n\n1. e4 e5 1-0\n");
        for pgn in [pgn.clone(), pgn.replace('\n', "\r\n")] {
            let games = parse_pgn(&pgn).unwrap();
            assert_eq!(games.len(), 1, "{event:?}");
            assert_eq!(games[0].event, "Good", "{event:?}");
            assert_eq!(games[0].result, GameResult::White, "{event:?}");
        }
    }
}

#[test]
fn test_annotated_null_moves() {
    // Like SCID, `--+` is the symbol for a crushing advantage for Black
    // (NAG 21), not a null move.
    assert_eq!(
        parse_sans("1. Z0! null? 2. e4 --+ 3. -- #?! *"),
        ["--", "--", "e4", "--"]
    );
    assert_illegal("", "1. e4 e5 2. Qh5 Nc6 3. Qxf7+ --! *", "--!");
}

#[test]
fn test_unsupported_variant_after_many_tags() {
    let custom: String = (0..40).map(|i| format!("[Custom{i} \"x\"]\n")).collect();
    let variant = "[Variant \"Chess960\"]";
    for tags in [format!("{custom}{variant}"), format!("{variant}\n{custom}")] {
        let pgn = format!("[Event \"Test\"]\n{tags}\n\n1. e4 *\n");
        assert!(
            matches!(
                parse_single_game(&pgn),
                Err(Error::UnsupportedVariant { ref variant }) if variant == "Chess960"
            ),
            "{tags}"
        );
        let next = "[Event \"Next\"]\n\n1. d4 *\n";
        assert_eq!(events(&format!("{pgn}\n{next}")), ["error", "Next"]);
    }
}

#[test]
fn test_largest_fen_move_number() {
    let tags = "[FEN \"4k3/8/8/8/8/8/8/4K3 b - - 0 4294967295\"]";
    let game = parse_single_game(&pgn_game(tags, "4294967295... Kd7 4294967296. Ke2 *")).unwrap();
    assert_eq!(sans(&game), ["Kd7", "Ke2"]);
    assert!(
        game.to_pgn()
            .contains("\n4294967295... Kd7 4294967296. Ke2\n"),
        "{}",
        game.to_pgn()
    );
}

#[test]
fn test_delimiters_attached_to_moves() {
    for movetext in [
        "1. e4; comment\n e5 *",
        "1. e4$1 e5 *",
        "1. e4 e5*",
        "1. e4{c}e5 *",
    ] {
        assert_eq!(parse_sans(movetext), ["e4", "e5"], "{movetext:?}");
        let pgn = pgn_game("", movetext).replace('\n', "\r\n");
        assert_eq!(sans(&parse_single_game(&pgn).unwrap()), ["e4", "e5"]);
    }
    assert_eq!(parse_sans("1. e4*"), ["e4"]);
}

/// The main line's SAN, every NAG in it, the result, and the warnings,
/// without line numbers.
fn summary(game: &Game) -> String {
    let mut nags = Vec::new();
    let mut idx = game.first_move;
    while let Some(i) = idx {
        nags.extend(game.moves[i].nags.iter().map(|nag| format!("${nag}")));
        idx = game.moves[i].next;
    }
    let warnings: Vec<String> = game.warnings.iter().map(|w| w.kind.to_string()).collect();
    format!(
        "{} | {} | {} | {}",
        sans(game).join(" "),
        nags.join(" "),
        game.result,
        warnings.join("; ")
    )
}

/// The reason a game failed.
fn failure(error: &Error) -> String {
    match error {
        Error::PgnGame { source, .. } => failure(source),
        Error::PgnMove { san, source, .. } => format!("error: {san}: {source}"),
        other => format!("error: {other}"),
    }
}

/// Each game in `tests/data/notation.pgn` is a case in `docs/PGN_IMPORT.md`,
/// named by its `Event` tag. The document records how SCID reads the same
/// file.
#[test]
fn test_notation_fixture() {
    let castled = "\"O-O\" castles without the castling right";
    let expected = [
        ("A1 castling with zeros or lowercase", "e4 e5 Nf3 Nf6 Bc4 Bc5 O-O O-O |  | * | ".to_string()),
        ("A2 pawn prefix", "e4 e5 |  | * | ".into()),
        ("A3 missing or extra capture sign", "e4 d5 exd5 Qxd5 Nc3 |  | * | ".into()),
        ("A4 pawn coordinates", "e4 e5 |  | * | ".into()),
        ("A5 null moves", "e4 -- -- -- d4 |  | * | ".into()),
        ("A6 check and mate signs", "e4 e5 Bc4 Nc6 Qh5 Nf6 Qxf7# |  | * | ".into()),
        ("A7 move numbers without dots", "e4 e5 Nf3 |  | * | ".into()),
        ("A8 double check sign", "e4 f6 d4 g5 Qh5# |  | * | ".into()),
        ("1a pawn capture without a rank", "e4 d5 exd5 |  | * | ".into()),
        ("1b pawn capture without a rank, with x", "e4 d5 exd5 |  | * | ".into()),
        ("1c pawn capture without a rank, en passant", "exd6 |  | * | ".into()),
        ("2 ambiguous pawn capture without a rank", "error: ed: ambiguous san".into()),
        ("3 colon for a capture", "e4 d5 exd5 Qxd5 |  | * | ".into()),
        ("4 hyphen between squares", "e4 e5 Nf3 Nc6 |  | * | ".into()),
        ("5 coordinate piece moves", "Nf3 Nf6 |  | * | ".into()),
        ("6a king moves two squares", "e4 e5 Nf3 Nf6 Bc4 Bc5 O-O O-O |  | * | ".into()),
        ("6b king coordinate castling", "e4 e5 Nf3 Nf6 Bc4 Bc5 O-O O-O |  | * | ".into()),
        ("6c king moves two squares without the castling right", "error: Kg1: illegal san".into()),
        ("7a promotion with no piece", "a8=Q+ |  | * | ".into()),
        ("7b capture promotion with no piece", "axb8=Q+ |  | * | ".into()),
        ("7c coordinate promotion with no piece", "error: a7a8: illegal uci".into()),
        ("8a lowercase promotion piece", "a8=Q+ |  | * | ".into()),
        ("8b lowercase coordinate promotion", "a8=Q+ |  | * | ".into()),
        ("8c lowercase capture promotion without a rank", "axb8=N |  | * | ".into()),
        ("8d lowercase bishop promotion", "a8=B |  | * | ".into()),
        ("8e lowercase coordinate bishop promotion", "a8=B |  | * | ".into()),
        ("9 lowercase piece after equals sign", "a8=N |  | * | ".into()),
        ("10a capture promotion without a rank", "axb8=Q+ |  | * | ".into()),
        ("10b capture promotion without a rank or equals sign", "axb8=Q+ |  | * | ".into()),
        ("11a en passant marker attached", "exd6 |  | * | ".into()),
        ("11b en passant marker after the move", "exd6 |  | * | ".into()),
        ("12 lowercase piece letter", "e4 e5 |  | * | skipped unknown text \"nf3\"".into()),
        ("13 lowercase b for a bishop", "error: bc4: illegal san".into()),
        ("14 castling without dashes", "O-O-O O-O |  | * | ".into()),
        ("15 castling with zeros and no dashes", "O-O-O O-O |  | * | ".into()),
        (
            "16a castling after the king moved",
            format!("e4 e5 Nf3 Nf6 Bc4 Bc5 Ke2 Ke7 Ke1 Ke8 O-O O-O |  | * | {castled}; {castled}"),
        ),
        (
            "16b castling after the rook moved",
            format!("e4 e5 Nf3 Nf6 Bc4 Bc5 Rg1 Rg8 Rh1 Rh8 O-O |  | * | {castled}"),
        ),
        (
            "16c queenside castling without the right in the FEN",
            "O-O-O |  | * | \"O-O-O\" castles without the castling right".into(),
        ),
        (
            "16d castling without the right in a variation",
            format!("e4 e5 Nf3 Nf6 Bc4 Bc5 Ke2 Ke7 Ke1 Ke8 d3 d6 |  | * | {castled}"),
        ),
        ("18a castling with no rook", "error: O-O: illegal san".into()),
        ("18b queenside castling with no rook", "error: O-O-O: illegal san".into()),
        ("19 castling through check", "error: O-O: illegal san".into()),
        (
            "20a kingside castling when only queenside is legal",
            "O-O-O |  | * | read \"O-O\" as O-O-O because only queenside castling is legal".into(),
        ),
        ("20b queenside castling when only kingside is legal", "error: O-O-O: illegal san".into()),
        ("21a annotation symbols attached", "e4 e5 Nf3 Nc6 Bb5 a6 | $1 $2 $3 $4 $5 $6 | * | ".into()),
        (
            "21b annotation symbols after the move",
            "e4 e5 Nf3 Nc6 Bb5 a6 Ba4 Nf6 O-O Be7 | $1 $2 $14 $15 $16 $17 $18 $19 $10 $13 | * | ".into(),
        ),
        (
            "21c other annotation symbols",
            "e4 e5 Nf3 Nc6 Bb5 a6 Ba4 Nf6 | $14 $15 $20 $21 $40 $41 $44 $44 | * | ".into(),
        ),
        ("21d annotation symbols after check", "e4 e5 Bc4 Nc6 Qh5 Nf6 Qxf7# | $1 | * | ".into()),
        (
            "22a annotation symbols attached by equals or hyphen",
            "e4 e5 Nf3 Nc6 Bb5 a6 Ba4 Nf6 | $15 $10 $19 $17 | * | ".into(),
        ),
        ("22b several annotation symbols attached", "e4 e5 | $3 $18 | * | ".into()),
        (
            "22c symbols starting with two hyphens or an arrow attached",
            "e4 e5 Nf3 Nc6 | $21 $41 | * | ".into(),
        ),
        ("23 tilde attached", "e4 e5 | $13 | * | ".into()),
        ("24 letter symbols", "e4 e5 Nf3 Nc6 | $146 $201 $144 | * | ".into()),
        (
            "25a unknown symbols",
            "e4 e5 Nf3 Nc6 |  | * | skipped unknown text \"zz\"; skipped unknown text \"/\\\\\"; skipped unknown text \"Diagram\"".into(),
        ),
        ("25b piece letter on its own", "e4 e5 |  | * | skipped unknown text \"B\"".into()),
        (
            "25c word that starts like a null move",
            "e4 e5 |  | * | skipped unknown text \"nullify\"".into(),
        ),
        ("26a result 1:0", "e4 e5 |  | 1-0 | ".into()),
        ("26b result 0:1", "e4 e5 |  | 0-1 | ".into()),
        ("26c result 1/2", "e4 e5 |  | 1/2-1/2 | ".into()),
        ("26d result 1/2:1/2", "e4 e5 |  | 1/2-1/2 | ".into()),
        (
            "27a result differs from the tag",
            "e4 e5 |  | 0-1 | the result 0-1 after the moves replaces the Result tag 1-0".into(),
        ),
        (
            "27b unknown result with a result tag",
            "e4 e5 |  | * | the result * after the moves replaces the Result tag 1-0".into(),
        ),
        ("27c result with no result tag", "e4 e5 |  | 1-0 | ".into()),
        (
            "28a no result before the next game",
            "e4 e5 |  | 1-0 | the moves don't end with a result, so the Result tag 1-0 is used".into(),
        ),
        ("29 move that can't be read", "error: Nf6: illegal san".into()),
        ("30 SetUp without FEN", "e4 |  | * | ".into()),
        (
            "31a FEN castling rights that don't fit",
            "Kd1 |  | * | dropped castling rights \"KQkq\" from the FEN because they don't fit the position".into(),
        ),
        (
            "31b FEN en passant square that doesn't fit",
            "Kd1 |  | * | dropped en passant square e3 from the FEN because no pawn could have just moved past it".into(),
        ),
        ("31c FEN en passant square with no capture", "Kd7 |  | * | ".into()),
        ("32 Variant tag", "error: Unsupported variant: Chess960".into()),
        (
            "28b no result at the end of the input",
            "e4 e5 |  | 0-1 | the moves don't end with a result, so the Result tag 0-1 is used".into(),
        ),
    ];

    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/notation.pgn");
    let pgn = std::fs::read_to_string(path).unwrap();
    let events = pgn
        .lines()
        .filter_map(|line| line.strip_prefix("[Event \"")?.strip_suffix("\"]"));
    let actual: Vec<(String, String)> = events
        .zip(PgnParser::new(&pgn))
        .map(|(event, result)| {
            let outcome = match result {
                Ok(game) => {
                    assert_eq!(game.event, event);
                    summary(&game)
                }
                Err(e) => failure(&e),
            };
            (event.to_string(), outcome)
        })
        .collect();
    let expected: Vec<(String, String)> = expected
        .into_iter()
        .map(|(event, outcome)| (event.to_string(), outcome))
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn test_annotation_symbols_match_nags() {
    // Where a NAG attaches is #38; a symbol attaches like its `$n`.
    for (symbols, nags) in [
        (
            "1. e4! e5? 2. Nf3!! Nc6?? 3. Bb5!? a6?! *",
            "1. e4 $1 e5 $2 2. Nf3 $3 Nc6 $4 3. Bb5 $5 a6 $6 *",
        ),
        (
            "1. e4 += e5 =+ 2. Nf3 +/- Nc6 -/+ 3. Bb5 +- a6 -+ 4. Ba4 = Nf6 ~ *",
            "1. e4 $14 e5 $15 2. Nf3 $16 Nc6 $17 3. Bb5 $18 a6 $19 4. Ba4 $10 Nf6 $13 *",
        ),
        (
            "1. e4=+ e5 2. Nf3= Nc6 3. Bb5-+ *",
            "1. e4 $15 e5 2. Nf3 $10 Nc6 3. Bb5 $19 *",
        ),
        ("1. e4~ e5 2. Qh5!!+- *", "1. e4 $13 e5 2. Qh5 $3 $18 *"),
        (
            "1. e4 N e5 D 2. Nf3 R *",
            "1. e4 $146 e5 $201 2. Nf3 $144 *",
        ),
    ] {
        let read = |movetext| {
            let game = parse_single_game(&pgn_game("", movetext)).unwrap();
            let nags: Vec<Vec<u8>> = game.moves.iter().map(|node| node.nags.clone()).collect();
            (sans(&game), nags)
        };
        assert_eq!(read(symbols), read(nags), "{symbols}");
    }
}

#[test]
fn test_words_that_start_like_a_null_move_are_skipped() {
    let game = parse_single_game(&pgn_game("", "1. e4 nullify e5 2. Nf3 *")).unwrap();
    assert_eq!(sans(&game), ["e4", "e5", "Nf3"]);
    assert_eq!(
        warning_kinds(&game),
        [PgnWarningKind::UnknownText {
            text: "nullify".to_string()
        }]
    );
}

#[test]
fn test_warnings_name_the_game_and_line() {
    let pgn = "[Event \"First\"]\n\n1. e4 *\n\n[Event \"Second\"]\n\n1. e4 e5 2. Nf3 Nf6 3. Bc4 Bc5\n4. Ke2 Ke7 5. Ke1 Ke8\n6. O-O zz *\n";
    let games: Vec<Game> = PgnParser::new(pgn).map(Result::unwrap).collect();
    assert!(games[0].warnings.is_empty());
    let warnings: Vec<String> = games[1].warnings.iter().map(ToString::to_string).collect();
    assert_eq!(
        warnings,
        [
            "PGN game 2, line 9: \"O-O\" castles without the castling right",
            "PGN game 2, line 9: skipped unknown text \"zz\"",
        ]
    );

    // The FEN warning names the FEN tag's line.
    let pgn = "[Event \"x\"]\n[FEN \"4k3/8/8/8/8/8/8/4K3 w K - 0 1\"]\n\n1. Kd1 *\n";
    let game = parse_single_game(pgn).unwrap();
    assert_eq!(
        game.warnings[0].to_string(),
        "PGN game 1, line 2: dropped castling rights \"K\" from the FEN because they don't fit the position"
    );
}

#[test]
fn test_rejected_game_reports_only_its_error() {
    let pgn = "[Event \"x\"]\n\n1. e4 zz e5 2. Nf6 *\n\n[Event \"y\"]\n\n1. d4 *\n";
    let results: Vec<_> = PgnParser::new(pgn).collect();
    assert!(matches!(results[0], Err(Error::PgnGame { number: 1, .. })));
    let next = results[1].as_ref().unwrap();
    assert_eq!(next.event, "y");
    assert!(next.warnings.is_empty());
}
