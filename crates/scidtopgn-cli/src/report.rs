use std::fmt;

/// How much a command writes to stderr.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verbosity {
    /// Nothing, not even errors.
    Quiet,
    /// Warnings and errors.
    Normal,
    /// Warnings, errors, progress, and summaries.
    Verbose,
}

impl Verbosity {
    /// Chooses from `--quiet` and `--verbose`, which clap keeps apart.
    pub fn new(quiet: bool, verbose: bool) -> Self {
        match (quiet, verbose) {
            (true, _) => Verbosity::Quiet,
            (false, true) => Verbosity::Verbose,
            (false, false) => Verbosity::Normal,
        }
    }

    /// Writes a warning unless quiet.
    pub fn warn(self, message: fmt::Arguments) {
        if self != Verbosity::Quiet {
            eprintln!("Warning: {message}");
        }
    }

    /// Writes a message when verbose.
    pub fn info(self, message: fmt::Arguments) {
        if self == Verbosity::Verbose {
            eprintln!("{message}");
        }
    }
}

/// Reports, when verbose, each time another tenth of the games is done.
pub struct Progress {
    verbosity: Verbosity,
    total: u64,
    done: u64,
    tenths: u64,
}

impl Progress {
    pub fn new(verbosity: Verbosity, total: u64) -> Self {
        Self {
            verbosity,
            total,
            done: 0,
            tenths: 0,
        }
    }

    /// Counts one more game done. When several tenths are reached at once,
    /// as with fewer than 10 games, only the last is reported.
    pub fn advance(&mut self) {
        if self.done == self.total {
            return;
        }
        self.done += 1;
        let tenths = self.done * 10 / self.total;
        if tenths > self.tenths {
            self.tenths = tenths;
            self.verbosity.info(format_args!(
                "Processed {}% ({}/{} games)",
                tenths * 10,
                self.done,
                self.total
            ));
        }
    }
}
