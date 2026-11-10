//! Runs the library's delete, undelete, and update operations on a SCID
//! database, which the CLI doesn't offer. `compat/run` uses it to change
//! SCID-written databases; see `docs/SCID_COMPAT.md`.
//!
//! Games are numbered from 1, as in SCID:
//!
//! ```text
//! compat_driver <database> delete <game>...
//! compat_driver <database> undelete <game>...
//! compat_driver <database> update <game> <pgn-file> <number in file>
//! ```

use std::error::Error;
use std::process::ExitCode;

use scidtopgn::{parse_pgn, Database};

const USAGE: &str = "usage: compat_driver <database> (delete <game>... | undelete <game>... | \
                     update <game> <pgn-file> <number in file>)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("compat_driver: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let [database, operation, rest @ ..] = args else {
        return Err(USAGE.into());
    };
    let mut db = Database::open(database)?;
    match (operation.as_str(), rest) {
        ("delete", [_, ..]) => {
            for game in rest {
                db.delete_game(index(game)?)?;
            }
        }
        ("undelete", [_, ..]) => {
            for game in rest {
                db.undelete_game(index(game)?)?;
            }
        }
        ("update", [game, pgn_file, number]) => {
            let pgn = std::fs::read_to_string(pgn_file)
                .map_err(|err| format!("cannot read {pgn_file}: {err}"))?;
            let games = parse_pgn(&pgn)?;
            let replacement = games
                .get(index(number)? as usize)
                .ok_or_else(|| format!("{pgn_file} has no game {number}"))?;
            db.update_game(index(game)?, replacement)?;
        }
        _ => return Err(USAGE.into()),
    }
    db.close()?;
    Ok(())
}

/// Converts a game number counted from 1 to an index counted from 0.
fn index(number: &str) -> Result<u32, String> {
    match number.parse::<u32>() {
        Ok(n) if n >= 1 => Ok(n - 1),
        _ => Err(format!("{number:?} isn't a game number, counting from 1")),
    }
}
