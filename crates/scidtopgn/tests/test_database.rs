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

/// The record of game `game_num` in the database at `path`.
fn record(path: &std::path::Path, game_num: u32) -> Vec<u8> {
    let db = Database::open(path).unwrap();
    let entry = db.get_entry(game_num).unwrap();
    let start = entry.get_offset() as usize;
    let sg4 = std::fs::read(path.with_extension("sg4")).unwrap();
    sg4[start..start + entry.get_length() as usize].to_vec()
}

#[test]
fn test_tags_are_encoded_like_scid() {
    // SCID stores its ten common tags as 241 to 250, and other tags by
    // name, matching case.
    const COMMON: [&str; 10] = [
        "WhiteCountry",
        "BlackCountry",
        "Annotator",
        "PlyCount",
        "EventDate",
        "Opening",
        "Variation",
        "Setup",
        "Source",
        "SetUp",
    ];
    let mut game = Game::new();
    let mut expected = Vec::new();
    for (code, name) in (241..).zip(COMMON) {
        game.add_tag(name, "v");
        expected.extend([code, 1, b'v']);
    }
    game.add_tag("setup", "ab");
    expected.push(5);
    expected.extend(b"setup");
    expected.push(2);
    expected.extend(b"ab");
    expected.push(0);

    let temp = tempfile::TempDir::new().unwrap();
    let path = temp.path().join("tags");
    let mut db = Database::create(&path).unwrap();
    db.add_game(&game).unwrap();
    db.close().unwrap();

    assert!(record(&path, 0).starts_with(&expected));
    let read = Database::open(&path).unwrap().get_game(0).unwrap();
    let tags = |game: &Game| -> Vec<(String, String)> {
        game.tags
            .iter()
            .map(|tag| (tag.name.clone(), tag.value.clone()))
            .collect()
    };
    assert_eq!(tags(&read), tags(&game));
}

#[test]
fn test_event_date_is_stored_only_in_the_index() {
    // The index holds an event date within 3 years of the game's year.
    // SCID drops any other, and stores none in the record.
    for (date, event_date, kept) in [
        ("2022.06.01", "2022.05.20", true),
        ("2022.06.01", "2019.05.20", true),
        ("2022.06.01", "2025.05.20", true),
        ("2022.06.01", "2022.??.??", true),
        ("2022.06.01", "2018.12.31", false),
        ("2022.06.01", "2026.01.01", false),
        ("2022.06.01", "????.??.??", false),
        ("????.??.??", "2022.05.20", false),
    ] {
        let pgn = format!("[Date \"{date}\"]\n[EventDate \"{event_date}\"]\n\n1. e4 *\n");
        let game = scidtopgn::parse_single_game(&pgn).unwrap();
        let expected = kept.then(|| Date::from_string(event_date));

        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("event_date");
        let mut db = Database::create(&path).unwrap();
        db.add_game(&game).unwrap();
        db.add_game(&Game::new()).unwrap();
        db.update_game(1, &game).unwrap();
        db.close().unwrap();

        let mut db = Database::open(&path).unwrap();
        for game_num in [0, 1] {
            assert_eq!(record(&path, game_num)[0], 0, "{event_date}: no tags");
            let read = db.get_game(game_num).unwrap();
            assert_eq!(read.event_date, expected, "{date} {event_date}");
            assert_eq!(
                read.to_pgn().contains("[EventDate "),
                kept,
                "{date} {event_date}"
            );
        }
    }
}

#[test]
fn test_reads_the_event_date_from_earlier_records() {
    // Earlier versions also stored the event date in the record, as 255
    // and three bytes, where it is the only copy of one the index can't
    // hold. Replace a 3-byte tag and a 3-byte value with one.
    let game =
        scidtopgn::parse_single_game("[Date \"2022.06.01\"]\n[Foo \"abc\"]\n\n1. e4 *\n").unwrap();
    let temp = tempfile::TempDir::new().unwrap();
    let path = temp.path().join("earlier");
    let mut db = Database::create(&path).unwrap();
    db.add_game(&game).unwrap();
    db.close().unwrap();

    let sg4_path = path.with_extension("sg4");
    let mut sg4 = std::fs::read(&sg4_path).unwrap();
    let start = Database::open(&path)
        .unwrap()
        .get_entry(0)
        .unwrap()
        .get_offset() as usize;
    let event_date = Date::new(2030, 5, 20).unwrap();
    let [_, high, mid, low] = event_date.raw().to_be_bytes();
    assert_eq!(&sg4[start..start + 8], b"\x03Foo\x03abc");
    sg4[start..start + 8].copy_from_slice(&[255, high, mid, low, 1, b'x', 1, b'y']);
    std::fs::write(&sg4_path, sg4).unwrap();

    let read = Database::open(&path).unwrap().get_game(0).unwrap();
    assert_eq!(read.event_date, Some(event_date));
    assert_eq!(read.find_tag("x").map(String::as_str), Some("y"));
    assert!(read.to_pgn().contains("[EventDate \"2030.05.20\"]\n"));
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

/// The raw index entries of the database at `path`.
fn raw_entries(path: &std::path::Path) -> Vec<Vec<u8>> {
    let si4 = std::fs::read(path.with_extension("si4")).unwrap();
    si4[scidtopgn::INDEX_HEADER_SIZE..]
        .chunks(scidtopgn::INDEX_ENTRY_SIZE)
        .map(<[u8]>::to_vec)
        .collect()
}

/// Copies `derived`, which SCID wrote, into a new directory, opens it once
/// for each of `changes`, closes it, and checks that every entry but those in
/// `changed` keeps SCID's bytes.
fn assert_keeps_scids_entries(changed: &[usize], changes: &[&dyn Fn(&mut Database)]) {
    let src = get_test_data_path("derived");
    let temp = tempfile::TempDir::new().unwrap();
    let path = temp.path().join("derived");
    for ext in ["si4", "sn4", "sg4"] {
        std::fs::copy(src.with_extension(ext), path.with_extension(ext)).unwrap();
    }
    for change in changes {
        let mut db = Database::open(&path).unwrap();
        change(&mut db);
        db.close().unwrap();
    }

    let entries = raw_entries(&path);
    for (n, entry) in raw_entries(&src).iter().enumerate() {
        if !changed.contains(&n) {
            assert_eq!(&entries[n], entry, "entry {n}");
        }
    }
}

#[test]
fn test_changes_keep_scids_other_entries() {
    // Game 0 of `derived` is 302 half-moves long, which needs the top bits of
    // the count.
    let db = Database::open(get_test_data_path("derived")).unwrap();
    assert_eq!(db.get_entry(0).unwrap().get_num_half_moves(), 302);

    let new_game = scidtopgn::parse_single_game(NEW_GAME).unwrap();
    assert_keeps_scids_entries(&[1], &[&|db| db.delete_game(1).unwrap()]);
    assert_keeps_scids_entries(
        &[],
        &[&|db| db.delete_game(1).unwrap(), &|db| {
            db.undelete_game(1).unwrap()
        }],
    );
    assert_keeps_scids_entries(&[1], &[&|db| db.update_game(1, &new_game).unwrap()]);
    assert_keeps_scids_entries(
        &[],
        &[&|db| {
            db.add_game(&new_game).unwrap();
        }],
    );
}

#[test]
fn test_delete_keeps_scids_name_order() {
    // `accents` was written by SCID with names that sort differently as
    // signed and as unsigned bytes. Deleting a game rewrites the name file,
    // which must stay as SCID wrote it.
    let temp = tempfile::TempDir::new().unwrap();
    let src = get_test_data_path("accents");
    let path = temp.path().join("accents");
    for ext in ["si4", "sn4", "sg4"] {
        std::fs::copy(src.with_extension(ext), path.with_extension(ext)).unwrap();
    }

    let mut db = Database::open(&path).unwrap();
    db.delete_game(0).unwrap();
    db.close().unwrap();

    assert_eq!(
        std::fs::read(path.with_extension("sn4")).unwrap(),
        std::fs::read(src.with_extension("sn4")).unwrap()
    );
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

/// The fields of `entry` that SCID derives from the game's moves, except
/// the stored line code.
fn derived_fields(entry: &scidtopgn::IndexEntry) -> (u16, [u8; 9], u32, bool, bool, [u16; 3]) {
    use scidtopgn::index::{IDX_MASK_PROMO, IDX_MASK_UPROMO};
    (
        entry.get_num_half_moves(),
        *entry.get_home_pawn_data(),
        entry.get_final_mat_sig(),
        entry.get_flag(IDX_MASK_PROMO),
        entry.get_flag(IDX_MASK_UPROMO),
        [
            entry.get_variation_count(),
            entry.get_comment_count(),
            entry.get_nag_count(),
        ],
    )
}

#[test]
fn test_update_game_sets_the_derived_fields() {
    // In `derived`, which SCID wrote, game 2 has a queen promotion, game 3 an
    // underpromotion, and game 5 no promotion but more than 10 variations,
    // comments, and NAGs.
    let src = get_test_data_path("derived");
    let scid = Database::open(&src).unwrap();
    let temp = tempfile::TempDir::new().unwrap();
    let path = temp.path().join("derived");
    for ext in ["si4", "sn4", "sg4"] {
        std::fs::copy(src.with_extension(ext), path.with_extension(ext)).unwrap();
    }
    let games =
        scidtopgn::parse_pgn(&std::fs::read_to_string(src.with_extension("pgn")).unwrap()).unwrap();

    let mut db = Database::open(&path).unwrap();
    db.update_game(2, &games[5]).unwrap();
    db.update_game(4, &games[3]).unwrap();
    db.close().unwrap();

    let db = Database::open(&path).unwrap();
    let fields = |db: &Database, n| derived_fields(db.get_entry(n).unwrap());
    assert_eq!(fields(&db, 2), fields(&scid, 5));
    assert_eq!(fields(&db, 4), fields(&scid, 3));
}

#[test]
fn test_games_without_moves_have_the_starting_material() {
    let temp = tempfile::TempDir::new().unwrap();
    let mut db = Database::create(temp.path().join("games")).unwrap();
    let standard = scidtopgn::parse_single_game("[Event \"x\"]\n\n*\n").unwrap();
    let setup = "[FEN \"4k3/8/8/8/8/8/4P3/4K3 w - - 0 1\"]\n[SetUp \"1\"]\n\n*\n";
    let setup = scidtopgn::parse_single_game(setup).unwrap();
    db.add_game(&standard).unwrap();
    db.add_game(&setup).unwrap();

    // SCID's signature of the standard starting position, and of one White
    // pawn.
    let standard_fields = (0, [0; 9], 0x6A86A8, false, false, [0; 3]);
    let setup_fields = (0, [0; 9], 1 << 12, false, false, [0; 3]);
    assert_eq!(derived_fields(db.get_entry(0).unwrap()), standard_fields);
    assert_eq!(derived_fields(db.get_entry(1).unwrap()), setup_fields);
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

/// A game with tag `name` set to `value`.
fn game_with_tag(name: &str, value: &str) -> Game {
    let mut game = scidtopgn::parse_single_game("[Event \"x\"]\n\n1. e4 *\n").unwrap();
    game.add_tag(name, value);
    game
}

/// `game`'s tags, as name and value pairs.
fn tags(game: &Game) -> Vec<(String, String)> {
    game.tags
        .iter()
        .map(|t| (t.name.clone(), t.value.clone()))
        .collect()
}

#[test]
fn test_pgn_tag_value_longer_than_255_bytes_is_dropped_on_import() {
    // #82: the game was written, but couldn't be read back.
    let pgn = format!(
        "[Event \"x\"]\n[Annotator \"{}\"]\n\n1. e4 *\n",
        "a".repeat(300)
    );
    let game = scid_round_trip(&scidtopgn::parse_single_game(&pgn).unwrap());
    assert!(game.tags.is_empty());
    assert_eq!(game.event, "x");
}

#[test]
fn test_tag_values_up_to_255_bytes_round_trip() {
    // 85 three-byte characters are 255 bytes.
    for value in ["a".repeat(255), "€".repeat(85)] {
        for name in ["Annotator", "Note"] {
            let game = scid_round_trip(&game_with_tag(name, &value));
            assert_eq!(tags(&game), [(name.to_string(), value.clone())]);
        }
    }
}

#[test]
fn test_add_game_rejects_a_tag_value_longer_than_255_bytes() {
    // 86 three-byte characters are 258 bytes.
    for (name, value) in [
        ("Annotator", "a".repeat(256)),
        ("Note", "a".repeat(300)),
        ("Note", "€".repeat(86)),
    ] {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("long_tag");
        Database::create(&path).unwrap().close().unwrap();
        let before = database_bytes(&path);

        let mut db = Database::open(&path).unwrap();
        match db.add_game(&game_with_tag(name, &value)) {
            Err(Error::TagTooLong {
                name: tag,
                length,
                max: 255,
            }) => {
                assert_eq!(tag, name);
                assert_eq!(length, value.len());
            }
            other => panic!("expected a tag too long error, got {other:?}"),
        }
        assert_eq!(db.num_games(), 0);
        assert!(all_names(&db).iter().all(Vec::is_empty));
        db.close().unwrap();

        assert_eq!(database_bytes(&path), before);
    }
}

#[test]
fn test_update_game_with_a_tag_value_too_long_changes_nothing() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = five_with_deleted(temp.path(), &[]);
    let before = database_bytes(&path);

    let mut db = Database::open(&path).unwrap();
    let entry = entry_bytes(&db, 0);
    let names = all_names(&db);
    let mut game = game_with_tag("Annotator", &"a".repeat(256));
    game.white = "A new player".to_string();
    assert!(matches!(
        db.update_game(0, &game),
        Err(Error::TagTooLong { length: 256, .. })
    ));
    assert_eq!(entry_bytes(&db, 0), entry);
    assert_eq!(all_names(&db), names);
    db.close().unwrap();

    assert_eq!(database_bytes(&path), before);
}

#[test]
fn test_tag_names_longer_than_240_bytes_are_cut() {
    let n = |len| "n".repeat(len);
    for (name, stored) in [
        (n(240), n(240)),
        (n(241), n(240)),
        (n(300), n(240)),
        // A three-byte character that the cut would split is dropped whole.
        (n(239) + "€", n(239)),
    ] {
        let game = scid_round_trip(&game_with_tag(&name, "v"));
        assert_eq!(tags(&game), [(stored, "v".to_string())], "{}", name.len());
    }

    // Names that share their first 240 bytes are read back as one tag, with
    // the last value, as in SCID.
    let mut game = game_with_tag(&(n(240) + "a"), "1");
    game.add_tag(&(n(240) + "b"), "2");
    let game = scid_round_trip(&game);
    assert_eq!(tags(&game), [(n(240), "2".to_string())]);
}

#[test]
fn test_names_longer_than_255_bytes_are_cut() {
    let a = |len| "a".repeat(len);
    // #53: 200 two-byte characters, which a cut at 255 bytes would split.
    let accented = "é".repeat(200);
    let temp = tempfile::TempDir::new().unwrap();
    let path = temp.path().join("long_names");
    let mut db = Database::create(&path).unwrap();
    // Names that differ only after byte 255 are one name, as in SCID.
    for (white, black) in [(accented, a(255) + "b"), (a(300), a(255) + "c")] {
        let mut game = scidtopgn::parse_single_game("[Event \"x\"]\n\n1. e4 *\n").unwrap();
        game.white = white;
        game.black = black;
        db.add_game(&game).unwrap();
    }

    let names = |db: &mut Database| {
        let games = [db.get_game(0).unwrap(), db.get_game(1).unwrap()];
        games.map(|g| (g.white, g.black))
    };
    let expected = [("é".repeat(127), a(255)), (a(255), a(255))];
    assert_eq!(names(&mut db), expected);
    db.close().unwrap();

    let mut db = Database::open(&path).unwrap();
    assert_eq!(names(&mut db), expected);
    assert_eq!(
        db.nfile().namebase().get_all_names(NameType::Player),
        [(0, "é".repeat(127), 1), (1, a(255), 3)]
    );
}
