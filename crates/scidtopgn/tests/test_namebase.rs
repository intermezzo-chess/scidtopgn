use std::fs::File;
use std::io::BufReader;

use scidtopgn::{NFile, NameBase, NameType};

const TEST_DATA_PATH: &str = "../../tests/data";

#[test]
fn test_read_one_sn4() {
    let file = File::open(format!("{}/one.sn4", TEST_DATA_PATH)).expect("Failed to open one.sn4");
    let mut reader = BufReader::new(file);

    let mut namebase = NameBase::new();
    namebase
        .read_sn4(&mut reader)
        .expect("Failed to read namebase");

    // Validate counts
    assert_eq!(namebase.count(NameType::Player), 2);
    assert_eq!(namebase.count(NameType::Event), 1);
    assert_eq!(namebase.count(NameType::Site), 1);
    assert_eq!(namebase.count(NameType::Round), 1);

    // Validate names by ID
    assert_eq!(
        namebase.get_name(NameType::Player, 0),
        Some("Hossain, Enam")
    );
    assert_eq!(namebase.get_name(NameType::Player, 1), Some("Murshed, N"));
    assert_eq!(
        namebase.get_name(NameType::Event, 0),
        Some("47th ch-Bangahbandhu 2022")
    );
    assert_eq!(namebase.get_name(NameType::Site, 0), Some("Dhaka BAN"));
    assert_eq!(namebase.get_name(NameType::Round, 0), Some("5.6"));

    // Validate lookup by name
    assert_eq!(
        namebase.find_name(NameType::Player, "Hossain, Enam"),
        Some(0)
    );
    assert_eq!(namebase.find_name(NameType::Player, "Murshed, N"), Some(1));
    assert_eq!(
        namebase.find_name(NameType::Event, "47th ch-Bangahbandhu 2022"),
        Some(0)
    );
    assert_eq!(namebase.find_name(NameType::Site, "Dhaka BAN"), Some(0));
    assert_eq!(namebase.find_name(NameType::Round, "5.6"), Some(0));
}

#[test]
fn test_nfile_open() {
    let nfile = NFile::open(format!("{}/one.sn4", TEST_DATA_PATH)).expect("Failed to open one.sn4");
    let nb = nfile.namebase();

    assert_eq!(nb.count(NameType::Player), 2);
    assert_eq!(nb.get_name(NameType::Player, 0), Some("Hossain, Enam"));
    assert_eq!(nb.get_name(NameType::Player, 1), Some("Murshed, N"));
}

#[test]
fn test_roundtrip() {
    // Read the original file
    let file = File::open(format!("{}/one.sn4", TEST_DATA_PATH)).expect("Failed to open one.sn4");
    let mut reader = BufReader::new(file);

    let mut namebase1 = NameBase::new();
    namebase1
        .read_sn4(&mut reader)
        .expect("Failed to read namebase");

    // Write to a buffer
    let mut buffer = Vec::new();
    namebase1
        .write_sn4(&mut buffer)
        .expect("Failed to write namebase");

    // Read back from buffer
    let mut cursor = std::io::Cursor::new(&buffer);
    let mut namebase2 = NameBase::new();
    namebase2
        .read_sn4(&mut cursor)
        .expect("Failed to read namebase from buffer");

    // Compare counts
    assert_eq!(
        namebase1.count(NameType::Player),
        namebase2.count(NameType::Player)
    );
    assert_eq!(
        namebase1.count(NameType::Event),
        namebase2.count(NameType::Event)
    );
    assert_eq!(
        namebase1.count(NameType::Site),
        namebase2.count(NameType::Site)
    );
    assert_eq!(
        namebase1.count(NameType::Round),
        namebase2.count(NameType::Round)
    );

    // Compare names
    for ntype in [
        NameType::Player,
        NameType::Event,
        NameType::Site,
        NameType::Round,
    ] {
        for id in 0..namebase1.count(ntype) {
            assert_eq!(
                namebase1.get_name(ntype, id),
                namebase2.get_name(ntype, id),
                "Name mismatch for {:?} id={}",
                ntype,
                id
            );
        }
    }
}

#[test]
fn test_binary_exact_match() {
    // Read the original file as bytes
    let original =
        std::fs::read(format!("{}/one.sn4", TEST_DATA_PATH)).expect("Failed to read one.sn4");

    // Read it
    let mut namebase = NameBase::new();
    namebase
        .read_sn4(&mut std::io::Cursor::new(&original))
        .expect("Failed to read namebase");

    // Write it back
    let mut written = Vec::new();
    namebase
        .write_sn4(&mut written)
        .expect("Failed to write namebase");

    // Compare byte-by-byte
    assert_eq!(
        original.len(),
        written.len(),
        "File size mismatch: {} vs {}",
        original.len(),
        written.len()
    );
    assert_eq!(original, written, "Binary content mismatch");
}

#[test]
fn test_read_five_sn4() {
    let file = File::open(format!("{}/five.sn4", TEST_DATA_PATH)).expect("Failed to open five.sn4");
    let mut reader = BufReader::new(file);

    let mut namebase = NameBase::new();
    namebase
        .read_sn4(&mut reader)
        .expect("Failed to read namebase");

    // five.sn4 has more entries than one.sn4
    assert!(
        namebase.count(NameType::Player) >= 2,
        "Expected at least 2 players"
    );
    assert!(
        namebase.count(NameType::Event) >= 1,
        "Expected at least 1 event"
    );
}

#[test]
fn test_five_sn4_roundtrip() {
    let original =
        std::fs::read(format!("{}/five.sn4", TEST_DATA_PATH)).expect("Failed to read five.sn4");

    let mut namebase = NameBase::new();
    namebase
        .read_sn4(&mut std::io::Cursor::new(&original))
        .expect("Failed to read namebase");

    let mut written = Vec::new();
    namebase
        .write_sn4(&mut written)
        .expect("Failed to write namebase");

    assert_eq!(
        original.len(),
        written.len(),
        "File size mismatch: {} vs {}",
        original.len(),
        written.len()
    );
    assert_eq!(original, written, "Binary content mismatch");
}
