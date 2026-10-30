use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

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
/// Help for `--log`, shared by every command.
const LOG_HELP: &str = "Also append warnings and errors to this file, each line starting \
                        with its level. With --verbose, progress and summaries too. \
                        Combine with --quiet to write them only to the file";

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

    #[arg(short, long, help = QUIET_HELP)]
    pub quiet: bool,

    // --quiet and --verbose together only make sense with a log.
    #[arg(
        long,
        value_name = "PATH",
        value_parser = log_path,
        required_if_eq_all = [("quiet", "true"), ("verbose", "true")],
        help = LOG_HELP
    )]
    pub log: Option<PathBuf>,
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
                log: self.log,
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

    // The command with the options moved after the subcommand and its
    // arguments, but before any `--`, if clap accepts it.
    let mut fixed = None;
    if let (Some(start), true) = (start, problems.is_empty()) {
        let tail = &args[start..];
        let end = tail
            .iter()
            .position(|arg| arg == "--")
            .unwrap_or(tail.len());
        let candidate: Vec<OsString> = args[..1]
            .iter()
            .chain(&tail[..end])
            .chain(&args[1..start])
            .chain(&tail[end..])
            .cloned()
            .collect();
        match Cli::command()
            .try_get_matches_from(&candidate)
            .and_then(|matches| Cli::from_arg_matches(&matches))
        {
            Ok(_) => fixed = Some(candidate),
            Err(err) => {
                let err = err.to_string();
                // The first paragraph, which can list arguments on the lines
                // after the first.
                let reason = err
                    .lines()
                    .take_while(|line| !line.is_empty())
                    .map(str::trim)
                    .collect::<Vec<_>>()
                    .join(" ");
                let reason = reason.strip_prefix("error: ").unwrap_or(&reason);
                problems.push(format!("with them after '{name}': {reason}"));
            }
        }
    }

    let mut message = format!("options must come after the subcommand '{name}'");
    if let Some(start) = start {
        let before = describe_words(&args[1..start]);
        message.push_str(&format!(", but these come before it: {before}"));
    }
    message.push_str("\n\nOptions before the subcommand would be ignored, so nothing was run.");
    if !problems.is_empty() {
        message.push_str(" They can't just be moved after it:\n");
        for problem in &problems {
            message.push_str(&format!("\n  - {problem}"));
        }
        message.push_str(&format!(
            "\n\nRun '{program} {name} --help' to see the options '{name}' takes."
        ));
    } else if let Some(command) = fixed
        .as_deref()
        .and_then(|fixed| literal_words(&fixed[1..]))
    {
        message.push_str(&format!(
            " Move them after the subcommand:\n\n    {program} {command}"
        ));
    } else {
        // Some argument can't be shown so that every shell reads it back
        // unchanged, so describe the change instead.
        message.push_str(&format!(" Move them after '{name}' and its arguments."));
    }
    Err(subcommand.error(ErrorKind::ArgumentConflict, message))
}

/// Whether `arg` reads back unchanged, unquoted, in sh, bash, zsh, fish,
/// PowerShell, and cmd.exe. Windows paths may also have backslashes and
/// short-name tildes.
fn is_literal(arg: &str) -> bool {
    let mut chars = arg.chars();
    // zsh expands a leading `=`, and PowerShell a leading `~`.
    let Some(first) = chars.next() else {
        return false;
    };
    let windows = |c: char| cfg!(windows) && c == '\\';
    let rest = |c: char| {
        c.is_ascii_alphanumeric()
            || "-_./:=".contains(c)
            // PowerShell reads a comma as an array.
            || (!cfg!(windows) && c == ',')
            || windows(c)
            || (cfg!(windows) && c == '~')
    };
    (first.is_ascii_alphanumeric() || "-_./".contains(first) || windows(first)) && chars.all(rest)
}

/// Joins `args` with spaces if each reads back unchanged in every common
/// shell, so the result can be run as shown.
fn literal_words(args: &[OsString]) -> Option<String> {
    let words: Option<Vec<&str>> = args
        .iter()
        .map(|arg| arg.to_str().filter(|arg| is_literal(arg)))
        .collect();
    words.map(|words| words.join(" "))
}

/// Joins `args` with spaces for a message, quoting any that aren't literal.
/// The quoting is for reading, not for a shell.
pub fn describe_words(args: &[OsString]) -> String {
    args.iter()
        .map(|arg| {
            let arg = arg.to_string_lossy();
            if is_literal(&arg) {
                arg.into_owned()
            } else {
                format!("\"{arg}\"")
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
    /// Check that every game in a SCID database can be read, and report
    /// each one that can't
    Validate(ValidateArgs),
    /// Copy the games of SCID databases, deleted ones too, into a new
    /// database without converting them
    Merge(MergeArgs),
}

impl Command {
    pub fn verbosity(&self) -> Verbosity {
        match self {
            Command::Export(args) => Verbosity::new(args.verbose),
            Command::Import(args) => Verbosity::new(args.verbose),
            Command::Merge(args) => Verbosity::new(args.verbose),
            Command::Info(_) | Command::Validate(_) => Verbosity::Normal,
        }
    }

    /// Whether to write nothing to stderr.
    pub fn quiet(&self) -> bool {
        match self {
            Command::Export(args) => args.quiet,
            Command::Import(args) => args.quiet,
            Command::Info(args) => args.quiet,
            Command::Validate(args) => args.quiet,
            Command::Merge(args) => args.quiet,
        }
    }

    /// The log to write messages to, if any.
    pub fn log(&self) -> Option<&Path> {
        match self {
            Command::Export(args) => args.log.as_deref(),
            Command::Import(args) => args.log.as_deref(),
            Command::Info(args) => args.log.as_deref(),
            Command::Validate(args) => args.log.as_deref(),
            Command::Merge(args) => args.log.as_deref(),
        }
    }

    /// The exit status for a run that fails. `validate` exits with 2, so
    /// that 1 means only that it found damaged games.
    pub fn failure(&self) -> ExitCode {
        match self {
            Command::Validate(_) => ExitCode::from(2),
            _ => ExitCode::FAILURE,
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

    #[arg(short, long, help = QUIET_HELP)]
    pub quiet: bool,

    // --quiet and --verbose together only make sense with a log.
    #[arg(
        long,
        value_name = "PATH",
        value_parser = log_path,
        required_if_eq_all = [("quiet", "true"), ("verbose", "true")],
        help = LOG_HELP
    )]
    pub log: Option<PathBuf>,
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

    #[arg(short, long, help = QUIET_HELP)]
    pub quiet: bool,

    // --quiet and --verbose together only make sense with a log.
    #[arg(
        long,
        value_name = "PATH",
        value_parser = log_path,
        required_if_eq_all = [("quiet", "true"), ("verbose", "true")],
        help = LOG_HELP
    )]
    pub log: Option<PathBuf>,
}

#[derive(Args)]
pub struct InfoArgs {
    /// Database path (without extension)
    pub database: String,

    #[arg(short, long, help = QUIET_HELP)]
    pub quiet: bool,

    #[arg(long, value_name = "PATH", value_parser = log_path, help = LOG_HELP)]
    pub log: Option<PathBuf>,
}

#[derive(Args)]
pub struct ValidateArgs {
    /// Database path (without extension)
    pub database: String,

    #[arg(short, long, help = QUIET_HELP)]
    pub quiet: bool,

    #[arg(long, value_name = "PATH", value_parser = log_path, help = LOG_HELP)]
    pub log: Option<PathBuf>,
}

#[derive(Args)]
pub struct MergeArgs {
    /// Source database paths (without extension), in the order to copy
    /// their games
    #[arg(required = true)]
    pub databases: Vec<String>,

    /// Output database path (without extension)
    #[arg(short, long)]
    pub output: String,

    /// Replace the output database if it exists
    #[arg(long)]
    pub overwrite: bool,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,

    #[arg(short, long, help = QUIET_HELP)]
    pub quiet: bool,

    // --quiet and --verbose together only make sense with a log.
    #[arg(
        long,
        value_name = "PATH",
        value_parser = log_path,
        required_if_eq_all = [("quiet", "true"), ("verbose", "true")],
        help = LOG_HELP
    )]
    pub log: Option<PathBuf>,
}

/// Parses `--log`, which must name a file. `-` would suggest stdout or
/// stderr.
fn log_path(value: &str) -> Result<PathBuf, String> {
    if value == "-" {
        Err("the log must be a file path; '-' isn't stdout or stderr here".to_owned())
    } else {
        Ok(PathBuf::from(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os_args(args: &[&str]) -> Vec<OsString> {
        std::iter::once("scidtopgn")
            .chain(args.iter().copied())
            .map(OsString::from)
            .collect()
    }

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_args_from(os_args(args))
    }

    /// The message for `args`, which must be rejected for misplaced options.
    fn misplaced(args: &[&str]) -> String {
        let err = parse(args).err().expect("should be rejected");
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict, "{err}");
        err.to_string()
    }

    /// The command the message for `args` suggests, if any.
    fn suggestion(args: &[&str]) -> Option<String> {
        let message = misplaced(args);
        let (_, rest) = message.split_once("Move them after the subcommand:\n\n    ")?;
        Some(rest.lines().next().unwrap().to_owned())
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
            &["export", "-q", "--", "mydb"],
            &["import", "in.pgn", "-o", "db", "--quiet"],
            &["info", "mydb", "-q"],
            &["mydb", "-q", "-v", "--log", "run.log"],
            &["export", "mydb", "-qv", "--log=run.log"],
            &[
                "import", "in.pgn", "-o", "db", "-q", "-v", "--log", "run.log",
            ],
            &["info", "mydb", "--log", "run.log"],
            &["validate", "mydb", "-q", "--log", "run.log"],
            &["merge", "db1", "db2", "-o", "merged", "--overwrite"],
            &["merge", "db1", "-o", "merged", "-qv", "--log", "run.log"],
        ] {
            assert!(parse(args).is_ok(), "{args:?}");
        }
    }

    #[test]
    fn combines_quiet_and_verbose_only_with_a_log() {
        for args in [
            &["mydb", "-q", "-v"][..],
            &["export", "mydb", "-qv"],
            &["import", "in.pgn", "-o", "db", "--quiet", "--verbose"],
            &["merge", "db1", "-o", "merged", "-qv"],
        ] {
            let err = parse(args).err().expect("should be rejected");
            assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument, "{err}");
            assert!(err.to_string().contains("--log <PATH>"), "{err}");
        }
    }

    #[test]
    fn merge_requires_a_source_and_an_output() {
        for args in [&["merge", "-o", "merged"][..], &["merge", "db1"]] {
            let err = parse(args).err().expect("should be rejected");
            assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument, "{err}");
        }
    }

    #[test]
    fn rejects_a_log_of_stdout() {
        for args in [&["mydb", "--log", "-"][..], &["info", "mydb", "--log=-"]] {
            let err = parse(args).err().expect("should be rejected");
            assert_eq!(err.kind(), ErrorKind::ValueValidation, "{err}");
        }
    }

    #[test]
    fn suggests_moving_options_after_the_subcommand() {
        let mut cases = vec![
            (
                &["--games", "6", "export", "mydb"][..],
                "export mydb --games 6",
            ),
            (
                &["-v", "export", "mydb", "-o", "out.pgn"],
                "export mydb -o out.pgn -v",
            ),
            (
                &["--quiet", "import", "in.pgn", "-o", "db"],
                "import in.pgn -o db --quiet",
            ),
            (&["-q", "info", "mydb"], "info mydb -q"),
            (
                &["-v", "merge", "db1", "db2", "-o", "merged"],
                "merge db1 db2 -o merged -v",
            ),
            (
                &["--log", "run.log", "validate", "mydb"],
                "validate mydb --log run.log",
            ),
            // The option's value has the subcommand's name.
            (&["-o", "export", "export", "mydb"], "export mydb -o export"),
            (
                &["--games=1-2", "-qo", "out", "export", "mydb"],
                "export mydb --games=1-2 -qo out",
            ),
            // Options go before the end of options.
            (&["-q", "export", "--", "mydb"], "export -q -- mydb"),
        ];
        if cfg!(windows) {
            cases.push((
                &["-o", r"C:\Users\RUNNER~1\out.pgn", "export", "mydb"],
                r"export mydb -o C:\Users\RUNNER~1\out.pgn",
            ));
        } else {
            cases.push((
                &["--games", "1-5,8", "export", "mydb"],
                "export mydb --games 1-5,8",
            ));
        }

        for (args, fixed) in cases {
            let command = suggestion(args).unwrap_or_else(|| panic!("{args:?}: no command"));
            assert_eq!(command, format!("scidtopgn {fixed}"), "{args:?}");

            // Each word reads back unchanged, so the command runs as shown,
            // with the same arguments in a different order.
            let words: Vec<&str> = command.split(' ').skip(1).collect();
            assert!(parse(&words).is_ok(), "{args:?}: {command}");
            let mut original = args.to_vec();
            let mut moved = words.clone();
            original.sort_unstable();
            moved.sort_unstable();
            assert_eq!(moved, original, "{args:?}");
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
            message.contains("would be ignored, so nothing was run"),
            "{message}"
        );
        assert!(
            message.contains("Usage: scidtopgn export [OPTIONS] <DATABASE>"),
            "{message}"
        );
    }

    #[test]
    fn shows_no_command_when_a_shell_could_change_an_argument() {
        let mut values = vec![
            "my file.pgn",
            "it's.pgn",
            "=ls",
            "$HOME.pgn",
            r"D:\Chess\$(Get-Date).pgn",
            "%TEMP%.pgn",
            "~/out.pgn",
            "@out.pgn",
            "out*.pgn",
            "",
        ];
        if cfg!(windows) {
            values.push("1-5,8");
        } else {
            values.push(r"C:\out.pgn");
        }
        for value in values {
            let args = ["-o", value, "export", "mydb"];
            assert_eq!(suggestion(&args), None, "{value:?}");
            let message = misplaced(&args);
            assert!(
                message.contains(&format!("before it: -o \"{value}\"\n")),
                "{message}"
            );
            assert!(
                message.contains("Move them after 'export' and its arguments."),
                "{message}"
            );
        }
    }

    #[test]
    fn explains_options_that_cannot_be_moved() {
        for (args, problems) in [
            (
                &["--count", "-v", "info", "mydb"][..],
                &["'info' has no --count option", "'info' has no --verbose option"][..],
            ),
            (
                &["-v", "validate", "mydb"],
                &["'validate' has no --verbose option"],
            ),
            (
                &["-o", "x", "import", "in.pgn", "-o", "db"],
                &["--output is also given after 'import'"],
            ),
            (
                &["--quiet", "export", "mydb", "-v"],
                &["with them after 'export': the following required arguments were not provided: --log <PATH>"],
            ),
            (
                &["--games", "1", "export", "mydb", "--count"],
                &["with them after 'export': the argument '--count' cannot be used with '--games <LIST>'"],
            ),
        ] {
            let message = misplaced(args);
            for problem in problems {
                assert!(message.contains(&format!("\n  - {problem}\n")), "{message}");
            }
            let name = args.iter().find(|arg| ["export", "import", "info", "validate", "merge"].contains(arg)).unwrap();
            assert!(message.contains(&format!("Run 'scidtopgn {name} --help'")), "{message}");
            assert_eq!(suggestion(args), None, "{message}");
        }
    }
}
