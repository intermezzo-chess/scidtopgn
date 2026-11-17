use std::cell::Cell;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

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
            self.notice(message);
        }
    }

    /// Writes a message that isn't a warning, verbose or not.
    pub fn notice(&self, message: fmt::Arguments) {
        if !self.quiet {
            eprintln!("{message}");
        }
        self.log_line("info", message);
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
    /// `path` is the same file as one of `files`, which the command reads or
    /// writes, and then nothing is written, and a file the log created is
    /// removed.
    pub fn open(path: &Path, files: &[PathBuf]) -> Result<Self> {
        let open_error = || format!("Failed to open log '{}'", path.display());
        let existed = path.exists();
        let mut options = OpenOptions::new();
        options.append(true).create(true);
        // Reading the file's identity from its handle needs read access.
        #[cfg(windows)]
        options.read(true);
        let file = options.open(path).with_context(open_error)?;

        // Compare once the log is open, so that a name that only becomes a
        // command file once created is caught too, such as a symlink to a
        // file that doesn't exist yet, or a name that differs only in case.
        let mut same = None;
        for other in files {
            if is_same_file(&file, other).with_context(open_error)? {
                same = Some(other);
                break;
            }
        }
        let Some(same) = same else {
            return Ok(Self {
                path: path.to_owned(),
                file,
                failed: Cell::new(false),
            });
        };
        let mut message = format!(
            "The log '{}' is the same file as '{}', which the command reads or writes",
            path.display(),
            same.display()
        );
        drop(file);
        if !existed {
            // Remove the empty file opening the log created. Its path with
            // symlinks resolved names the file itself, so symlinks to it,
            // at `path` or at a command file, are left as they were.
            if let Err(err) = path.canonicalize().and_then(std::fs::remove_file) {
                message.push_str(&format!(
                    ". The empty file it created couldn't be removed: {err}"
                ));
            }
        }
        anyhow::bail!(message)
    }
}

/// Whether `other` names the open `file`. Fails only if `file`'s identity
/// can't be read. On Unix, `other` isn't opened, so that a FIFO isn't read.
#[cfg(unix)]
fn is_same_file(file: &File, other: &Path) -> io::Result<bool> {
    use std::os::unix::fs::MetadataExt;
    let file = file.metadata()?;
    Ok(other
        .metadata()
        .is_ok_and(|other| other.dev() == file.dev() && other.ino() == file.ino()))
}

/// Whether `other` names the open `file`, by volume and file index. Fails
/// only if `file`'s identity can't be read.
#[cfg(not(unix))]
fn is_same_file(file: &File, other: &Path) -> io::Result<bool> {
    // std reads neither without nightly features.
    let file = same_file::Handle::from_file(file.try_clone()?)?;
    Ok(same_file::Handle::from_path(other).is_ok_and(|other| other == file))
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

/// Reports, when verbose, an import's progress through its input: each
/// tenth of its bytes read, if its size is known, and otherwise the games
/// read so far, at most once a second.
pub struct ReadProgress<'a> {
    reporter: &'a Reporter,
    games: u64,
    by: ProgressBy,
}

enum ProgressBy {
    /// The input's size, the bytes read so far, and the tenths reported.
    Size {
        size: u64,
        read: Arc<AtomicU64>,
        tenths: u64,
    },
    /// When the games were last reported, or the import started.
    Time { last: Instant },
}

impl<'a> ReadProgress<'a> {
    /// Reports progress through `size` bytes, if known, given the count of
    /// bytes read so far, such as a [`CountingReader`] keeps.
    pub fn new(reporter: &'a Reporter, size: Option<u64>, read: Arc<AtomicU64>) -> Self {
        let by = match size {
            Some(size) => ProgressBy::Size {
                size,
                read,
                tenths: 0,
            },
            None => ProgressBy::Time {
                last: Instant::now(),
            },
        };
        Self {
            reporter,
            games: 0,
            by,
        }
    }

    /// Counts one more game read.
    pub fn advance(&mut self) {
        self.advance_at(Instant::now());
    }

    fn advance_at(&mut self, now: Instant) {
        self.games += 1;
        match &mut self.by {
            ProgressBy::Size { size, read, .. } => {
                // Bytes are read ahead of the games parsed from them, so the
                // last tenth waits for `finish`.
                let tenths = read.load(Ordering::Relaxed).saturating_mul(10) / (*size).max(1);
                self.report_tenths(tenths.min(9));
            }
            ProgressBy::Time { last } => {
                if now.duration_since(*last) >= Duration::from_secs(1) {
                    *last = now;
                    self.reporter
                        .info(format_args!("Processed {} games", self.games));
                }
            }
        }
    }

    /// Reports the tenths not yet reported, once the input is read.
    pub fn finish(&mut self) {
        if matches!(self.by, ProgressBy::Size { size, .. } if size > 0) {
            self.report_tenths(10);
        }
    }

    /// Reports each tenth up to `upto` not yet reported.
    fn report_tenths(&mut self, upto: u64) {
        let ProgressBy::Size { tenths, .. } = &mut self.by else {
            return;
        };
        while *tenths < upto {
            *tenths += 1;
            self.reporter.info(format_args!(
                "Processed {}% ({} games)",
                *tenths * 10,
                self.games
            ));
        }
    }
}

/// A reader that counts the bytes read from it.
pub struct CountingReader<R> {
    inner: R,
    count: Arc<AtomicU64>,
}

impl<R> CountingReader<R> {
    /// Counts the bytes read from `inner` in `count`.
    pub fn new(inner: R, count: Arc<AtomicU64>) -> Self {
        Self { inner, count }
    }
}

impl<R: Read> Read for CountingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.count.fetch_add(n as u64, Ordering::Relaxed);
        Ok(n)
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
        let mut reporter = Reporter::new(Verbosity::Normal, true, Some(log));
        reporter.warn(format_args!("lost"));
        assert!(reporter.log_failed());
        // Any further write now succeeds, so it would change the file.
        reporter.log.as_mut().unwrap().file = OpenOptions::new().append(true).open(&path).unwrap();
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

    /// A verbose reporter that writes only to a log at `path`.
    fn logging_reporter(path: &Path) -> Reporter {
        Reporter::new(
            Verbosity::Verbose,
            true,
            Some(Log::open(path, &[]).unwrap()),
        )
    }

    #[test]
    fn reports_each_tenth_of_the_bytes_read_and_the_last_at_the_end() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("run.log");
        let reporter = logging_reporter(&path);
        let read = Arc::new(AtomicU64::new(0));
        let mut progress = ReadProgress::new(&reporter, Some(1000), Arc::clone(&read));
        for bytes in [0, 150, 150, 990, 1000, 1000] {
            read.store(bytes, Ordering::Relaxed);
            progress.advance();
        }
        progress.finish();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "info: Processed 10% (2 games)\n\
             info: Processed 20% (4 games)\n\
             info: Processed 30% (4 games)\n\
             info: Processed 40% (4 games)\n\
             info: Processed 50% (4 games)\n\
             info: Processed 60% (4 games)\n\
             info: Processed 70% (4 games)\n\
             info: Processed 80% (4 games)\n\
             info: Processed 90% (4 games)\n\
             info: Processed 100% (6 games)\n"
        );
    }

    #[test]
    fn reports_nothing_for_an_empty_file() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("run.log");
        let reporter = logging_reporter(&path);
        let mut progress = ReadProgress::new(&reporter, Some(0), Arc::default());
        progress.finish();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "");
    }

    #[test]
    fn reports_the_games_read_at_most_once_a_second_without_a_size() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("run.log");
        let reporter = logging_reporter(&path);
        let mut progress = ReadProgress::new(&reporter, None, Arc::default());
        let start = match progress.by {
            ProgressBy::Time { last } => last,
            ProgressBy::Size { .. } => unreachable!(),
        };
        for millis in [0, 999, 1000, 1500, 1999, 2000, 5000] {
            progress.advance_at(start + Duration::from_millis(millis));
        }
        progress.finish();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "info: Processed 3 games\n\
             info: Processed 6 games\n\
             info: Processed 7 games\n"
        );
    }

    #[test]
    fn counts_the_bytes_read() {
        let count = Arc::new(AtomicU64::new(0));
        let mut reader = CountingReader::new(&b"abcde"[..], Arc::clone(&count));
        let mut buf = [0; 3];
        assert_eq!(reader.read(&mut buf).unwrap(), 3);
        assert_eq!(reader.read(&mut buf).unwrap(), 2);
        assert_eq!(reader.read(&mut buf).unwrap(), 0);
        assert_eq!(count.load(Ordering::Relaxed), 5);
    }
}
