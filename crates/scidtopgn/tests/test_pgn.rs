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
use scidtopgn::{Error, PgnParser};

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
    // The king has moved.
    assert_illegal(
        "",
        "1. e4 e5 2. Nf3 Nf6 3. Bc4 Bc5 4. Ke2 Ke7 5. Ke1 Ke8 6. O-O *",
        "O-O",
    );
    // The rook has moved.
    assert_illegal(
        "",
        "1. e4 e5 2. Nf3 Nf6 3. Bc4 Bc5 4. Rg1 Rg8 5. Rh1 Rh8 6. O-O *",
        "O-O",
    );
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
    assert_eq!(
        parse_sans("1. Z0! null? 2. e4 --+ 3. -- #?! *"),
        ["--", "--", "e4", "--", "--"]
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
