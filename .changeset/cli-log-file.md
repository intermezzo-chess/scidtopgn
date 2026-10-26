---
default: minor
---

# Write the CLI's warnings, errors, and progress to a log file

- `--log <path>` on export, import, and `info` appends the messages stderr gets without `--quiet` to a file: warnings and errors, and with `--verbose` progress and summaries too. Each message is one line starting with `error: `, `warning: `, or `info: `, and each run starts with a line giving the version and the arguments.
- `--quiet` and `--verbose` can now be combined, but only together with `--log`, so that a quiet run can still record progress.
- The log is opened before any other file. A log that can't be opened, or that is a file the command reads or writes, such as the `-o` output or the database, is an error, and nothing is written. If writing the log fails part way, the command finishes but exits with status 1.
