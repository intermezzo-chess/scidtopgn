use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "scidtopgn", version)]
#[command(about = "Convert between SCID databases and PGN format", long_about = None)]
pub struct Cli {
    /// Database path (without extension) - shorthand for 'export'
    pub database: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,

    #[arg(short, long)]
    pub output: Option<String>,

    #[arg(long)]
    pub count: bool,

    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Export SCID database to PGN format
    Export(ExportArgs),
    /// Import PGN file to SCID database
    Import(ImportArgs),
}

#[derive(Args)]
pub struct ExportArgs {
    /// Database path (without extension)
    pub database: String,

    /// Output file (default: stdout)
    #[arg(short, long)]
    pub output: Option<String>,

    /// Show game count only
    #[arg(long)]
    pub count: bool,

    /// Overwrite output file if it exists
    #[arg(long)]
    pub overwrite: bool,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
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
}
