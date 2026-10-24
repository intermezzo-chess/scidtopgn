//! Errors for damaged databases, damaged game records, and games too large to
//! store.

use std::error::Error as _;
use std::path::{Path, PathBuf};

use scidtopgn::index::INDEX_HEADER_SIZE;
use scidtopgn::{
    parse_single_game, ByteBuffer, CorruptGameKind, Database, DatabaseFile, Error, GFile, Game,
    NameBase, NameType, ENCODE_END_GAME, ENCODE_END_MARKER, ENCODE_START_MARKER, GAME_DECODE_ALL,
    GF_BLOCKSIZE,
};
use tempfile::TempDir;

/// 1.e4 and 1.d4 from the standard starting position.
const E4: u8 = 0xCF;
const D4: u8 = 0xBF;

/// Copies the `one` test database into `dir` and returns its path.
fn copy_one(dir: &Path) -> PathBuf {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/data/one");
    let dst = dir.join("one");
    for ext in ["si4", "sn4", "sg4"] {
        std::fs::copy(src.with_extension(ext), dst.with_extension(ext)).unwrap();
    }
    dst
}

/// Applies `change` to the bytes of `db`'s file with extension `ext`.
fn edit(db: &Path, ext: &str, change: impl FnOnce(&mut Vec<u8>)) {
    let path = db.with_extension(ext);
    let mut bytes = std::fs::read(&path).unwrap();
    change(&mut bytes);
    std::fs::write(&path, bytes).unwrap();
}

/// Sets the offset and length of the first game's index entry.
fn set_first_entry(db: &Path, offset: u32, length: u16) {
    edit(db, "si4", |bytes| {
        let entry = INDEX_HEADER_SIZE;
        bytes[entry..entry + 4].copy_from_slice(&offset.to_be_bytes());
        bytes[entry + 4..entry + 6].copy_from_slice(&length.to_be_bytes());
        // The high bit of this byte is bit 16 of the length.
        bytes[entry + 6] &= 0x7F;
    });
}

/// The error from opening `db`, which should be in `file`.
fn open_error(db: &Path, file: DatabaseFile) -> Error {
    match Database::open(db) {
        Err(Error::File { file: f, source }) if f == file => *source,
        Err(other) => panic!("expected an error in the {file}, got {other:?}"),
        Ok(_) => panic!("expected an error in the {file}"),
    }
}

/// The error from reading `db`'s first game.
fn first_game_error(db: &Path) -> Error {
    let mut db = Database::open(db).unwrap();
    match db.get_game(0) {
        Err(Error::Game { number: 1, source }) => *source,
        other => panic!("expected an error for game 1, got {other:?}"),
    }
}

fn decode(record: &[u8]) -> scidtopgn::Result<Game> {
    let mut game = Game::new();
    game.decode(&mut ByteBuffer::from_slice(record), GAME_DECODE_ALL)?;
    Ok(game)
}

fn corrupt_kind(record: &[u8]) -> (usize, CorruptGameKind) {
    match decode(record) {
        Err(Error::CorruptGame { offset, kind }) => (offset, kind),
        other => panic!("expected damaged game data, got {other:?}"),
    }
}

#[test]
fn test_open_reports_which_file_is_missing() {
    let temp = TempDir::new().unwrap();
    let db = copy_one(temp.path());
    std::fs::remove_file(db.with_extension("sg4")).unwrap();

    let error = open_error(&db, DatabaseFile::Game);
    assert!(
        matches!(&error, Error::Io(e) if e.kind() == std::io::ErrorKind::NotFound),
        "{error:?}"
    );
}

#[test]
fn test_open_rejects_wrong_magic_numbers() {
    for (ext, file) in [("si4", DatabaseFile::Index), ("sn4", DatabaseFile::Name)] {
        let temp = TempDir::new().unwrap();
        let db = copy_one(temp.path());
        edit(&db, ext, |bytes| bytes[0] = b'X');

        assert!(
            matches!(open_error(&db, file), Error::InvalidMagic),
            "{ext}"
        );
    }
}

#[test]
fn test_open_reports_truncated_files() {
    for (ext, file) in [("si4", DatabaseFile::Index), ("sn4", DatabaseFile::Name)] {
        let temp = TempDir::new().unwrap();
        let db = copy_one(temp.path());
        edit(&db, ext, |bytes| bytes.truncate(bytes.len() - 3));

        assert!(matches!(open_error(&db, file), Error::Truncated), "{ext}");
    }
}

#[test]
fn test_file_error_message_names_the_file() {
    let temp = TempDir::new().unwrap();
    let db = copy_one(temp.path());
    edit(&db, "si4", |bytes| bytes[0] = b'X');

    let error = Database::open(&db).err().unwrap();
    assert_eq!(error.to_string(), "SCID index file (.si4)");
    assert_eq!(
        error.source().unwrap().to_string(),
        "Not a SCID file: wrong magic number"
    );
}

#[test]
fn test_get_game_numbers_missing_games_from_1() {
    let mut db = Database::open(Path::new("../../tests/data/one")).unwrap();
    assert!(matches!(
        db.get_game(1),
        Err(Error::GameNotFound { number: 2 })
    ));
}

#[test]
fn test_get_game_rejects_a_zero_length_record() {
    let temp = TempDir::new().unwrap();
    let db = copy_one(temp.path());
    set_first_entry(&db, 0, 0);

    assert!(matches!(
        first_game_error(&db),
        Error::CorruptGame {
            offset: 0,
            kind: CorruptGameKind::UnexpectedEnd
        }
    ));
}

#[test]
fn test_get_game_rejects_a_record_past_the_end_of_the_file() {
    let temp = TempDir::new().unwrap();
    let db = copy_one(temp.path());
    let size = std::fs::metadata(db.with_extension("sg4")).unwrap().len() as u32;
    set_first_entry(&db, size - 1, 2);

    assert!(matches!(
        first_game_error(&db),
        Error::GameOutOfRange { length: 2, .. }
    ));
}

#[test]
fn test_read_game_rejects_a_record_across_blocks() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("blocks");
    let mut gfile = GFile::create(&path).unwrap();
    gfile.add_game_data(&vec![0; GF_BLOCKSIZE - 10]).unwrap();
    let second = gfile.add_game_data(&[0; 100]).unwrap();
    assert_eq!(second as usize, GF_BLOCKSIZE);
    gfile.flush().unwrap();

    let mut buf = ByteBuffer::new();
    let offset = GF_BLOCKSIZE as u32 - 5;
    assert!(matches!(
        gfile.read_game(&mut buf, offset, 10),
        Err(Error::GameSpansBlocks { length: 10, .. })
    ));
}

#[test]
fn test_name_prefix_longer_than_the_name() {
    let mut sn4 = b"Scid.sn\0".to_vec();
    sn4.extend([0; 4]); // timestamp
    sn4.extend([0, 0, 2]); // two player names
    sn4.extend([0; 9]); // no other names
    sn4.extend([0; 12]); // maximum frequencies below 256
    sn4.extend([0, 0, 0, 1, b'a']); // id 0, frequency 0, length 1, "a"
    sn4.extend([0, 1, 0, 1, 5]); // id 1, frequency 0, length 1, prefix 5

    let error = NameBase::new().read_sn4(&mut sn4.as_slice()).unwrap_err();
    assert!(
        matches!(
            error,
            Error::NamePrefixTooLong {
                name_type: NameType::Player,
                prefix: 5,
                length: 1
            }
        ),
        "{error:?}"
    );
}

#[test]
fn test_add_game_rejects_a_game_too_large_to_store() {
    let comment = "x".repeat(140_000);
    let game = parse_single_game(&format!("[Event \"x\"]\n\n1. e4 {{{comment}}} *\n")).unwrap();

    let temp = TempDir::new().unwrap();
    let mut db = Database::create(temp.path().join("large")).unwrap();
    match db.add_game(&game) {
        Err(Error::GameTooLarge { length, max }) => {
            assert!(length > 140_000);
            assert_eq!(max, (1 << 17) - 1);
        }
        other => panic!("expected a game too large error, got {other:?}"),
    }
    assert_eq!(db.num_games(), 0);
}

#[test]
fn test_game_file_rejects_a_record_larger_than_a_block() {
    let temp = TempDir::new().unwrap();
    let mut gfile = GFile::create(temp.path().join("large")).unwrap();
    assert!(matches!(
        gfile.add_game_data(&vec![0; GF_BLOCKSIZE + 1]),
        Err(Error::GameTooLarge {
            max: GF_BLOCKSIZE,
            ..
        })
    ));
}

#[test]
fn test_damaged_game_records() {
    // Each record is the end of the tags, the flags byte, then moves.
    let cases = [
        (vec![0, 0, E4], 3, CorruptGameKind::UnexpectedEnd),
        (
            vec![0, 0, ENCODE_START_MARKER, E4, ENCODE_END_MARKER],
            2,
            CorruptGameKind::VariationBeforeMove,
        ),
        (
            vec![0, 0, E4, ENCODE_END_MARKER],
            3,
            CorruptGameKind::UnmatchedEndMarker,
        ),
        (
            vec![0, 0, E4, ENCODE_START_MARKER, D4, ENCODE_END_GAME],
            5,
            CorruptGameKind::UnclosedVariation,
        ),
    ];
    for (record, offset, kind) in cases {
        assert_eq!(corrupt_kind(&record), (offset, kind), "{record:?}");
    }
}

#[test]
fn test_invalid_move_byte_gives_its_offset() {
    // After 1.e4, 0xC2 is Black's e-pawn capturing on an empty square.
    let (offset, kind) = corrupt_kind(&[0, 0, E4, 0xC2, ENCODE_END_GAME]);
    assert_eq!(offset, 3);
    assert!(
        matches!(kind, CorruptGameKind::InvalidMove { byte: 0xC2, .. }),
        "{kind:?}"
    );
}
