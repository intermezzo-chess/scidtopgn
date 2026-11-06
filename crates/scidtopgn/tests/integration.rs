use tempfile::TempDir;

fn get_workspace_root() -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crate should be two levels below the workspace root")
        .to_string_lossy()
        .into_owned()
}

#[test]
fn test_one_database() {
    let workspace = get_workspace_root();
    let db_path = format!("{}/tests/data/one", workspace);

    let mut db = scidtopgn::Database::open(&db_path).expect("Failed to open database");
    assert_eq!(db.num_games(), 1);

    let game = db.get_game(0).expect("Failed to get game");
    assert_eq!(game.white, "Hossain, Enam");
    assert_eq!(game.black, "Murshed, N");
    assert_eq!(game.event, "47th ch-Bangahbandhu 2022");
    assert_eq!(game.result, scidtopgn::GameResult::Draw);

    let pgn = game.to_pgn();
    assert!(pgn.contains("[Event \"47th ch-Bangahbandhu 2022\"]"));
    assert!(pgn.contains("[White \"Hossain, Enam\"]"));
    assert!(pgn.contains("[Black \"Murshed, N\"]"));
    assert!(pgn.contains("[Result \"1/2-1/2\"]"));
}

#[test]
fn test_five_database() {
    let workspace = get_workspace_root();
    let db_path = format!("{}/tests/data/five", workspace);

    let mut db = scidtopgn::Database::open(&db_path).expect("Failed to open database");
    assert_eq!(db.num_games(), 5);

    for i in 0..5 {
        let game = db
            .get_game(i)
            .unwrap_or_else(|_| panic!("Failed to get game {}", i));
        let pgn = game.to_pgn();
        assert!(!pgn.is_empty(), "Game {} PGN should not be empty", i);
        assert!(
            pgn.contains("[Event \""),
            "Game {} should have Event tag",
            i
        );
    }
}

/// Asserts that `after` has the tags, moves, and annotations of `before`.
fn assert_same_game(before: &scidtopgn::Game, after: &scidtopgn::Game) {
    assert_eq!(after.white, before.white);
    assert_eq!(after.black, before.black);
    assert_eq!(after.event, before.event);
    assert_eq!(after.site, before.site);
    assert_eq!(after.round, before.round);
    assert_eq!(after.date, before.date);
    assert_eq!(after.event_date, before.event_date);
    assert_eq!(after.result, before.result);
    assert_eq!(after.white_elo, before.white_elo);
    assert_eq!(after.black_elo, before.black_elo);
    assert_eq!(after.white_rating_type, before.white_rating_type);
    assert_eq!(after.black_rating_type, before.black_rating_type);
    assert_eq!(after.eco, before.eco);
    assert_eq!(after.num_half_moves, before.num_half_moves);

    let tags = |game: &scidtopgn::Game| -> Vec<(String, String)> {
        game.tags
            .iter()
            .map(|tag| (tag.name.clone(), tag.value.clone()))
            .collect()
    };
    assert_eq!(tags(after), tags(before));

    let nodes = |game: &scidtopgn::Game| -> Vec<_> {
        game.moves
            .iter()
            .map(|node| {
                (
                    node.move_data,
                    node.san.clone(),
                    node.comment.clone(),
                    node.nags.clone(),
                    node.marker,
                    node.num_variations,
                )
            })
            .collect()
    };
    assert_eq!(nodes(after), nodes(before));

    assert_eq!(after.to_pgn(), before.to_pgn());
}

#[test]
fn test_roundtrip() {
    let workspace = get_workspace_root();
    // Text from each fixture's games, so that the games compared have them.
    for (name, count, expected) in [
        (
            "one",
            1,
            &[
                "[White \"Hossain, Enam\"]",
                "[Result \"1/2-1/2\"]",
                "[WhiteElo \"2372\"]",
                "[ECO \"B36f\"]",
                "19. Nxe6",
            ][..],
        ),
        (
            "annotated",
            2,
            &[
                "{Intro}",
                "{after e5}",
                "(2. f4 !? {gambit}",
                "[FEN \"r1bqkbnr/pppp1ppp/2n5/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3\"]",
                "3... a6 {Morphy}",
            ],
        ),
    ] {
        let db1_path = format!("{workspace}/tests/data/{name}");
        let mut db1 =
            scidtopgn::Database::open_read_only(&db1_path).expect("Failed to open database");
        let games: Vec<_> = db1
            .games()
            .collect::<Result<_, _>>()
            .expect("Failed to read games");
        assert_eq!(games.len(), count, "{name}");

        let temp = TempDir::new().expect("Failed to create temp dir");
        let db2_path = temp.path().join("test");
        let mut db2 = scidtopgn::Database::create(&db2_path).expect("Failed to create database");
        for game in &games {
            db2.add_game(game).expect("Failed to add game");
        }
        db2.flush().expect("Failed to flush");

        let mut db3 =
            scidtopgn::Database::open(&db2_path).expect("Failed to open roundtrip database");
        let reread: Vec<_> = db3
            .games()
            .collect::<Result<_, _>>()
            .expect("Failed to read roundtrip games");
        assert_eq!(reread.len(), count, "{name}");

        for (before, after) in games.iter().zip(&reread) {
            assert_same_game(before, after);
        }
        let pgn: String = reread.iter().map(|game| game.to_pgn()).collect();
        for text in expected {
            assert!(pgn.contains(text), "{name} should have {text}: {pgn}");
        }
    }
}

#[test]
fn test_eco_decoding() {
    let workspace = get_workspace_root();
    let db_path = format!("{}/tests/data/one", workspace);

    let mut db = scidtopgn::Database::open(&db_path).expect("Failed to open database");
    let game = db.get_game(0).expect("Failed to get game");

    assert!(game.eco.is_some(), "ECO should be set");
    assert_eq!(game.eco.as_ref().unwrap(), "B36f");
}

#[test]
fn test_check_symbol_in_moves() {
    let workspace = get_workspace_root();
    let db_path = format!("{}/tests/data/one", workspace);

    let mut db = scidtopgn::Database::open(&db_path).expect("Failed to open database");
    let game = db.get_game(0).expect("Failed to get game");
    let pgn = game.to_pgn();

    assert!(
        pgn.contains("Qxd2+"),
        "PGN should contain check symbol: {}",
        pgn
    );
}
