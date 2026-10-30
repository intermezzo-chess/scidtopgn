//! Finding twin games and marking them deleted.
//!
//! `tests/data/twins` was written by `pgnscid` from `tests/data/twins.pgn`.
//! Each pair of players there has its own case of twins, named by the
//! games' `Case` tags. The games SCID deletes with each set of options below
//! are SCID vs. PC's: `tests/data/twins.tcl` ran `sc_base duplicates` on
//! copies of the database with `tcscid`.

use std::path::{Path, PathBuf};

use scidtopgn::{parse_single_game, Database, DeleteTwin, Error, Index, IndexEntry, TwinOptions};
use tempfile::TempDir;

/// The number of games in `twins`.
const TWINS_GAMES: u32 = 61;

const SHORT: &str = "1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 Nf6 5. O-O Be7";
const LONG: &str = "1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 Nf6 5. O-O Be7 6. Re1 b5";

fn data_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data")
        .join(name)
}

/// Copies the fixture `name` into `dir`.
fn copy_of(dir: &Path, name: &str) -> PathBuf {
    let dst = dir.join(name);
    for ext in ["si4", "sn4", "sg4"] {
        std::fs::copy(data_path(name).with_extension(ext), dst.with_extension(ext)).unwrap();
    }
    dst
}

/// The bytes of the files of the database at `path`.
fn database_bytes(path: &Path) -> Vec<Vec<u8>> {
    ["si4", "sn4", "sg4"]
        .map(|ext| std::fs::read(path.with_extension(ext)).unwrap())
        .into()
}

fn gui() -> TwinOptions {
    TwinOptions::default()
}

/// Each set of options `twins.tcl` runs, and the games SCID deletes with it,
/// counting from 1.
fn scid_runs() -> Vec<(&'static str, TwinOptions, Vec<u32>)> {
    vec![
        (
            "gui",
            gui(),
            vec![1, 2, 4, 25, 26, 28, 32, 38, 44, 46, 48, 54, 55, 58, 59, 60],
        ),
        (
            "older",
            TwinOptions {
                delete: DeleteTwin::Older,
                ..gui()
            },
            vec![1, 2, 4, 26, 28, 32, 38, 44, 46, 48, 54, 55, 58, 60],
        ),
        (
            "newer",
            TwinOptions {
                delete: DeleteTwin::Newer,
                ..gui()
            },
            vec![
                2, 3, 5, 21, 23, 25, 27, 29, 33, 39, 45, 47, 49, 55, 56, 58, 59, 61,
            ],
        ),
        (
            "command",
            TwinOptions {
                players: false,
                event: true,
                skip_short: false,
                ..gui()
            },
            vec![
                1, 2, 4, 10, 25, 26, 28, 38, 44, 46, 48, 50, 54, 55, 58, 59, 60,
            ],
        ),
        (
            "placeholders",
            TwinOptions {
                skip_short: false,
                ..gui()
            },
            vec![
                1, 2, 4, 10, 17, 25, 26, 28, 32, 38, 44, 46, 48, 54, 55, 58, 59, 60,
            ],
        ),
        (
            "every",
            TwinOptions {
                event: true,
                day: true,
                result: true,
                skip_short: false,
                ..gui()
            },
            vec![1, 2, 4, 10, 25, 26, 28, 46, 48, 54, 55, 58, 59, 60],
        ),
        (
            "fewest",
            TwinOptions {
                players: false,
                colors: false,
                event: false,
                site: false,
                round: false,
                year: false,
                month: false,
                day: false,
                result: false,
                eco: false,
                moves: true,
                skip_short: false,
                comments: false,
                variations: false,
                copy_ratings: false,
                delete: DeleteTwin::Shorter,
            },
            vec![
                1, 2, 4, 6, 10, 17, 20, 22, 25, 26, 28, 32, 34, 36, 38, 40, 42, 44, 46, 48, 50, 54,
                55, 58, 59, 60,
            ],
        ),
        (
            "unprotected",
            TwinOptions {
                comments: false,
                variations: false,
                copy_ratings: true,
                ..gui()
            },
            vec![
                1, 2, 4, 20, 22, 25, 26, 28, 32, 38, 44, 46, 48, 54, 55, 58, 59, 60,
            ],
        ),
        (
            "nomoves",
            TwinOptions {
                moves: false,
                skip_short: false,
                ..gui()
            },
            vec![
                1, 2, 4, 8, 10, 12, 15, 17, 19, 25, 26, 28, 30, 32, 38, 44, 46, 48, 54, 55, 58, 59,
                60,
            ],
        ),
    ]
}

/// Runs `delete_twins` on a copy of `twins` and returns the games it
/// deleted, counting from 1, and the database.
fn delete_twins_in_copy(dir: &Path, options: &TwinOptions) -> (Vec<u32>, Database) {
    let mut db = Database::open(copy_of(dir, "twins")).unwrap();
    let deleted = db.delete_twins(options).unwrap();
    (deleted.iter().map(|n| n + 1).collect(), db)
}

fn delete_flags(db: &Database) -> Vec<u32> {
    (0..db.num_games())
        .filter(|&n| db.get_entry(n).unwrap().get_delete_flag())
        .map(|n| n + 1)
        .collect()
}

/// White's and Black's ratings of each game with one, counting from 1.
fn ratings(db: &Database) -> Vec<(u32, u16, u16)> {
    (0..db.num_games())
        .map(|n| (n, db.get_entry(n).unwrap()))
        .filter(|(_, e)| e.get_white_elo() != 0 || e.get_black_elo() != 0)
        .map(|(n, e)| (n + 1, e.get_white_elo(), e.get_black_elo()))
        .collect()
}

#[test]
fn test_deletes_the_games_scid_deletes() {
    for (name, options, expected) in scid_runs() {
        let dir = TempDir::new().unwrap();
        let (deleted, db) = delete_twins_in_copy(dir.path(), &options);
        assert_eq!(deleted, expected, "{name}");
        assert_eq!(delete_flags(&db), expected, "{name}");
    }
}

#[test]
fn test_compares_basic_eco_codes_as_scid_does() {
    // Games 46 and 47 have C78 and C78a, and games 48 and 49 C78 and C79.
    // SCID's result for games without an ECO code is undefined.
    let dir = TempDir::new().unwrap();
    let options = TwinOptions { eco: true, ..gui() };
    let (deleted, _) = delete_twins_in_copy(dir.path(), &options);
    let with_eco: Vec<u32> = deleted
        .into_iter()
        .filter(|n| (46..=49).contains(n))
        .collect();
    assert_eq!(with_eco, [46]);
}

#[test]
fn test_games_without_an_eco_code_match() {
    let dir = TempDir::new().unwrap();
    let options = TwinOptions { eco: true, ..gui() };
    let (deleted, _) = delete_twins_in_copy(dir.path(), &options);
    // Three equal copies without an ECO code.
    assert_eq!(deleted[..2], [1, 2]);
}

#[test]
fn test_copies_ratings_as_scid_does() {
    let dir = TempDir::new().unwrap();
    let before = ratings(&Database::open_read_only(data_path("twins")).unwrap());
    assert_eq!(before, [(26, 2500, 0), (27, 0, 2400)]);

    for (name, options, _) in scid_runs() {
        let (_, db) = delete_twins_in_copy(dir.path(), &options);
        let expected = if options.copy_ratings {
            [(26, 2500, 0), (27, 2500, 2400)]
        } else {
            [(26, 2500, 0), (27, 0, 2400)]
        };
        assert_eq!(ratings(&db), expected, "{name}");
    }
}

#[test]
fn test_copies_the_kind_of_rating() {
    let dir = TempDir::new().unwrap();
    let options = TwinOptions {
        copy_ratings: true,
        ..gui()
    };
    let (_, db) = delete_twins_in_copy(dir.path(), &options);
    let deleted = db.get_entry(25).unwrap();
    let kept = db.get_entry(26).unwrap();
    assert_eq!(
        kept.get_white_rating_type(),
        deleted.get_white_rating_type()
    );
}

/// `entry` as the index file stores it, without its delete flag.
fn undeleted_bytes(entry: &IndexEntry) -> Vec<u8> {
    let mut entry = entry.clone();
    entry.set_delete_flag(false);
    let mut bytes = Vec::new();
    entry.write(&mut bytes).unwrap();
    bytes
}

#[test]
fn test_changes_only_delete_flags() {
    let dir = TempDir::new().unwrap();
    let path = copy_of(dir.path(), "twins");
    let before = database_bytes(&path);
    let before_index = Index::open(path.with_extension("si4")).unwrap();

    let mut db = Database::open(&path).unwrap();
    let deleted = db.delete_twins(&gui()).unwrap();
    assert_eq!(deleted.len(), 16);
    db.close().unwrap();

    let after = database_bytes(&path);
    assert_eq!(after[1..], before[1..], "the name and game files");
    let after_index = Index::open(path.with_extension("si4")).unwrap();
    assert_eq!(after_index.num_games(), TWINS_GAMES);
    for (n, (old, new)) in (0u32..).zip(before_index.entries().iter().zip(after_index.entries())) {
        assert_eq!(undeleted_bytes(old), undeleted_bytes(new), "game {}", n + 1);
        assert_eq!(
            new.get_delete_flag(),
            deleted.contains(&n),
            "game {}",
            n + 1
        );
    }
}

#[test]
fn test_compact_games_removes_the_twins() {
    let dir = TempDir::new().unwrap();
    let (deleted, mut db) = delete_twins_in_copy(dir.path(), &gui());
    db.compact_games().unwrap();
    assert_eq!(db.num_games(), TWINS_GAMES - deleted.len() as u32);
    // Games 1 and 2 were deleted, so game 3 is the first.
    let first = db.get_game(0).unwrap();
    assert_eq!(
        first.find_tag("Case").map(String::as_str),
        Some("Game 3: Three equal copies")
    );
    assert_eq!(db.delete_twins(&gui()).unwrap(), []);
}

#[test]
fn test_skips_deleted_games() {
    let dir = TempDir::new().unwrap();
    let mut db = Database::open(copy_of(dir.path(), "twins")).unwrap();
    // The game that would be kept of three equal copies.
    db.delete_game(2).unwrap();
    let deleted = db.delete_twins(&gui()).unwrap();
    assert!(deleted.contains(&0));
    assert!(!deleted.contains(&1));
    assert!(!deleted.contains(&2));
}

#[test]
fn test_read_only_database_is_refused() {
    let mut db = Database::open_read_only(data_path("twins")).unwrap();
    assert!(matches!(db.delete_twins(&gui()), Err(Error::ReadOnly)));
}

#[test]
fn test_unreadable_record_changes_nothing() {
    let dir = TempDir::new().unwrap();
    let path = copy_of(dir.path(), "twins");
    // Game 5, the longer of two twins, starts past the end of the game file.
    let si4 = path.with_extension("si4");
    let mut index = Index::open(&si4).unwrap();
    index.get_entry_mut(4).unwrap().set_offset(1 << 20);
    index
        .flush(&mut std::fs::File::create(&si4).unwrap())
        .unwrap();
    let before = database_bytes(&path);

    let mut db = Database::open(&path).unwrap();
    let error = db.delete_twins(&gui()).unwrap_err();
    assert!(
        matches!(&error, Error::Game { number: 5, source } if matches!(**source, Error::GameOutOfRange { .. })),
        "{error:?}"
    );
    assert_eq!(delete_flags(&db), []);
    db.close().unwrap();
    assert_eq!(database_bytes(&path), before);
}

/// A game with `moves` between `white` and `black`.
fn game(white: &str, black: &str, moves: &str) -> scidtopgn::Game {
    parse_single_game(&format!(
        "[White \"{white}\"]\n[Black \"{black}\"]\n[Site \"Oslo\"]\n[Round \"1\"]\n\n{moves} *"
    ))
    .unwrap()
}

#[test]
fn test_finds_twins_not_yet_written() {
    let dir = TempDir::new().unwrap();
    let mut db = Database::create(dir.path().join("games")).unwrap();
    db.add_game(&game("Anand", "Carlsen", LONG)).unwrap();
    db.add_game(&game("Anand", "Carlsen", SHORT)).unwrap();
    db.add_game(&game("Anand", "Caruana", SHORT)).unwrap();
    assert_eq!(db.delete_twins(&gui()).unwrap(), [1]);
}

#[test]
fn test_player_prefixes_compare_utf8_names_by_bytes() {
    // The first 4 bytes of both names are the UTF-8 of "Öd" and the first
    // byte of "ö".
    let dir = TempDir::new().unwrap();
    let mut db = Database::create(dir.path().join("games")).unwrap();
    db.add_game(&game("Ödön", "Black", SHORT)).unwrap();
    db.add_game(&game("Ödöx", "Black", SHORT)).unwrap();
    let options = TwinOptions {
        players: false,
        ..gui()
    };
    assert_eq!(db.delete_twins(&options).unwrap(), [0]);
}

/// Writes a database whose players are "Ödön" and "Ödöx" stored as Latin-1,
/// and returns its path.
fn latin1_players(dir: &Path) -> PathBuf {
    let path = dir.join("latin1");
    let mut db = Database::create(&path).unwrap();
    db.add_game(&game("Qdqn", "Black", SHORT)).unwrap();
    db.add_game(&game("Qdqx", "Black", SHORT)).unwrap();
    db.close().unwrap();

    // "Qdqx" shares its first 3 bytes with "Qdqn", which the name file
    // stores whole.
    let sn4 = path.with_extension("sn4");
    let mut names = std::fs::read(&sn4).unwrap();
    let at = names.windows(4).position(|w| w == b"Qdqn").unwrap();
    names[at..at + 3].copy_from_slice(&[0xD6, b'd', 0xF6]);
    std::fs::write(&sn4, names).unwrap();
    path
}

#[test]
fn test_player_prefixes_compare_names_as_stored() {
    let dir = TempDir::new().unwrap();
    let mut db = Database::open(latin1_players(dir.path())).unwrap();
    let names = db.nfile().namebase().converted_names().len();
    assert_eq!(names, 2);

    // As Latin-1, the names differ in their fourth byte.
    let options = TwinOptions {
        players: false,
        ..gui()
    };
    assert_eq!(db.delete_twins(&options).unwrap(), []);

    // Writing the database stores the names as UTF-8.
    db.add_game(&game("Other", "Black", "1. d4 *")).unwrap();
    db.flush().unwrap();
    assert_eq!(db.delete_twins(&options).unwrap(), [0]);
}
