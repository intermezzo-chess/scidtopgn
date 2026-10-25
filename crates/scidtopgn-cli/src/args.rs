use clap::{Args, Parser, Subcommand};

use crate::report::Verbosity;
use crate::selection::GameSelection;

/// Help for `--count`, shared by the default invocation and `export`.
const COUNT_HELP: &str = "Show the number of games stored and exit. This includes logically \
                          deleted games, so it can be more than the number exported";
/// Help for `--games`, shared by the default invocation and `export`.
const GAMES_HELP: &str = "Export only these games, numbered from 1, such as 10-20 or 1-5,8. \
                          Logically deleted games are still skipped";
/// Help for `--quiet`, shared by every command.
const QUIET_HELP: &str = "Write nothing to stderr, not even errors. The exit status still \
                          shows whether the command failed";

#[derive(Parser)]
#[command(name = "scidtopgn", version)]
#[command(about = "Convert between SCID databases and PGN format", long_about = None)]
pub struct Cli {
    /// Database path (without extension) - shorthand for 'export'
    pub database: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,

    /// Output file (default: stdout)
    #[arg(short, long)]
    pub output: Option<String>,

    #[arg(long, help = COUNT_HELP)]
    pub count: bool,

    #[arg(long, value_name = "LIST", conflicts_with = "count", help = GAMES_HELP)]
    pub games: Option<GameSelection>,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,

    #[arg(short, long, conflicts_with = "verbose", help = QUIET_HELP)]
    pub quiet: bool,
}

impl Cli {
    /// Returns the command to run, treating a database without a
    /// subcommand as `export`, or `None` if the arguments name neither or
    /// both.
    pub fn into_command(self) -> Option<Command> {
        match (self.database, self.command) {
            (Some(database), None) => Some(Command::Export(ExportArgs {
                database,
                output: self.output,
                count: self.count,
                games: self.games,
                overwrite: false,
                verbose: self.verbose,
                quiet: self.quiet,
            })),
            (None, Some(command)) => Some(command),
            _ => None,
        }
    }
}

#[derive(Subcommand)]
pub enum Command {
    /// Export SCID database to PGN format
    Export(ExportArgs),
    /// Import PGN file to SCID database
    Import(ImportArgs),
    /// Show the number of games and names in a SCID database
    Info(InfoArgs),
}

impl Command {
    pub fn verbosity(&self) -> Verbosity {
        match self {
            Command::Export(args) => Verbosity::new(args.quiet, args.verbose),
            Command::Import(args) => Verbosity::new(args.quiet, args.verbose),
            Command::Info(args) => Verbosity::new(args.quiet, false),
        }
    }
}

#[derive(Args)]
pub struct ExportArgs {
    /// Database path (without extension)
    pub database: String,

    /// Output file (default: stdout)
    #[arg(short, long)]
    pub output: Option<String>,

    #[arg(long, help = COUNT_HELP)]
    pub count: bool,

    #[arg(long, value_name = "LIST", conflicts_with = "count", help = GAMES_HELP)]
    pub games: Option<GameSelection>,

    /// Overwrite output file if it exists
    #[arg(long)]
    pub overwrite: bool,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,

    #[arg(short, long, conflicts_with = "verbose", help = QUIET_HELP)]
    pub quiet: bool,
}

#[derive(Args)]
pub struct ImportArgs {
    /// PGN file to import (use - for stdin)
    pub pgn_file: String,

    /// Output database path (without extension)
    #[arg(short, long)]
    pub output: String,

    /// Overwrite existing database (default: append)
    #[arg(long)]
    pub overwrite: bool,

    /// Error handling: skip (default) or abort
    #[arg(long, value_parser = ["skip", "abort"], default_value = "skip")]
    pub on_error: String,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,

    #[arg(short, long, conflicts_with = "verbose", help = QUIET_HELP)]
    pub quiet: bool,
}

#[derive(Args)]
pub struct InfoArgs {
    /// Database path (without extension)
    pub database: String,

    #[arg(short, long, help = QUIET_HELP)]
    pub quiet: bool,
}
