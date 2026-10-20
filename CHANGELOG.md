# Changelog

All notable changes to the `scidtopgn` library and the `scidtopgn-cli` crate are recorded here. Both crates share one version. Entries are generated from changesets by the Version workflow; see [docs/RELEASING.md](docs/RELEASING.md).

## 0.1.0

### Features

- Initial release.
- The `scidtopgn` library reads and writes SCID databases (`.si4`, `.sn4`, and `.sg4`) and parses PGN.
- The `scidtopgn` command exports a SCID database to PGN on stdout or to a file, with `--count` and `--overwrite`.
- `scidtopgn --version` prints the version.
- `scidtopgn import` imports PGN from a file or stdin into a new or existing SCID database. Invalid games are skipped with a warning, or abort the import with `--on-error=abort`.
- Export skips logically deleted games, matching the SCID reference converter.
- Locked builds (`cargo install scidtopgn-cli --locked`) support Rust 1.70 and later.
