mod args;
mod report;
mod selection;

use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;

use args::{Cli, Command, ExportArgs, ImportArgs, InfoArgs};
use report::{Progress, Verbosity};
use scidtopgn::{Database, NameType, PgnParser};

fn main() -> ExitCode {
    let cli = Cli::parse();
    let quiet = cli.quiet;
    let Some(command) = cli.into_command() else {
        if !quiet {
            eprintln!("Error: Specify a database or use a subcommand (export/import/info)");
        }
        return ExitCode::FAILURE;
    };

    let verbosity = command.verbosity();
    let result = match &command {
        Command::Export(args) => export_command(args, verbosity),
        Command::Import(args) => import_command(args, verbosity),
        Command::Info(args) => info_command(args),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            // The same report as returning the error from `main`.
            if verbosity != Verbosity::Quiet {
                eprintln!("Error: {err:?}");
            }
            ExitCode::FAILURE
        }
    }
}

fn export_command(args: &ExportArgs, verbosity: Verbosity) -> Result<()> {
    if let Some(ref output) = args.output {
        let path = std::path::Path::new(output);
        if path.exists() && !args.overwrite {
            anyhow::bail!(
                "Output file '{}' exists. Use --overwrite to replace.",
                output
            );
        }
    }

    let mut db = Database::open(&args.database)
        .with_context(|| format!("Failed to open database '{}'", args.database))?;

    if args.count {
        println!("{} games", db.num_games());
        return Ok(());
    }

    let total = db.num_games();
    // Check the selection before creating the output file.
    let (selected, indexes): (u32, Box<dyn Iterator<Item = u32>>) = match &args.games {
        Some(games) => {
            if games.last() > total {
                anyhow::bail!(
                    "--games: no game {} in the database, which has {} games",
                    games.last(),
                    total
                );
            }
            (games.count(), Box::new(games.indexes()))
        }
        None => (total, Box::new(0..total)),
    };

    let mut writer: Box<dyn Write> = match &args.output {
        Some(path) => {
            let file = File::create(path)?;
            Box::new(BufWriter::new(file))
        }
        None => Box::new(std::io::stdout()),
    };

    let mut progress = Progress::new(verbosity, selected.into());
    let mut exported = 0u32;
    for i in indexes {
        // Skip logically deleted games, matching the reference converter.
        if db.get_entry(i).is_some_and(|e| !e.get_delete_flag()) {
            let game = db.get_game(i)?;
            let pgn = game.to_pgn();
            writeln!(writer, "{}", pgn)?;
            exported += 1;
        }
        progress.advance();
    }

    verbosity.info(format_args!("Done. {} games exported.", exported));

    Ok(())
}

fn import_command(args: &ImportArgs, verbosity: Verbosity) -> Result<()> {
    // Read as bytes: the parser reads text that isn't UTF-8 as Windows-1252.
    let pgn = if args.pgn_file == "-" {
        let mut buf = Vec::new();
        std::io::stdin().read_to_end(&mut buf)?;
        buf
    } else {
        std::fs::read(&args.pgn_file)?
    };

    let abort_on_error = args.on_error == "abort";

    // Parse every game before touching the database, so that aborting on a
    // parse error leaves it unchanged.
    let mut games = Vec::new();
    let mut skipped = 0u32;
    for result in PgnParser::from_bytes(&pgn) {
        match result {
            Ok(game) => {
                // Warnings don't stop the game from being imported.
                for warning in &game.warnings {
                    verbosity.warn(format_args!("{warning}"));
                }
                games.push(game);
            }
            Err(e) if abort_on_error => return Err(e.into()),
            Err(e) => {
                verbosity.warn(format_args!("skipping {:#}", anyhow::Error::new(e)));
                skipped += 1;
            }
        }
    }

    verbosity.info(format_args!("Parsed {} games from PGN", games.len()));

    let db_path = std::path::Path::new(&args.output);
    let si4_exists = db_path.with_extension("si4").exists();

    let mut db = if si4_exists && !args.overwrite {
        verbosity.info(format_args!(
            "Appending to existing database: {}",
            args.output
        ));
        Database::open(&args.output)
            .with_context(|| format!("Failed to open database '{}'", args.output))?
    } else {
        if si4_exists {
            verbosity.info(format_args!(
                "Overwriting existing database: {}",
                args.output
            ));
        }
        Database::create(&args.output)
            .with_context(|| format!("Failed to create database '{}'", args.output))?
    };

    let initial_count = db.num_games();
    let mut added = 0u32;

    let mut progress = Progress::new(verbosity, games.len() as u64);
    for game in &games {
        match db.add_game(game) {
            Ok(_) => added += 1,
            Err(e) if abort_on_error => {
                anyhow::bail!("Failed to add game: {}", e);
            }
            Err(e) => {
                verbosity.info(format_args!(
                    "Warning: Failed to add game (skipping): {}",
                    e
                ));
                skipped += 1;
            }
        }
        progress.advance();
    }

    db.flush()?;

    verbosity.info(format_args!(
        "Done. {} games imported, {} skipped. Database now has {} games (was {}).",
        added,
        skipped,
        db.num_games(),
        initial_count
    ));

    Ok(())
}

fn info_command(args: &InfoArgs) -> Result<()> {
    // Read no game records, so damaged games don't matter.
    let db = Database::open(&args.database)
        .with_context(|| format!("Failed to open database '{}'", args.database))?;

    let mut deleted = 0u32;
    let mut years: Option<(u16, u16)> = None;
    for entry in db.index().entries() {
        if entry.get_delete_flag() {
            deleted += 1;
            continue;
        }
        // A year of 0 is unknown.
        let year = entry.get_year();
        if year != 0 {
            years = Some(years.map_or((year, year), |(first, last)| {
                (first.min(year), last.max(year))
            }));
        }
    }

    let total = db.num_games();
    let names = db.nfile().namebase();
    println!("Total games: {total}");
    println!("Deleted games: {deleted}");
    println!("Active games: {}", total - deleted);
    println!("Player names: {}", names.count(NameType::Player));
    println!("Event names: {}", names.count(NameType::Event));
    println!("Site names: {}", names.count(NameType::Site));
    println!("Round names: {}", names.count(NameType::Round));
    match years {
        Some((first, last)) => println!("Years of active games: {first}-{last}"),
        None => println!("Years of active games: unknown"),
    }
    Ok(())
}
