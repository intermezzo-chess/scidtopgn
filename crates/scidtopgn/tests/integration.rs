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

/// The PGN of one game with the given names, whose moves differ from every
/// other helper game's, so that games never collapse into twins.
fn game_pgn(event: &str, white: &str, black: &str, moves: &str) -> String {
    format!(
        "[Event \"{event}\"]\n[Site \"Bonn\"]\n[Date \"2022.12.24\"]\n[Round \"1\"]\n\
         [White \"{white}\"]\n[Black \"{black}\"]\n[Result \"*\"]\n\n{moves} *\n"
    )
}

/// Imports `pgn` into a new database at `path`, as `scidtopgn import` does
/// through the library's API.
fn import(path: &std::path::Path, pgn: &str) -> scidtopgn::Database {
    let mut db = scidtopgn::Database::create(path).expect("Failed to create database");
    for result in scidtopgn::PgnParser::new(pgn) {
        let game = result.expect("Failed to parse game");
        db.add_game(&game).expect("Failed to add game");
    }
    db.flush().expect("Failed to flush");
    db
}

/// Asserts that games `nums` have the given `event`, `white`, and `black`.
fn assert_names(db: &mut scidtopgn::Database, expected: &[(u32, &str, &str, &str)]) {
    for (num, event, white, black) in expected {
        let game = db.get_game(*num).expect("Failed to get game");
        assert_eq!(game.event, *event, "game {num} event");
        assert_eq!(game.white, *white, "game {num} white");
        assert_eq!(game.black, *black, "game {num} black");
    }
}

#[test]
fn test_import_keeps_control_characters_in_names_through_writes() {
    // Issue #142: an event with a C0 control character, as in Caissabase's
    // `M\x01nster Open`. Import stores it, an unrelated append leaves it,
    // export writes it as SCID's converter does, and reimporting that PGN
    // gives the same names.
    let pgn = game_pgn(
        "M\u{1}nster Open",
        "M\u{1}nster, Open",
        "Anand, Viswanathan",
        "1. e4 e5",
    );
    let temp = TempDir::new().expect("Failed to create temp dir");
    let mut db = import(&temp.path().join("db"), &pgn);

    assert_eq!(db.name_count(scidtopgn::NameType::Event), 1);
    assert_names(
        &mut db,
        &[(
            0,
            "M\u{1}nster Open",
            "M\u{1}nster, Open",
            "Anand, Viswanathan",
        )],
    );

    // Appending a game with no shared names changes no stored name.
    let append = game_pgn(
        "Winter Open",
        "Carlsen, Magnus",
        "Caruana, Fabiano",
        "1. d4 d5",
    );
    let game = scidtopgn::parse_single_game(&append).expect("Failed to parse append");
    db.add_game(&game).expect("Failed to append");
    db.flush().expect("Failed to flush after append");
    assert_eq!(db.name_count(scidtopgn::NameType::Event), 2);
    assert_names(
        &mut db,
        &[
            (
                0,
                "M\u{1}nster Open",
                "M\u{1}nster, Open",
                "Anand, Viswanathan",
            ),
            (1, "Winter Open", "Carlsen, Magnus", "Caruana, Fabiano"),
        ],
    );

    // Export writes the control character, byte for byte as SCID's
    // reference converter does.
    let pgn_export: String = db
        .games()
        .map(|result| result.expect("Failed to read game").to_pgn())
        .collect();
    let event_line = format!("[Event \"{}\"]", "M\u{1}nster Open");
    assert!(
        pgn_export
            .lines()
            .any(|line| line.as_bytes() == event_line.as_bytes()),
        "export should have the tag line {event_line:?}: {pgn_export:?}"
    );

    // Importing the export gives the same names again.
    let temp2 = TempDir::new().expect("Failed to create second temp dir");
    let mut db2 = import(&temp2.path().join("db2"), &pgn_export);
    assert_eq!(db2.name_count(scidtopgn::NameType::Event), 2);
    assert_names(
        &mut db2,
        &[
            (
                0,
                "M\u{1}nster Open",
                "M\u{1}nster, Open",
                "Anand, Viswanathan",
            ),
            (1, "Winter Open", "Carlsen, Magnus", "Caruana, Fabiano"),
        ],
    );
}

#[test]
fn test_names_differing_only_by_control_characters_stay_separate_through_writes() {
    // Issue #142: two events that differ only by a control character are two
    // names, as in SCID, and no append, update, or delete merges them.
    let pgn = [
        game_pgn("M\u{1}nster Open", "Adam, Zac", "Anand, V", "1. e4 e5"),
        game_pgn("Mnster Open", "Berger, Vera", "Carlsen, M", "1. d4 d5"),
    ]
    .concat();
    let temp = TempDir::new().expect("Failed to create temp dir");
    let mut db = import(&temp.path().join("db"), &pgn);
    assert_eq!(db.name_count(scidtopgn::NameType::Event), 2);

    let append = game_pgn("Winter Open", "Carlsen, Magnus", "Caruana, F", "1. c4 e5");
    let game = scidtopgn::parse_single_game(&append).expect("Failed to parse append");
    db.add_game(&game).expect("Failed to append");
    db.flush().expect("Failed to flush after append");
    assert_eq!(db.name_count(scidtopgn::NameType::Event), 3);

    // Updating game 0 with the same game changes no name.
    let same = scidtopgn::parse_single_game(&pgn).expect("Failed to parse update");
    db.update_game(0, &same).expect("Failed to update");
    db.flush().expect("Failed to flush after update");
    assert_eq!(db.name_count(scidtopgn::NameType::Event), 3);
    assert_names(
        &mut db,
        &[
            (0, "M\u{1}nster Open", "Adam, Zac", "Anand, V"),
            (1, "Mnster Open", "Berger, Vera", "Carlsen, M"),
            (2, "Winter Open", "Carlsen, Magnus", "Caruana, F"),
        ],
    );

    // Deleting the appended game leaves both events.
    db.delete_game(2).expect("Failed to delete");
    db.flush().expect("Failed to flush after delete");
    // The deleted game's names stay, unused, as SCID keeps them until a
    // compact, and neither event name was merged away.
    assert_eq!(db.name_count(scidtopgn::NameType::Event), 3);
    assert_names(
        &mut db,
        &[
            (0, "M\u{1}nster Open", "Adam, Zac", "Anand, V"),
            (1, "Mnster Open", "Berger, Vera", "Carlsen, M"),
        ],
    );
}

#[test]
fn test_control_character_name_shapes_survive_database_writes() {
    // The four shapes #142 requires: a C0 control character inside a name,
    // a tab, a name that is only control characters, and a control
    // character at the start or end of a name. Import trims spaces at the
    // edges of a player name and tabs and spaces at its end, as SCID does,
    // and keeps every control character.
    let pgn = [
        game_pgn(
            "M\u{1}nster Open",
            "A\u{1}dam, Z",
            "B\u{9}ob, Y",
            "1. e4 e5",
        ),
        game_pgn(
            "Mnster\tOpen",
            " \u{1}Space, L",
            "Trail\u{3}, T",
            "1. d4 d5",
        ),
        game_pgn("\u{1}\u{2}", "\u{1}\u{2}", "Black, B", "1. c4 e5"),
        game_pgn(
            "\u{1}Lead\u{3}",
            "\u{1}Lead, L",
            "T, Trail\u{3}",
            "1. Nf3 d5",
        ),
    ]
    .concat();
    let temp = TempDir::new().expect("Failed to create temp dir");
    let mut db = import(&temp.path().join("db"), &pgn);

    let expected = [
        (0, "M\u{1}nster Open", "A\u{1}dam, Z", "B\u{9}ob, Y"),
        // The player's leading space is tidied away; the event's tab is not.
        (1, "Mnster\tOpen", "\u{1}Space, L", "Trail\u{3}, T"),
        (2, "\u{1}\u{2}", "\u{1}\u{2}", "Black, B"),
        (3, "\u{1}Lead\u{3}", "\u{1}Lead, L", "T, Trail\u{3}"),
    ];
    assert_eq!(db.name_count(scidtopgn::NameType::Event), 4);
    assert_names(&mut db, &expected);

    // Append, update, and flush, and delete, change no stored name.
    let append = game_pgn("Winter Open", "Carlsen, Magnus", "Caruana, F", "1. g3 d5");
    let game = scidtopgn::parse_single_game(&append).expect("Failed to parse append");
    db.add_game(&game).expect("Failed to append");
    db.flush().expect("Failed to flush after append");
    let same = scidtopgn::parse_single_game(&pgn).expect("Failed to parse update");
    db.update_game(0, &same).expect("Failed to update");
    db.flush().expect("Failed to flush after update");
    db.delete_game(4).expect("Failed to delete");
    db.flush().expect("Failed to flush after delete");
    assert_eq!(db.name_count(scidtopgn::NameType::Event), 5);
    assert_names(&mut db, &expected);
}
