//! Compares the Rust reader and writer with SCID's own encoding.
//!
//! `tests/data/five` was written by SCID. `tests/data/edge` was written by
//! the C++ reference converter in `scid-cpp` from `tests/data/edge.pgn`, which
//! covers variations, null moves, promotions, en passant, castling, and FEN
//! starts. The expected movetext below is python-chess 1.11.2's export of
//! `edge.pgn`. `tests/data/castling` was written by the C++ reference
//! converter from `tests/data/castling.pgn`, which castles after the king has
//! moved and returned, as SCID allows. `tests/data/annotated` was written by
//! the C++ reference converter from `tests/data/annotated.pgn`, which has
//! NAGs and comments in the main line and in variations.

use std::path::PathBuf;

use scidtopgn::private::skip_tags;
use scidtopgn::shakmaty::{Color, Position};
use scidtopgn::{parse_pgn, ByteBuffer, Database, Game, Marker, ENCODE_END_GAME};
use tempfile::TempDir;

const EDGE_MOVETEXT: [&str; 4] = [
    "1. e4 e5 2. Nf3 ( 2. f4 exf4 ( 2... d5 3. exd5 Qxd5 ) 3. Nf3 ) ( 2. Nc3 Nf6 ( 2... Nc6 ) 3. f4 ) 2... Nc6 3. Bb5 ( 3. Bc4 Bc5 ( 3... Nf6 4. Ng5 d5 5. exd5 Na5 ( 5... Nxd5 6. Nxf7 Kxf7 ) 6. Bb5+ c6 ) 4. O-O ) 3... a6 4. Ba4 Nf6 5. O-O Be7 6. Re1 b5 7. Bb3 d6 8. c3 O-O 9. h3 Nb8 10. d4 Nbd7 *",
    "25... dxe3 26. bxa8=N ( 26. bxa8=Q+ Kd7 ) ( 26. bxa8=R+ Kd7 ) ( 26. b8=B Rxb8 ) 26... gxh1=Q+ ( 26... gxh1=R+ 27. Ke2 ) ( 26... gxh1=B 27. Rd1 ) ( 26... g1=N 27. Rd1 ) 27. Ke2 Qxa1 28. Nc7+ Kd7 29. -- ( 29. Nb5 Rh2+ ) 29... Qb2+ 30. Kxe3 Rh3+ 31. Kf4 Qf2+ 32. Kg4 Rg3+ 33. Kh4 Qh2# 0-1",
    "1. e4 -- 2. d4 e6 3. -- ( 3. Nc3 Bb4 ) 3... d5 4. e5 c5 5. c3 Nc6 6. Nf3 Qb6 7. Be2 cxd4 8. cxd4 Nge7 9. Na3 Nf5 10. Nc2 Bb4+ 11. Kf1 O-O 12. g4 Nfe7 13. h4 f6 14. exf6 Rxf6 15. g5 Rf7 16. Bd3 Ng6 17. h5 Nf4 18. Bxf4 Rxf4 19. Bxh7+ Kxh7 20. Qd3+ Kg8 21. Qh7+ Kf8 22. Qh8+ Ke7 23. Qxg7+ Kd8 24. g6 1-0",
    "40... Kd7 41. O-O-O+ Kc6 42. Rd4 e5 43. Rd3 e4 44. Rd4 e3 45. Kd1 e2+ 46. Kxe2 Kc5 1/2-1/2",
];

/// SCID's export of `tests/data/annotated`, with each annotation where SCID
/// stores it, in this crate's notation.
const ANNOTATED_MOVETEXT: [&str; 2] = [
    "{Intro} 1. e4 ! e5 {after e5} 2. Nf3 ? (2. f4 !? {gambit} 2... exf4 (2... d5 ! {counter}) 3. Nf3) (2. Nc3 {quiet and solid}) 2... Nc6 += $36 3. Bb5 ! {Spanish} ({Or} 3. Bc4 Bc5 =) 3... a6 *",
    "3... a6 {Morphy} (3... Nf6 !? 4. O-O) 4. Ba4 Nf6 *",
];

const CASTLING_MOVETEXT: [&str; 3] = [
    "1. e4 e5 2. Nf3 Nf6 3. Bc4 Bc5 4. Ke2 Ke7 5. Ke1 Ke8 6. O-O O-O 7. d3 d6 *",
    "1. O-O-O O-O 2. Kb1 Kg7 *",
    "1. e4 e5 2. Nf3 Nf6 3. Bc4 Bc5 4. Ke2 Ke7 5. Ke1 Ke8 6. d3 ( 6. O-O d6 ) 6... d6 *",
];

fn data_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data")
        .join(name)
}

fn read_pgn(name: &str) -> Vec<Game> {
    parse_pgn(&std::fs::read_to_string(data_path(name)).unwrap()).unwrap()
}

fn read_database(name: &str) -> Vec<Game> {
    let mut db = Database::open(data_path(name)).unwrap();
    db.games().collect::<Result<_, _>>().unwrap()
}

/// Writes `games` to a new database and returns its game records.
fn write_records(games: &[Game]) -> Vec<Vec<u8>> {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("written");
    let mut db = Database::create(&path).unwrap();
    for game in games {
        db.add_game(game).unwrap();
    }
    db.flush().unwrap();
    records(&path)
}

/// The game records in a database's `.sg4` file.
fn records(path: &std::path::Path) -> Vec<Vec<u8>> {
    let db = Database::open(path).unwrap();
    let sg4 = std::fs::read(path.with_extension("sg4")).unwrap();
    (0..db.num_games())
        .map(|i| {
            let entry = db.get_entry(i).unwrap();
            let start = entry.get_offset() as usize;
            sg4[start..start + entry.get_length() as usize].to_vec()
        })
        .collect()
}

/// The parts of a game record that encode the game rather than its tags:
/// whether it starts from a FEN, the FEN, the moves, and the comments. SCID
/// also stores promotion flags, which this crate doesn't write.
fn move_section(record: &[u8]) -> (bool, Vec<u8>) {
    let mut buf = ByteBuffer::from_slice(record);
    skip_tags(&mut buf).unwrap();
    let flags = buf.get_byte().unwrap();
    let rest = record[buf.position()..].to_vec();
    assert!(rest.contains(&ENCODE_END_GAME));
    (flags & 1 != 0, rest)
}

/// The game's movetext with variations, formatted like python-chess.
fn movetext(game: &Game) -> String {
    let ply = match game.start_position {
        Some(ref start) => {
            2 * (start.fullmoves().get() - 1) + u32::from(start.turn() == Color::Black)
        }
        None => 0,
    };
    let mut tokens = Vec::new();
    line(game, game.first_move, ply, &mut tokens);
    tokens.push(game.result.to_string());
    tokens.join(" ")
}

fn line(game: &Game, mut idx: Option<usize>, mut ply: u32, tokens: &mut Vec<String>) {
    let mut number_black_move = true;
    while let Some(i) = idx {
        let node = &game.moves[i];
        idx = node.next;
        if matches!(node.marker, Marker::EndMarker | Marker::EndGame) {
            break;
        }
        if node.move_data.is_none() {
            continue;
        }
        let number = ply / 2 + 1;
        if ply.is_multiple_of(2) {
            tokens.push(format!("{number}."));
        } else if number_black_move {
            tokens.push(format!("{number}..."));
        }
        tokens.push(node.san.clone());
        number_black_move = false;

        let mut variation = node.var_child;
        while let Some(start) = variation {
            tokens.push("(".to_string());
            line(game, game.moves[start].next, ply, tokens);
            tokens.push(")".to_string());
            number_black_move = true;
            variation = game.moves[start].var_child;
        }
        ply += 1;
    }
}

#[test]
fn test_reads_scid_written_variations_and_setups() {
    let games = read_database("edge");
    let movetext: Vec<String> = games.iter().map(movetext).collect();
    assert_eq!(movetext, EDGE_MOVETEXT);
}

#[test]
fn test_reads_scid_written_castling_without_the_right() {
    let games = read_database("castling");
    let movetext: Vec<String> = games.iter().map(movetext).collect();
    assert_eq!(movetext, CASTLING_MOVETEXT);
}

#[test]
fn test_parses_castling_without_the_right() {
    let games = read_pgn("castling.pgn");
    let movetext: Vec<String> = games.iter().map(movetext).collect();
    assert_eq!(movetext, CASTLING_MOVETEXT);
    let warnings: Vec<usize> = games.iter().map(|game| game.warnings.len()).collect();
    assert_eq!(warnings, [2, 2, 1]);
}

#[test]
fn test_parses_pgn_variations_and_setups() {
    let games = read_pgn("edge.pgn");
    let movetext: Vec<String> = games.iter().map(movetext).collect();
    assert_eq!(movetext, EDGE_MOVETEXT);
}

#[test]
fn test_writes_moves_like_scid() {
    let expected: Vec<_> = records(&data_path("edge"))
        .iter()
        .map(|r| move_section(r))
        .collect();
    let written: Vec<_> = write_records(&read_pgn("edge.pgn"))
        .iter()
        .map(|r| move_section(r))
        .collect();
    assert_eq!(written, expected);

    for name in ["five", "castling", "annotated"] {
        let expected: Vec<_> = records(&data_path(name))
            .iter()
            .map(|r| move_section(r))
            .collect();
        let written: Vec<_> = write_records(&read_pgn(&format!("{name}.pgn")))
            .iter()
            .map(|r| move_section(r))
            .collect();
        assert_eq!(written, expected, "{name}");
    }
}

/// The movetext `Game::to_pgn` writes, with its lines joined.
fn exported_movetext(game: &Game) -> String {
    let pgn = game.to_pgn();
    let (_, movetext) = pgn.split_once("\n\n").unwrap();
    movetext.lines().collect::<Vec<_>>().join(" ")
}

#[test]
fn test_reads_scid_written_annotations() {
    let exported: Vec<String> = read_database("annotated")
        .iter()
        .map(exported_movetext)
        .collect();
    assert_eq!(exported, ANNOTATED_MOVETEXT);

    let exported: Vec<String> = read_pgn("annotated.pgn")
        .iter()
        .map(exported_movetext)
        .collect();
    assert_eq!(exported, ANNOTATED_MOVETEXT);
}

#[test]
fn test_scid_to_pgn_to_scid() {
    for name in ["five", "edge", "castling", "annotated"] {
        let original: Vec<_> = records(&data_path(name))
            .iter()
            .map(|r| move_section(r))
            .collect();
        let pgn: String = read_database(name).iter().map(Game::to_pgn).collect();
        let written: Vec<_> = write_records(&parse_pgn(&pgn).unwrap())
            .iter()
            .map(|r| move_section(r))
            .collect();
        assert_eq!(written, original, "{name}");
    }
}

#[test]
fn test_pgn_to_scid_to_pgn() {
    for name in ["five.pgn", "edge.pgn", "castling.pgn", "annotated.pgn"] {
        let games = read_pgn(name);
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("roundtrip");
        let mut db = Database::create(&path).unwrap();
        for game in &games {
            db.add_game(game).unwrap();
        }
        db.flush().unwrap();

        let reread = read_database_at(&path);
        assert_eq!(reread.len(), games.len(), "{name}");
        for (before, after) in games.iter().zip(&reread) {
            assert_eq!(movetext(after), movetext(before), "{name}");
            assert_eq!(after.result, before.result, "{name}");
            assert_eq!(after.to_pgn(), before.to_pgn(), "{name}");
        }
    }
}

#[test]
fn test_five_export_matches_fixture() {
    let expected: Vec<Vec<String>> = read_pgn("five.pgn").iter().map(main_line).collect();
    let exported: Vec<Vec<String>> = read_database("five").iter().map(main_line).collect();
    assert_eq!(exported, expected);

    // #25 and the false mate in game 2.
    let pgn: String = read_database("five").iter().map(Game::to_pgn).collect();
    for san in [
        "15. Nbd2", "5. Ngf3", "14. Nfd4", "6. Nbd2", "22. Nfe5", "Ndxe5", "Qxa1+",
    ] {
        assert!(pgn.contains(san), "missing {san}");
    }
    assert!(!pgn.contains("Qxa1#"));
}

fn main_line(game: &Game) -> Vec<String> {
    let mut sans = Vec::new();
    let mut idx = game.first_move;
    while let Some(i) = idx {
        let node = &game.moves[i];
        if node.move_data.is_some() {
            sans.push(node.san.clone());
        }
        idx = node.next;
    }
    sans
}

fn read_database_at(path: &std::path::Path) -> Vec<Game> {
    let mut db = Database::open(path).unwrap();
    db.games().collect::<Result<_, _>>().unwrap()
}

/// Decodes one game record's moves.
fn read_database_record(record: &[u8]) -> Vec<String> {
    let mut game = Game::new();
    game.decode(
        &mut ByteBuffer::from_slice(record),
        scidtopgn::GAME_DECODE_ALL,
    )
    .unwrap();
    main_line(&game)
}

#[test]
fn test_index_start_flags_match_scid() {
    let start_flags = |path: &std::path::Path| -> Vec<bool> {
        let db = Database::open(path).unwrap();
        (0..db.num_games())
            .map(|i| db.get_entry(i).unwrap().get_start_flag())
            .collect()
    };
    let expected = start_flags(&data_path("edge"));
    assert_eq!(expected, [false, true, false, true]);

    let temp = TempDir::new().unwrap();
    let path = temp.path().join("edge");
    let mut db = Database::create(&path).unwrap();
    for game in read_pgn("edge.pgn") {
        db.add_game(&game).unwrap();
    }
    db.flush().unwrap();
    assert_eq!(start_flags(&path), expected);
}

#[test]
fn test_annotated_null_moves_round_trip() {
    let games = parse_pgn("[Event \"Nulls\"]\n\n1. Z0! null? 2. e4 *\n").unwrap();
    let records = write_records(&games);
    assert_eq!(read_database_record(&records[0]), ["--", "--", "e4"]);
}
