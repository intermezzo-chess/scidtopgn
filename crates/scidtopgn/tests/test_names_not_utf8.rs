//! Databases whose name files hold names that aren't valid UTF-8.
//!
//! `tests/data/latin1` was written by the C++ reference converter from
//! `tests/data/latin1.pgn`, whose names are Latin-1 and Windows-1252, and
//! stores them as they are. `tests/data/mixed` was written by the reference
//! converter from `tests/data/mixed.pgn`, which has "Müller, Hans" and
//! "Événement" both as Latin-1 and as UTF-8, so its name file holds each
//! name twice.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use scidtopgn::{parse_single_game, Database, Error, Index, NameBase, NameType};
use tempfile::TempDir;

const NAME_TYPES: [NameType; 4] = [
    NameType::Player,
    NameType::Event,
    NameType::Site,
    NameType::Round,
];

const NEW_GAME: &str = "[Event \"Rapid\"]\n[Site \"Oslo\"]\n[Round \"4\"]\n\
    [White \"Müller, Hans\"]\n[Black \"Carlsen, Magnus\"]\n\n1. Nf3 *\n";

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

/// The names of the files in `dir`, sorted.
fn files_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

fn all_pgn(db: &mut Database) -> Vec<String> {
    db.games().map(|game| game.unwrap().to_pgn()).collect()
}

/// The White, Black, Event, Site, and Round of each game.
fn game_names(db: &mut Database) -> Vec<[String; 5]> {
    db.games()
        .map(|game| {
            let game = game.unwrap();
            [game.white, game.black, game.event, game.site, game.round]
        })
        .collect()
}

/// Asserts that the database at `path` reads no name that isn't UTF-8, and
/// that, within each kind, its names are different, its IDs count from 0
/// without gaps, and every game refers to one of them.
fn assert_names_written_as_utf8(path: &Path) {
    let db = Database::open_read_only(path).unwrap();
    let nb = db.nfile().namebase();
    assert_eq!(nb.converted_names(), []);
    for ntype in NAME_TYPES {
        let names = nb.get_all_names(ntype);
        let ids: Vec<u32> = names.iter().map(|(id, _, _)| *id).collect();
        assert_eq!(ids, (0..nb.count(ntype)).collect::<Vec<_>>(), "{ntype}");
        let unique: HashSet<&String> = names.iter().map(|(_, name, _)| name).collect();
        assert_eq!(unique.len(), names.len(), "{ntype}");
    }
    let mut db = Database::open_read_only(path).unwrap();
    assert!(game_names(&mut db).iter().flatten().all(|name| name != "?"));
}

/// Asserts that each name's frequency in the database at `path` is the
/// number of times its games use it.
fn assert_frequencies_counted(path: &Path) {
    let db = Database::open_read_only(path).unwrap();
    let nb = db.nfile().namebase();
    for ntype in NAME_TYPES {
        let names = nb.get_all_names(ntype);
        let mut uses = vec![0; names.len()];
        for entry in db.index().entries() {
            let ids = match ntype {
                NameType::Player => vec![entry.get_white(), entry.get_black()],
                NameType::Event => vec![entry.get_event()],
                NameType::Site => vec![entry.get_site()],
                NameType::Round => vec![entry.get_round()],
            };
            for id in ids {
                uses[id as usize] += 1;
            }
        }
        let frequencies: Vec<u32> = names.iter().map(|(_, _, f)| *f).collect();
        assert_eq!(frequencies, uses, "{ntype}");
    }
}

fn names(names: [&str; 5]) -> [String; 5] {
    names.map(String::from)
}

#[test]
fn test_reading_converts_names_that_arent_utf8() {
    let path = data_path("latin1");
    let before = database_bytes(&path);
    let mut db = Database::open_read_only(&path).unwrap();

    assert_eq!(
        game_names(&mut db),
        [
            names([
                "Müller, Hans",
                "Anand, Viswanathan",
                "Événement",
                "Zürich",
                "1"
            ]),
            names(["Peña, José", "Šimek, Jan", "Événement", "Zürich", "2"]),
        ]
    );
    let converted = |ntype, name: &str| (ntype, name.to_string());
    assert_eq!(
        db.nfile().namebase().converted_names(),
        [
            converted(NameType::Player, "Müller, Hans"),
            converted(NameType::Player, "Peña, José"),
            converted(NameType::Player, "Šimek, Jan"),
            converted(NameType::Event, "Événement"),
            converted(NameType::Site, "Zürich"),
        ]
    );
    assert_eq!(database_bytes(&path), before);
}

#[test]
fn test_closing_without_changes_writes_nothing() {
    let temp = TempDir::new().unwrap();
    let path = copy_of(temp.path(), "mixed");
    let before = database_bytes(&path);

    Database::open(&path).unwrap().close().unwrap();
    assert_eq!(database_bytes(&path), before);
}

#[test]
fn test_appending_writes_the_names_as_utf8() {
    let temp = TempDir::new().unwrap();
    let path = copy_of(temp.path(), "latin1");
    let mut db = Database::open(&path).unwrap();
    let before = all_pgn(&mut db);

    db.add_game(&parse_single_game(NEW_GAME).unwrap()).unwrap();
    db.close().unwrap();

    assert_names_written_as_utf8(&path);
    let mut db = Database::open(&path).unwrap();
    let after = all_pgn(&mut db);
    assert_eq!(after[..2], before);
    // The new game uses the name read from Latin-1, whose stored frequency
    // is 0: the reference converter doesn't count names.
    let nb = db.nfile().namebase();
    assert_eq!(nb.count(NameType::Player), 5);
    let muller = nb.find_name(NameType::Player, "Müller, Hans").unwrap();
    assert_eq!(nb.get_frequency(NameType::Player, muller), 1);
    assert_eq!(
        files_in(temp.path()),
        ["latin1.sg4", "latin1.si4", "latin1.sn4"]
    );
}

#[test]
fn test_appending_merges_names_stored_two_ways() {
    let temp = TempDir::new().unwrap();
    let path = copy_of(temp.path(), "mixed");
    let mut db = Database::open(&path).unwrap();
    let before = all_pgn(&mut db);
    let nb = db.nfile().namebase();
    assert_eq!(nb.count(NameType::Player), 4);
    assert_eq!(nb.count(NameType::Event), 3);

    db.add_game(&parse_single_game(NEW_GAME).unwrap()).unwrap();
    db.close().unwrap();

    assert_names_written_as_utf8(&path);
    let mut db = Database::open(&path).unwrap();
    assert_eq!(all_pgn(&mut db)[..3], before);
    let nb = db.nfile().namebase();
    // Müller, Anand, Mueller, and Carlsen; Événement, Open, and Rapid.
    assert_eq!(nb.count(NameType::Player), 4);
    assert_eq!(nb.count(NameType::Event), 3);
    assert_eq!(
        files_in(temp.path()),
        ["mixed.sg4", "mixed.si4", "mixed.sn4"]
    );
}

#[test]
fn test_a_merged_names_frequency_is_the_sum_of_theirs() {
    let temp = TempDir::new().unwrap();
    let path = copy_of(temp.path(), "mixed");
    // Count the names, which the reference converter doesn't. The file
    // written holds "Müller, Hans" and "Événement" twice, now as UTF-8.
    let sn4 = path.with_extension("sn4");
    let mut nb = NameBase::new();
    nb.read_sn4(&mut std::fs::File::open(&sn4).unwrap())
        .unwrap();
    for entry in Database::open_read_only(&path).unwrap().index().entries() {
        nb.increment_frequency(NameType::Player, entry.get_white());
        nb.increment_frequency(NameType::Player, entry.get_black());
        nb.increment_frequency(NameType::Event, entry.get_event());
        nb.increment_frequency(NameType::Site, entry.get_site());
        nb.increment_frequency(NameType::Round, entry.get_round());
    }
    nb.write_sn4(&mut std::fs::File::create(&sn4).unwrap())
        .unwrap();

    let mut db = Database::open(&path).unwrap();
    assert_eq!(db.nfile().namebase().count(NameType::Player), 4);
    db.add_game(&parse_single_game(NEW_GAME).unwrap()).unwrap();
    db.close().unwrap();

    assert_names_written_as_utf8(&path);
    assert_frequencies_counted(&path);
    let db = Database::open(&path).unwrap();
    let nb = db.nfile().namebase();
    let muller = nb.find_name(NameType::Player, "Müller, Hans").unwrap();
    assert_eq!(nb.get_frequency(NameType::Player, muller), 4);
}

#[test]
fn test_compacting_merges_names_stored_two_ways() {
    let temp = TempDir::new().unwrap();
    let expected = all_pgn(&mut Database::open_read_only(data_path("mixed")).unwrap());

    let compact_games = |db: &mut Database| db.compact_games();
    let compact_names = |db: &mut Database| db.compact_names();
    for (dir, compact) in [
        (
            "games",
            &compact_games as &dyn Fn(&mut Database) -> scidtopgn::Result<()>,
        ),
        ("names", &compact_names),
    ] {
        let dir = temp.path().join(dir);
        std::fs::create_dir(&dir).unwrap();
        let path = copy_of(&dir, "mixed");
        let mut db = Database::open(&path).unwrap();

        compact(&mut db).unwrap();
        assert_eq!(all_pgn(&mut db), expected);
        assert_eq!(db.nfile().namebase().count(NameType::Player), 3);
        drop(db);

        assert_names_written_as_utf8(&path);
        assert_frequencies_counted(&path);
        assert_eq!(all_pgn(&mut Database::open(&path).unwrap()), expected);
    }
}

#[test]
fn test_a_merge_stops_at_a_name_the_name_file_lacks() {
    let temp = TempDir::new().unwrap();
    let path = copy_of(temp.path(), "mixed");
    let si4 = path.with_extension("si4");
    let mut index = Index::open(&si4).unwrap();
    index.get_entry_mut(1).unwrap().set_white(99);
    index
        .flush(&mut std::fs::File::create(&si4).unwrap())
        .unwrap();
    let before = database_bytes(&path);

    let mut db = Database::open(&path).unwrap();
    db.add_game(&parse_single_game(NEW_GAME).unwrap()).unwrap();
    let err = db.close().unwrap_err();

    match err {
        Error::Game { number: 2, source } => assert!(
            matches!(
                *source,
                Error::NameNotFound {
                    name_type: NameType::Player,
                    id: 99
                }
            ),
            "{source:?}"
        ),
        err => panic!("{err:?}"),
    }
    let after = database_bytes(&path);
    assert_eq!(after[..2], before[..2]);
    assert_eq!(
        files_in(temp.path()),
        ["mixed.sg4", "mixed.si4", "mixed.sn4"]
    );
}
