mod args;
mod report;
mod selection;

use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};

use args::{Cli, Command, ExportArgs, ImportArgs, InfoArgs, MergeArgs, ValidateArgs};
use report::{Log, Progress, Reporter};
use scidtopgn::{Database, Error, Index, NFile, NameBase, NameType, PgnParser};

fn main() -> ExitCode {
    let cli = Cli::parse_args();
    let quiet = cli.quiet;
    let Some(command) = cli.into_command() else {
        if !quiet {
            eprintln!(
                "Error: Specify a database or use a subcommand (export/import/info/validate/merge)"
            );
        }
        return ExitCode::FAILURE;
    };

    // Open the log before any other file, so that a log that can't be
    // opened leaves them untouched.
    let log = match command.log() {
        Some(path) => match Log::open(path, &command_files(&command)) {
            Ok(log) => Some(log),
            Err(err) => {
                if !command.quiet() {
                    eprintln!("Error: {err:?}");
                }
                return command.failure();
            }
        },
        None => None,
    };
    let reporter = Reporter::new(command.verbosity(), command.quiet(), log);
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    reporter.start(&args::describe_words(&args));

    let result = match &command {
        Command::Export(args) => export_command(args, &reporter).map(|()| ExitCode::SUCCESS),
        Command::Import(args) => import_command(args, &reporter).map(|()| ExitCode::SUCCESS),
        Command::Info(args) => info_command(args, &reporter).map(|()| ExitCode::SUCCESS),
        Command::Validate(args) => validate_command(args, &reporter),
        Command::Merge(args) => merge_command(args, &reporter).map(|()| ExitCode::SUCCESS),
    };
    match result {
        // A log missing messages fails the run, so it doesn't pass unnoticed.
        Ok(_) if reporter.log_failed() => command.failure(),
        Ok(status) => status,
        Err(err) => {
            reporter.error(&err);
            command.failure()
        }
    }
}

/// The files `command` reads or writes, which can't also be its log.
fn command_files(command: &Command) -> Vec<PathBuf> {
    let database = |path: &str| DATABASE_EXTENSIONS.map(|ext| Path::new(path).with_extension(ext));
    let mut files = Vec::new();
    match command {
        Command::Export(args) => {
            files.extend(database(&args.database));
            files.extend(args.output.as_ref().map(PathBuf::from));
        }
        Command::Import(args) => {
            if args.pgn_file != "-" {
                files.push(PathBuf::from(&args.pgn_file));
            }
            files.extend(database(&args.output));
        }
        Command::Info(args) => files.extend(database(&args.database)),
        Command::Validate(args) => files.extend(database(&args.database)),
        Command::Merge(args) => {
            for source in &args.databases {
                files.extend(database(source));
            }
            files.extend(database(&args.output));
            files.push(Path::new(&args.output).with_extension("stc"));
        }
    }
    files
}

fn export_command(args: &ExportArgs, reporter: &Reporter) -> Result<()> {
    if let Some(ref output) = args.output {
        let path = std::path::Path::new(output);
        if path.exists() && !args.overwrite {
            anyhow::bail!(
                "Output file '{}' exists. Use --overwrite to replace.",
                output
            );
        }
    }

    let mut db = Database::open_read_only(&args.database)
        .with_context(|| format!("Failed to open database '{}'", args.database))?;
    warn_converted_names(reporter, db.nfile().namebase());

    if args.count {
        writeln!(std::io::stdout(), "{} games", db.num_games())?;
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

    let mut progress = Progress::new(reporter, selected.into());
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
    writer.flush()?;

    reporter.info(format_args!("Done. {} games exported.", exported));

    Ok(())
}

fn import_command(args: &ImportArgs, reporter: &Reporter) -> Result<()> {
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
    // parse error leaves it unchanged. Each game keeps its number and first
    // line, to name it if it can't be added.
    let mut games = Vec::new();
    let mut skipped = 0u32;
    let mut parser = PgnParser::from_bytes(&pgn);
    while let Some(result) = parser.next() {
        match result {
            Ok(game) => {
                // Warnings don't stop the game from being imported.
                for warning in &game.warnings {
                    reporter.warn(format_args!("{warning}"));
                }
                games.push((parser.game_number(), parser.game_line(), game));
            }
            Err(e) if abort_on_error => return Err(e.into()),
            Err(e) => {
                reporter.warn(format_args!("skipping {:#}", anyhow::Error::new(e)));
                skipped += 1;
            }
        }
    }

    reporter.info(format_args!("Parsed {} games from PGN", games.len()));

    let db_path = Path::new(&args.output);
    let si4_exists = db_path.with_extension("si4").exists();

    // A new database is built beside its path and moved into place once
    // every game is added, so a failed import leaves no database, or the
    // one it would have replaced.
    let (mut db, staging) = if si4_exists && !args.overwrite {
        reporter.info(format_args!(
            "Appending to existing database: {}",
            args.output
        ));
        let db = Database::open(&args.output)
            .with_context(|| format!("Failed to open database '{}'", args.output))?;
        warn_converted_names(reporter, db.nfile().namebase());
        (db, None)
    } else {
        if si4_exists {
            reporter.info(format_args!(
                "Overwriting existing database: {}",
                args.output
            ));
        }
        let create = || -> Result<_> {
            let staging = Staging::new(db_path)?;
            let db = Database::create(staging.database())?;
            Ok((db, Some(staging)))
        };
        create().with_context(|| format!("Failed to create database '{}'", args.output))?
    };

    let initial_count = db.num_games();
    let mut added = 0u32;

    let mut progress = Progress::new(reporter, games.len() as u64);
    for (number, line, game) in &games {
        if let Err(e) = db.add_game(game) {
            let err = anyhow::Error::new(e)
                .context(format!("PGN game {number}, starting at line {line}"));
            if abort_on_error {
                // Leave an existing database as it was. A new one is
                // removed with `staging`.
                if let Err(discard_err) = db.discard() {
                    reporter.error(
                        &anyhow::Error::new(discard_err)
                            .context(format!("Failed to restore database '{}'", args.output)),
                    );
                }
                return Err(err);
            }
            reporter.warn(format_args!("skipping {err:#}"));
            skipped += 1;
        } else {
            added += 1;
        }
        progress.advance();
    }

    let total = db.num_games();
    db.close()?;
    if let Some(staging) = staging {
        staging.finish(db_path)?;
    }

    reporter.info(format_args!(
        "Done. {} games imported, {} skipped. Database now has {} games (was {}).",
        added, skipped, total, initial_count
    ));

    Ok(())
}

/// The extensions of a database's index, name, and game files.
const DATABASE_EXTENSIONS: [&str; 3] = ["si4", "sn4", "sg4"];

/// Copies every game of the source databases, in order, to a new database,
/// which replaces the output database only once it is complete.
fn merge_command(args: &MergeArgs, reporter: &Reporter) -> Result<()> {
    let target = Path::new(&args.output);
    let target_files = DATABASE_EXTENSIONS.map(|ext| target.with_extension(ext));
    for source in &args.databases {
        let is_source = DATABASE_EXTENSIONS
            .iter()
            .zip(&target_files)
            .any(|(ext, file)| same_file(file, &Path::new(source).with_extension(ext)));
        if is_source {
            anyhow::bail!(
                "The output database '{}' is also a source, '{}'",
                args.output,
                source
            );
        }
    }
    if target_files
        .iter()
        .any(|file| file.symlink_metadata().is_ok())
    {
        if !args.overwrite {
            anyhow::bail!(
                "Output database '{}' exists. Use --overwrite to replace.",
                args.output
            );
        }
        reporter.info(format_args!(
            "Overwriting existing database: {}",
            args.output
        ));
    }

    // Declared after `staging`, so that the database is closed before its
    // directory is removed.
    let staging = Staging::new(target)?;
    let mut db = Database::create(staging.database())
        .with_context(|| format!("Failed to create database '{}'", args.output))?;
    for source_path in &args.databases {
        let mut source = Database::open_read_only(source_path)
            .with_context(|| format!("Failed to open database '{source_path}'"))?;
        warn_converted_names(reporter, source.nfile().namebase());

        let count = source.num_games();
        reporter.info(format_args!("Copying {count} games from {source_path}"));
        let mut progress = Progress::new(reporter, count.into());
        for i in 0..count {
            db.copy_game(&mut source, i)
                .map_err(|err| match err {
                    // Name the game below, with its database.
                    Error::Game { source, .. } => anyhow::Error::new(*source),
                    other => anyhow::Error::new(other),
                })
                .with_context(|| {
                    format!("Failed to copy SCID game {} of '{source_path}'", i + 1)
                })?;
            progress.advance();
        }
    }

    let total = db.num_games();
    db.close()
        .with_context(|| format!("Failed to write database '{}'", args.output))?;
    // SCID's tree cache would be out of date, as `scmerge` removes it.
    let stc = target.with_extension("stc");
    match std::fs::remove_file(&stc) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            return Err(e).with_context(|| format!("Failed to remove '{}'", stc.display()));
        }
        _ => {}
    }
    staging.finish(target)?;

    reporter.notice(format_args!(
        "Done. {} games merged from {} databases into {}.",
        total,
        args.databases.len(),
        args.output
    ));
    Ok(())
}

/// Whether `a` and `b` name the same file. A file that can't be read is the
/// same as none.
#[cfg(unix)]
fn same_file(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (a.metadata(), b.metadata()) {
        (Ok(a), Ok(b)) => a.dev() == b.dev() && a.ino() == b.ino(),
        _ => false,
    }
}

/// Whether `a` and `b` name the same file. A file that can't be read is the
/// same as none.
#[cfg(not(unix))]
fn same_file(a: &Path, b: &Path) -> bool {
    same_file::is_same_file(a, b).unwrap_or(false)
}

/// A directory beside a database being created, which holds its files until
/// [`finish`](Self::finish) moves them into place. Dropping it removes it
/// and anything still in it.
struct Staging {
    dir: tempfile::TempDir,
}

impl Staging {
    /// Creates the directory beside `database`, a path without an extension,
    /// so that its files can be renamed onto that path.
    fn new(database: &Path) -> Result<Self> {
        let parent = match database.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            _ => Path::new("."),
        };
        let dir = tempfile::Builder::new()
            .prefix(".scidtopgn-new-")
            .tempdir_in(parent)
            .with_context(|| {
                format!(
                    "Failed to create a temporary directory in '{}'",
                    parent.display()
                )
            })?;
        Ok(Self { dir })
    }

    /// The path, without an extension, to create the database at.
    fn database(&self) -> PathBuf {
        self.dir.path().join("database")
    }

    /// Moves the database's files onto `database`, replacing any there. The
    /// index goes last, as `Database::flush` writes it last.
    ///
    /// The files already at `database` are moved into the directory first.
    /// If a file can't be moved, they are put back and the new files placed
    /// so far are removed, so that `database` is as it was. If that fails
    /// too, the directory is kept, so that no file of the old database is
    /// lost.
    fn finish(self, database: &Path) -> Result<()> {
        let old = |ext: &str| self.dir.path().join("old").with_extension(ext);
        let mut moved_aside = Vec::new();
        let mut placed = Vec::new();
        let mut result = Ok(());
        for ext in ["sg4", "sn4", "si4"] {
            let to = database.with_extension(ext);
            if to.symlink_metadata().is_ok() {
                result = std::fs::rename(&to, old(ext));
                if result.is_err() {
                    break;
                }
                moved_aside.push(ext);
            }
        }
        if result.is_ok() {
            for ext in ["sg4", "sn4", "si4"] {
                result = std::fs::rename(
                    self.database().with_extension(ext),
                    database.with_extension(ext),
                );
                if result.is_err() {
                    break;
                }
                placed.push(ext);
            }
        }
        let Err(err) = result else {
            return Ok(());
        };

        let mut restored = true;
        for ext in placed {
            if !moved_aside.contains(&ext) {
                restored &= std::fs::remove_file(database.with_extension(ext)).is_ok();
            }
        }
        for ext in moved_aside {
            restored &= std::fs::rename(old(ext), database.with_extension(ext)).is_ok();
        }
        let mut err = anyhow::Error::new(err)
            .context(format!("Failed to write database '{}'", database.display()));
        if !restored {
            let kept = self.dir.keep();
            err = err.context(format!(
                "The database couldn't be put back; its files are in '{}'",
                kept.display()
            ));
        }
        Err(err)
    }
}

/// Warns about each name that was read from bytes that weren't UTF-8.
fn warn_converted_names(reporter: &Reporter, names: &NameBase) {
    for (name_type, name) in names.converted_names() {
        reporter.warn(format_args!(
            "read {name_type} name \"{name}\" that isn't UTF-8 as Windows-1252"
        ));
    }
}

fn info_command(args: &InfoArgs, reporter: &Reporter) -> Result<()> {
    // Read only the index and name files, so the game file can be damaged,
    // missing, or read-only.
    let path = std::path::Path::new(&args.database);
    let open = || -> scidtopgn::Result<(Index, NFile)> {
        Ok((Index::open(path.with_extension("si4"))?, NFile::open(path)?))
    };
    let (index, nfile) =
        open().with_context(|| format!("Failed to open database '{}'", args.database))?;
    warn_converted_names(reporter, nfile.namebase());

    let mut deleted = 0u32;
    let mut years: Option<(u16, u16)> = None;
    for entry in index.entries() {
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

    let total = index.num_games();
    let names = nfile.namebase();
    let mut out = std::io::stdout().lock();
    writeln!(out, "Total games: {total}")?;
    writeln!(out, "Deleted games: {deleted}")?;
    writeln!(out, "Active games: {}", total - deleted)?;
    writeln!(out, "Player names: {}", names.count(NameType::Player))?;
    writeln!(out, "Event names: {}", names.count(NameType::Event))?;
    writeln!(out, "Site names: {}", names.count(NameType::Site))?;
    writeln!(out, "Round names: {}", names.count(NameType::Round))?;
    match years {
        Some((first, last)) => writeln!(out, "Years of active games: {first}-{last}")?,
        None => writeln!(out, "Years of active games: unknown")?,
    }
    out.flush()?;
    Ok(())
}

/// Reads every game, including logically deleted games, and warns about
/// each one that can't be read. Exits with 1 if any can't.
fn validate_command(args: &ValidateArgs, reporter: &Reporter) -> Result<ExitCode> {
    let mut db = Database::open_read_only(&args.database)
        .with_context(|| format!("Failed to open database '{}'", args.database))?;
    warn_converted_names(reporter, db.nfile().namebase());

    let total = db.num_games();
    let mut damaged = 0u32;
    for i in 0..total {
        let Err(err) = db.get_game(i) else {
            continue;
        };
        damaged += 1;
        // Name the game here, so the line can say whether it is deleted.
        let cause = match err {
            Error::Game { source, .. } => anyhow::Error::new(*source),
            other => anyhow::Error::new(other),
        };
        let deleted = if db.get_entry(i).is_some_and(|e| e.get_delete_flag()) {
            " (deleted)"
        } else {
            ""
        };
        reporter.warn(format_args!("SCID game {}{deleted}: {cause:#}", i + 1));
    }

    reporter.notice(format_args!("{total} games checked, {damaged} damaged"));
    Ok(if damaged == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes files at `path` that name `contents` and their extension.
    fn write_database(path: &Path, contents: &str) {
        for ext in DATABASE_EXTENSIONS {
            std::fs::write(path.with_extension(ext), format!("{contents} {ext}")).unwrap();
        }
    }

    fn read_database(path: &Path) -> Vec<Option<String>> {
        DATABASE_EXTENSIONS
            .iter()
            .map(|ext| std::fs::read_to_string(path.with_extension(ext)).ok())
            .collect()
    }

    /// A staged database whose index is missing, so that moving it into
    /// place fails after the other files are moved.
    fn incomplete_staging(target: &Path) -> Staging {
        let staging = Staging::new(target).unwrap();
        write_database(&staging.database(), "new");
        std::fs::remove_file(staging.database().with_extension("si4")).unwrap();
        staging
    }

    fn file_names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn staging_leaves_the_target_as_it_was_if_a_file_cant_be_moved() {
        let temp = tempfile::tempdir().unwrap();
        let existing = temp.path().join("db");
        write_database(&existing, "old");
        let missing = temp.path().join("missing");

        assert!(incomplete_staging(&existing).finish(&existing).is_err());
        assert!(incomplete_staging(&missing).finish(&missing).is_err());

        let old = ["old si4", "old sn4", "old sg4"].map(|s| Some(s.to_owned()));
        assert_eq!(read_database(&existing), old);
        assert_eq!(read_database(&missing), [None, None, None]);
        assert_eq!(file_names(temp.path()), ["db.sg4", "db.si4", "db.sn4"]);

        let staging = Staging::new(&existing).unwrap();
        write_database(&staging.database(), "new");
        staging.finish(&existing).unwrap();
        let new = ["new si4", "new sn4", "new sg4"].map(|s| Some(s.to_owned()));
        assert_eq!(read_database(&existing), new);
        assert_eq!(file_names(temp.path()), ["db.sg4", "db.si4", "db.sn4"]);
    }
}
