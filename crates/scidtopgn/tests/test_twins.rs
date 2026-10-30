//! Finding twin games and marking them deleted.
//!
//! `tests/data/twins` was written by `pgnscid` from `tests/data/twins.pgn`.
//! Each pair of players there has its own case of twins, named by the
//! games' `Case` tags. The games SCID deletes with each set of options below
//! are SCID vs. PC's: `tests/data/twins.tcl` ran `sc_base duplicates` on
//! copies of the database with `tcscid`.

use std::path::{Path, PathBuf};

use scidtopgn::{
    parse_single_game, Database, DeleteTwin, Error, Game, Index, IndexEntry, TwinOptions,
};
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

/// White's and Black's ratings of game `game_num`, each with its kind.
fn ratings_of(db: &Database, game_num: u32) -> [(u16, u8); 2] {
    let entry = db.get_entry(game_num).unwrap();
    [
        (entry.get_white_elo(), entry.get_white_rating_type()),
        (entry.get_black_elo(), entry.get_black_rating_type()),
    ]
}

/// Sets the ratings of game `game_num` of the database at `path`, each with
/// its kind.
fn set_ratings(
    path: &Path,
    game_num: u32,
    [(white, white_kind), (black, black_kind)]: [(u16, u8); 2],
) {
    let si4 = path.with_extension("si4");
    let mut index = Index::open(&si4).unwrap();
    let entry = index.get_entry_mut(game_num).unwrap();
    entry.set_white_elo(white);
    entry.set_white_rating_type(white_kind);
    entry.set_black_elo(black);
    entry.set_black_rating_type(black_kind);
    index
        .flush(&mut std::fs::File::create(&si4).unwrap())
        .unwrap();
}

fn copying_ratings() -> TwinOptions {
    TwinOptions {
        copy_ratings: true,
        ..gui()
    }
}

#[test]
fn test_copies_ratings_with_their_kinds() {
    let dir = TempDir::new().unwrap();
    let path = written_moves(dir.path(), &[SHORT, LONG]);
    set_ratings(&path, 0, [(2500, 3), (2400, 5)]);

    let mut db = Database::open(&path).unwrap();
    assert_eq!(db.delete_twins(&copying_ratings()).unwrap(), [0]);
    assert_eq!(ratings_of(&db, 1), [(2500, 3), (2400, 5)]);
    db.close().unwrap();
    let db = Database::open_read_only(&path).unwrap();
    assert_eq!(ratings_of(&db, 1), [(2500, 3), (2400, 5)]);
}

#[test]
fn test_keeps_the_ratings_of_the_kept_game() {
    let dir = TempDir::new().unwrap();
    let path = written_moves(dir.path(), &[SHORT, LONG]);
    set_ratings(&path, 0, [(2500, 3), (2400, 5)]);
    set_ratings(&path, 1, [(2600, 2), (2300, 4)]);

    let mut db = Database::open(&path).unwrap();
    assert_eq!(db.delete_twins(&copying_ratings()).unwrap(), [0]);
    assert_eq!(ratings_of(&db, 1), [(2600, 2), (2300, 4)]);
}

#[test]
fn test_copies_white_rating_to_white_when_colors_are_swapped() {
    let dir = TempDir::new().unwrap();
    let games = [
        game("Anand", "Carlsen", SHORT),
        game("Carlsen", "Anand", LONG),
    ];
    let path = written(dir.path(), &games);
    set_ratings(&path, 0, [(2500, 3), (2400, 5)]);

    let options = TwinOptions {
        colors: false,
        ..copying_ratings()
    };
    let mut db = Database::open(&path).unwrap();
    assert_eq!(db.delete_twins(&options).unwrap(), [0]);
    assert_eq!(ratings_of(&db, 1), [(2500, 3), (2400, 5)]);
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
fn game(white: &str, black: &str, moves: &str) -> Game {
    parse_single_game(&format!(
        "[Event \"Rapid\"]\n[Site \"Oslo\"]\n[Date \"2020.01.01\"]\n[Round \"1\"]\n\
         [White \"{white}\"]\n[Black \"{black}\"]\n\n{moves} *"
    ))
    .unwrap()
}

/// Writes a database of `games` in `dir` and returns its path.
fn written(dir: &Path, games: &[Game]) -> PathBuf {
    let path = dir.join("games");
    let mut db = Database::create(&path).unwrap();
    for game in games {
        db.add_game(game).unwrap();
    }
    db.close().unwrap();
    path
}

/// Writes a database of games between Anand and Carlsen with each of
/// `moves`, and returns its path.
fn written_moves(dir: &Path, moves: &[&str]) -> PathBuf {
    let games: Vec<Game> = moves.iter().map(|m| game("Anand", "Carlsen", m)).collect();
    written(dir, &games)
}

/// The games `delete_twins` deletes from the database at `path` with
/// `options`, leaving its files as they are.
fn deleted_by(path: &Path, options: &TwinOptions) -> Vec<u32> {
    Database::open(path).unwrap().delete_twins(options).unwrap()
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

/// Ten half-moves of knight moves, and a game that repeats them and goes on.
const KNIGHTS: &str = "1. Nf3 Nf6 2. Ng1 Ng8 3. Nf3 Nf6 4. Ng1 Ng8 5. Nf3 Nf6";
const MORE_KNIGHTS: &str = "1. Nf3 Nf6 2. Ng1 Ng8 3. Nf3 Nf6 4. Ng1 Ng8 5. Nf3 Nf6 6. Ng1 Ng8";

/// A move of White's knight from g1 with a value SCID's `decodeKnight`
/// rejects.
const UNDECODABLE: u8 = 0x60;
/// White's rook on a1 moving to a3 through its pawn, which SCID decodes.
const BLOCKED_ROOK: u8 = 0x1A;

/// Writes a database of two games between the same players, with `shorter`
/// and `longer` moves, and replaces the bytes of some of their half-moves,
/// each given by game and half-move, counting from 0. Returns its path.
fn with_moves_replaced(
    dir: &Path,
    shorter: &str,
    longer: &str,
    replaced: &[(u32, usize, u8)],
) -> PathBuf {
    let path = dir.join("games");
    let mut db = Database::create(&path).unwrap();
    db.add_game(&game("Anand", "Carlsen", shorter)).unwrap();
    db.add_game(&game("Anand", "Carlsen", longer)).unwrap();
    db.close().unwrap();

    let db = Database::open_read_only(&path).unwrap();
    let sg4 = path.with_extension("sg4");
    let mut records = std::fs::read(&sg4).unwrap();
    for &(game_num, half_move, byte) in replaced {
        let offset = db.get_entry(game_num).unwrap().get_offset() as usize;
        // The record has no tags and starts from the standard position, so
        // its moves start at its third byte, one byte each.
        assert_eq!(records[offset..offset + 2], [0, 0]);
        let at = offset + 2 + half_move;
        assert_ne!(records[at], 0x0F, "the half-move is in the game");
        records[at] = byte;
    }
    std::fs::write(&sg4, records).unwrap();
    path
}

/// Returns the twins `delete_twins` deletes from the games of `path`, and
/// whether each game reads.
fn twins_and_readable(path: PathBuf) -> (Vec<u32>, [bool; 2]) {
    let mut db = Database::open(path).unwrap();
    let readable = [0, 1].map(|n| db.get_game(n).is_ok());
    (db.delete_twins(&gui()).unwrap(), readable)
}

#[test]
fn test_compares_the_shorter_game_up_to_a_move_that_cant_be_decoded() {
    // The shorter game differs at its ninth half-move, which can't be
    // decoded, so only the first 8 are compared.
    let differs = "1. Nf3 Nf6 2. Ng1 Ng8 3. Nf3 Nf6 4. Ng1 Ng8 5. Nc3 Nc6";
    let dir = TempDir::new().unwrap();
    let intact_dir = TempDir::new().unwrap();
    let intact = with_moves_replaced(intact_dir.path(), differs, MORE_KNIGHTS, &[]);
    assert_eq!(twins_and_readable(intact), (vec![], [true, true]));

    let bad = &[(0, 8, UNDECODABLE)];
    let path = with_moves_replaced(dir.path(), differs, MORE_KNIGHTS, bad);
    assert_eq!(twins_and_readable(path), (vec![0], [false, true]));
}

#[test]
fn test_longer_game_with_a_move_that_cant_be_decoded_is_no_twin() {
    let dir = TempDir::new().unwrap();
    let intact_dir = TempDir::new().unwrap();
    let intact = with_moves_replaced(intact_dir.path(), KNIGHTS, MORE_KNIGHTS, &[]);
    assert_eq!(twins_and_readable(intact), (vec![0], [true, true]));

    let bad = &[(1, 2, UNDECODABLE)];
    let path = with_moves_replaced(dir.path(), KNIGHTS, MORE_KNIGHTS, bad);
    assert_eq!(twins_and_readable(path), (vec![], [true, false]));
}

#[test]
fn test_compares_an_illegal_move_scid_can_decode() {
    // SCID decodes the rook move through its pawn, so the ninth half-moves
    // differ and the games aren't twins.
    let dir = TempDir::new().unwrap();
    let bad = &[(0, 8, BLOCKED_ROOK)];
    let path = with_moves_replaced(dir.path(), KNIGHTS, MORE_KNIGHTS, bad);
    assert_eq!(twins_and_readable(path), (vec![], [false, true]));
}

#[test]
fn test_compares_moves_after_an_illegal_move_scid_can_decode() {
    // Both games have the rook move, and the shorter game's last move after
    // it is the same in both.
    let dir = TempDir::new().unwrap();
    let bad = &[(0, 8, BLOCKED_ROOK), (1, 8, BLOCKED_ROOK)];
    let path = with_moves_replaced(dir.path(), KNIGHTS, MORE_KNIGHTS, bad);
    assert_eq!(twins_and_readable(path), (vec![0], [false, false]));
}

#[test]
fn test_compares_moves_that_differ_after_an_illegal_move_scid_can_decode() {
    // Black's knight goes to c6 instead of f6 after the shared rook move.
    let dir = TempDir::new().unwrap();
    let bad = &[(0, 8, BLOCKED_ROOK), (1, 8, BLOCKED_ROOK), (1, 9, 0x28)];
    let path = with_moves_replaced(dir.path(), KNIGHTS, MORE_KNIGHTS, bad);
    assert_eq!(twins_and_readable(path), (vec![], [false, false]));
}

/// `full_moves` moves of White's and Black's kingside knights going out and
/// back, as numbered movetext.
fn knight_moves(full_moves: usize) -> String {
    (1..=full_moves)
        .map(|n| match n % 2 {
            1 => format!("{n}. Nf3 Nf6 "),
            _ => format!("{n}. Ng1 Ng8 "),
        })
        .collect()
}

#[test]
fn test_home_pawns_must_match_after_the_moves_compared() {
    // The games' first 120 half-moves are the same, but not their pawn moves
    // after them.
    let dir = TempDir::new().unwrap();
    let knights = knight_moves(60);
    let e4 = format!("{knights}61. e4 e5");
    let d4 = format!("{knights}61. d4 d5");
    let path = written_moves(dir.path(), &[&e4, &d4]);
    assert_eq!(deleted_by(&path, &gui()), []);
    let options = TwinOptions {
        moves: false,
        ..gui()
    };
    assert_eq!(deleted_by(&path, &options), [0]);
}

#[test]
fn test_compares_the_120th_half_move() {
    let dir = TempDir::new().unwrap();
    let knights = knight_moves(59);
    let back = format!("{knights}60. Ng1 Ng8");
    let forward = format!("{knights}60. Ng1 Ne4");
    let path = written_moves(dir.path(), &[&back, &forward]);
    assert_eq!(deleted_by(&path, &gui()), []);
}

#[test]
fn test_placeholders_have_at_most_two_half_moves() {
    let options = TwinOptions {
        skip_short: false,
        ..gui()
    };
    for (placeholder, expected) in [
        ("1. d4", vec![0]),
        ("1. d4 d5", vec![0]),
        ("1. d4 d5 2. Nf3", vec![]),
    ] {
        let dir = TempDir::new().unwrap();
        let path = written_moves(dir.path(), &[placeholder, SHORT]);
        assert_eq!(deleted_by(&path, &options), expected, "{placeholder}");
    }
}

#[test]
fn test_skip_short_leaves_out_games_of_nine_half_moves() {
    let dir = TempDir::new().unwrap();
    let nine = "1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 Nf6 5. O-O";
    let path = written_moves(dir.path(), &[nine, SHORT]);
    assert_eq!(deleted_by(&path, &gui()), []);
    let options = TwinOptions {
        skip_short: false,
        ..gui()
    };
    assert_eq!(deleted_by(&path, &options), [0]);
}

#[test]
fn test_compares_promotions() {
    let dir = TempDir::new().unwrap();
    let start = "[Event \"Rapid\"]\n[White \"Anand\"]\n[Black \"Carlsen\"]\n\
        [SetUp \"1\"]\n[FEN \"4k3/P7/8/8/8/8/8/4K3 w - - 0 1\"]\n\n";
    let games = ["Q", "R"].map(|piece| {
        parse_single_game(&format!("{start}1. Kf2 Kf7 2. Ke3 Ke6 3. a8={piece} *")).unwrap()
    });
    let path = written(dir.path(), &games);
    let options = TwinOptions {
        skip_short: false,
        ..gui()
    };
    assert_eq!(deleted_by(&path, &options), []);
}

#[test]
fn test_skips_nags_and_variations_in_the_main_line() {
    // NAG 14 is also the byte that ends a variation.
    let differs = "1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 Nf6 5. O-O Bd6 6. Re1 b5";
    let options = TwinOptions {
        comments: false,
        variations: false,
        ..gui()
    };
    for annotation in [" $14", " (3. Bc4 Bc5)", " (3. Bc4 (3. Nc3 Nf6) 3... Bc5)"] {
        let annotated = SHORT.replace("3. Bb5", &format!("3. Bb5{annotation}"));
        for (other, expected) in [(LONG, vec![0]), (differs, vec![])] {
            let dir = TempDir::new().unwrap();
            let path = written_moves(dir.path(), &[&annotated, other]);
            assert_eq!(
                deleted_by(&path, &options),
                expected,
                "{annotation}, {other}"
            );
        }
    }
}

#[test]
fn test_player_prefixes_match_when_scids_hashes_do() {
    // As signed bytes, 0x63 0xC3 and 0x62 0x43 add to the same hash.
    let dir = TempDir::new().unwrap();
    let games = ["AbcÀ", "AbbC"].map(|white| game(white, "Black", SHORT));
    let path = written(dir.path(), &games);
    let options = TwinOptions {
        players: false,
        ..gui()
    };
    assert_eq!(deleted_by(&path, &options), [0]);
}

/// Asserts that a game with `moves` is the twin of a copy, and not of
/// `different`, which differs only in its last half-move.
fn assert_compared_to_the_end(moves: &str, different: &str) {
    for (other, expected) in [(moves, vec![0]), (different, vec![])] {
        let dir = TempDir::new().unwrap();
        let path = written_moves(dir.path(), &[moves, other]);
        assert_eq!(deleted_by(&path, &gui()), expected, "{other}");
    }
}

#[test]
fn test_compares_moves_after_a_diagonal_queen_move() {
    assert_compared_to_the_end(
        "1. e4 e5 2. Qh5 Nc6 3. Bc4 Nf6 4. Qf3 d6 5. Ne2 Be7",
        "1. e4 e5 2. Qh5 Nc6 3. Bc4 Nf6 4. Qf3 d6 5. Ne2 Bd7",
    );
}

#[test]
fn test_compares_moves_of_pieces_renumbered_by_captures() {
    // Each capture gives the captured piece's number to its side's h-pawn.
    assert_compared_to_the_end(
        "1. e4 d5 2. exd5 Qxd5 3. Nc3 Qd8 4. h3 h6 5. Nf3 h5",
        "1. e4 d5 2. exd5 Qxd5 3. Nc3 Qd8 4. h3 h6 5. Nf3 Nf6",
    );
}

#[test]
fn test_compares_moves_after_null_moves() {
    assert_compared_to_the_end(
        "1. Nf3 Nf6 2. -- -- 3. Ng1 Ng8 4. Nf3 Nf6 5. Ng1 Ng8",
        "1. Nf3 Nf6 2. -- -- 3. Ng1 Ng8 4. Nf3 Nf6 5. Ng1 Nh5",
    );
}
