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
