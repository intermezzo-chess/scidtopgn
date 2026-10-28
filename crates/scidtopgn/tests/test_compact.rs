use std::path::{Path, PathBuf};

use scidtopgn::{
    parse_single_game, Database, Error, Index, NameBase, NameType, GF_BLOCKSIZE, INDEX_ENTRY_SIZE,
    INDEX_HEADER_SIZE,
};

const NAME_TYPES: [NameType; 4] = [
    NameType::Player,
    NameType::Event,
    NameType::Site,
    NameType::Round,
];

const NEW_GAME: &str = "[Event \"Rapid\"]\n[Site \"Oslo\"]\n[Date \"2024.05.01\"]\n\
    [Round \"7\"]\n[White \"Kasparov, Garry\"]\n[Black \"Karpov, Anatoly\"]\n\
    [Result \"1-0\"]\n\n1. c4 e5 1-0\n";

fn get_test_data_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/data")
        .join(name)
}

/// Copies the fixture `name` into `dir`.
fn copy_of(dir: &Path, name: &str) -> PathBuf {
    let src = get_test_data_path(name);
    let dst = dir.join(name);
    for ext in ["si4", "sn4", "sg4"] {
        std::fs::copy(src.with_extension(ext), dst.with_extension(ext)).unwrap();
    }
    dst
}

/// Changes the index file of the database at `path` with `change`.
fn change_index(path: &Path, change: impl FnOnce(&mut Index)) {
    let si4 = path.with_extension("si4");
    let mut index = Index::open(&si4).unwrap();
    change(&mut index);
    index
        .flush(&mut std::fs::File::create(&si4).unwrap())
        .unwrap();
}

/// Copies `five` into `dir` and sets the logical delete flag on the `deleted`
/// games.
fn five_with_deleted(dir: &Path, deleted: &[u32]) -> PathBuf {
    let path = copy_of(dir, "five");
    change_index(&path, |index| {
        for &n in deleted {
            index.get_entry_mut(n).unwrap().set_delete_flag(true);
        }
    });
    path
}

fn all_pgn(db: &mut Database) -> Vec<String> {
    db.games().map(|game| game.unwrap().to_pgn()).collect()
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

/// The names of the three files of the database `name`.
fn database_files(name: &str) -> Vec<String> {
    ["sg4", "si4", "sn4"]
        .map(|ext| format!("{name}.{ext}"))
        .into()
}

/// Every name of every kind, with its ID and frequency.
fn all_names(db: &Database) -> Vec<Vec<(u32, String, u32)>> {
    let nb = db.nfile().namebase();
    NAME_TYPES.map(|ntype| nb.get_all_names(ntype)).into()
}

/// The raw index entries of the database at `path`.
fn raw_entries(path: &Path) -> Vec<Vec<u8>> {
    let si4 = std::fs::read(path.with_extension("si4")).unwrap();
    si4[INDEX_HEADER_SIZE..]
        .chunks(INDEX_ENTRY_SIZE)
        .map(<[u8]>::to_vec)
        .collect()
}

/// Asserts that every name's frequency is the number of times the games of
/// `db` use it.
fn assert_frequencies_counted(db: &Database) {
    let nb = db.nfile().namebase();
    let mut counted = all_names(db);
    for names in &mut counted {
        for (_, _, frequency) in names.iter_mut() {
            *frequency = 0;
        }
    }
    for entry in db.index().entries() {
        for (ntype, id) in [
            (NameType::Player, entry.get_white()),
            (NameType::Player, entry.get_black()),
            (NameType::Event, entry.get_event()),
            (NameType::Site, entry.get_site()),
            (NameType::Round, entry.get_round()),
        ] {
            let names = &mut counted[ntype.as_usize()];
            let name = names.iter_mut().find(|(n, _, _)| *n == id).unwrap();
            name.2 += 1;
        }
    }
    assert_eq!(all_names(db), counted);
    for ntype in NAME_TYPES {
        let max = counted[ntype.as_usize()]
            .iter()
            .map(|(_, _, f)| *f)
            .max()
            .unwrap_or(0);
        assert_eq!(nb.max_frequency(ntype), max, "{ntype}");
    }
}

/// The total length of the records of `db`'s games.
fn record_bytes(db: &Database) -> u64 {
    db.index()
        .entries()
        .iter()
        .map(|e| u64::from(e.get_length()))
        .sum()
}

fn game_size(path: &Path) -> u64 {
    std::fs::metadata(path.with_extension("sg4")).unwrap().len()
}

#[test]
fn test_compact_games_removes_deleted_games() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = five_with_deleted(temp.path(), &[1, 3]);
    let stored = all_pgn(&mut Database::open(get_test_data_path("five")).unwrap());
    let kept = [stored[0].clone(), stored[2].clone(), stored[4].clone()];
    let entries = raw_entries(&path);
    let mut db = Database::open(&path).unwrap();
    let names = all_names(&db);

    db.compact_games().unwrap();

    assert_eq!(db.num_games(), 3);
    assert_eq!(all_pgn(&mut db), kept);
    assert!(db.index().entries().iter().all(|e| !e.get_delete_flag()));
    assert_frequencies_counted(&db);
    let count = |names: &[Vec<(u32, String, u32)>]| names.iter().map(Vec::len).sum::<usize>();
    assert_eq!(count(&all_names(&db)), count(&names), "no name is dropped");
    assert_eq!(game_size(&path), record_bytes(&db));
    assert_eq!(files_in(temp.path()), database_files("five"));

    // Each entry is copied unchanged apart from its offset, its first 4 bytes.
    let after = raw_entries(&path);
    for (new, old) in after.iter().zip([0, 2, 4]) {
        assert_eq!(new[4..], entries[old][4..], "game {old}");
    }

    drop(db);
    let mut db = Database::open(&path).unwrap();
    assert_eq!(all_pgn(&mut db), kept);
    assert_frequencies_counted(&db);
}

#[test]
fn test_compact_games_removes_replaced_records_and_writes_pending_changes() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = copy_of(temp.path(), "five");
    let mut db = Database::open(&path).unwrap();
    db.update_game(0, &parse_single_game(NEW_GAME).unwrap())
        .unwrap();
    db.add_game(&parse_single_game(NEW_GAME).unwrap()).unwrap();
    let before = all_pgn(&mut db);
    assert!(db.gfile().file_size() > record_bytes(&db));

    db.compact_games().unwrap();

    assert_eq!(all_pgn(&mut db), before);
    assert_eq!(game_size(&path), record_bytes(&db));
    assert_frequencies_counted(&db);
    db.discard().unwrap();

    let mut db = Database::open(&path).unwrap();
    assert_eq!(db.num_games(), 6);
    assert_eq!(all_pgn(&mut db), before);
    assert_eq!(game_size(&path), record_bytes(&db));
}

/// The inodes of the files of the database at `path`, which change when a
/// file is replaced.
#[cfg(unix)]
fn inodes(path: &Path) -> Vec<u64> {
    use std::os::unix::fs::MetadataExt;

    ["si4", "sn4", "sg4"]
        .map(|ext| std::fs::metadata(path.with_extension(ext)).unwrap().ino())
        .into()
}

#[test]
fn test_compacting_a_database_scid_wrote_changes_nothing() {
    for name in ["derived", "accents", "long"] {
        let temp = tempfile::TempDir::new().unwrap();
        let path = copy_of(temp.path(), name);
        let before = database_bytes(&path);

        #[cfg(unix)]
        let before_inodes = inodes(&path);
        let mut db = Database::open(&path).unwrap();
        db.compact_names().unwrap();
        assert_eq!(database_bytes(&path), before, "{name}: names");
        #[cfg(unix)]
        assert_eq!(inodes(&path), before_inodes, "{name}: no file is replaced");
        db.compact_games().unwrap();
        assert_eq!(database_bytes(&path), before, "{name}: games");
        assert_eq!(files_in(temp.path()), database_files(name));
    }
}

/// A database whose second record starts in the first block and ends in
/// the second, as SCID never writes but reads.
fn database_with_a_record_across_blocks(dir: &Path) -> (PathBuf, Vec<String>) {
    let path = dir.join("across");
    let mut db = Database::create(&path).unwrap();
    for white in ["A", "B"] {
        let comment = "x".repeat(70_000);
        let pgn = format!("[White \"{white}\"]\n\n1. e4 {{{comment}}} *\n");
        db.add_game(&parse_single_game(&pgn).unwrap()).unwrap();
    }
    db.close().unwrap();

    let mut db = Database::open(&path).unwrap();
    let sg4 = std::fs::read(path.with_extension("sg4")).unwrap();
    let records: Vec<&[u8]> = db
        .index()
        .entries()
        .iter()
        .map(|e| &sg4[e.get_offset() as usize..][..e.get_length() as usize])
        .collect();
    let records = records.concat();
    let first = db.get_entry(0).unwrap().get_length();
    let pgn = all_pgn(&mut db);
    drop(db);

    std::fs::write(path.with_extension("sg4"), records).unwrap();
    change_index(&path, |index| {
        index.get_entry_mut(1).unwrap().set_offset(first);
    });
    (path, pgn)
}

#[test]
fn test_compact_games_moves_a_record_across_blocks_to_the_next_block() {
    let temp = tempfile::TempDir::new().unwrap();
    let (path, pgn) = database_with_a_record_across_blocks(temp.path());
    let mut db = Database::open(&path).unwrap();
    assert_eq!(all_pgn(&mut db), pgn);
    let first = db.get_entry(0).unwrap().get_length() as usize;
    let second = db.get_entry(1).unwrap().get_length() as u64;
    assert!(first + second as usize > GF_BLOCKSIZE);

    db.compact_games().unwrap();

    assert_eq!(db.get_entry(1).unwrap().get_offset() as usize, GF_BLOCKSIZE);
    assert_eq!(game_size(&path), GF_BLOCKSIZE as u64 + second);
    let sg4 = std::fs::read(path.with_extension("sg4")).unwrap();
    assert!(sg4[first..GF_BLOCKSIZE].iter().all(|&b| b == 0), "padding");
    assert_eq!(all_pgn(&mut db), pgn);
    drop(db);
    assert_eq!(all_pgn(&mut Database::open(&path).unwrap()), pgn);
}

#[test]
fn test_compacting_a_database_with_every_game_deleted_empties_it() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = five_with_deleted(temp.path(), &[0, 1, 2, 3, 4]);
    let mut db = Database::open(&path).unwrap();

    db.compact_games().unwrap();
    assert_eq!(db.num_games(), 0);
    assert_eq!(game_size(&path), 0);
    assert_frequencies_counted(&db);
    assert!(db.nfile().namebase().count(NameType::Player) > 0);

    db.compact_names().unwrap();
    drop(db);
    let db = Database::open(&path).unwrap();
    assert_eq!(db.num_games(), 0);
    for ntype in NAME_TYPES {
        assert_eq!(db.nfile().namebase().count(ntype), 0, "{ntype}");
    }
    assert_eq!(files_in(temp.path()), database_files("five"));
}

#[test]
fn test_compact_games_keeps_the_index_header() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = five_with_deleted(temp.path(), &[0]);
    change_index(&path, |index| {
        let header = index.header_mut();
        header.base_type = 3;
        header.auto_load = 4;
        header.set_description("My games");
        header.custom_flag_desc[2][..5].copy_from_slice(b"Wins!");
    });
    let header = Index::open(path.with_extension("si4"))
        .unwrap()
        .header()
        .clone();

    let mut db = Database::open(&path).unwrap();
    db.compact_games().unwrap();
    db.update_game(0, &parse_single_game(NEW_GAME).unwrap())
        .unwrap();
    db.compact_names().unwrap();
    drop(db);

    let after = Index::open(path.with_extension("si4")).unwrap();
    let after = after.header();
    assert_eq!(after.num_games, 4);
    assert_eq!(
        (after.magic, after.version, after.base_type, after.auto_load),
        (
            header.magic,
            header.version,
            header.base_type,
            header.auto_load
        )
    );
    assert_eq!(after.description, header.description);
    assert_eq!(after.custom_flag_desc, header.custom_flag_desc);
}

#[test]
fn test_compact_games_deletes_the_tree_cache() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = five_with_deleted(temp.path(), &[]);
    let stc = path.with_extension("stc");
    std::fs::write(&stc, "cache").unwrap();

    let mut db = Database::open(&path).unwrap();
    db.update_game(0, &parse_single_game(NEW_GAME).unwrap())
        .unwrap();
    db.compact_names().unwrap();
    assert!(stc.exists(), "compacting names keeps game numbers");

    db.compact_games().unwrap();
    assert!(!stc.exists());
    db.compact_games().unwrap();
}

#[test]
fn test_compact_games_stops_at_a_record_it_cant_read() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = copy_of(temp.path(), "five");
    let end = game_size(&path) as u32;
    change_index(&path, |index| {
        index.get_entry_mut(1).unwrap().set_delete_flag(true);
        index.get_entry_mut(2).unwrap().set_offset(end);
    });
    let before = database_bytes(&path);

    let mut db = Database::open(&path).unwrap();
    let err = db.compact_games().unwrap_err();
    match &err {
        Error::Game { number: 3, source } => {
            assert!(matches!(**source, Error::GameOutOfRange { .. }), "{source}")
        }
        other => panic!("unexpected error: {other}"),
    }
    assert_eq!(database_bytes(&path), before);
    assert_eq!(files_in(temp.path()), database_files("five"));

    // The database can still be used, and a deleted game that can't be read
    // doesn't stop compaction.
    assert_eq!(db.num_games(), 5);
    assert!(db.get_game(0).is_ok());
    db.delete_game(2).unwrap();
    db.compact_games().unwrap();
    assert_eq!(db.num_games(), 3);
    drop(db);
    assert_eq!(Database::open(&path).unwrap().num_games(), 3);
}

#[test]
fn test_compact_names_drops_names_no_game_uses() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = copy_of(temp.path(), "five");
    let mut db = Database::open(&path).unwrap();
    let old = db.get_game(0).unwrap();
    assert_eq!(
        db.nfile()
            .namebase()
            .get_frequency(NameType::Player, db.get_entry(0).unwrap().get_white()),
        1,
        "game 0's White plays only game 0"
    );
    db.update_game(0, &parse_single_game(NEW_GAME).unwrap())
        .unwrap();
    let before = all_pgn(&mut db);
    let names = all_names(&db);

    db.compact_names().unwrap();

    assert_eq!(all_pgn(&mut db), before);
    let nb = db.nfile().namebase();
    assert_eq!(nb.find_name(NameType::Player, &old.white), None);
    // The names left keep their order and are numbered from 0.
    let kept: Vec<Vec<(u32, String, u32)>> = names
        .iter()
        .map(|names| {
            (0..)
                .zip(names.iter().filter(|(_, _, f)| *f > 0))
                .map(|(id, (_, name, f))| (id, name.clone(), *f))
                .collect()
        })
        .collect();
    assert_eq!(all_names(&db), kept);
    assert_frequencies_counted(&db);
    assert_eq!(files_in(temp.path()), database_files("five"));

    drop(db);
    let mut db = Database::open(&path).unwrap();
    assert_eq!(all_pgn(&mut db), before);
    assert_eq!(all_names(&db), kept);
}

#[test]
fn test_compact_names_keeps_names_of_deleted_games() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = five_with_deleted(temp.path(), &[0]);
    let before = database_bytes(&path);
    let mut db = Database::open(&path).unwrap();
    let white = db.get_game(0).unwrap().white;

    db.compact_names().unwrap();
    assert_eq!(database_bytes(&path), before);

    db.compact_games().unwrap();
    db.compact_names().unwrap();
    assert_eq!(
        db.nfile().namebase().find_name(NameType::Player, &white),
        None
    );
}

#[test]
fn test_compact_names_counts_the_games_again() {
    // Every name is used, though the name file says none is.
    let temp = tempfile::TempDir::new().unwrap();
    let path = copy_of(temp.path(), "five");
    let db = Database::open(&path).unwrap();
    let mut zeroed = NameBase::new();
    for ntype in NAME_TYPES {
        for (_, name, _) in db.nfile().namebase().get_all_names(ntype) {
            zeroed.add_name(ntype, &name).unwrap();
        }
    }
    drop(db);
    let mut sn4 = Vec::new();
    zeroed.write_sn4(&mut sn4).unwrap();
    std::fs::write(path.with_extension("sn4"), sn4).unwrap();
    let before = database_bytes(&path);

    Database::open(&path).unwrap().compact_names().unwrap();
    assert_eq!(database_bytes(&path), before);

    // Game 0's White is used by no game, though the name file counts it.
    change_index(&path, |index| {
        let white = index.get_entry(1).unwrap().get_white();
        index.get_entry_mut(0).unwrap().set_white(white);
    });
    let mut db = Database::open(&path).unwrap();
    let count = db.nfile().namebase().count(NameType::Player);
    db.compact_names().unwrap();
    assert_eq!(db.nfile().namebase().count(NameType::Player), count - 1);
    assert_frequencies_counted(&db);
}

#[test]
fn test_compact_names_stops_at_a_name_the_name_file_lacks() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = copy_of(temp.path(), "five");
    change_index(&path, |index| {
        index.get_entry_mut(0).unwrap().set_round(0);
        index.get_entry_mut(2).unwrap().set_site(9_999);
    });
    let before = database_bytes(&path);

    let mut db = Database::open(&path).unwrap();
    let err = db.compact_names().unwrap_err();
    match &err {
        Error::Game { number: 3, source } => assert!(matches!(
            **source,
            Error::NameNotFound {
                name_type: NameType::Site,
                id: 9_999
            }
        )),
        other => panic!("unexpected error: {other}"),
    }
    assert_eq!(
        std::error::Error::source(&err).unwrap().to_string(),
        "No site name has ID 9999"
    );
    assert_eq!(database_bytes(&path), before);
    assert_eq!(files_in(temp.path()), database_files("five"));
    assert!(db.get_game(0).is_ok());
}

#[test]
fn test_compacting_a_read_only_database_changes_nothing() {
    let temp = tempfile::TempDir::new().unwrap();
    let path = five_with_deleted(temp.path(), &[1]);
    let before = database_bytes(&path);

    let mut db = Database::open_read_only(&path).unwrap();
    assert!(matches!(db.compact_games(), Err(Error::ReadOnly)));
    assert!(matches!(db.compact_names(), Err(Error::ReadOnly)));
    assert_eq!(db.num_games(), 5);
    drop(db);

    assert_eq!(database_bytes(&path), before);
    assert_eq!(files_in(temp.path()), database_files("five"));
}

#[test]
fn test_compacting_one_database_leaves_another_beside_it() {
    let temp = tempfile::TempDir::new().unwrap();
    let five = five_with_deleted(temp.path(), &[0]);
    let one = copy_of(temp.path(), "one");
    let before = database_bytes(&one);

    let mut db = Database::open(&five).unwrap();
    db.compact_games().unwrap();
    db.compact_names().unwrap();

    assert_eq!(database_bytes(&one), before);
    let mut files = database_files("five");
    files.extend(database_files("one"));
    assert_eq!(files_in(temp.path()), files);
}

#[cfg(unix)]
#[test]
fn test_compaction_keeps_the_files_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::TempDir::new().unwrap();
    let path = five_with_deleted(temp.path(), &[0]);
    let modes = [0o640, 0o604, 0o660];
    for (ext, mode) in ["si4", "sn4", "sg4"].iter().zip(modes) {
        let permissions = std::fs::Permissions::from_mode(mode);
        std::fs::set_permissions(path.with_extension(ext), permissions).unwrap();
    }

    let mut db = Database::open(&path).unwrap();
    db.compact_games().unwrap();
    db.compact_names().unwrap();

    for (ext, mode) in ["si4", "sn4", "sg4"].iter().zip(modes) {
        let permissions = std::fs::metadata(path.with_extension(ext))
            .unwrap()
            .permissions();
        assert_eq!(permissions.mode() & 0o777, mode, "{ext}");
    }
}
