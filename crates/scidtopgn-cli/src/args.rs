use std::ffi::OsString;

use clap::error::ErrorKind;
use clap::parser::ValueSource;
use clap::{ArgMatches, Args, CommandFactory, FromArgMatches, Parser, Subcommand};

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
    /// Parses the command line like [`Parser::parse`], and also rejects
    /// options given before a subcommand, which would otherwise be ignored.
    /// Exits with a usage error on failure.
    pub fn parse_args() -> Self {
        Self::try_parse_args_from(std::env::args_os().collect()).unwrap_or_else(|err| err.exit())
    }

    /// Parses `args`, starting with the program name, as
    /// [`parse_args`](Self::parse_args) does.
    pub fn try_parse_args_from(args: Vec<OsString>) -> Result<Self, clap::Error> {
        let matches = Self::command().try_get_matches_from(&args)?;
        check_option_placement(&args, &matches)?;
        Self::from_arg_matches(&matches)
    }

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

/// Rejects options given before the subcommand, as in
/// `scidtopgn --games 1-5 export mydb`. clap reads them as the top-level
/// options of the default export, so the subcommand would run without them.
fn check_option_placement(args: &[OsString], matches: &ArgMatches) -> Result<(), clap::Error> {
    let Some((name, sub_matches)) = matches.subcommand() else {
        return Ok(());
    };
    // A database before the subcommand is reported when the command runs.
    if matches.contains_id("database") {
        return Ok(());
    }
    let given_here = |m: &ArgMatches, id: &str| {
        m.try_contains_id(id).unwrap_or(false)
            && m.value_source(id) == Some(ValueSource::CommandLine)
    };
    let mut cli = Cli::command();
    cli.build();
    let program = cli.get_name().to_owned();
    // The options given before the subcommand.
    let misplaced: Vec<clap::Arg> = cli
        .get_arguments()
        .filter(|arg| !arg.is_positional() && given_here(matches, arg.get_id().as_str()))
        .cloned()
        .collect();
    if misplaced.is_empty() {
        return Ok(());
    }

    let subcommand = cli
        .find_subcommand_mut(name)
        .expect("clap matched this subcommand");

    // The subcommand is the first argument with its name that isn't the
    // value of an option before it.
    let start = (1..args.len())
        .find(|&i| args[i] == name && Cli::command().try_get_matches_from(&args[..i]).is_ok());

    let mut problems = Vec::new();
    for arg in &misplaced {
        let id = arg.get_id().as_str();
        let option = format!("--{}", arg.get_long().unwrap_or(id));
        if !subcommand.get_arguments().any(|arg| arg.get_id() == id) {
            problems.push(format!("'{name}' has no {option} option"));
        } else if given_here(sub_matches, id) {
            problems.push(format!("{option} is also given after '{name}'"));
        }
    }

    let mut message = format!("options must come after the subcommand '{name}'");
    if let Some(start) = start {
        let before = shell_words(&args[1..start]);
        message.push_str(&format!(", but these come before it: {before}"));
    }
    message.push_str("\n\nOptions before the subcommand would be ignored, so nothing was run.");
    match start {
        Some(start) if problems.is_empty() => {
            let moved: Vec<OsString> = args[start..]
                .iter()
                .chain(&args[1..start])
                .cloned()
                .collect();
            message.push_str(&format!(
                " Move them to the end of the command:\n\n    {program} {}",
                shell_words(&moved)
            ));
        }
        _ => {
            if !problems.is_empty() {
                message.push_str(" They can't just be moved to the end:\n");
                for problem in &problems {
                    message.push_str(&format!("\n  - {problem}"));
                }
            }
            message.push_str(&format!(
                "\n\nRun '{program} {name} --help' to see the options '{name}' takes."
            ));
        }
    }
    Err(subcommand.error(ErrorKind::ArgumentConflict, message))
}

/// Joins `args` with spaces, quoting any that the shell would split or
/// change. On Windows, paths keep their backslashes, and quotes are double,
/// as both cmd.exe and PowerShell read them.
fn shell_words(args: &[OsString]) -> String {
    let plain = |c: char| {
        c.is_ascii_alphanumeric()
            || "-_./:=,+@%".contains(c)
            || (cfg!(windows) && "\\~".contains(c))
    };
    args.iter()
        .map(|arg| {
            let arg = arg.to_string_lossy();
            if !arg.is_empty() && arg.chars().all(plain) {
                arg.into_owned()
            } else if cfg!(windows) {
                format!("\"{arg}\"")
            } else {
                format!("'{}'", arg.replace('\'', "'\\''"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_args_from(
            std::iter::once("scidtopgn")
                .chain(args.iter().copied())
                .map(OsString::from)
                .collect(),
        )
    }

    /// The message for `args`, which must be rejected for misplaced options.
    fn misplaced(args: &[&str]) -> String {
        let err = parse(args).err().expect("should be rejected");
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict, "{err}");
        err.to_string()
    }

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }

    #[test]
    fn accepts_options_after_the_subcommand_or_without_one() {
        for args in [
            &["mydb", "--games", "1-5", "-v"][..],
            &["-v", "--games", "1-5", "mydb"],
            &["export", "mydb", "--games", "1-5", "-q"],
            &["import", "in.pgn", "-o", "db", "--quiet"],
            &["info", "mydb", "-q"],
        ] {
            assert!(parse(args).is_ok(), "{args:?}");
        }
    }

    #[test]
    fn suggests_moving_options_after_the_subcommand() {
        for (args, fixed) in [
            (
                &["--games", "6", "export", "mydb"][..],
                "scidtopgn export mydb --games 6",
            ),
            (
                &["-v", "export", "mydb", "-o", "out.pgn"],
                "scidtopgn export mydb -o out.pgn -v",
            ),
            (
                &["--quiet", "import", "in.pgn", "-o", "db"],
                "scidtopgn import in.pgn -o db --quiet",
            ),
            (&["-q", "info", "mydb"], "scidtopgn info mydb -q"),
            // The option's value has the subcommand's name.
            (
                &["-o", "export", "export", "mydb"],
                "scidtopgn export mydb -o export",
            ),
            (
                &["--games=1-2", "-qo", "out", "export", "mydb"],
                "scidtopgn export mydb --games=1-2 -qo out",
            ),
            (
                &["-o", "my file's.pgn", "export", "mydb"],
                if cfg!(windows) {
                    r#"scidtopgn export mydb -o "my file's.pgn""#
                } else {
                    r"scidtopgn export mydb -o 'my file'\''s.pgn'"
                },
            ),
            (
                &["-o", r"C:\Users\RUNNER~1\out.pgn", "export", "mydb"],
                if cfg!(windows) {
                    r"scidtopgn export mydb -o C:\Users\RUNNER~1\out.pgn"
                } else {
                    r"scidtopgn export mydb -o 'C:\Users\RUNNER~1\out.pgn'"
                },
            ),
        ] {
            let message = misplaced(args);
            assert!(
                message.contains(&format!("\n    {fixed}\n")),
                "{args:?}: {message}"
            );
            assert!(
                message.contains("would be ignored, so nothing was run"),
                "{message}"
            );
        }

        let message = misplaced(&["--games", "6", "-v", "export", "mydb"]);
        assert!(
            message.starts_with(
                "error: options must come after the subcommand 'export', but these come \
                 before it: --games 6 -v\n"
            ),
            "{message}"
        );
        assert!(
            message.contains("Usage: scidtopgn export [OPTIONS] <DATABASE>"),
            "{message}"
        );
    }

    #[test]
    fn explains_options_that_cannot_be_moved() {
        let message = misplaced(&["--count", "-v", "info", "mydb"]);
        assert!(
            message.contains("\n  - 'info' has no --count option\n"),
            "{message}"
        );
        assert!(
            message.contains("\n  - 'info' has no --verbose option\n"),
            "{message}"
        );
        assert!(message.contains("Run 'scidtopgn info --help'"), "{message}");
        assert!(!message.contains("Move them"), "{message}");

        let message = misplaced(&["-o", "x", "import", "in.pgn", "-o", "db"]);
        assert!(
            message.contains("\n  - --output is also given after 'import'\n"),
            "{message}"
        );
    }
}
