use std::cell::Cell;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Which messages a command reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verbosity {
    /// Warnings and errors.
    Normal,
    /// Warnings, errors, progress, and summaries.
    Verbose,
}

impl Verbosity {
    /// Chooses from `--verbose`.
    pub fn new(verbose: bool) -> Self {
        if verbose {
            Verbosity::Verbose
        } else {
            Verbosity::Normal
        }
    }
}

/// Writes a command's messages to stderr, unless quiet, and to its log, if
/// it has one. The verbosity chooses the messages for both.
pub struct Reporter {
    verbosity: Verbosity,
    quiet: bool,
    log: Option<Log>,
}

impl Reporter {
    pub fn new(verbosity: Verbosity, quiet: bool, log: Option<Log>) -> Self {
        Self {
            verbosity,
            quiet,
            log,
        }
    }

    /// Writes the line that starts a run, given its arguments, to the log
    /// only, so that runs appended to one log can be told apart.
    pub fn start(&self, args: &str) {
        self.log_line(
            "info",
            format_args!("Running scidtopgn {}: {args}", env!("CARGO_PKG_VERSION")),
        );
    }

    /// Writes an error with its causes.
    pub fn error(&self, err: &anyhow::Error) {
        if !self.quiet {
            // The same report as returning the error from `main`.
            eprintln!("Error: {err:?}");
        }
        self.log_line("error", format_args!("{err:#}"));
    }

    /// Writes a warning.
    pub fn warn(&self, message: fmt::Arguments) {
        if !self.quiet {
            eprintln!("Warning: {message}");
        }
        self.log_line("warning", message);
    }

    /// Writes a message when verbose.
    pub fn info(&self, message: fmt::Arguments) {
        if self.verbosity == Verbosity::Verbose {
            if !self.quiet {
                eprintln!("{message}");
            }
            self.log_line("info", message);
        }
    }

    /// Whether writing to the log failed, so that it is missing messages.
    pub fn log_failed(&self) -> bool {
        self.log.as_ref().is_some_and(|log| log.failed.get())
    }

    /// Writes `message` to the log as one line starting with `level`. After
    /// a write fails, reports it once and writes nothing more.
    fn log_line(&self, level: &str, message: fmt::Arguments) {
        let Some(log) = &self.log else {
            return;
        };
        if log.failed.get() {
            return;
        }
        // One write per line, so lines from runs appending at once don't mix.
        let message = message.to_string().replace(['\r', '\n'], " ");
        let line = format!("{level}: {message}\n");
        if let Err(err) = (&log.file).write_all(line.as_bytes()) {
            log.failed.set(true);
            if !self.quiet {
                eprintln!("Error: Failed to write log '{}': {err}", log.path.display());
            }
        }
    }
}

/// A log file, appended to as messages are reported.
pub struct Log {
    path: PathBuf,
    file: File,
    /// Whether a write failed, after which nothing more is written.
    failed: Cell<bool>,
}

impl Log {
    /// Opens `path` for appending, creating it if missing. It is an error if
    /// `path` is one of `files`, which the command reads or writes, and then
    /// nothing is created.
    pub fn open(path: &Path, files: &[PathBuf]) -> Result<Self> {
        if let Some(file) = files.iter().find(|file| same_file(path, file)) {
            anyhow::bail!(
                "The log '{}' is the same file as '{}', which the command reads or writes",
                path.display(),
                file.display()
            );
        }
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .with_context(|| format!("Failed to open log '{}'", path.display()))?;
        Ok(Self {
            path: path.to_owned(),
            file,
            failed: Cell::new(false),
        })
    }
}

/// Whether `a` and `b` name the same file, or would once it is created, such
/// as `out.pgn` and `./out.pgn`.
fn same_file(a: &Path, b: &Path) -> bool {
    match (a.exists(), b.exists()) {
        (true, true) => same_existing_file(a, b),
        (false, false) => new_file_path(a).is_some_and(|a| new_file_path(b) == Some(a)),
        // One exists and the other doesn't, so they differ.
        _ => false,
    }
}

#[cfg(unix)]
fn same_existing_file(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (a.metadata(), b.metadata()) {
        (Ok(a), Ok(b)) => a.dev() == b.dev() && a.ino() == b.ino(),
        _ => false,
    }
}

#[cfg(not(unix))]
fn same_existing_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// The absolute path of a file that doesn't exist yet, or `None` if its
/// directory doesn't exist either.
fn new_file_path(path: &Path) -> Option<PathBuf> {
    let name = path.file_name()?;
    let dir = match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    };
    Some(dir.canonicalize().ok()?.join(name))
}

/// Reports, when verbose, each time another tenth of the games is done.
pub struct Progress<'a> {
    reporter: &'a Reporter,
    total: u64,
    done: u64,
    tenths: u64,
}

impl<'a> Progress<'a> {
    pub fn new(reporter: &'a Reporter, total: u64) -> Self {
        Self {
            reporter,
            total,
            done: 0,
            tenths: 0,
        }
    }

    /// Counts one more game done. Each tenth is reported once, so with fewer
    /// than 10 games one game can report several.
    pub fn advance(&mut self) {
        if self.done == self.total {
            return;
        }
        self.done += 1;
        let tenths = self.done * 10 / self.total;
        while self.tenths < tenths {
            self.tenths += 1;
            self.reporter.info(format_args!(
                "Processed {}% ({}/{} games)",
                self.tenths * 10,
                self.done,
                self.total
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::TempDir;

    #[test]
    fn writes_each_message_on_one_line_with_its_level() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("run.log");
        let log = Log::open(&path, &[]).unwrap();
        let reporter = Reporter::new(Verbosity::Verbose, true, Some(log));
        reporter.start("export mydb");
        reporter.warn(format_args!("two\nlines"));
        reporter.info(format_args!("done"));
        let err = anyhow::anyhow!("cause").context("failed");
        reporter.error(&err);
        assert!(!reporter.log_failed());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            format!(
                "info: Running scidtopgn {}: export mydb\n\
                 warning: two lines\n\
                 info: done\n\
                 error: failed: cause\n",
                env!("CARGO_PKG_VERSION")
            )
        );
    }

    #[test]
    fn writes_info_only_when_verbose() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("run.log");
        let reporter = Reporter::new(
            Verbosity::Normal,
            true,
            Some(Log::open(&path, &[]).unwrap()),
        );
        reporter.info(format_args!("done"));
        reporter.warn(format_args!("odd"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "warning: odd\n");
    }

    #[test]
    fn stops_writing_after_a_failed_write() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("run.log");
        std::fs::write(&path, "kept\n").unwrap();
        // A file opened only for reading can't be written.
        let log = Log {
            path: path.clone(),
            file: File::open(&path).unwrap(),
            failed: Cell::new(false),
        };
        let reporter = Reporter::new(Verbosity::Normal, true, Some(log));
        reporter.warn(format_args!("lost"));
        assert!(reporter.log_failed());
        reporter.warn(format_args!("also lost"));
        assert!(reporter.log_failed());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "kept\n");
    }

    #[test]
    fn refuses_a_log_that_is_a_file_of_the_command() {
        let temp = TempDir::new().unwrap();
        let existing = temp.path().join("out.pgn");
        std::fs::write(&existing, "keep").unwrap();
        let new = temp.path().join("new.pgn");
        let dir = temp.path().join("dir");
        std::fs::create_dir(&dir).unwrap();

        for (log, file) in [
            (existing.clone(), existing.clone()),
            (temp.path().join("dir/../out.pgn"), existing.clone()),
            (new.clone(), new.clone()),
            (temp.path().join("dir/../new.pgn"), new.clone()),
        ] {
            let err = Log::open(&log, &[dir.join("other"), file.clone()])
                .err()
                .unwrap_or_else(|| panic!("{log:?} should be refused"));
            assert!(err.to_string().contains("is the same file as"), "{err}");
        }
        assert_eq!(std::fs::read_to_string(&existing).unwrap(), "keep");
        assert!(!new.exists());

        assert!(Log::open(&new, &[existing.clone(), dir.join("new.pgn")]).is_ok());
        assert!(new.exists());
    }
}
