use scidtopgn::{
    Index, IndexEntry, INDEX_ENTRY_SIZE, INDEX_HEADER_SIZE, INDEX_MAGIC, SCID_VERSION,
};
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
            .unwrap_or_else(|| panic!("Entry {} should exist", i));
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

#[test]
fn test_index_full() {
    use scidtopgn::{Error, MAX_GAMES};

    // Sets the count instead of adding millions of entries.
    let mut index = Index::new();
    index.header_mut().num_games = MAX_GAMES - 1;
    index.add_entry(IndexEntry::new()).unwrap();
    assert_eq!(index.num_games(), MAX_GAMES);

    match index.add_entry(IndexEntry::new()) {
        Err(error @ Error::IndexFull) => assert_eq!(
            error.to_string(),
            "The index is full: a SCID database holds at most 16777214 games"
        ),
        other => panic!("expected a full index, got {other:?}"),
    }
    assert_eq!(index.num_games(), MAX_GAMES);
    assert_eq!(MAX_GAMES, 16_777_214, "SCID's limit");
}

#[test]
fn test_index_description() {
    use scidtopgn::index::SCID_DESC_LENGTH;

    let mut index = Index::default();
    assert!(!index.is_dirty());
    assert_eq!(index.description(), "");

    index.set_description("My games");
    assert!(index.is_dirty());
    assert_eq!(index.description(), "My games");
    assert_eq!(index.header().description_str(), "My games");

    // A description is cut to SCID_DESC_LENGTH bytes, and a shorter one
    // replaces all of it.
    index.set_description(&"x".repeat(200));
    assert_eq!(index.description(), "x".repeat(SCID_DESC_LENGTH));
    index.set_description("Short");
    assert_eq!(index.description(), "Short");
}

#[test]
fn test_write_entry_rewrites_one_entry() {
    use scidtopgn::Error;
    use std::io::Cursor;

    let mut index = Index::new();
    for offset in [10, 20] {
        let mut entry = IndexEntry::new();
        entry.set_offset(offset);
        index.add_entry(entry).unwrap();
    }
    let mut file = Cursor::new(Vec::new());
    index.flush(&mut file).unwrap();
    assert!(!index.is_dirty());

    index.get_entry_mut(1).unwrap().set_delete_flag(true);
    index.write_entry(&mut file, 1).unwrap();
    assert!(matches!(
        index.write_entry(&mut file, 2),
        Err(Error::GameNotFound { number: 3 })
    ));

    let temp = tempfile::TempDir::new().unwrap();
    let path = temp.path().join("index.si4");
    std::fs::write(&path, file.into_inner()).unwrap();
    let reread = Index::open(&path).unwrap();
    let flags: Vec<_> = reread
        .entries()
        .iter()
        .map(|e| e.get_delete_flag())
        .collect();
    assert_eq!(flags, [false, true]);
    assert_eq!(reread.get_entry(1).unwrap().get_offset(), 20);
}

/// `entry` written and read back.
fn reread(entry: &IndexEntry) -> IndexEntry {
    let mut bytes = Vec::new();
    entry.write(&mut bytes).unwrap();
    assert_eq!(bytes.len(), INDEX_ENTRY_SIZE);
    IndexEntry::read(&mut bytes.as_slice()).unwrap()
}

#[test]
fn test_entry_flags() {
    use scidtopgn::index::{IDX_MASK_DELETE, IDX_MASK_PROMO, IDX_MASK_USER};

    let mut entry = IndexEntry::new();
    entry.set_delete_flag(true);
    entry.set_flag(IDX_MASK_PROMO | IDX_MASK_USER, true);
    let read = reread(&entry);
    assert!(read.get_delete_flag());
    assert!(read.get_promotions_flag());
    assert!(read.get_flag(IDX_MASK_USER));
    assert!(!read.get_start_flag());

    entry.set_flag(IDX_MASK_DELETE | IDX_MASK_USER, false);
    let read = reread(&entry);
    assert!(!read.get_delete_flag());
    assert!(!read.get_flag(IDX_MASK_USER));
    assert!(read.get_promotions_flag());
}

#[test]
fn test_entry_ratings() {
    let mut entry = IndexEntry::new();
    entry.set_white_elo(2850);
    entry.set_black_elo(5000);
    entry.set_white_rating_type(3);
    entry.set_black_rating_type(15);
    let read = reread(&entry);
    assert_eq!(read.get_white_elo(), 2850);
    // Ratings above MAX_ELO (4000) are stored as 4000.
    assert_eq!(read.get_black_elo(), 4000);
    assert_eq!(read.get_white_rating_type(), 3);
    assert_eq!(read.get_black_rating_type(), 15);
}

#[test]
fn test_read_entry_limits_ratings() {
    // Ratings stored above 4000, as 4095, read as 4000.
    let mut bytes = Vec::new();
    IndexEntry::new().write(&mut bytes).unwrap();
    bytes[29..33].copy_from_slice(&[0x0F, 0xFF, 0x0F, 0xFF]);
    let entry = IndexEntry::read(&mut bytes.as_slice()).unwrap();
    assert_eq!(entry.get_white_elo(), 4000);
    assert_eq!(entry.get_black_elo(), 4000);
}

#[test]
fn test_entry_counts() {
    // SCID stores each count in 4 bits, exactly up to 10, and then as 15,
    // 20, 30, 40, or 50 for "50 or more".
    for (count, stored) in [
        (0, 0),
        (10, 10),
        (12, 10),
        (13, 15),
        (17, 15),
        (18, 20),
        (24, 20),
        (25, 30),
        (34, 30),
        (35, 40),
        (44, 40),
        (45, 50),
        (1000, 50),
    ] {
        let mut entry = IndexEntry::new();
        entry.set_variation_count(count);
        entry.set_comment_count(count);
        entry.set_nag_count(count);
        let read = reread(&entry);
        assert_eq!(read.get_variation_count(), stored, "{count}");
        assert_eq!(read.get_comment_count(), stored, "{count}");
        assert_eq!(read.get_nag_count(), stored, "{count}");
    }
}

#[test]
fn test_entry_material_and_pawn_signatures() {
    let mut entry = IndexEntry::new();
    entry.set_final_mat_sig(0x00AB_CDEF);
    entry.set_stored_line_code(0x12);
    // The top two bits of the first byte hold other data.
    let pawns = [0x3F, 1, 2, 3, 4, 5, 6, 7, 8];
    entry.set_home_pawn_data(&pawns);
    let read = reread(&entry);
    assert_eq!(read.get_final_mat_sig(), 0x00AB_CDEF);
    assert_eq!(read.get_stored_line_code(), 0x12);
    assert_eq!(read.get_home_pawn_data(), &pawns);

    // The two fields share four bytes without overlapping.
    entry.set_final_mat_sig(0xFFFF_FFFF);
    assert_eq!(entry.get_final_mat_sig(), 0x00FF_FFFF);
    assert_eq!(entry.get_stored_line_code(), 0x12);
}

#[test]
fn test_entry_half_move_count_keeps_10_bits() {
    let pawns = [0x3F, 1, 2, 3, 4, 5, 6, 7, 8];
    for (count, stored) in [
        (0, 0),
        (255, 255),
        (256, 256),
        (300, 300),
        (1023, 1023),
        (1024, 0),
        (1030, 6),
    ] {
        let mut entry = IndexEntry::new();
        entry.set_home_pawn_data(&pawns);
        entry.set_num_half_moves(count);
        assert_eq!(entry.get_num_half_moves(), stored, "{count}");
        let read = reread(&entry);
        assert_eq!(read.get_num_half_moves(), stored, "{count}");
        assert_eq!(read.get_home_pawn_data(), &pawns, "{count}");
    }
}

#[test]
fn test_entry_half_move_count_above_255_survives_a_rewrite() {
    // A count of 300 as SCID stores it: 44 in byte 37, and 1 in the top two
    // bits of byte 38, the first home pawn byte.
    let mut bytes = vec![0; INDEX_ENTRY_SIZE];
    bytes[37] = 44;
    bytes[38] = (1 << 6) | 3;
    let entry = IndexEntry::read(&mut bytes.as_slice()).unwrap();
    assert_eq!(entry.get_num_half_moves(), 300);
    assert_eq!(entry.get_home_pawn_data()[0], 3);

    let mut written = Vec::new();
    entry.write(&mut written).unwrap();
    assert_eq!(written, bytes);
}

#[test]
fn test_event_date_must_be_near_the_game_date() {
    use scidtopgn::date_make;

    // SCID stores the event year as an offset of -3 to +3 from the game's
    // year. Other event dates aren't stored.
    for (year, stored) in [(2019, true), (2025, true), (2018, false), (2026, false)] {
        let mut entry = IndexEntry::new();
        entry.set_date(date_make(2022, 6, 1));
        entry.set_event_date(date_make(year, 5, 20));
        let expected = if stored { date_make(year, 5, 20) } else { 0 };
        assert_eq!(reread(&entry).get_event_date(), expected, "{year}");
    }
}

#[test]
fn test_eco_codes() {
    use scidtopgn::index::{eco_from_string, eco_to_string};

    for (text, written) in [
        ("A00", "A00"),
        ("E99", "E99"),
        ("b91", "B91"),
        ("B91a", "B91a"),
        ("B91a3", "B91a3"),
        ("B91z4", "B91z4"),
        // Partial codes are filled with zeros.
        ("C", "C00"),
        ("C1", "C10"),
        // A fourth character that isn't a lowercase letter, or a fifth
        // that isn't 1 to 4, is ignored.
        ("C12X", "C12"),
        ("C12a9", "C12a"),
    ] {
        let code = eco_from_string(text);
        assert_eq!(eco_to_string(code).as_deref(), Some(written), "{text}");
    }
    for text in ["", "F00", "Cx", "C1x"] {
        assert_eq!(eco_from_string(text), 0, "{text}");
    }
    assert_eq!(eco_to_string(0), None);
}

#[test]
fn test_entry_init_clears_every_field() {
    use scidtopgn::{date_make, GameResult};

    let mut entry = IndexEntry::new();
    entry.set_offset(1);
    entry.set_length(2);
    entry.set_white(3);
    entry.set_event(4);
    entry.set_date(date_make(2022, 1, 2));
    entry.set_result(GameResult::Draw);
    entry.set_white_elo(2000);
    entry.set_eco_code(5);
    entry.set_delete_flag(true);
    entry.set_final_mat_sig(6);
    entry.set_home_pawn_data(&[1; 9]);
    entry.init();

    let mut cleared = Vec::new();
    entry.write(&mut cleared).unwrap();
    let mut new = Vec::new();
    IndexEntry::new().write(&mut new).unwrap();
    assert_eq!(cleared, new);
}
