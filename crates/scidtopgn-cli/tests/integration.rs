use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

fn get_workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crate should be two levels below the workspace root")
        .to_path_buf()
}

/// Returns a command for the CLI binary, run from the workspace root so the
/// fixture paths under `tests/data` resolve.
fn cli() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_scidtopgn"));
    cmd.current_dir(get_workspace_root());
    cmd
}

#[test]
fn test_cli_count() {
    let output = cli()
        .args(["tests/data/one", "--count"])
        .output()
        .expect("Failed to run CLI");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("1 games"),
        "Expected '1 games' in output, got: {}",
        stdout
    );
}

#[test]
fn test_cli_version() {
    let output = cli().arg("--version").output().expect("Failed to run CLI");

    assert!(output.status.success(), "--version should succeed");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("scidtopgn {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn test_cli_output() {
    let output = cli()
        .arg("tests/data/one")
        .output()
        .expect("Failed to run CLI");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[Event \"47th ch-Bangahbandhu 2022\"]"));
    assert!(stdout.contains("[White \"Hossain, Enam\"]"));
    assert!(stdout.contains("1. e4 c5"));
    assert!(stdout.contains("1/2-1/2"));
}

#[test]
fn test_cli_five_games() {
    let output = cli()
        .arg("tests/data/five")
        .output()
        .expect("Failed to run CLI");

    let stdout = String::from_utf8_lossy(&output.stdout);

    let game_count = stdout.matches("[Event \"").count();
    assert_eq!(game_count, 5, "Expected 5 games, found {}", game_count);
}

#[test]
fn test_cli_output_to_file() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let output_path = temp.path().join("output.pgn");

    let output = cli()
        .args(["tests/data/one", "-o"])
        .arg(&output_path)
        .output()
        .expect("Failed to run CLI");

    assert!(output.status.success(), "CLI should succeed");
    assert!(output_path.exists(), "Output file should exist");

    let contents = std::fs::read_to_string(&output_path).expect("Failed to read output");
    assert!(contents.contains("[Event \"47th ch-Bangahbandhu 2022\"]"));
}

#[test]
fn test_cli_import() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp.path().join("imported");

    let output = cli()
        .args(["import", "tests/data/one.pgn", "-o"])
        .arg(&db_path)
        .output()
        .expect("Failed to run CLI");

    assert!(
        output.status.success(),
        "Import should succeed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    let mut db = scidtopgn::Database::open(&db_path).expect("Failed to open imported database");
    assert_eq!(db.num_games(), 1, "Should have 1 game");

    let game = db.get_game(0).expect("Failed to get game");
    assert_eq!(game.white, "Hossain, Enam");
    assert_eq!(game.black, "Murshed, N");
    assert!(game.eco.is_some(), "ECO should be preserved");

    let pgn = game.to_pgn();
    assert!(
        pgn.contains("1. e4 c5"),
        "Moves should be preserved: {}",
        pgn
    );
}

#[test]
fn test_cli_import_append() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp.path().join("append_test");

    let output1 = cli()
        .args(["import", "tests/data/one.pgn", "-o"])
        .arg(&db_path)
        .output()
        .expect("Failed to run CLI");
    assert!(output1.status.success());

    let output2 = cli()
        .args(["import", "tests/data/one.pgn", "-o"])
        .arg(&db_path)
        .output()
        .expect("Failed to run CLI");
    assert!(output2.status.success());

    let db = scidtopgn::Database::open(&db_path).expect("Failed to open database");
    assert_eq!(db.num_games(), 2, "Should have 2 games after append");
}

#[test]
fn test_cli_import_overwrite() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp.path().join("overwrite_test");

    let output1 = cli()
        .args(["import", "tests/data/one.pgn", "-o"])
        .arg(&db_path)
        .output()
        .expect("Failed to run CLI");
    assert!(output1.status.success());

    let output2 = cli()
        .args(["import", "tests/data/one.pgn", "--overwrite", "-o"])
        .arg(&db_path)
        .output()
        .expect("Failed to run CLI");
    assert!(
        output2.status.success(),
        "Overwrite should succeed: {:?}",
        String::from_utf8_lossy(&output2.stderr)
    );

    let db = scidtopgn::Database::open(&db_path).expect("Failed to open database");
    assert_eq!(db.num_games(), 1, "Should have 1 game after overwrite");
}

#[test]
fn test_cli_pgn_roundtrip() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp.path().join("roundtrip");
    let pgn_path = temp.path().join("exported.pgn");

    let import_output = cli()
        .args(["import", "tests/data/one.pgn", "-o"])
        .arg(&db_path)
        .output()
        .expect("Failed to run import");
    assert!(import_output.status.success());

    let export_output = cli()
        .arg("export")
        .arg(&db_path)
        .arg("-o")
        .arg(&pgn_path)
        .output()
        .expect("Failed to run export");
    assert!(export_output.status.success());

    let exported = std::fs::read_to_string(&pgn_path).expect("Failed to read exported");

    assert!(exported.contains("[Event \"47th ch-Bangahbandhu 2022\"]"));
    assert!(exported.contains("[White \"Hossain, Enam\"]"));
    assert!(exported.contains("[ECO \"B36f\"]"));
    assert!(exported.contains("1. e4 c5"));
    assert!(exported.contains("19. Nxe6"));
}

// Byte offset of the low byte of an index entry's big-endian flags field
// (offset u32, length u16, length-high u8, then flags u16).
const ENTRY_FLAGS_LOW_BYTE: usize = 8;

/// Copies a test database into `dir` and sets the logical delete flag on the
/// given entries directly in the copied .si4, leaving every record in place.
fn copy_with_deleted(dir: &std::path::Path, name: &str, deleted: &[u32]) -> std::path::PathBuf {
    let src = get_workspace_root().join("tests/data").join(name);
    let dst = dir.join(name);
    for ext in ["si4", "sn4", "sg4"] {
        std::fs::copy(src.with_extension(ext), dst.with_extension(ext))
            .expect("Failed to copy database");
    }
    set_deleted(&dst, deleted);
    dst
}

/// Sets the logical delete flag on the given entries of the database at
/// `db`, directly in its .si4. No other entry may already be deleted.
fn set_deleted(db: &Path, deleted: &[u32]) {
    let si4 = db.with_extension("si4");
    let mut bytes = std::fs::read(&si4).expect("Failed to read index");
    for &n in deleted {
        let pos = scidtopgn::index::INDEX_HEADER_SIZE
            + n as usize * scidtopgn::index::INDEX_ENTRY_SIZE
            + ENTRY_FLAGS_LOW_BYTE;
        bytes[pos] |= scidtopgn::index::IDX_MASK_DELETE as u8;
    }
    std::fs::write(&si4, bytes).expect("Failed to write index");

    let db = scidtopgn::Database::open(db).expect("Failed to open database");
    for n in 0..db.num_games() {
        assert_eq!(
            db.get_entry(n).unwrap().get_delete_flag(),
            deleted.contains(&n)
        );
    }
}

fn database_bytes(db: &std::path::Path) -> Vec<Vec<u8>> {
    ["si4", "sn4", "sg4"]
        .iter()
        .map(|ext| std::fs::read(db.with_extension(ext)).expect("Failed to read database file"))
        .collect()
}

/// Runs the default invocation and the explicit export subcommand, asserting
/// both succeed with identical stdout, and returns that output.
fn export_both_ways(db: &std::path::Path) -> String {
    let bin = env!("CARGO_BIN_EXE_scidtopgn");
    let default = Command::new(bin)
        .arg(db)
        .output()
        .expect("Failed to run CLI");
    let explicit = Command::new(bin)
        .arg("export")
        .arg(db)
        .output()
        .expect("Failed to run CLI");

    for output in [&default, &explicit] {
        assert!(
            output.status.success(),
            "Export should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let stdout = String::from_utf8(default.stdout).expect("Output should be UTF-8");
    assert_eq!(
        stdout,
        String::from_utf8_lossy(&explicit.stdout),
        "Both invocations should match"
    );
    stdout
}

#[test]
fn test_cli_export_skips_deleted_games() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let deleted = [1, 3];
    let db_path = copy_with_deleted(temp.path(), "five", &deleted);
    let before = database_bytes(&db_path);

    let stdout = export_both_ways(&db_path);

    let mut db = scidtopgn::Database::open(&db_path).expect("Failed to open database");
    let expected: String = (0..db.num_games())
        .filter(|n| !deleted.contains(n))
        .map(|n| format!("{}\n", db.get_game(n).unwrap().to_pgn()))
        .collect();
    assert_eq!(stdout.matches("[Event \"").count(), 3);
    assert_eq!(
        stdout, expected,
        "Nondeleted games should export in order, unchanged"
    );

    assert_eq!(db.num_games(), 5, "Deleted records should remain");
    assert!(
        database_bytes(&db_path) == before,
        "Export should not modify the database"
    );
}

#[test]
fn test_cli_export_all_deleted() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let db_path = copy_with_deleted(temp.path(), "five", &[0, 1, 2, 3, 4]);
    let before = database_bytes(&db_path);

    let stdout = export_both_ways(&db_path);

    assert!(stdout.is_empty(), "Expected no games, got: {}", stdout);
    assert!(
        database_bytes(&db_path) == before,
        "Export should not modify the database"
    );
}

/// The main lines of the games in `pgn`, in SAN.
fn main_lines(pgn: &str) -> Vec<Vec<String>> {
    scidtopgn::parse_pgn(pgn)
        .expect("PGN should parse")
        .iter()
        .map(|game| {
            let mut sans = Vec::new();
            let mut idx = game.first_move;
            while let Some(i) = idx {
                if game.moves[i].move_data.is_some() {
                    sans.push(game.moves[i].san.clone());
                }
                idx = game.moves[i].next;
            }
            sans
        })
        .collect()
}

#[test]
fn test_cli_import_five_games() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp.path().join("five");

    let output = cli()
        .args(["import", "tests/data/five.pgn", "--on-error", "abort", "-o"])
        .arg(&db_path)
        .output()
        .expect("Failed to run CLI");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let exported = cli().arg(&db_path).output().expect("Failed to run CLI");
    let source = std::fs::read_to_string(get_workspace_root().join("tests/data/five.pgn"))
        .expect("Failed to read five.pgn");
    let exported = String::from_utf8_lossy(&exported.stdout);
    assert_eq!(main_lines(&exported), main_lines(&source));
    assert_eq!(main_lines(&exported).len(), 5);
}

const VALID_GAME: &str = "[Event \"Valid\"]\n[Result \"1-0\"]\n\n1. e4 e5 1-0\n";
const ILLEGAL_GAME: &str = "[Event \"Illegal\"]\n[Result \"1-0\"]\n\n1. e4 e5 2. Ke3 1-0\n";

fn write_mixed_pgn(dir: &Path) -> PathBuf {
    let path = dir.join("mixed.pgn");
    std::fs::write(&path, [VALID_GAME, ILLEGAL_GAME, VALID_GAME].join("\n"))
        .expect("Failed to write PGN");
    path
}

#[test]
fn test_cli_import_skips_invalid_games() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let pgn = write_mixed_pgn(temp.path());
    let db_path = temp.path().join("mixed");

    let output = cli()
        .arg("import")
        .arg(&pgn)
        .arg("-o")
        .arg(&db_path)
        .arg("--verbose")
        .output()
        .expect("Failed to run CLI");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(
        stderr.contains("Warning: skipping PGN game 2, starting at line 6"),
        "{stderr}"
    );
    assert!(stderr.contains("\"Ke3\""), "{stderr}");
    assert!(stderr.contains("2 games imported, 1 skipped"), "{stderr}");

    let db = scidtopgn::Database::open(&db_path).expect("Failed to open database");
    assert_eq!(db.num_games(), 2);
}

#[test]
fn test_cli_import_aborts_on_invalid_game() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let pgn = write_mixed_pgn(temp.path());
    let db_path = temp.path().join("mixed");

    let output = cli()
        .arg("import")
        .arg(&pgn)
        .arg("-o")
        .arg(&db_path)
        .args(["--on-error", "abort"])
        .output()
        .expect("Failed to run CLI");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("PGN game 2"), "{stderr}");
    assert!(
        !db_path.with_extension("si4").exists(),
        "an aborted import shouldn't create the database"
    );
}

/// Moves `buf` past the tags at the start of a game record: the compact
/// event date (255), common tags stored by code (241 to 254), and tags
/// stored by name, up to the 0 that ends them.
fn skip_tags(buf: &mut scidtopgn::ByteBuffer) {
    loop {
        match buf.get_byte().unwrap() {
            0 => break,
            255 => buf.skip(3).unwrap(),
            code if code > 240 => {
                let len = buf.get_byte().unwrap();
                buf.skip(usize::from(len)).unwrap();
            }
            name_len => {
                buf.skip(usize::from(name_len)).unwrap();
                let len = buf.get_byte().unwrap();
                buf.skip(usize::from(len)).unwrap();
            }
        }
    }
}

#[test]
fn test_cli_export_names_corrupt_game() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let db = copy_with_deleted(temp.path(), "one", &[]);

    // Replace 1.e4 (piece 12, value 15) with the illegal exf3.
    let offset = scidtopgn::Database::open(&db)
        .expect("Failed to open database")
        .get_entry(0)
        .expect("one has a game")
        .get_offset() as usize;
    let sg4 = db.with_extension("sg4");
    let mut bytes = std::fs::read(&sg4).expect("Failed to read .sg4");
    let mut record = scidtopgn::ByteBuffer::from_slice(&bytes[offset..]);
    skip_tags(&mut record);
    // The flags byte comes before the first move.
    let first_move = offset + record.position() + 1;
    assert_eq!(bytes[first_move], 0xCF, "one starts with 1.e4");
    bytes[first_move] = 0xC2;
    std::fs::write(&sg4, bytes).expect("Failed to write .sg4");

    let output = cli().arg(&db).output().expect("Failed to run CLI");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stderr.contains("SCID game 1"), "{stderr}");
    let damaged = format!("Damaged game data at byte {}", first_move - offset);
    assert!(stderr.contains(&damaged), "{stderr}");
    assert!(stderr.contains("invalid move byte 0xc2"), "{stderr}");
}

/// Imports `pgn` with `--verbose` and returns the database and stderr.
fn import_pgn(dir: &Path, pgn: &str) -> (PathBuf, String) {
    let pgn_path = dir.join("input.pgn");
    std::fs::write(&pgn_path, pgn).expect("Failed to write PGN");
    let db_path = dir.join("imported");
    let output = cli()
        .arg("import")
        .arg(&pgn_path)
        .arg("-o")
        .arg(&db_path)
        .arg("--verbose")
        .output()
        .expect("Failed to run CLI");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "{stderr}");
    (db_path, stderr)
}

fn imported_events(db_path: &Path) -> Vec<String> {
    let mut db = scidtopgn::Database::open(db_path).expect("Failed to open database");
    db.games()
        .map(|game| game.expect("Failed to read game").event)
        .collect()
}

#[test]
fn test_cli_import_keeps_game_after_unterminated_tag() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let pgn =
        "[Event \"Broken]\n\n1. d4 d5 *\n\n[Event \"Good\"]\n[Result \"1-0\"]\n\n1. e4 e5 1-0\n";
    let (db_path, stderr) = import_pgn(temp.path(), pgn);
    assert!(stderr.contains("Warning: skipping PGN game 1"), "{stderr}");
    assert!(stderr.contains("1 games imported, 1 skipped"), "{stderr}");
    assert_eq!(imported_events(&db_path), ["Good"]);
}

#[test]
fn test_cli_import_recovers_at_indented_tags() {
    for indent in ["  ", "\t"] {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let pgn = [
            format!("{indent}[Event \"First\"]\n\n1. e4 e5 1-0\n"),
            format!("{indent}[Event \"Illegal\"]\n\n1. Ke3 *\n"),
            format!("{indent}[Event \"Last\"]\n{indent}[Result \"1-0\"]\n\n1. d4 d5 1-0\n"),
        ]
        .join("\n");
        let (db_path, stderr) = import_pgn(temp.path(), &pgn);
        assert!(stderr.contains("Warning: skipping PGN game 2"), "{stderr}");
        assert!(stderr.contains("2 games imported, 1 skipped"), "{stderr}");
        assert_eq!(imported_events(&db_path), ["First", "Last"]);
    }
}

#[test]
fn test_cli_import_recovers_past_line_comments() {
    for comment in ["; brace in a line comment: {", "% note {"] {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let pgn = format!(
            "[Event \"Bad\"]\n\n1. Ke3 *\n{comment}\n\n[Event \"Good\"]\n[Result \"1-0\"]\n\n1. e4 e5 1-0\n"
        );
        let (db_path, stderr) = import_pgn(temp.path(), &pgn);
        assert!(stderr.contains("Warning: skipping PGN game 1"), "{stderr}");
        assert!(stderr.contains("1 games imported, 1 skipped"), "{stderr}");
        assert_eq!(imported_events(&db_path), ["Good"]);
    }
}

#[test]
fn test_cli_import_keeps_text_before_games() {
    let pgn = "Games from the 2026 club championship.\nDownloaded from example.org.\n\n\
        [Event \"First\"]\n[Result \"1-0\"]\n\n1. e4 e5 1-0 [Round 2 notes]\n\n\
        Round 2 starts here.\n\n\
        [Event \"Second\"]\n[Result \"0-1\"]\n\n1. d4 zz d5 0-1\n";
    for mode in ["skip", "abort"] {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let pgn_path = temp.path().join("pregame.pgn");
        std::fs::write(&pgn_path, pgn).expect("Failed to write PGN");
        let db_path = temp.path().join("pregame");
        let output = cli()
            .arg("import")
            .arg(&pgn_path)
            .arg("-o")
            .arg(&db_path)
            .args(["--on-error", mode, "--verbose"])
            .output()
            .expect("Failed to run CLI");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{mode}: {stderr}");
        assert!(
            stderr.contains("Warning: PGN game 2, line 14: skipped unknown text \"zz\""),
            "{mode}: {stderr}"
        );
        assert!(
            stderr.contains("2 games imported, 0 skipped"),
            "{mode}: {stderr}"
        );
        assert_eq!(imported_events(&db_path), ["First", "Second"]);

        // The text is stored and exported as each game's first comment.
        let output = cli().arg(&db_path).output().expect("Failed to run CLI");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(
                "{Games from the 2026 club championship.\nDownloaded from example.org.} 1. e4 e5"
            ),
            "{stdout}"
        );
        assert!(
            stdout.contains("{[Round 2 notes]\n\nRound 2 starts here.} 1. d4 d5"),
            "{stdout}"
        );
    }
}

#[test]
fn test_cli_import_accepts_tag_pairs_across_lines() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let pgn_path = temp.path().join("split.pgn");
    std::fs::write(
        &pgn_path,
        "[Event\r\n \"Good\"]\r\n[Result \"1-0\"\r\n]\r\n\r\n1. e4 e5 1-0\r\n",
    )
    .expect("Failed to write PGN");
    let db_path = temp.path().join("split");
    let output = cli()
        .arg("import")
        .arg(&pgn_path)
        .arg("-o")
        .arg(&db_path)
        .args(["--on-error", "abort"])
        .output()
        .expect("Failed to run CLI");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(imported_events(&db_path), ["Good"]);
    let mut db = scidtopgn::Database::open(&db_path).expect("Failed to open database");
    assert_eq!(
        db.get_game(0).expect("Failed to read game").result,
        scidtopgn::GameResult::White
    );
}

#[test]
fn test_cli_import_accepts_delimiters_attached_to_moves() {
    let pgn = "[Event \"A\"]\n\n1. e4; comment\n e5 *\n\n[Event \"B\"]\n\n1. d4$1 d5*\n";
    for mode in ["skip", "abort"] {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let pgn_path = temp.path().join("adjacent.pgn");
        std::fs::write(&pgn_path, pgn).expect("Failed to write PGN");
        let db_path = temp.path().join("adjacent");
        let output = cli()
            .arg("import")
            .arg(&pgn_path)
            .arg("-o")
            .arg(&db_path)
            .args(["--on-error", mode])
            .output()
            .expect("Failed to run CLI");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{mode}: {stderr}");
        assert!(!stderr.contains("Warning"), "{mode}: {stderr}");
        let exported = cli().arg(&db_path).output().expect("Failed to run CLI");
        assert_eq!(
            main_lines(&String::from_utf8_lossy(&exported.stdout)),
            [["e4", "e5"], ["d4", "d5"]],
            "{mode}"
        );
    }
}

#[test]
fn test_cli_import_prints_warnings_and_keeps_the_game() {
    let pgn = "[Event \"Castled\"]\n[Result \"1-0\"]\n\n1. e4 e5 2. Nf3 Nf6 3. Bc4 Bc5\n4. Ke2 Ke7 5. Ke1 Ke8 6. O-O zz 1-0\n\n[Event \"Plain\"]\n\n1. d4 d5 *\n";
    for mode in ["skip", "abort"] {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let pgn_path = temp.path().join("input.pgn");
        std::fs::write(&pgn_path, pgn).expect("Failed to write PGN");
        let db_path = temp.path().join("imported");
        let output = cli()
            .arg("import")
            .arg(&pgn_path)
            .arg("-o")
            .arg(&db_path)
            .args(["--on-error", mode, "--verbose"])
            .output()
            .expect("Failed to run CLI");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{mode}: {stderr}");
        assert!(
            stderr.contains(
                "Warning: PGN game 1, line 5: \"O-O\" castles without the castling right"
            ),
            "{mode}: {stderr}"
        );
        assert!(
            stderr.contains("Warning: PGN game 1, line 5: skipped unknown text \"zz\""),
            "{mode}: {stderr}"
        );
        assert!(
            stderr.contains("2 games imported, 0 skipped"),
            "{mode}: {stderr}"
        );
        assert_eq!(imported_events(&db_path), ["Castled", "Plain"]);

        // The castle is exported like any other.
        let exported = cli().arg(&db_path).output().expect("Failed to run CLI");
        let exported = String::from_utf8_lossy(&exported.stdout);
        assert!(exported.contains("6. O-O"), "{exported}");
    }
}

#[test]
fn test_cli_import_reads_text_that_isnt_utf8_as_windows_1252() {
    let pgn = b"[Event \"\xe9v\xe9nement\"]\n[White \"M\xfcller\"]\n\n1. e4 *\n";
    for mode in ["skip", "abort"] {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let pgn_path = temp.path().join("latin1.pgn");
        std::fs::write(&pgn_path, pgn).expect("Failed to write PGN");
        // From a file, and from stdin.
        for (input, name) in [(pgn_path.as_os_str(), "file"), ("-".as_ref(), "stdin")] {
            let db_path = temp.path().join(name);
            let output = cli()
                .arg("import")
                .arg(input)
                .arg("-o")
                .arg(&db_path)
                .args(["--on-error", mode])
                .stdin(std::fs::File::open(&pgn_path).expect("Failed to open PGN"))
                .output()
                .expect("Failed to run CLI");
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(output.status.success(), "{mode} {name}: {stderr}");
            assert_eq!(
                stderr, "Warning: PGN game 1, line 1: read text that isn't UTF-8 as Windows-1252\n",
                "{mode} {name}"
            );

            let exported = cli().arg(&db_path).output().expect("Failed to run CLI");
            let exported = String::from_utf8(exported.stdout).expect("Output should be UTF-8");
            assert!(
                exported.contains("[Event \"événement\"]\n"),
                "{mode} {name}: {exported}"
            );
            assert!(
                exported.contains("[White \"Müller\"]\n"),
                "{mode} {name}: {exported}"
            );
        }
    }
}

#[test]
fn test_cli_round_trip_keeps_annotations_and_variations() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let pgn_path = temp.path().join("nag.pgn");
    std::fs::write(
        &pgn_path,
        "[Event \"x\"]\n\n1. e4 $1 e5 {after e5} 2. Nf3 $2 (2. f4 $5) *\n",
    )
    .expect("Failed to write PGN");
    let db_path = temp.path().join("nag");
    let output = cli()
        .arg("import")
        .arg(&pgn_path)
        .arg("-o")
        .arg(&db_path)
        .output()
        .expect("Failed to run CLI");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let exported = cli().arg(&db_path).output().expect("Failed to run CLI");
    let stdout = String::from_utf8_lossy(&exported.stdout);
    assert!(
        stdout.contains("1. e4 ! e5 {after e5} 2. Nf3 ? (2. f4 !?)\n*\n"),
        "{stdout}"
    );
}

#[test]
fn test_cli_without_arguments() {
    let output = cli().output().expect("Failed to run CLI");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Specify a database or use a subcommand (export/import/info)"),
        "{stderr}"
    );
}

#[test]
fn test_cli_export_keeps_an_existing_output_file() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let output_path = temp.path().join("out.pgn");
    std::fs::write(&output_path, "keep").expect("Failed to write file");

    for args in [&["tests/data/one"][..], &["export", "tests/data/one"]] {
        let output = cli()
            .args(args)
            .arg("-o")
            .arg(&output_path)
            .output()
            .expect("Failed to run CLI");
        assert!(!output.status.success(), "{args:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("exists. Use --overwrite to replace."),
            "{stderr}"
        );
        assert_eq!(std::fs::read_to_string(&output_path).unwrap(), "keep");
    }

    let output = cli()
        .args(["export", "tests/data/one", "--overwrite", "-o"])
        .arg(&output_path)
        .output()
        .expect("Failed to run CLI");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let pgn = std::fs::read_to_string(&output_path).unwrap();
    assert!(pgn.starts_with("[Event "), "{pgn}");
}

#[test]
fn test_cli_missing_inputs() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let missing = temp.path().join("missing");

    let output = cli().arg(&missing).output().expect("Failed to run CLI");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to open database"), "{stderr}");

    let output = cli()
        .arg("import")
        .arg(missing.with_extension("pgn"))
        .arg("-o")
        .arg(temp.path().join("db"))
        .output()
        .expect("Failed to run CLI");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // The message differs by platform; the error code doesn't.
    assert!(stderr.contains("(os error 2)"), "{stderr}");
    assert!(!temp.path().join("db.si4").exists());
}

#[test]
fn test_cli_verbose_progress() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let pgn: String = (1..=100)
        .map(|n| format!("[Event \"Game {n}\"]\n\n1. e4 *\n\n"))
        .collect();
    let (db_path, stderr) = import_pgn(temp.path(), &pgn);
    let tenths: String = (1..=10)
        .map(|n| format!("Processed {}% ({}/100 games)\n", n * 10, n * 10))
        .collect();
    assert!(stderr.contains(&tenths), "{stderr}");

    let output = cli()
        .arg(&db_path)
        .arg("--verbose")
        .output()
        .expect("Failed to run CLI");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr, format!("{tenths}Done. 100 games exported.\n"));

    // Importing again appends, and --overwrite replaces.
    let pgn_path = temp.path().join("input.pgn");
    for (args, message, count) in [
        (&[][..], "Appending to existing database", 200),
        (&["--overwrite"], "Overwriting existing database", 100),
    ] {
        let output = cli()
            .arg("import")
            .arg(&pgn_path)
            .args(args)
            .arg("-o")
            .arg(&db_path)
            .arg("--verbose")
            .output()
            .expect("Failed to run CLI");
        assert!(output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(message), "{stderr}");
        let db = scidtopgn::Database::open(&db_path).expect("Failed to open database");
        assert_eq!(db.num_games(), count, "{args:?}");
    }
}

#[test]
fn test_cli_round_trip_keeps_games_without_moves() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let fen = "[SetUp \"1\"]\n[FEN \"4k3/8/8/8/8/8/8/4K3 w - - 0 1\"]\n";
    let pgn = format!(
        "[Event \"A\"]\n[Result \"*\"]\n\n*\n\n\
         [Event \"B\"]\n[Result \"1-0\"]\n\n{{only a comment}} 1-0\n\n\
         [Event \"C\"]\n[Result \"1/2-1/2\"]\n{fen}\n1/2-1/2\n"
    );
    let (db_path, _) = import_pgn(temp.path(), &pgn);
    assert_eq!(imported_events(&db_path), ["A", "B", "C"]);

    let exported = cli().arg(&db_path).output().expect("Failed to run CLI");
    assert!(exported.status.success());
    let stdout = String::from_utf8_lossy(&exported.stdout);
    assert!(stdout.contains("[Result \"*\"]\n\n*\n"), "{stdout}");
    assert!(
        stdout.contains("[Result \"1-0\"]\n\n{only a comment}\n1-0\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains(&format!("[Result \"1/2-1/2\"]\n{fen}\n1/2-1/2\n")),
        "{stdout}"
    );
}

/// Runs the CLI on `db` with `args`, without and then with the `export`
/// subcommand.
fn export_outputs(db: &Path, args: &[&str]) -> [std::process::Output; 2] {
    [&[][..], &["export"][..]].map(|subcommand| {
        cli()
            .args(subcommand)
            .arg(db)
            .args(args)
            .output()
            .expect("Failed to run CLI")
    })
}

#[test]
fn test_cli_count_includes_deleted_games() {
    for (deleted, exported) in [(&[][..], 5), (&[1, 3][..], 3), (&[0, 1, 2, 3, 4][..], 0)] {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let db = copy_with_deleted(temp.path(), "five", deleted);
        for output in export_outputs(&db, &["--count"]) {
            assert!(output.status.success(), "{deleted:?}");
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                "5 games\n",
                "{deleted:?}"
            );
        }
        let pgn = export_both_ways(&db);
        assert_eq!(pgn.matches("[Event \"").count(), exported, "{deleted:?}");
    }

    for args in [&["--help"][..], &["export", "--help"]] {
        let output = cli().args(args).output().expect("Failed to run CLI");
        let help = String::from_utf8_lossy(&output.stdout);
        assert!(
            help.contains(
                "This includes logically deleted games, so it can be more than the number exported"
            ),
            "{help}"
        );
    }
}

/// The PGN that export writes for the given games, counting from 0.
fn pgn_of(db: &Path, games: &[u32]) -> String {
    let mut db = scidtopgn::Database::open(db).expect("Failed to open database");
    games
        .iter()
        .map(|&n| format!("{}\n", db.get_game(n).unwrap().to_pgn()))
        .collect()
}

#[test]
fn test_cli_export_selected_games() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    // Game 3 is deleted.
    let db = copy_with_deleted(temp.path(), "five", &[2]);
    for (games, expected) in [
        ("2", &[1][..]),
        ("4,1-2,2", &[0, 1, 3]),
        ("2-4", &[1, 3]),
        ("3", &[]),
        ("5,1-5", &[0, 1, 3, 4]),
    ] {
        for output in export_outputs(&db, &["--games", games]) {
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(output.status.success(), "{games}: {stderr}");
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                pgn_of(&db, expected),
                "{games}"
            );
        }
    }
}

#[test]
fn test_cli_export_rejects_bad_game_selections() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let out = temp.path().join("out.pgn");
    let out = out.to_str().expect("temp path should be UTF-8");
    let five = Path::new("tests/data/five");
    for (games, message) in [
        ("0", "games are numbered from 1"),
        ("3-1", "the range 3-1 ends before it starts"),
        ("x", "\"x\" isn't a game number"),
        ("1,,2", "\"\" isn't a game number"),
        ("6", "--games: no game 6 in the database, which has 5 games"),
        (
            "2,4-9",
            "--games: no game 9 in the database, which has 5 games",
        ),
    ] {
        for output in export_outputs(five, &["--games", games, "-o", out]) {
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(!output.status.success(), "{games}");
            assert!(stderr.contains(message), "{games}: {stderr}");
            assert!(!Path::new(out).exists(), "{games}: no output file");
        }
    }

    for output in export_outputs(five, &["--games", "1", "--count"]) {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success());
        assert!(stderr.contains("cannot be used with"), "{stderr}");
    }
}

#[test]
fn test_cli_verbose_progress_for_selected_games() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    // Game 3 is deleted, and still counts toward the progress.
    let db = copy_with_deleted(temp.path(), "five", &[2]);
    for output in export_outputs(&db, &["--games", "2-4", "--verbose"]) {
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            format!("{}Done. 2 games exported.\n", progress_of_3())
        );
    }

    let pgn: String = (1..=3)
        .map(|n| format!("[Event \"Game {n}\"]\n\n1. e4 *\n\n"))
        .collect();
    let (_, stderr) = import_pgn(temp.path(), &pgn);
    assert!(stderr.contains(&progress_of_3()), "{stderr}");
}

/// The progress `--verbose` reports for 3 games: each tenth once, with
/// the game that reached it.
fn progress_of_3() -> String {
    (1..=10u32)
        .map(|tenth| {
            let done = (tenth * 3).div_ceil(10);
            format!("Processed {}% ({done}/3 games)\n", tenth * 10)
        })
        .collect()
}

/// Runs `info` on `db` and returns its output.
fn info(db: &Path) -> String {
    let output = cli()
        .arg("info")
        .arg(db)
        .output()
        .expect("Failed to run CLI");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(stderr.is_empty(), "{stderr}");
    String::from_utf8(output.stdout).expect("Output should be UTF-8")
}

#[test]
fn test_cli_info() {
    let names = "Player names: 10\nEvent names: 3\nSite names: 3\nRound names: 5\n";
    let temp = TempDir::new().expect("Failed to create temp dir");
    let db = copy_with_deleted(temp.path(), "five", &[1, 3]);
    assert_eq!(
        info(&db),
        format!(
            "Total games: 5\nDeleted games: 2\nActive games: 3\n{names}\
             Years of active games: 2022-2022\n"
        )
    );

    set_deleted(&db, &[0, 1, 2, 3, 4]);
    assert_eq!(
        info(&db),
        format!(
            "Total games: 5\nDeleted games: 5\nActive games: 0\n{names}\
             Years of active games: unknown\n"
        )
    );
}

#[test]
fn test_cli_info_years() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let pgn = "[Event \"A\"]\n[Date \"2005.03.01\"]\n\n1. e4 *\n\n\
               [Event \"B\"]\n[Date \"2020.??.??\"]\n\n1. d4 *\n\n\
               [Event \"C\"]\n\n1. c4 *\n\n\
               [Event \"D\"]\n[Date \"1990.01.01\"]\n\n1. f4 *\n";
    let (db, _) = import_pgn(temp.path(), pgn);
    assert!(
        info(&db).ends_with("Active games: 4\nPlayer names: 1\nEvent names: 4\nSite names: 1\nRound names: 1\nYears of active games: 1990-2020\n"),
        "{}",
        info(&db)
    );

    // Deleted games don't count.
    set_deleted(&db, &[1]);
    assert!(info(&db).ends_with("Years of active games: 1990-2005\n"));

    let temp = TempDir::new().expect("Failed to create temp dir");
    let (db, _) = import_pgn(temp.path(), "[Event \"C\"]\n\n1. c4 *\n");
    assert!(info(&db).ends_with("Years of active games: unknown\n"));
}

#[test]
fn test_cli_info_reads_only_the_index_and_name_files() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let db = copy_with_deleted(temp.path(), "five", &[]);
    let sg4 = db.with_extension("sg4");
    let expected = info(&db);

    // Read-only, so it can't be opened for writing as `Database::open` does.
    let mut permissions = std::fs::metadata(&sg4).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&sg4, permissions.clone()).unwrap();
    assert_eq!(info(&db), expected);
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    std::fs::set_permissions(&sg4, permissions).unwrap();

    // Damaged, so export fails.
    std::fs::write(&sg4, []).expect("Failed to truncate .sg4");
    let output = cli().arg(&db).output().expect("Failed to run CLI");
    assert!(!output.status.success(), "export should fail");
    assert_eq!(info(&db), expected);

    std::fs::remove_file(&sg4).expect("Failed to remove .sg4");
    assert_eq!(info(&db), expected);
}

#[test]
fn test_cli_quiet_hides_stdout_errors() {
    for args in [
        &["tests/data/five", "--quiet"][..],
        &["tests/data/five", "--count", "--quiet"],
        &["export", "tests/data/five", "--count", "--quiet"],
        &["info", "tests/data/five", "--quiet"],
        &["info", "tests/data/five"],
    ] {
        // Writing to a pipe whose reading end is closed fails.
        let (reader, writer) = std::io::pipe().expect("Failed to create pipe");
        drop(reader);
        let output = cli()
            .args(args)
            .stdout(writer)
            .output()
            .expect("Failed to run CLI");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "{args:?}: {stderr}");
        assert!(!stderr.contains("panicked"), "{args:?}: {stderr}");
        if args.contains(&"--quiet") {
            assert!(stderr.is_empty(), "{args:?}: {stderr}");
        } else {
            assert!(stderr.starts_with("Error: "), "{args:?}: {stderr}");
        }
    }
}

#[test]
fn test_cli_quiet_import_writes_nothing_to_stderr() {
    let castled = "[Event \"Castled\"]\n[Result \"1-0\"]\n\n1. e4 e5 2. Nf3 Nf6 3. Bc4 Bc5\n4. Ke2 Ke7 5. Ke1 Ke8 6. O-O 1-0\n";
    let temp = TempDir::new().expect("Failed to create temp dir");
    let pgn = temp.path().join("input.pgn");
    std::fs::write(&pgn, [castled, ILLEGAL_GAME, VALID_GAME].join("\n"))
        .expect("Failed to write PGN");

    for (flag, warns) in [("--on-error=skip", true), ("--quiet", false)] {
        let db = temp.path().join(flag.trim_start_matches('-'));
        let output = cli()
            .arg("import")
            .arg(&pgn)
            .arg("-o")
            .arg(&db)
            .arg(flag)
            .output()
            .expect("Failed to run CLI");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{flag}: {stderr}");
        assert_eq!(
            stderr.contains(
                "Warning: PGN game 1, line 5: \"O-O\" castles without the castling right"
            ),
            warns,
            "{flag}: {stderr}"
        );
        assert_eq!(
            stderr.contains("Warning: skipping PGN game 2"),
            warns,
            "{flag}: {stderr}"
        );
        if !warns {
            assert!(stderr.is_empty(), "{flag}: {stderr}");
        }
        assert_eq!(imported_events(&db), ["Castled", "Valid"], "{flag}");
    }
}

#[test]
fn test_cli_quiet_hides_errors() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let missing = temp.path().join("missing");
    let missing = missing.to_str().expect("temp path should be UTF-8");
    let db = temp.path().join("db");
    let db = db.to_str().expect("temp path should be UTF-8");
    for args in [
        &[missing, "--quiet"][..],
        &["export", missing, "-q"],
        &["info", missing, "--quiet"],
        &["import", missing, "-o", db, "--quiet"],
    ] {
        let output = cli().args(args).output().expect("Failed to run CLI");
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stderr.is_empty(), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
    }

    // A command's results are still written.
    let five = Path::new("tests/data/five");
    for output in export_outputs(five, &["--quiet"]) {
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout.matches("[Event \"").count(), 5);
    }
    for output in export_outputs(five, &["--count", "--quiet"]) {
        assert!(output.stderr.is_empty());
        assert_eq!(String::from_utf8_lossy(&output.stdout), "5 games\n");
    }
    let output = cli()
        .args(["info", "tests/data/five", "--quiet"])
        .output()
        .expect("Failed to run CLI");
    assert!(output.stderr.is_empty());
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("Total games: 5\n"));

    // --quiet and --verbose can't be combined.
    for args in [
        &["tests/data/five", "-q", "-v"][..],
        &["export", "tests/data/five", "--quiet", "--verbose"],
        &["import", "tests/data/five.pgn", "-o", db, "-q", "-v"],
    ] {
        let output = cli().args(args).output().expect("Failed to run CLI");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{args:?}");
        assert!(stderr.contains("cannot be used with"), "{args:?}: {stderr}");
    }
    assert!(!Path::new(db).with_extension("si4").exists());
}

#[test]
fn test_cli_rejects_options_before_a_subcommand() {
    // Run in the temp dir with relative paths, so the suggested command
    // needs no quoting on any platform.
    let temp = TempDir::new().expect("Failed to create temp dir");
    copy_with_deleted(temp.path(), "five", &[]);
    for (args, fixed) in [
        (
            &["--games", "6", "export", "five", "-o", "out.pgn"][..],
            "scidtopgn export five -o out.pgn --games 6",
        ),
        (
            &["--games", "1", "export", "five", "-o", "out.pgn"],
            "scidtopgn export five -o out.pgn --games 1",
        ),
        (
            &["--quiet", "export", "missing"],
            "scidtopgn export missing --quiet",
        ),
    ] {
        let output = cli()
            .current_dir(temp.path())
            .args(args)
            .output()
            .expect("Failed to run CLI");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(2), "{args:?}: {stderr}");
        assert!(stderr.contains(fixed), "{args:?}: {stderr}");
        assert!(output.stdout.is_empty(), "{args:?}");
        assert!(
            !temp.path().join("out.pgn").exists(),
            "{args:?}: nothing is exported"
        );
    }
}
