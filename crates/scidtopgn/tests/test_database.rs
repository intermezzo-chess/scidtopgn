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
