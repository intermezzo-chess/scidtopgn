mod args;

use std::fs::File;
use std::io::{BufWriter, Read, Write};

use anyhow::{Context, Result};
use clap::Parser;

use args::{Cli, Command, ExportArgs, ImportArgs};
use scidtopgn::{Database, PgnParser};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match (&cli.database, &cli.command) {
        (Some(db), None) => {
            let args = ExportArgs {
                database: db.clone(),
                output: cli.output,
                count: cli.count,
                overwrite: false,
                verbose: cli.verbose,
            };
            export_command(&args)
        }
        (None, Some(Command::Export(args))) => export_command(args),
        (None, Some(Command::Import(args))) => import_command(args),
        _ => anyhow::bail!("Specify a database or use a subcommand (export/import)"),
    }
}

fn export_command(args: &ExportArgs) -> Result<()> {
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

    let mut writer: Box<dyn Write> = match &args.output {
        Some(path) => {
            let file = File::create(path)?;
            Box::new(BufWriter::new(file))
        }
        None => Box::new(std::io::stdout()),
    };

    let total = db.num_games();
    let mut exported = 0u32;
    for i in 0..total {
        // Skip logically deleted games, matching the reference converter.
        if db.get_entry(i).is_none_or(|e| e.get_delete_flag()) {
            continue;
        }

        let game = db.get_game(i)?;
        let pgn = game.to_pgn();
        writeln!(writer, "{}", pgn)?;
        exported += 1;

        if args.verbose && (i + 1) % 100 == 0 {
            eprintln!("Processed {}/{} games", i + 1, total);
        }
    }

    if args.verbose {
        eprintln!("Done. {} games exported.", exported);
    }

    Ok(())
}

fn import_command(args: &ImportArgs) -> Result<()> {
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
                    eprintln!("Warning: {warning}");
                }
                games.push(game);
            }
            Err(e) if abort_on_error => return Err(e.into()),
            Err(e) => {
                eprintln!("Warning: skipping {:#}", anyhow::Error::new(e));
                skipped += 1;
            }
        }
    }

    if args.verbose {
        eprintln!("Parsed {} games from PGN", games.len());
    }

    let db_path = std::path::Path::new(&args.output);
    let si4_exists = db_path.with_extension("si4").exists();

    let mut db = if si4_exists && !args.overwrite {
        if args.verbose {
            eprintln!("Appending to existing database: {}", args.output);
        }
        Database::open(&args.output)
            .with_context(|| format!("Failed to open database '{}'", args.output))?
    } else {
        if args.verbose && si4_exists {
            eprintln!("Overwriting existing database: {}", args.output);
        }
        Database::create(&args.output)
            .with_context(|| format!("Failed to create database '{}'", args.output))?
    };

    let initial_count = db.num_games();
    let mut added = 0u32;

    for game in &games {
        match db.add_game(game) {
            Ok(_) => added += 1,
            Err(e) if abort_on_error => {
                anyhow::bail!("Failed to add game: {}", e);
            }
            Err(e) => {
                if args.verbose {
                    eprintln!("Warning: Failed to add game (skipping): {}", e);
                }
                skipped += 1;
            }
        }

        if args.verbose && added.is_multiple_of(100) && added > 0 {
            eprintln!("Imported {} games...", added);
        }
    }

    db.flush()?;

    if args.verbose {
        let final_count = db.num_games();
        eprintln!(
            "Done. {} games imported, {} skipped. Database now has {} games (was {}).",
            added, skipped, final_count, initial_count
        );
    }

    Ok(())
}
