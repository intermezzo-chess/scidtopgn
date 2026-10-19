use scidtopgn::{Index, IndexEntry, INDEX_HEADER_SIZE, INDEX_MAGIC, SCID_VERSION};
use std::path::PathBuf;

fn get_test_data_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("..");
    path.push("..");
    path.push("tests");
    path.push("data");
    path.push(filename);
    path
}

#[test]
fn test_read_one_si4() {
    let path = get_test_data_path("one.si4");
    let index = Index::open(&path).expect("Failed to open one.si4");

    assert_eq!(index.version(), SCID_VERSION, "Version should be 400");
    assert_eq!(index.num_games(), 1, "Should have exactly 1 game");
}

#[test]
fn test_one_si4_header() {
    let path = get_test_data_path("one.si4");
    let index = Index::open(&path).expect("Failed to open one.si4");

    let header = index.header();
    assert_eq!(&header.magic, INDEX_MAGIC, "Magic should match");
    assert_eq!(header.version, 400, "Version should be 400");
    assert_eq!(header.num_games, 1, "numGames should be 1");
}

#[test]
fn test_one_si4_entry() {
    let path = get_test_data_path("one.si4");
    let index = Index::open(&path).expect("Failed to open one.si4");

    let entry = index.get_entry(0).expect("Entry 0 should exist");

    assert!(entry.get_length() > 0, "Length should be non-zero");

    assert!(entry.get_white() < 100, "White ID should be valid");
    assert!(entry.get_black() < 100, "Black ID should be valid");

    let year = entry.get_year();
    assert!(
        year > 1900 && year < 2100,
        "Year should be reasonable: {}",
        year
    );
}

#[test]
fn test_one_si4_detailed_values() {
    let path = get_test_data_path("one.si4");
    let index = Index::open(&path).expect("Failed to open one.si4");

    let entry = index.get_entry(0).expect("Entry 0 should exist");

    assert_eq!(entry.get_offset(), 0, "Offset should be 0");
    assert_eq!(entry.get_length(), 168, "Length should be 168");

    assert_eq!(entry.get_white(), 0, "White ID should be 0");
    assert_eq!(entry.get_black(), 1, "Black ID should be 1");

    assert_eq!(entry.get_event(), 0, "Event ID should be 0");
    assert_eq!(entry.get_site(), 0, "Site ID should be 0");
    assert_eq!(entry.get_round(), 0, "Round ID should be 0");

    assert_eq!(
        entry.get_result(),
        scidtopgn::GameResult::Draw,
        "Result should be Draw"
    );

    assert_eq!(entry.get_eco_code(), 0x45b3, "ECO code should be 0x45b3");
}

#[test]
fn test_one_si4_date() {
    let path = get_test_data_path("one.si4");
    let index = Index::open(&path).expect("Failed to open one.si4");

    let entry = index.get_entry(0).expect("Entry 0 should exist");

    let date = entry.get_date_obj();
    assert!(date.is_valid(), "Date should be valid");

    let year = date.year();
    assert!(year > 1900 && year < 2100, "Year should be reasonable");
}

#[test]
fn test_read_five_si4() {
    let path = get_test_data_path("five.si4");
    let index = Index::open(&path).expect("Failed to open five.si4");

    assert_eq!(index.version(), SCID_VERSION, "Version should be 400");
    assert_eq!(index.num_games(), 5, "Should have exactly 5 games");

    for i in 0..5 {
        let entry = index
            .get_entry(i)
            .expect(&format!("Entry {} should exist", i));
        assert!(
            entry.get_offset() > 0 || i == 0,
            "Offset {} should be non-zero",
            i
        );
        assert!(entry.get_length() > 0, "Length {} should be non-zero", i);
    }
}

#[test]
fn test_index_entry_size() {
    let entry = IndexEntry::new();
    let mut buffer = Vec::new();
    use std::io::Cursor;
    let mut cursor = Cursor::new(&mut buffer);
    entry.write(&mut cursor).unwrap();
    assert_eq!(buffer.len(), 47, "Entry should be 47 bytes");
}

#[test]
fn test_index_header_size() {
    assert_eq!(INDEX_HEADER_SIZE, 182, "Header should be 182 bytes");
}

#[test]
fn test_entry_roundtrip() {
    use scidtopgn::{date_make, GameResult};
    use std::io::Cursor;

    let mut entry = IndexEntry::new();
    entry.set_offset(12345);
    entry.set_length(67890);
    entry.set_white(0x12345);
    entry.set_black(0xABCDE);
    entry.set_event(0x1A2B3);
    entry.set_site(0x2B3C4);
    entry.set_round(0x3C4D);
    entry.set_date(date_make(2022, 12, 19));
    entry.set_result(GameResult::White);
    entry.set_white_elo(2850);
    entry.set_black_elo(2700);
    entry.set_eco_code(0x1234);

    let mut buffer = Vec::new();
    {
        let mut cursor = Cursor::new(&mut buffer);
        entry.write(&mut cursor).unwrap();
    }

    let mut cursor = Cursor::new(&buffer);
    let entry2 = IndexEntry::read(&mut cursor).unwrap();

    assert_eq!(entry2.get_offset(), 12345);
    assert_eq!(entry2.get_length(), 67890);
    assert_eq!(entry2.get_white(), 0x12345);
    assert_eq!(entry2.get_black(), 0xABCDE);
    assert_eq!(entry2.get_event(), 0x1A2B3);
    assert_eq!(entry2.get_site(), 0x2B3C4);
    assert_eq!(entry2.get_round(), 0x3C4D);
    assert_eq!(entry2.get_year(), 2022);
    assert_eq!(entry2.get_month(), 12);
    assert_eq!(entry2.get_day(), 19);
    assert_eq!(entry2.get_result(), GameResult::White);
    assert_eq!(entry2.get_white_elo(), 2850);
    assert_eq!(entry2.get_black_elo(), 2700);
    assert_eq!(entry2.get_eco_code(), 0x1234);
}
