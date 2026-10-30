//! Searching games by their index entries and names.
//!
//! `tests/data/search` was written by `pgnscid` from
//! `tests/data/search.pgn`. `tests/data/search.tcl` then set SCID's flags on
//! some of its games, and ran SCID vs. PC's `sc_search header` with each set
//! of options below, with `tcscid`. The games each selected are SCID's.

use std::path::{Path, PathBuf};

use scidtopgn::shakmaty::Color;
use scidtopgn::{Database, Date, Eco, FlagMatch, GameResult, HeaderFlags, HeaderSearch};
use tempfile::TempDir;

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

fn date(year: u16, month: u8, day: u8) -> Date {
    Date::new(year, month, day).unwrap()
}

fn eco(code: &str) -> Eco {
    code.parse().unwrap()
}

/// The games `search` selects in `db`, counting from 1, as SCID numbers
/// them.
fn selected(db: &Database, search: &HeaderSearch) -> Vec<u32> {
    let games = db.search_header(search).unwrap();
    games.into_iter().map(|game| game + 1).collect()
}

/// Each set of options `search.tcl` runs, other than flags, and the games
/// SCID selects with it, counting from 1.
// SCID swaps a rating difference range whose start is above its end, which
// `delo-inverted` checks.
#[allow(clippy::reversed_empty_ranges)]
fn scid_runs() -> Vec<(&'static str, HeaderSearch, &'static [u32])> {
    vec![
        (
            "default",
            HeaderSearch::default(),
            &[
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22, 23, 24,
            ],
        ),
        (
            "white",
            HeaderSearch {
                white: Some("Carlsen".into()),
                ..HeaderSearch::default()
            },
            &[1, 3, 19],
        ),
        (
            "white-case",
            HeaderSearch {
                white: Some("CARLSEN".into()),
                ..HeaderSearch::default()
            },
            &[1, 3, 19],
        ),
        (
            "white-spaces",
            HeaderSearch {
                white: Some("Carl sen".into()),
                ..HeaderSearch::default()
            },
            &[1, 3, 19],
        ),
        (
            "white-space-end",
            HeaderSearch {
                white: Some("Carlsen ".into()),
                ..HeaderSearch::default()
            },
            &[],
        ),
        (
            "white-space-start",
            HeaderSearch {
                white: Some(" Carlsen".into()),
                ..HeaderSearch::default()
            },
            &[1, 3, 19],
        ),
        (
            "white-only-spaces",
            HeaderSearch {
                white: Some("  ".into()),
                ..HeaderSearch::default()
            },
            &[],
        ),
        (
            "white-empty",
            HeaderSearch {
                white: Some("".into()),
                ..HeaderSearch::default()
            },
            &[
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22, 23, 24,
            ],
        ),
        (
            "white-comma",
            HeaderSearch {
                white: Some("Doe, John".into()),
                ..HeaderSearch::default()
            },
            &[6],
        ),
        (
            "white-not-ascii",
            HeaderSearch {
                white: Some("müller".into()),
                ..HeaderSearch::default()
            },
            &[],
        ),
        (
            "black-not-ascii",
            HeaderSearch {
                black: Some("müller".into()),
                ..HeaderSearch::default()
            },
            &[9],
        ),
        (
            "wild-prefix",
            HeaderSearch {
                white: Some(r#""Carlsen*""#.into()),
                ..HeaderSearch::default()
            },
            &[1, 3, 19],
        ),
        (
            "wild-case",
            HeaderSearch {
                white: Some(r#""carlsen*""#.into()),
                ..HeaderSearch::default()
            },
            &[],
        ),
        (
            "wild-whole",
            HeaderSearch {
                white: Some(r#""Carlsen""#.into()),
                ..HeaderSearch::default()
            },
            &[],
        ),
        (
            "wild-question",
            HeaderSearch {
                white: Some(r#""Anand, ?iswanathan""#.into()),
                ..HeaderSearch::default()
            },
            &[7],
        ),
        (
            "wild-class",
            HeaderSearch {
                white: Some(r#""[A-D]*""#.into()),
                ..HeaderSearch::default()
            },
            &[1, 2, 3, 6, 7, 8, 18, 19, 20],
        ),
        (
            "wild-class-reversed",
            HeaderSearch {
                white: Some(r#""[D-A]*""#.into()),
                ..HeaderSearch::default()
            },
            &[1, 2, 3, 6, 7, 8, 18, 19, 20],
        ),
        (
            "wild-class-list",
            HeaderSearch {
                white: Some(r#""[CS]*""#.into()),
                ..HeaderSearch::default()
            },
            &[1, 2, 3, 11, 12, 15, 16, 18, 19, 20],
        ),
        (
            "wild-escape",
            HeaderSearch {
                white: Some(r#""Smith \[A\]""#.into()),
                ..HeaderSearch::default()
            },
            &[11],
        ),
        (
            "wild-bracket",
            HeaderSearch {
                white: Some(r#""Smith [A]""#.into()),
                ..HeaderSearch::default()
            },
            &[],
        ),
        (
            "wild-not-ascii",
            HeaderSearch {
                black: Some(r#""M?ller, Hans""#.into()),
                ..HeaderSearch::default()
            },
            &[9],
        ),
        (
            "wild-not-ascii-upper",
            HeaderSearch {
                white: Some(r#""M?LLER*""#.into()),
                ..HeaderSearch::default()
            },
            &[10],
        ),
        (
            "wild-empty",
            HeaderSearch {
                white: Some(r#""""#.into()),
                ..HeaderSearch::default()
            },
            &[],
        ),
        (
            "wild-quote",
            HeaderSearch {
                white: Some(r#"""#.into()),
                ..HeaderSearch::default()
            },
            &[],
        ),
        (
            "wild-unknown",
            HeaderSearch {
                white: Some(r#""?""#.into()),
                ..HeaderSearch::default()
            },
            &[14],
        ),
        (
            "wild-star",
            HeaderSearch {
                white: Some(r#""*""#.into()),
                ..HeaderSearch::default()
            },
            &[
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22, 23, 24,
            ],
        ),
        (
            "wild-unclosed",
            HeaderSearch {
                white: Some(r#""[A-C*""#.into()),
                ..HeaderSearch::default()
            },
            &[],
        ),
        (
            "black",
            HeaderSearch {
                black: Some("Carlsen".into()),
                ..HeaderSearch::default()
            },
            &[2, 4, 18, 20],
        ),
        (
            "event",
            HeaderSearch {
                event: Some("championship".into()),
                ..HeaderSearch::default()
            },
            &[4, 11, 12, 21],
        ),
        (
            "event-wild",
            HeaderSearch {
                event: Some(r#""World Championship*""#.into()),
                ..HeaderSearch::default()
            },
            &[4, 21],
        ),
        (
            "site",
            HeaderSearch {
                site: Some("wijk aan zee".into()),
                ..HeaderSearch::default()
            },
            &[5, 6, 19],
        ),
        (
            "round",
            HeaderSearch {
                round: Some("1".into()),
                ..HeaderSearch::default()
            },
            &[1, 5, 6, 7, 8, 11, 19],
        ),
        (
            "round-wild",
            HeaderSearch {
                round: Some(r#""1""#.into()),
                ..HeaderSearch::default()
            },
            &[1, 11],
        ),
        (
            "date",
            HeaderSearch {
                dates: date(2023, 1, 1)..=date(2023, 12, 31),
                ..HeaderSearch::default()
            },
            &[1, 2, 21],
        ),
        (
            "date-year",
            HeaderSearch {
                dates: date(2023, 0, 0)..=date(2023, 12, 31),
                ..HeaderSearch::default()
            },
            &[1, 2, 3, 21],
        ),
        (
            "date-inverted",
            HeaderSearch {
                dates: date(2023, 12, 31)..=date(2023, 1, 1),
                ..HeaderSearch::default()
            },
            &[],
        ),
        (
            "date-unknown",
            HeaderSearch {
                dates: Date::ZERO..=Date::ZERO,
                ..HeaderSearch::default()
            },
            &[13, 14],
        ),
        (
            "results-white",
            HeaderSearch {
                results: vec![GameResult::White],
                ..HeaderSearch::default()
            },
            &[1, 5, 7, 11, 15, 20, 21, 22, 23],
        ),
        (
            "results-draw-none",
            HeaderSearch {
                results: vec![GameResult::Draw, GameResult::None],
                ..HeaderSearch::default()
            },
            &[3, 6, 9, 12, 13, 14, 18, 19, 24],
        ),
        (
            "welo",
            HeaderSearch {
                white_elo: 2840..=4000,
                ..HeaderSearch::default()
            },
            &[1, 3],
        ),
        (
            "welo-zero",
            HeaderSearch {
                white_elo: 0..=0,
                ..HeaderSearch::default()
            },
            &[13, 14, 15, 16, 22, 23, 24],
        ),
        (
            "belo",
            HeaderSearch {
                black_elo: 2800..=2900,
                ..HeaderSearch::default()
            },
            &[1, 2, 4, 18, 20],
        ),
        (
            "delo",
            HeaderSearch {
                elo_diff: -20..=20,
                ..HeaderSearch::default()
            },
            &[11, 12, 13, 14, 15, 16, 20, 21, 22, 23, 24],
        ),
        (
            "delo-inverted",
            HeaderSearch {
                elo_diff: 20..=-20,
                ..HeaderSearch::default()
            },
            &[11, 12, 13, 14, 15, 16, 20, 21, 22, 23, 24],
        ),
        (
            "delo-positive",
            HeaderSearch {
                elo_diff: 100..=4000,
                ..HeaderSearch::default()
            },
            &[5, 7, 9, 19],
        ),
        (
            "eco",
            HeaderSearch {
                eco: eco("B20")..=eco("B99"),
                ..HeaderSearch::default()
            },
            &[1, 2, 3, 13, 14, 15, 16, 19],
        ),
        (
            "eco-exclude",
            HeaderSearch {
                eco: eco("B20")..=eco("B99"),
                with_no_eco: false,
                ..HeaderSearch::default()
            },
            &[1, 2, 19],
        ),
        (
            "eco-subcodes",
            HeaderSearch {
                eco: eco("B07")..=eco("B07"),
                with_no_eco: false,
                ..HeaderSearch::default()
            },
            &[5, 6],
        ),
        (
            "eco-subcode-upper",
            HeaderSearch {
                eco: eco("B07")..=eco("B07a"),
                with_no_eco: false,
                ..HeaderSearch::default()
            },
            &[5],
        ),
        (
            "eco-subcode-lower",
            HeaderSearch {
                eco: eco("B07z")..=eco("B08"),
                with_no_eco: false,
                ..HeaderSearch::default()
            },
            &[6, 7],
        ),
        (
            "eco-inverted",
            HeaderSearch {
                eco: eco("B99")..=eco("B20"),
                with_no_eco: false,
                ..HeaderSearch::default()
            },
            &[],
        ),
        (
            "length",
            HeaderSearch {
                length: 5..=10,
                ..HeaderSearch::default()
            },
            &[2, 4, 5, 7, 12, 18, 20, 21, 22, 23, 24],
        ),
        (
            "length-zero",
            HeaderSearch {
                length: 0..=0,
                ..HeaderSearch::default()
            },
            &[13, 14],
        ),
        (
            "length-long",
            HeaderSearch {
                length: 1000..=1023,
                ..HeaderSearch::default()
            },
            &[17],
        ),
        (
            "length-max",
            HeaderSearch {
                length: 0..=1023,
                ..HeaderSearch::default()
            },
            &[
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
                24,
            ],
        ),
        (
            "to-move-white",
            HeaderSearch {
                to_move: Some(Color::White),
                ..HeaderSearch::default()
            },
            &[1, 3, 5, 6, 7, 8, 9, 10, 11, 13, 14, 18, 19, 20],
        ),
        (
            "to-move-black",
            HeaderSearch {
                to_move: Some(Color::Black),
                ..HeaderSearch::default()
            },
            &[2, 4, 12, 15, 16, 21, 22, 23, 24],
        ),
        (
            "game-number",
            HeaderSearch {
                game_numbers: (5, 10),
                ..HeaderSearch::default()
            },
            &[5, 6, 7, 8, 9, 10],
        ),
        (
            "game-number-end",
            HeaderSearch {
                game_numbers: (-5, -1),
                ..HeaderSearch::default()
            },
            &[20, 21, 22, 23, 24],
        ),
        (
            "game-number-swapped",
            HeaderSearch {
                game_numbers: (10, 5),
                ..HeaderSearch::default()
            },
            &[5, 6, 7, 8, 9, 10],
        ),
        (
            "game-number-clamped",
            HeaderSearch {
                game_numbers: (-100, 3),
                ..HeaderSearch::default()
            },
            &[1, 2, 3],
        ),
        (
            "game-number-past",
            HeaderSearch {
                game_numbers: (20, 100),
                ..HeaderSearch::default()
            },
            &[20, 21, 22, 23, 24],
        ),
        (
            "flip-results",
            HeaderSearch {
                white: Some("Carlsen".into()),
                results: vec![GameResult::White],
                ignore_colors: true,
                ..HeaderSearch::default()
            },
            &[1, 2, 4],
        ),
        (
            "flip-elo",
            HeaderSearch {
                white_elo: 2750..=4000,
                black_elo: 0..=2700,
                ignore_colors: true,
                ..HeaderSearch::default()
            },
            &[7, 8, 19],
        ),
        (
            "flip-elo-diff",
            HeaderSearch {
                elo_diff: 30..=4000,
                ignore_colors: true,
                ..HeaderSearch::default()
            },
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 18, 19],
        ),
        (
            "flip-to-move",
            HeaderSearch {
                white: Some("Carlsen".into()),
                to_move: Some(Color::White),
                ignore_colors: true,
                ..HeaderSearch::default()
            },
            &[1, 2, 3, 4, 19],
        ),
        (
            "flip-names",
            HeaderSearch {
                white: Some(r#""Smith*""#.into()),
                black: Some(r#""Smith, Bob""#.into()),
                ignore_colors: true,
                ..HeaderSearch::default()
            },
            &[11, 12],
        ),
        (
            "combined",
            HeaderSearch {
                white: Some("Carlsen".into()),
                dates: date(2019, 1, 1)..=date(2023, 12, 31),
                eco: eco("B20")..=eco("B99"),
                with_no_eco: false,
                white_elo: 2800..=4000,
                ..HeaderSearch::default()
            },
            &[1, 19],
        ),
    ]
}

/// Each flag option `search.tcl` runs, the field it sets, and the games
/// SCID selects with it set to yes and to no, counting from 1.
type FlagRun = (
    &'static str,
    fn(&mut HeaderFlags) -> &mut FlagMatch,
    &'static [u32],
    &'static [u32],
);

fn scid_flag_runs() -> Vec<FlagRun> {
    vec![
        (
            "fStdStart",
            |flags| &mut flags.non_standard_start,
            &[15, 16],
            &[
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 18, 19, 20, 21, 22, 23, 24,
            ],
        ),
        (
            "fPromotions",
            |flags| &mut flags.promotions,
            &[15, 16, 22, 23],
            &[
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 18, 19, 20, 21, 24,
            ],
        ),
        (
            "fUnderPromo",
            |flags| &mut flags.underpromotions,
            &[16, 23],
            &[
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 18, 19, 20, 21, 22, 24,
            ],
        ),
        (
            "fComments",
            |flags| &mut flags.comments,
            &[9],
            &[
                1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22, 23, 24,
            ],
        ),
        (
            "fVariations",
            |flags| &mut flags.variations,
            &[10, 24],
            &[
                1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22, 23,
            ],
        ),
        (
            "fAnnotations",
            |flags| &mut flags.annotations,
            &[11],
            &[
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22, 23, 24,
            ],
        ),
        (
            "fDelete",
            |flags| &mut flags.deleted,
            &[6, 12],
            &[
                1, 2, 3, 4, 5, 7, 8, 9, 10, 11, 13, 14, 15, 16, 18, 19, 20, 21, 22, 23, 24,
            ],
        ),
        (
            "fWhiteOpening",
            |flags| &mut flags.white_opening,
            &[1, 7],
            &[
                2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22, 23, 24,
            ],
        ),
        (
            "fBlackOpening",
            |flags| &mut flags.black_opening,
            &[2],
            &[
                1, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22, 23, 24,
            ],
        ),
        (
            "fMiddlegame",
            |flags| &mut flags.middlegame,
            &[3],
            &[
                1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22, 23, 24,
            ],
        ),
        (
            "fEndgame",
            |flags| &mut flags.endgame,
            &[4],
            &[
                1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22, 23, 24,
            ],
        ),
        (
            "fNovelty",
            |flags| &mut flags.novelty,
            &[5],
            &[
                1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22, 23, 24,
            ],
        ),
        (
            "fPawnStructure",
            |flags| &mut flags.pawn_structure,
            &[7],
            &[
                1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22, 23, 24,
            ],
        ),
        (
            "fTactics",
            |flags| &mut flags.tactics,
            &[8],
            &[
                1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22, 23, 24,
            ],
        ),
        (
            "fKingside",
            |flags| &mut flags.kingside,
            &[9],
            &[
                1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22, 23, 24,
            ],
        ),
        (
            "fQueenside",
            |flags| &mut flags.queenside,
            &[10],
            &[
                1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22, 23, 24,
            ],
        ),
        (
            "fBrilliancy",
            |flags| &mut flags.brilliancy,
            &[11],
            &[
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22, 23, 24,
            ],
        ),
        (
            "fBlunder",
            |flags| &mut flags.blunder,
            &[19],
            &[
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 20, 21, 22, 23, 24,
            ],
        ),
        (
            "fUser",
            |flags| &mut flags.user,
            &[20],
            &[
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19, 21, 22, 23, 24,
            ],
        ),
        (
            "fCustom1",
            |flags| &mut flags.custom[0],
            &[1],
            &[
                2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22, 23, 24,
            ],
        ),
        (
            "fCustom2",
            |flags| &mut flags.custom[1],
            &[2],
            &[
                1, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22, 23, 24,
            ],
        ),
        (
            "fCustom3",
            |flags| &mut flags.custom[2],
            &[3],
            &[
                1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22, 23, 24,
            ],
        ),
        (
            "fCustom4",
            |flags| &mut flags.custom[3],
            &[4],
            &[
                1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22, 23, 24,
            ],
        ),
        (
            "fCustom5",
            |flags| &mut flags.custom[4],
            &[5],
            &[
                1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19, 20, 21, 22, 23, 24,
            ],
        ),
        (
            "fCustom6",
            |flags| &mut flags.custom[5],
            &[20],
            &[
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19, 21, 22, 23, 24,
            ],
        ),
    ]
}

#[test]
fn selects_the_games_scid_selects() {
    let db = Database::open_read_only(data_path("search")).unwrap();
    assert_eq!(db.num_games(), 24);
    for (name, search, expected) in scid_runs() {
        assert_eq!(selected(&db, &search), expected, "{name}");
    }
}

#[test]
fn selects_the_games_scid_selects_by_flag() {
    let db = Database::open_read_only(data_path("search")).unwrap();
    for (name, field, yes, no) in scid_flag_runs() {
        for (value, expected) in [(FlagMatch::Yes, yes), (FlagMatch::No, no)] {
            let mut search = HeaderSearch::default();
            *field(&mut search.flags) = value;
            assert_eq!(selected(&db, &search), expected, "{name} {value:?}");
        }
    }
}

#[test]
fn reads_no_game_records() {
    let dir = TempDir::new().unwrap();
    let path = copy_of(dir.path(), "search");
    let game_file = path.with_extension("sg4");
    let size = std::fs::metadata(&game_file).unwrap().len();
    std::fs::write(&game_file, vec![0xFF; size as usize]).unwrap();

    let mut db = Database::open_read_only(&path).unwrap();
    assert!(db.get_game(0).is_err());
    for (name, search, expected) in scid_runs() {
        assert_eq!(selected(&db, &search), expected, "{name}");
    }
}

#[test]
fn selects_every_game_by_default_up_to_999_half_moves() {
    for name in ["five", "edge", "derived", "twins", "mixed"] {
        let db = Database::open_read_only(data_path(name)).unwrap();
        let short: Vec<u32> = (0..db.num_games())
            .filter(|&game| db.get_entry(game).unwrap().get_num_half_moves() <= 999)
            .collect();
        assert_eq!(
            db.search_header(&HeaderSearch::default()).unwrap(),
            short,
            "{name}"
        );
    }
}

#[test]
fn accepts_any_rating_difference_bounds() {
    let db = Database::open_read_only(data_path("search")).unwrap();
    for ignore_colors in [false, true] {
        for (wide, narrow) in [
            (i32::MIN..=0, -4000..=0),
            (0..=i32::MAX, 0..=4000),
            (i32::MIN..=i32::MAX, -4000..=4000),
        ] {
            let search = |elo_diff| HeaderSearch {
                elo_diff,
                ignore_colors,
                ..HeaderSearch::default()
            };
            assert_eq!(
                selected(&db, &search(wide.clone())),
                selected(&db, &search(narrow.clone())),
                "{wide:?} {ignore_colors}"
            );
        }
    }
}

#[test]
fn finds_names_and_ratings_past_a_gap_in_the_ids() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("gap");
    let mut db = Database::create(&path).unwrap();
    for pgn in [
        "[White \"A\"]\n[Black \"B\"]\n[BlackElo \"2800\"]\n\n1. e4 *\n",
        "[White \"A\"]\n[Black \"B\"]\n\n1. d4 *\n",
    ] {
        db.add_game(&scidtopgn::parse_single_game(pgn).unwrap())
            .unwrap();
    }
    db.close().unwrap();

    // Give B the player ID 2, so that no name has ID 1. The name file
    // starts with 36 bytes of header, then A's 5 bytes; B's ID is next.
    let sn4_path = path.with_extension("sn4");
    let mut sn4 = std::fs::read(&sn4_path).unwrap();
    assert_eq!(sn4[36..41], [0, 0, 2, 1, b'A']);
    assert_eq!(sn4[41..43], [0, 1]);
    sn4[42] = 2;
    std::fs::write(&sn4_path, sn4).unwrap();
    let si4_path = path.with_extension("si4");
    let mut index = scidtopgn::Index::open(&si4_path).unwrap();
    for game in 0..2 {
        index.get_entry_mut(game).unwrap().set_black(2);
    }
    index
        .flush(&mut std::fs::File::create(&si4_path).unwrap())
        .unwrap();

    let db = Database::open_read_only(&path).unwrap();
    let by_black = HeaderSearch {
        black: Some("B".into()),
        ..HeaderSearch::default()
    };
    assert_eq!(db.search_header(&by_black).unwrap(), [0, 1]);
    // The unrated game takes B's rating from the other.
    let rated = HeaderSearch {
        black_elo: 2500..=4000,
        ..HeaderSearch::default()
    };
    assert_eq!(db.search_header(&rated).unwrap(), [0, 1]);
}
