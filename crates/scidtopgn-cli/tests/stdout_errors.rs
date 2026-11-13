//! Tests how the CLI handles a failed write to stdout.
//!
//! This is its own test binary because no other test may start a process
//! while it runs. `std::io::pipe()` sets close-on-exec only after it creates
//! the pipe on targets without `pipe2`, such as macOS. A process another test
//! starts in between inherits the read end and keeps it open, so the CLI's
//! write succeeds. Only add tests here that don't start processes.

use std::path::Path;
use std::process::Command;

/// Returns a command for the CLI binary, run from the workspace root so the
/// fixture paths under `tests/data` resolve.
fn cli() -> Command {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crate should be two levels below the workspace root");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_scidtopgn"));
    cmd.current_dir(root);
    cmd
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
