use scidtopgn::{Database, Date, Error, Game, GameResult, Index, NameType};

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

/// Copies `five` into `dir` and sets the logical delete flag on the `deleted`
/// games, leaving every record in place.
fn five_with_deleted(dir: &std::path::Path, deleted: &[u32]) -> std::path::PathBuf {
    let src = get_test_data_path("five");
    let dst = dir.join("five");
    for ext in ["si4", "sn4", "sg4"] {
        std::fs::copy(src.with_extension(ext), dst.with_extension(ext)).unwrap();
    }

    let si4 = dst.with_extension("si4");
    let mut index = Index::open(&si4).unwrap();
    for &n in deleted {
        index.get_entry_mut(n).unwrap().set_delete_flag(true);
    }
    index
        .flush(&mut std::fs::File::create(&si4).unwrap())
        .unwrap();
    dst
}

fn all_pgn(db: &mut Database) -> Vec<String> {
    db.games().map(|game| game.unwrap().to_pgn()).collect()
}

/// The PGN of the games that aren't logically deleted, in order.
fn nondeleted_pgn(db: &mut Database) -> Vec<String> {
    let numbers: Vec<u32> = (0..db.num_games())
        .filter(|&n| db.get_entry(n).is_some_and(|e| !e.get_delete_flag()))
        .collect();
    numbers
        .into_iter()
        .map(|n| db.get_game(n).unwrap().to_pgn())
        .collect()
}

#[test]
fn test_all_record_api_includes_deleted_games() {
    let stored = all_pgn(&mut Database::open(get_test_data_path("five")).unwrap());
    let temp = tempfile::TempDir::new().unwrap();
    let mut db = Database::open(five_with_deleted(temp.path(), &[1, 3])).unwrap();

    let flags: Vec<_> = (0..5)
        .map(|n| db.get_entry(n).unwrap().get_delete_flag())
        .collect();
    assert_eq!(flags, [false, true, false, true, false]);

    assert_eq!(db.num_games(), 5, "num_games counts deleted games");
    assert_eq!(all_pgn(&mut db), stored, "games() yields deleted games");
    assert_eq!(db.get_game(3).unwrap().to_pgn(), stored[3]);

    assert_eq!(
        nondeleted_pgn(&mut db),
        [stored[0].clone(), stored[2].clone(), stored[4].clone()],
        "filtering keeps the other games in order"
    );
}

#[test]
fn test_all_deleted_database_reads_every_record() {
    let stored = all_pgn(&mut Database::open(get_test_data_path("five")).unwrap());
    let temp = tempfile::TempDir::new().unwrap();
    let mut db = Database::open(five_with_deleted(temp.path(), &[0, 1, 2, 3, 4])).unwrap();

    assert_eq!(db.num_games(), 5);
    assert_eq!(all_pgn(&mut db), stored);
    assert!(nondeleted_pgn(&mut db).is_empty());
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

/// Asserts that the database at `path` opens with no games and no names.
fn assert_opens_empty(path: &std::path::Path) {
    let db = Database::open(path).unwrap();
    assert_eq!(db.num_games(), 0);
    for ntype in [
        NameType::Player,
        NameType::Event,
        NameType::Site,
        NameType::Round,
    ] {
        assert_eq!(db.nfile().namebase().count(ntype), 0, "{ntype:?}");
    }
}

#[test]
fn test_create_without_games_opens_empty() {
    let temp = tempfile::TempDir::new().unwrap();

    let closed = temp.path().join("closed");
    Database::create(&closed).unwrap().close().unwrap();
    assert_opens_empty(&closed);

    let flushed = temp.path().join("flushed");
    let mut db = Database::create(&flushed).unwrap();
    db.flush().unwrap();
    assert_opens_empty(&flushed);
    drop(db);

    let dropped = temp.path().join("dropped");
    drop(Database::create(&dropped).unwrap());
    assert_opens_empty(&dropped);
}

#[test]
fn test_create_replaces_every_file_of_an_existing_database() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = temp.path().join("five");
    for ext in ["si4", "sn4", "sg4"] {
        std::fs::copy(
            get_test_data_path("five").with_extension(ext),
            path.with_extension(ext),
        )
        .unwrap();
    }

    drop(Database::create(&path).unwrap());

    assert_opens_empty(&path);
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
    game.moves[1].comment = Some(multi_line.to_string());

    let reread = scid_round_trip(&game);
    assert_eq!(comments(&reread), [multi_line]);
}

/// The SAN and comment of each of `game`'s moves that has a comment.
fn move_comments(game: &Game) -> Vec<(String, String)> {
    game.moves
        .iter()
        .filter_map(|node| Some((node.san.clone(), node.comment.clone()?)))
        .collect()
}

#[test]
fn test_long_comment_round_trips() {
    let long = "x".repeat(60_000);
    for (movetext, san) in [
        (format!("1. e4 {{{long}}} e5 *"), "e4"),
        (format!("1. e4 e5 (1... c5 {{{long}}}) *"), "c5"),
    ] {
        let expected = [(san.to_string(), long.clone())];
        let pgn = format!("[Event \"x\"]\n\n{movetext}\n");
        let game = scidtopgn::parse_single_game(&pgn).unwrap();
        assert_eq!(move_comments(&game), expected, "{san}: after PGN import");

        let reread = scid_round_trip(&game);
        assert_eq!(
            move_comments(&reread),
            expected,
            "{san}: after a SCID round trip"
        );
        let reimported = scidtopgn::parse_single_game(&reread.to_pgn()).unwrap();
        assert_eq!(
            move_comments(&reimported),
            expected,
            "{san}: after PGN to SCID to PGN"
        );
        assert_eq!(reimported.moves.len(), game.moves.len(), "{san}");
    }
}

/// A game whose SCID record is `length` bytes long.
fn game_with_record_length(length: usize) -> Game {
    let game = |comment: usize| {
        let comment = "x".repeat(comment);
        scidtopgn::parse_single_game(&format!("[Event \"x\"]\n\n1. e4 {{{comment}}} *\n")).unwrap()
    };
    let temp = tempfile::TempDir::new().unwrap();
    let mut db = Database::create(temp.path().join("measure")).unwrap();
    db.add_game(&game(1)).unwrap();
    let rest = db.get_entry(0).unwrap().get_length() as usize - 1;
    game(length - rest)
}

#[test]
fn test_records_up_to_the_longest_the_index_can_record_round_trip() {
    for length in [65_535, 65_536, 131_071] {
        let game = game_with_record_length(length);
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("long");
        let mut db = Database::create(&path).unwrap();
        db.add_game(&game).unwrap();
        db.close().unwrap();

        let mut db = Database::open(&path).unwrap();
        assert_eq!(db.get_entry(0).unwrap().get_length() as usize, length);
        let reread = db.get_game(0).unwrap();
        assert_eq!(reread.to_pgn(), game.to_pgn(), "{length}");
    }

    let temp = tempfile::TempDir::new().unwrap();
    let mut db = Database::create(temp.path().join("too_long")).unwrap();
    assert!(matches!(
        db.add_game(&game_with_record_length(131_072)),
        Err(scidtopgn::Error::GameTooLarge {
            length: 131_072,
            max: 131_071
        })
    ));
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

/// The bytes of the files of the database at `path`.
fn database_bytes(path: &std::path::Path) -> Vec<Vec<u8>> {
    ["si4", "sn4", "sg4"]
        .map(|ext| std::fs::read(path.with_extension(ext)).unwrap())
        .into()
}

/// Every name of every kind, with its ID and frequency.
fn all_names(db: &Database) -> Vec<Vec<(u32, String, u32)>> {
    let nb = db.nfile().namebase();
    [
        NameType::Player,
        NameType::Event,
        NameType::Site,
        NameType::Round,
    ]
    .map(|ntype| nb.get_all_names(ntype))
    .into()
}

/// The frequency of `name`, which must be in the name file.
fn frequency(db: &Database, ntype: NameType, name: &str) -> u32 {
    let nb = db.nfile().namebase();
    nb.get_frequency(ntype, nb.find_name(ntype, name).unwrap())
}

/// The name frequencies of `game`'s White, Black, event, site, and round.
fn name_frequencies(db: &Database, game: &Game) -> [u32; 5] {
    [
        frequency(db, NameType::Player, &game.white),
        frequency(db, NameType::Player, &game.black),
        frequency(db, NameType::Event, &game.event),
        frequency(db, NameType::Site, &game.site),
        frequency(db, NameType::Round, &game.round),
    ]
}

/// The bytes of game `n`'s index entry.
fn entry_bytes(db: &Database, n: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    db.get_entry(n).unwrap().write(&mut bytes).unwrap();
    bytes
}

fn delete_flags(db: &Database) -> Vec<bool> {
    (0..db.num_games())
        .map(|n| db.get_entry(n).unwrap().get_delete_flag())
        .collect()
}

const NEW_GAME: &str = "[Event \"Rapid\"]\n[Site \"Oslo\"]\n[Date \"2024.05.01\"]\n\
    [Round \"7\"]\n[White \"Kasparov, Garry\"]\n[Black \"Karpov, Anatoly\"]\n\
    [Result \"1-0\"]\n[WhiteElo \"2800\"]\n[ECO \"A10\"]\n\n1. c4 e5 1-0\n";

#[test]
fn test_delete_and_undelete_game() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = five_with_deleted(temp.path(), &[]);
    let mut db = Database::open(&path).unwrap();
    let stored = all_pgn(&mut db);
    let names = all_names(&db);

    db.delete_game(1).unwrap();
    db.delete_game(3).unwrap();
    assert_eq!(delete_flags(&db), [false, true, false, true, false]);
    db.close().unwrap();

    let mut db = Database::open(&path).unwrap();
    assert_eq!(delete_flags(&db), [false, true, false, true, false]);
    assert_eq!(db.num_games(), 5);
    assert_eq!(all_pgn(&mut db), stored, "records and numbers are kept");
    assert_eq!(all_names(&db), names, "name frequencies don't change");

    db.undelete_game(1).unwrap();
    db.close().unwrap();
    let db = Database::open(&path).unwrap();
    assert_eq!(delete_flags(&db), [false, false, false, true, false]);
}

#[test]
fn test_deleting_a_deleted_game_changes_nothing() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = five_with_deleted(temp.path(), &[1]);
    let before = database_bytes(&path);

    let mut db = Database::open(&path).unwrap();
    db.delete_game(1).unwrap();
    db.undelete_game(0).unwrap();
    assert_eq!(delete_flags(&db), [false, true, false, false, false]);
    db.close().unwrap();

    assert_eq!(database_bytes(&path), before);
}

#[test]
fn test_changing_a_game_past_the_last() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = five_with_deleted(temp.path(), &[]);
    let before = database_bytes(&path);
    let game = scidtopgn::parse_single_game(NEW_GAME).unwrap();

    let mut db = Database::open(&path).unwrap();
    let not_found = |result| matches!(result, Err(Error::GameNotFound { number: 6 }));
    assert!(not_found(db.delete_game(5)));
    assert!(not_found(db.undelete_game(5)));
    assert!(not_found(db.update_game(5, &game)));
    assert!(matches!(
        db.delete_game(u32::MAX),
        Err(Error::GameNotFound {
            number: 4_294_967_296
        })
    ));
    db.close().unwrap();

    assert_eq!(database_bytes(&path), before);
}

#[test]
fn test_update_game() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = five_with_deleted(temp.path(), &[]);
    let mut db = Database::open(&path).unwrap();
    let stored = all_pgn(&mut db);
    let old = db.get_game(2).unwrap();
    let old_frequencies = name_frequencies(&db, &old);
    let players = db.nfile().namebase().count(NameType::Player);
    let size = db.gfile().file_size();

    let game = scidtopgn::parse_single_game(NEW_GAME).unwrap();
    let expected = scid_round_trip(&game).to_pgn();
    db.update_game(2, &game).unwrap();

    let check = |db: &mut Database| {
        assert_eq!(db.num_games(), 5);
        let mut games = all_pgn(db);
        assert_eq!(games.remove(2), expected);
        let mut others = stored.clone();
        others.remove(2);
        assert_eq!(games, others, "the other games don't change");

        assert_eq!(name_frequencies(db, &game), [1; 5]);
        assert_eq!(
            name_frequencies(db, &old),
            old_frequencies.map(|f| f - 1),
            "the old names lose 1 and stay in the name file"
        );
        assert_eq!(db.nfile().namebase().count(NameType::Player), players + 2);
    };
    check(&mut db);
    let length = db.get_entry(2).unwrap().get_length();
    assert_eq!(
        db.gfile().file_size(),
        size + u64::from(length),
        "the new record is added and the old one stays"
    );
    db.close().unwrap();

    check(&mut Database::open(&path).unwrap());
}

#[test]
fn test_update_game_with_the_same_names() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = five_with_deleted(temp.path(), &[]);
    let mut db = Database::open(&path).unwrap();
    let names = all_names(&db);

    let mut game = db.get_game(0).unwrap();
    game.result = GameResult::Black;
    db.update_game(0, &game).unwrap();

    assert_eq!(all_names(&db), names);
    assert_eq!(db.get_game(0).unwrap().result, GameResult::Black);
}

#[test]
fn test_update_game_keeps_user_flags_and_sets_the_start_flag() {
    use scidtopgn::index::{IDX_MASK_DELETE, IDX_MASK_NOVELTY};

    let temp = tempfile::TempDir::new().unwrap();
    let path = five_with_deleted(temp.path(), &[1]);
    let si4 = path.with_extension("si4");
    let mut index = Index::open(&si4).unwrap();
    index
        .get_entry_mut(1)
        .unwrap()
        .set_flag(IDX_MASK_NOVELTY, true);
    index
        .flush(&mut std::fs::File::create(&si4).unwrap())
        .unwrap();

    let flags = |db: &Database| {
        let entry = db.get_entry(1).unwrap();
        (
            entry.get_flag(IDX_MASK_DELETE),
            entry.get_flag(IDX_MASK_NOVELTY),
            entry.get_start_flag(),
        )
    };

    let mut db = Database::open(&path).unwrap();
    let setup = "[FEN \"4k3/8/8/8/8/8/4P3/4K3 w - - 0 1\"]\n[SetUp \"1\"]\n\n1. e4 *\n";
    db.update_game(1, &scidtopgn::parse_single_game(setup).unwrap())
        .unwrap();
    assert_eq!(flags(&db), (true, true, true));

    db.update_game(1, &scidtopgn::parse_single_game(NEW_GAME).unwrap())
        .unwrap();
    assert_eq!(flags(&db), (true, true, false));
    db.close().unwrap();

    assert_eq!(flags(&Database::open(&path).unwrap()), (true, true, false));
}

#[test]
fn test_update_game_before_flush_in_new_database() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = temp.path().join("new");
    let parse = |white: &str| {
        scidtopgn::parse_single_game(&format!("[White \"{white}\"]\n\n1. e4 *\n")).unwrap()
    };

    let mut db = Database::create(&path).unwrap();
    db.add_game(&parse("A")).unwrap();
    db.add_game(&parse("B")).unwrap();
    db.update_game(0, &parse("C")).unwrap();
    assert_eq!(db.get_game(0).unwrap().white, "C");
    db.close().unwrap();

    let mut db = Database::open(&path).unwrap();
    assert_eq!(db.num_games(), 2);
    assert_eq!(db.get_game(0).unwrap().white, "C");
    assert_eq!(db.get_game(1).unwrap().white, "B");
    assert_eq!(frequency(&db, NameType::Player, "A"), 0);
    assert_eq!(frequency(&db, NameType::Player, "C"), 1);
    // Every game's Black is "?".
    assert_eq!(frequency(&db, NameType::Player, "?"), 2);
}

#[test]
fn test_update_damaged_game_with_missing_names() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = five_with_deleted(temp.path(), &[]);
    let si4 = path.with_extension("si4");
    let mut index = Index::open(&si4).unwrap();
    let entry = index.get_entry_mut(0).unwrap();
    entry.set_offset(u32::MAX - 10);
    entry.set_white(999_999);
    index
        .flush(&mut std::fs::File::create(&si4).unwrap())
        .unwrap();

    let mut db = Database::open(&path).unwrap();
    assert!(db.get_game(0).is_err());
    let game = scidtopgn::parse_single_game(NEW_GAME).unwrap();
    db.update_game(0, &game).unwrap();
    db.close().unwrap();

    let mut db = Database::open(&path).unwrap();
    assert_eq!(
        db.get_game(0).unwrap().to_pgn(),
        scid_round_trip(&game).to_pgn()
    );
}

#[test]
fn test_update_game_too_large_changes_nothing() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = five_with_deleted(temp.path(), &[]);
    let before = database_bytes(&path);

    let mut db = Database::open(&path).unwrap();
    let entry = entry_bytes(&db, 0);
    let names = all_names(&db);
    assert!(matches!(
        db.update_game(0, &game_with_record_length(131_072)),
        Err(Error::GameTooLarge { .. })
    ));
    assert_eq!(entry_bytes(&db, 0), entry);
    assert_eq!(all_names(&db), names);
    db.close().unwrap();

    assert_eq!(database_bytes(&path), before);
}
