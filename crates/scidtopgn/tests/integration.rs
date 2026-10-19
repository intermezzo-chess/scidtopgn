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

#[test]
fn test_roundtrip() {
    let workspace = get_workspace_root();
    let db1_path = format!("{}/tests/data/one", workspace);

    let mut db1 = scidtopgn::Database::open(&db1_path).expect("Failed to open database");
    let game1 = db1.get_game(0).expect("Failed to get game");

    let temp = TempDir::new().expect("Failed to create temp dir");
    let db2_path = temp.path().join("test");

    let mut db2 = scidtopgn::Database::create(&db2_path).expect("Failed to create database");
    db2.add_game(&game1).expect("Failed to add game");
    db2.flush().expect("Failed to flush");

    let mut db3 = scidtopgn::Database::open(&db2_path).expect("Failed to open roundtrip database");
    let game2 = db3.get_game(0).expect("Failed to get roundtrip game");

    assert_eq!(game1.white, game2.white);
    assert_eq!(game1.black, game2.black);
    assert_eq!(game1.event, game2.event);
    assert_eq!(game1.result, game2.result);
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
