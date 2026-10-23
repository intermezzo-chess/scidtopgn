use scidtopgn::{parse_single_game, ByteBuffer, Game, Index, NFile, GAME_DECODE_ALL};
use std::path::Path;

fn get_test_data_path(filename: &str) -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    workspace_root.join("tests/data").join(filename)
}

#[test]
fn test_game_new() {
    let game = Game::new();
    assert_eq!(game.white, "?");
    assert_eq!(game.black, "?");
    assert_eq!(game.event, "?");
}

#[test]
fn test_game_decode_from_one_sg4() {
    let si4_path = get_test_data_path("one.si4");
    let sg4_path = get_test_data_path("one.sg4");
    let sn4_path = get_test_data_path("one.sn4");

    let index = Index::open(&si4_path).expect("Failed to open index");
    assert_eq!(index.num_games(), 1);

    let entry = index.get_entry(0).expect("Failed to get entry");
    let offset = entry.get_offset();
    let length = entry.get_length();

    let nfile = NFile::open(&sn4_path).expect("Failed to open namebase");
    let namebase = nfile.namebase();

    let white = namebase
        .get_name(scidtopgn::NameType::Player, entry.get_white())
        .unwrap_or("?");
    let black = namebase
        .get_name(scidtopgn::NameType::Player, entry.get_black())
        .unwrap_or("?");
    let event = namebase
        .get_name(scidtopgn::NameType::Event, entry.get_event())
        .unwrap_or("?");
    let site = namebase
        .get_name(scidtopgn::NameType::Site, entry.get_site())
        .unwrap_or("?");
    let round = namebase
        .get_name(scidtopgn::NameType::Round, entry.get_round())
        .unwrap_or("?");

    let sg4_data = std::fs::read(&sg4_path).expect("Failed to read sg4");

    let game_data = &sg4_data[offset as usize..(offset + length) as usize];
    let mut buf = ByteBuffer::from_slice(game_data);

    let mut game = Game::new();
    game.decode(&mut buf, GAME_DECODE_ALL)
        .expect("Failed to decode game");

    game.load_standard_tags(
        white,
        black,
        event,
        site,
        round,
        entry.get_date_obj(),
        entry.get_result(),
    );

    assert_eq!(game.white, "Hossain, Enam");
    assert_eq!(game.black, "Murshed, N");
    assert_eq!(game.event, "47th ch-Bangahbandhu 2022");
}

#[test]
fn test_game_to_pgn() {
    let si4_path = get_test_data_path("one.si4");
    let sg4_path = get_test_data_path("one.sg4");
    let sn4_path = get_test_data_path("one.sn4");

    let index = Index::open(&si4_path).expect("Failed to open index");
    let entry = index.get_entry(0).expect("Failed to get entry");
    let offset = entry.get_offset();
    let length = entry.get_length();

    let nfile = NFile::open(&sn4_path).expect("Failed to open namebase");
    let namebase = nfile.namebase();

    let white = namebase
        .get_name(scidtopgn::NameType::Player, entry.get_white())
        .unwrap_or("?");
    let black = namebase
        .get_name(scidtopgn::NameType::Player, entry.get_black())
        .unwrap_or("?");
    let event = namebase
        .get_name(scidtopgn::NameType::Event, entry.get_event())
        .unwrap_or("?");
    let site = namebase
        .get_name(scidtopgn::NameType::Site, entry.get_site())
        .unwrap_or("?");
    let round = namebase
        .get_name(scidtopgn::NameType::Round, entry.get_round())
        .unwrap_or("?");

    let sg4_data = std::fs::read(&sg4_path).expect("Failed to read sg4");

    let game_data = &sg4_data[offset as usize..(offset + length) as usize];
    let mut buf = ByteBuffer::from_slice(game_data);

    let mut game = Game::new();
    game.decode(&mut buf, GAME_DECODE_ALL)
        .expect("Failed to decode game");

    game.load_standard_tags(
        white,
        black,
        event,
        site,
        round,
        entry.get_date_obj(),
        entry.get_result(),
    );

    let pgn = game.to_pgn();

    assert!(pgn.contains("[Event \"47th ch-Bangahbandhu 2022\"]"));
    assert!(pgn.contains("[White \"Hossain, Enam\"]"));
    assert!(pgn.contains("[Black \"Murshed, N\"]"));
}

/// The movetext `Game::to_pgn` writes for `pgn`, with its lines joined.
fn exported_movetext(pgn: &str) -> String {
    let pgn = parse_single_game(pgn).unwrap().to_pgn();
    let (_, movetext) = pgn.split_once("\n\n").unwrap();
    movetext.lines().collect::<Vec<_>>().join(" ")
}

#[test]
fn test_to_pgn_writes_variations_and_annotations() {
    for (movetext, expected) in [
        (
            "1. e4 $1 e5 {after e5} 2. Nf3 $2 *",
            "1. e4 ! e5 {after e5} 2. Nf3 ? *",
        ),
        (
            "1. e4 e5 (1... c5 2. Nf3) 2. Nf3 Nc6 *",
            "1. e4 e5 (1... c5 2. Nf3) 2. Nf3 Nc6 *",
        ),
        ("1. e4 (1. d4) e5 *", "1. e4 (1. d4) 1... e5 *"),
        (
            "1. e4 e5 {c} 2. Nf3 {d} Nc6 *",
            "1. e4 e5 {c} 2. Nf3 {d} 2... Nc6 *",
        ),
        (
            "1. e4 (1. d4 d5 (1... Nf6 2. c4) 2. c4) (1. c4) e5 *",
            "1. e4 (1. d4 d5 (1... Nf6 2. c4) 2. c4) (1. c4) 1... e5 *",
        ),
        (
            "{Intro} 1. e4 ({Or} 1. d4 $1) $14 e5 *",
            "{Intro} 1. e4 += ({Or} 1. d4 !) 1... e5 *",
        ),
        ("1. e4 () e5 *", "1. e4 () 1... e5 *"),
        (
            "1. e4 e5 (1... -- 2. d4) 2. Nf3 *",
            "1. e4 e5 (1... -- 2. d4) 2. Nf3 *",
        ),
    ] {
        let pgn = format!("[Event \"x\"]\n\n{movetext}\n");
        assert_eq!(exported_movetext(&pgn), expected, "{movetext}");
        // The export reads back as the same game.
        let exported = parse_single_game(&pgn).unwrap().to_pgn();
        assert_eq!(parse_single_game(&exported).unwrap().to_pgn(), exported);
    }
}

#[test]
fn test_to_pgn_numbers_variations_from_the_fen() {
    let pgn = "[Event \"x\"]\n[FEN \"r1bqkbnr/pppp1ppp/2n5/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3\"]\n\n3... a6 (3... Nf6 4. O-O) 4. Ba4 *\n";
    assert_eq!(exported_movetext(pgn), "3... a6 (3... Nf6 4. O-O) 4. Ba4 *");
}

#[test]
fn test_to_pgn_wraps_variations() {
    let mut movetext = String::new();
    for number in (1..40).step_by(2) {
        let next = number + 1;
        movetext.push_str(&format!(
            "{number}. Nf3 {{out}} ({number}. Nc3 Nc6 ({number}... Nf6 {{a comment}}) {next}. Nb1) Nf6 {next}. Ng1 Ng8 "
        ));
    }
    let pgn = format!("[Event \"x\"]\n\n{movetext}*\n");
    let exported = parse_single_game(&pgn).unwrap().to_pgn();
    let (_, lines) = exported.split_once("\n\n").unwrap();
    assert!(lines.lines().count() > 1, "{exported}");
    for line in lines.lines() {
        assert!(line.len() <= 80, "{line}");
    }
    assert_eq!(parse_single_game(&exported).unwrap().to_pgn(), exported);
}
