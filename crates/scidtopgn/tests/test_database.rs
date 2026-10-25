use scidtopgn::{Database, Date, Game, GameResult};

fn get_test_data_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/data")
        .join(name)
}

#[test]
fn test_open_one_database() {
    let path = get_test_data_path("one");
    let db = Database::open(&path).unwrap();

    assert_eq!(db.num_games(), 1);
}

#[test]
fn test_get_game_from_one() {
    let path = get_test_data_path("one");
    let mut db = Database::open(&path).unwrap();

    let game = db.get_game(0).unwrap();

    assert_eq!(game.white, "Hossain, Enam");
    assert_eq!(game.black, "Murshed, N");
    assert_eq!(game.event, "47th ch-Bangahbandhu 2022");
    assert_eq!(game.site, "Dhaka BAN");
    assert_eq!(game.round, "5.6");
    assert_eq!(game.result, GameResult::Draw);

    assert_eq!(game.date.year(), 2022);
    assert_eq!(game.date.month(), 12);
    assert_eq!(game.date.day(), 19);

    assert_eq!(game.white_elo, Some(2372));
    assert_eq!(game.black_elo, Some(2419));
}

#[test]
fn test_game_to_pgn() {
    let path = get_test_data_path("one");
    let mut db = Database::open(&path).unwrap();

    let game = db.get_game(0).unwrap();
    let pgn = game.to_pgn();

    assert!(pgn.contains("[Event \"47th ch-Bangahbandhu 2022\"]"));
    assert!(pgn.contains("[Site \"Dhaka BAN\"]"));
    assert!(pgn.contains("[White \"Hossain, Enam\"]"));
    assert!(pgn.contains("[Black \"Murshed, N\"]"));
    assert!(pgn.contains("[Result \"1/2-1/2\"]"));
    assert!(pgn.contains("[WhiteElo \"2372\"]"));
    assert!(pgn.contains("[BlackElo \"2419\"]"));

    assert!(pgn.contains("1. e4"));
}

#[test]
fn test_open_five_database() {
    let path = get_test_data_path("five");
    let db = Database::open(&path).unwrap();

    assert_eq!(db.num_games(), 5);
}

#[test]
fn test_iterate_five_games() {
    let path = get_test_data_path("five");
    let mut db = Database::open(&path).unwrap();

    let mut count = 0;
    for result in db.games() {
        let game = result.unwrap();
        assert!(!game.white.is_empty());
        assert!(!game.black.is_empty());
        count += 1;
    }

    assert_eq!(count, 5);
}

#[test]
fn test_get_index_entry() {
    let path = get_test_data_path("one");
    let db = Database::open(&path).unwrap();

    let entry = db.get_entry(0).unwrap();

    assert!(entry.get_length() > 0);
    assert_eq!(entry.get_result(), GameResult::Draw);
}

#[test]
fn test_database_roundtrip() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let path = temp_dir.path().join("test_db");

    {
        let mut db = Database::create(&path).unwrap();

        let mut game = Game::new();
        game.white = "Player1, White".to_string();
        game.black = "Player2, Black".to_string();
        game.event = "Test Event".to_string();
        game.site = "Test Site".to_string();
        game.round = "1".to_string();
        game.date = Date::new(2024, 1, 15).unwrap();
        game.result = GameResult::White;
        game.white_elo = Some(2500);
        game.black_elo = Some(2400);

        db.add_game(&game).unwrap();
        db.flush().unwrap();
    }

    {
        let db = Database::open(&path).unwrap();
        assert_eq!(db.num_games(), 1);
    }

    {
        let mut db = Database::open(&path).unwrap();
        let game = db.get_game(0).unwrap();

        assert_eq!(game.white, "Player1, White");
        assert_eq!(game.black, "Player2, Black");
        assert_eq!(game.event, "Test Event");
        assert_eq!(game.site, "Test Site");
        assert_eq!(game.round, "1");
        assert_eq!(game.date.year(), 2024);
        assert_eq!(game.date.month(), 1);
        assert_eq!(game.date.day(), 15);
        assert_eq!(game.result, GameResult::White);
        assert_eq!(game.white_elo, Some(2500));
        assert_eq!(game.black_elo, Some(2400));
    }
}

#[test]
fn test_open_and_close_without_changes() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = temp.path().join("five");
    let read = |ext: &str| std::fs::read(path.with_extension(ext)).unwrap();
    for ext in ["si4", "sn4", "sg4"] {
        std::fs::copy(
            get_test_data_path("five").with_extension(ext),
            path.with_extension(ext),
        )
        .unwrap();
    }
    let before: Vec<_> = ["si4", "sn4", "sg4"].map(read).into();

    let db = Database::open(&path).unwrap();
    assert_eq!(db.path(), path);
    assert_eq!(db.index().num_games(), 5);
    assert!(db.nfile().namebase().count(scidtopgn::NameType::Player) > 0);
    assert_eq!(db.gfile().file_size(), before[2].len() as u64);
    db.close().unwrap();

    // Nothing changed, so nothing is written.
    let after: Vec<_> = ["si4", "sn4", "sg4"].map(read).into();
    assert_eq!(after, before);
}

/// `game` written to a new database and read back.
fn scid_round_trip(game: &Game) -> Game {
    let temp = tempfile::TempDir::new().unwrap();
    let path = temp.path().join("round_trip");
    let mut db = Database::create(&path).unwrap();
    db.add_game(game).unwrap();
    db.close().unwrap();
    Database::open(&path).unwrap().get_game(0).unwrap()
}

/// The movetext `pgn` exports, after its tags.
fn movetext(pgn: &str) -> &str {
    pgn.split_once("\n\n").unwrap().1
}

#[test]
fn test_games_without_moves_round_trip() {
    let fen = "[SetUp \"1\"]\n[FEN \"4k3/8/8/8/8/8/8/4K3 w - - 0 1\"]\n";
    for (tags, moves, result, exported) in [
        ("", "*", "*", "*\n"),
        ("", "{only a comment} 1-0", "1-0", "{only a comment}\n1-0\n"),
        (fen, "1/2-1/2", "1/2-1/2", "1/2-1/2\n"),
    ] {
        let pgn = format!("[Event \"x\"]\n[Result \"{result}\"]\n{tags}\n{moves}\n");
        let game = scidtopgn::parse_single_game(&pgn).unwrap();
        assert!(game.warnings.is_empty(), "{pgn}");
        let pgn = scid_round_trip(&game).to_pgn();
        assert!(pgn.contains(&format!("[Result \"{result}\"]\n")), "{pgn}");
        assert!(pgn.contains(tags), "{pgn}");
        assert_eq!(movetext(&pgn), exported, "{pgn}");

        let reread = scidtopgn::parse_single_game(&pgn).unwrap();
        assert_eq!(scid_round_trip(&reread).to_pgn(), pgn);
    }
}

/// The comments of `game`'s nodes, in order, without the empty ones.
fn comments(game: &Game) -> Vec<String> {
    game.moves
        .iter()
        .filter_map(|node| node.comment.clone())
        .collect()
}

#[test]
fn test_comment_text_round_trips() {
    // Comments with one space between words read the same in SCID.
    let pgn = "[Event \"x\"]\n\n1. e4 {é 漢字 😀 \" \\ [ ] ( ) ; $1} e5 \
               (1... c5 {变化 \"q\" \\ [x] (y) ; $2}) 2. Nf3 {ends with \\} *\n";
    let expected = [
        "é 漢字 😀 \" \\ [ ] ( ) ; $1",
        "变化 \"q\" \\ [x] (y) ; $2",
        "ends with \\",
    ];
    let game = scidtopgn::parse_single_game(pgn).unwrap();
    assert!(game.warnings.is_empty(), "{:?}", game.warnings);
    assert_eq!(comments(&game), expected, "after PGN import");

    let reread = scid_round_trip(&game);
    assert_eq!(comments(&reread), expected, "after a SCID round trip");
    let exported = reread.to_pgn();
    let reimported = scidtopgn::parse_single_game(&exported).unwrap();
    assert_eq!(comments(&reimported), expected, "after PGN to SCID to PGN");
    assert_eq!(reimported.moves.len(), game.moves.len());
}

#[test]
fn test_comments_keep_their_whitespace_in_the_database() {
    let mut game = scidtopgn::parse_single_game("[Event \"x\"]\n\n1. e4 e5 *\n").unwrap();
    let multi_line = "line one\nline two\n\n  indented\ttab ";
    let long = "x".repeat(60_000);
    game.moves[1].comment = Some(multi_line.to_string());
    game.moves[2].comment = Some(long.clone());

    let reread = scid_round_trip(&game);
    assert_eq!(comments(&reread), [multi_line, long.as_str()]);
}

#[test]
fn test_a_comment_with_a_tag_line_stays_in_its_game() {
    let pgn = "[Event \"a\"]\n\n1. e4 {note\n\n[Event \"b\"]\n} e5 *\n";
    let games: Vec<_> = scidtopgn::PgnParser::new(pgn).collect();
    assert_eq!(games.len(), 1);
    let game = games[0].as_ref().unwrap();
    assert_eq!(game.event, "a");
    assert_eq!(game.moves.len(), 3);
    assert!(comments(game)[0].contains("[Event \"b\"]"));
}

#[test]
fn test_many_nags_round_trip() {
    // 300 half-moves, each with 1 to 7 NAGs, at most what SCID keeps on
    // one move.
    let nags = |ply: usize| -> Vec<u8> {
        (0..=ply % 7)
            .map(|k| ((ply * 7 + k) % 255 + 1) as u8)
            .collect()
    };
    let mut movetext = String::new();
    for (ply, san) in ["Nf3", "Nf6", "Ng1", "Ng8"].repeat(75).iter().enumerate() {
        movetext.push_str(san);
        for nag in nags(ply) {
            movetext.push_str(&format!(" ${nag}"));
        }
        movetext.push(' ');
    }
    let pgn = format!("[Event \"x\"]\n\n{movetext}*\n");
    let game = scidtopgn::parse_single_game(&pgn).unwrap();
    assert_eq!(game.num_half_moves, 300);

    let main_line_nags = |game: &Game| -> Vec<Vec<u8>> {
        game.moves[1..]
            .iter()
            .map(|node| node.nags.clone())
            .collect()
    };
    let expected: Vec<_> = (0..300).map(nags).collect();
    assert_eq!(main_line_nags(&game), expected, "after PGN import");

    let reread = scid_round_trip(&game);
    assert_eq!(main_line_nags(&reread), expected, "after a SCID round trip");
    let reimported = scidtopgn::parse_single_game(&reread.to_pgn()).unwrap();
    assert_eq!(
        main_line_nags(&reimported),
        expected,
        "after PGN to SCID to PGN"
    );
}
