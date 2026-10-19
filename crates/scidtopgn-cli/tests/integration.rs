use std::process::Command;

use tempfile::TempDir;

fn get_workspace_root() -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    if manifest_dir.contains("/crates/") {
        let parent = std::path::Path::new(&manifest_dir)
            .parent()
            .and_then(|p| p.parent())
            .unwrap();
        parent.to_string_lossy().to_string()
    } else {
        manifest_dir
    }
}

#[test]
fn test_cli_count() {
    let workspace = get_workspace_root();

    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "scidtopgn-cli",
            "--release",
            "--",
            "tests/data/one",
            "--count",
        ])
        .current_dir(&workspace)
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
fn test_cli_output() {
    let workspace = get_workspace_root();

    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "scidtopgn-cli",
            "--release",
            "--",
            "tests/data/one",
        ])
        .current_dir(&workspace)
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
    let workspace = get_workspace_root();

    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "scidtopgn-cli",
            "--release",
            "--",
            "tests/data/five",
        ])
        .current_dir(&workspace)
        .output()
        .expect("Failed to run CLI");

    let stdout = String::from_utf8_lossy(&output.stdout);

    let game_count = stdout.matches("[Event \"").count();
    assert_eq!(game_count, 5, "Expected 5 games, found {}", game_count);
}

#[test]
fn test_cli_output_to_file() {
    let workspace = get_workspace_root();
    let temp = TempDir::new().expect("Failed to create temp dir");
    let output_path = temp.path().join("output.pgn");

    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "scidtopgn-cli",
            "--release",
            "--",
            "tests/data/one",
            "-o",
        ])
        .arg(&output_path)
        .current_dir(&workspace)
        .output()
        .expect("Failed to run CLI");

    assert!(output.status.success(), "CLI should succeed");
    assert!(output_path.exists(), "Output file should exist");

    let contents = std::fs::read_to_string(&output_path).expect("Failed to read output");
    assert!(contents.contains("[Event \"47th ch-Bangahbandhu 2022\"]"));
}

#[test]
fn test_cli_import() {
    let workspace = get_workspace_root();
    let temp = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp.path().join("imported");

    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "scidtopgn-cli",
            "--release",
            "--",
            "import",
            "tests/data/one.pgn",
            "-o",
        ])
        .arg(&db_path)
        .current_dir(&workspace)
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
    let workspace = get_workspace_root();
    let temp = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp.path().join("append_test");

    let output1 = Command::new("cargo")
        .args([
            "run",
            "-p",
            "scidtopgn-cli",
            "--release",
            "--",
            "import",
            "tests/data/one.pgn",
            "-o",
        ])
        .arg(&db_path)
        .current_dir(&workspace)
        .output()
        .expect("Failed to run CLI");
    assert!(output1.status.success());

    let output2 = Command::new("cargo")
        .args([
            "run",
            "-p",
            "scidtopgn-cli",
            "--release",
            "--",
            "import",
            "tests/data/one.pgn",
            "-o",
        ])
        .arg(&db_path)
        .current_dir(&workspace)
        .output()
        .expect("Failed to run CLI");
    assert!(output2.status.success());

    let db = scidtopgn::Database::open(&db_path).expect("Failed to open database");
    assert_eq!(db.num_games(), 2, "Should have 2 games after append");
}

#[test]
fn test_cli_import_overwrite() {
    let workspace = get_workspace_root();
    let temp = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp.path().join("overwrite_test");

    let output1 = Command::new("cargo")
        .args([
            "run",
            "-p",
            "scidtopgn-cli",
            "--release",
            "--",
            "import",
            "tests/data/one.pgn",
            "-o",
        ])
        .arg(&db_path)
        .current_dir(&workspace)
        .output()
        .expect("Failed to run CLI");
    assert!(output1.status.success());

    let output2 = Command::new("cargo")
        .args([
            "run",
            "-p",
            "scidtopgn-cli",
            "--release",
            "--",
            "import",
            "tests/data/one.pgn",
            "--overwrite",
            "-o",
        ])
        .arg(&db_path)
        .current_dir(&workspace)
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
    let workspace = get_workspace_root();
    let temp = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp.path().join("roundtrip");
    let pgn_path = temp.path().join("exported.pgn");

    let import_output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "scidtopgn-cli",
            "--release",
            "--",
            "import",
            "tests/data/one.pgn",
            "-o",
        ])
        .arg(&db_path)
        .current_dir(&workspace)
        .output()
        .expect("Failed to run import");
    assert!(import_output.status.success());

    let export_output = Command::new("cargo")
        .args(["run", "-p", "scidtopgn-cli", "--release", "--", "export"])
        .arg(&db_path)
        .arg("-o")
        .arg(&pgn_path)
        .current_dir(&workspace)
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
    let workspace = get_workspace_root();
    let src = std::path::Path::new(&workspace)
        .join("tests/data")
        .join(name);
    let dst = dir.join(name);
    for ext in ["si4", "sn4", "sg4"] {
        std::fs::copy(src.with_extension(ext), dst.with_extension(ext))
            .expect("Failed to copy database");
    }

    let si4 = dst.with_extension("si4");
    let mut bytes = std::fs::read(&si4).expect("Failed to read index");
    for &n in deleted {
        let pos = scidtopgn::index::INDEX_HEADER_SIZE
            + n as usize * scidtopgn::index::INDEX_ENTRY_SIZE
            + ENTRY_FLAGS_LOW_BYTE;
        bytes[pos] |= scidtopgn::index::IDX_MASK_DELETE as u8;
    }
    std::fs::write(&si4, bytes).expect("Failed to write index");

    let db = scidtopgn::Database::open(&dst).expect("Failed to open copied database");
    for n in 0..db.num_games() {
        assert_eq!(
            db.get_entry(n).unwrap().get_delete_flag(),
            deleted.contains(&n)
        );
    }
    dst
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
