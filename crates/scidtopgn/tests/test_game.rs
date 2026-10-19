use scidtopgn::{ByteBuffer, Game, Index, NFile, GAME_DECODE_ALL};
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
