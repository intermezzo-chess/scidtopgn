# SCID compatibility

scidtopgn is file-compatible with SCID vs. PC 4. It reads the databases SCID vs. PC writes, and it writes, appends to, and changes databases so that SCID vs. PC still reads them. It isn't a replacement for the SCID application: it converts between SCID databases and PGN, and changes a database's games, but doesn't offer SCID's other features.

This document says what that compatibility covers, what scidtopgn keeps and changes when it writes a database, and where it deliberately differs from SCID vs. PC, and why. Each statement names what checks it: a test in this repository, a scenario of the [compatibility check](SCID_COMPAT.md), or neither, in which case it says the statement is untested. "SCID" below means SCID vs. PC unless it says otherwise.

## Contents

- [Supported format](#supported-format)
- [Reading](#reading)
- [Writing](#writing)
- [Deliberate differences](#deliberate-differences)
- [Limits shared with SCID](#limits-shared-with-scid)
- [Out of scope](#out-of-scope)
- [Using a database with SCID](#using-a-database-with-scid)
- [How compatibility is checked](#how-compatibility-is-checked)

## Supported format

scidtopgn reads and writes SCID vs. PC 4 databases: an index file (`.si4`), a name file (`.sn4`), and a game file (`.sg4`), with index format version 400. It is tested with SCID vs. PC 4.26, the version every fixture and the compatibility check use. Other 4.x releases of SCID vs. PC are untested.

- **Other format versions are rejected.** Opening an index file of any other version, such as 300, fails with an error naming both versions ([#107](https://github.com/intermezzo-chess/scidtopgn/issues/107)). Checked by `test_open_rejects_other_versions` and `test_unsupported_version_names_both_versions` in `crates/scidtopgn/src/tests/test_errors.rs`, and `test_cli_rejects_other_versions` in `crates/scidtopgn-cli/tests/integration.rs`.
- **Other SCID programs and formats aren't supported**, including the original SCID and its later `.si5` format, which scidtopgn can't open. If the original SCID wrote a database with index version 400, scidtopgn opens it, but this is untested.

## Reading

scidtopgn reads the databases SCID vs. PC 4.26 writes. The claims below are checked against fixtures in `tests/data`, most written by SCID vs. PC's PGN converter, `pgnscid`: `long`, `accents`, `derived`, `twins`, `search`, and `stored`. [CONTRIBUTING.md](../CONTRIBUTING.md#test-data) describes each fixture.

- **Games, index fields, and names.** Importing the PGN of `long`, `accents`, `derived`, `twins`, or `stored` gives files byte-identical to the fixture (`test_imports_match_scid_byte_for_byte` in `crates/scidtopgn/src/tests/test_interop.rs`), and the index fields SCID derives from the moves read as SCID wrote them (`test_index_derived_fields_match_scid`). scidtopgn's header search and twin deletion select the same games as SCID's on the `search` and `twins` fixtures (`crates/scidtopgn/src/tests/test_search.rs` and `test_twins.rs`).
- **Records over 64 KiB.** A game record can be up to 131,071 bytes, more than a 16-bit length holds. The second game of `long` is 72,015 bytes, and reads back as its PGN (`test_reads_a_scid_written_record_longer_than_64_kib`).
- **Names that aren't UTF-8.** SCID vs. PC stores names as UTF-8, converting the PGN it imports. Older versions of SCID, and the C++ reference converter in `scid-cpp`, store the bytes of a Latin-1 or Windows-1252 PGN file's names as they are. scidtopgn reads the bytes of such a name that aren't valid UTF-8 as Windows-1252, and warns about each name ([#49](https://github.com/intermezzo-chess/scidtopgn/issues/49)). The `latin1` and `mixed` fixtures, written by the reference converter, hold such names. Checked by `test_reading_converts_names_that_arent_utf8` in `crates/scidtopgn/src/tests/test_names_not_utf8.rs` and `test_cli_reads_names_that_arent_utf8` in `crates/scidtopgn-cli/tests/integration.rs`.

The compatibility check's append, merge, delete, and update scenarios also start by reading the databases `pgnscid` writes from each of its inputs.

## Writing

Import, append, update, delete, and merge are each a [scenario](SCID_COMPAT.md#scenarios) of the compatibility check. In each, SCID's own code must read every game and name of the database scidtopgn writes, and the scenario compares the database with one SCID wrote, as each section below says.

### Import into a new database

- **Matches SCID:** the files are byte-identical to those `pgnscid` writes from the same PGN, for every input of the compatibility check (`import` scenario), and for `long`, `accents`, `derived`, `twins`, and `stored` (`test_imports_match_scid_byte_for_byte`). `import --overwrite` replaces an existing database's files with the same result (`overwrite` scenario).
- **PGN differences:** where scidtopgn reads PGN differently from SCID, the games stored differ too, as [PGN_IMPORT.md](PGN_IMPORT.md) describes. The check leaves out the two fixtures with such games.

### Append

`scidtopgn import` onto an existing database, and `Database::add_game`.

- **Kept:** the description, base type, auto-load game, custom flag names, and each game's flags, which SCID sets on the database before the append. Each existing game's index entry stays byte-identical (`append` scenario).
- **Changed:** the game count, and the name file, which gains the new names.
- **Matches SCID:** the appended entries and the names match the database `pgnscid` writes from both PGN files (`append` scenario).

### Update

`Database::update_game`, which replaces a game and keeps its number. The CLI doesn't offer it.

- **Kept:** the game's number and flags, apart from the start flag, which follows the new game (`test_update_game_keeps_user_flags_and_sets_the_start_flag` in `crates/scidtopgn/src/tests/test_database.rs`). Every other game's index entry stays byte-identical (`test_changes_keep_scids_other_entries`). The index header keeps the base type, auto-load game, description, and custom flag names (`test_compact_games_keeps_the_index_header` in `crates/scidtopgn/src/tests/test_compact.rs`, which updates a game after compacting). The compatibility check doesn't check this header: its update scenarios start from a database without a description or custom flag names ([#128](https://github.com/intermezzo-chess/scidtopgn/issues/128)).
- **Changed:** the new record is added at the end of the game file. The old record stays there, unused, until the games are compacted. Names the old game used and the new one doesn't stay in the name file with their frequency lowered, to 0 if no game uses them, until the names are compacted, as SCID does ([#72](https://github.com/intermezzo-chess/scidtopgn/issues/72)). SCID's tree cache is deleted (see [Tree cache](#tree-cache)).
- **Matches SCID:** the games read back, the index entries, and the names match `pgnscid`'s database from the PGN with that game replaced, apart from game file offsets and unused names (`update-same-names`, `update-new-names`, `update-long`, and `update-twice` scenarios).

### Delete and undelete

`Database::delete_game` and `undelete_game`. The CLI doesn't offer them.

- **Kept:** the game's record and number, and every name and its frequency, until the games are compacted (`test_delete_and_undelete_game`). Every other game's index entry stays byte-identical (`test_changes_keep_scids_other_entries`), and the name file keeps SCID's order (`test_delete_keeps_scids_name_order`).
- **Changed:** only the game's delete flag.
- **Matches SCID:** deleting the first, middle, and last games gives files byte-identical to those SCID writes when it deletes the same games, and undeleting them gives back the original files (`delete` and `undelete` scenarios). These start from a database without a description or custom flag names, so whether a delete keeps those fields is untested ([#128](https://github.com/intermezzo-chess/scidtopgn/issues/128)).

### Merge

`scidtopgn merge`, and `Database::copy_game`.

- **Kept:** each game's record, index entry, and flags, copied as stored, including logically deleted games (`test_copy_game_copies_the_record_and_entry_as_stored` and `test_copy_game_keeps_custom_and_user_flags`). The sources don't change.
- **Changed:** names are rewritten as UTF-8, with frequencies that count the new database's games. The new database has no description and no custom flag names, as with SCID's `scmerge` (`test_cli_merge_gives_the_output_a_default_header`).
- **Matches SCID:** the entries, names, and games read back match `pgnscid`'s database from both PGN files (`merge` scenario).

### Names that aren't UTF-8

Any write to a database with names that aren't UTF-8 rewrites every name as UTF-8, as SCID vs. PC stores the names of PGN it imports. If a name was stored both ways, such as `Müller, Hans` once as Latin-1 and once as UTF-8, the two become one name, because SCID can't open a name file that holds the same name twice. The names then get new IDs, and every game keeps its names (`test_appending_writes_the_names_as_utf8` and `test_appending_merges_names_stored_two_ways` in `crates/scidtopgn/src/tests/test_names_not_utf8.rs`). SCID reads the result after an append, a delete, and an update (`append`, `delete`, and `update` scenarios on `latin1` and `mixed`).

### Tree cache

SCID keeps a database's opening tree statistics in a tree cache file, `.stc`, and uses it as long as the database has the same number of games. scidtopgn deletes it when a write could leave the same number of games with different moves, as SCID does when it saves a game ([#108](https://github.com/intermezzo-chess/scidtopgn/issues/108)).

| Write | Tree cache | Checked by |
| --- | --- | --- |
| Update | Deleted | `test_writing_an_updated_game_deletes_the_tree_cache`; `update-*` scenarios |
| Compacting games | Deleted | `test_compact_games_deletes_the_tree_cache` |
| `import --overwrite`, `Database::create` | Deleted | `test_cli_import_removes_the_tree_cache_only_when_it_overwrites`, `test_create_deletes_the_tree_cache` |
| `merge` replacing a database | Deleted | `test_cli_merge_replaces_an_existing_database_only_with_overwrite` |
| Append, delete, undelete, copying a game | Kept | `test_other_changes_keep_the_tree_cache`, `test_cli_import_removes_the_tree_cache_only_when_it_overwrites` |
| Deleting twins, compacting names | Kept | `test_keeps_the_tree_cache` in `crates/scidtopgn/src/tests/test_twins.rs`, `test_compact_games_deletes_the_tree_cache` |

### If a write fails

scidtopgn writes a changed database's index and name files beside it, and replaces the originals only once both are complete. If writing fails, for example because the disk is full, the database reads the same games as before. The new games' records may stay at the end of the game file as unused bytes. See [Deliberate differences](#deliberate-differences) for the limits.

## Deliberate differences

Each difference below is a decision, with its reason.

- **PGN import and export.** scidtopgn reads some PGN SCID rejects or misreads, rejects some PGN SCID stores as a corrupt game, and reports more warnings. It exports PGN in the PGN standard's export format. [PGN_IMPORT.md](PGN_IMPORT.md) lists every case, with its reason.
- **NAG export symbols** ([#45](https://github.com/intermezzo-chess/scidtopgn/issues/45)). PGN export writes a NAG as a symbol only when both SCID and scidtopgn read that symbol back as the same NAG, and as `$n` otherwise. SCID writes some NAGs as symbols it can't read back, such as `→` for `$40`, so they are lost when a game SCID exports is imported again. With scidtopgn, every NAG survives export and import (`test_every_nag_survives_export_and_import` in `crates/scidtopgn/src/game.rs`). See [How NAGs are exported](PGN_IMPORT.md#how-nags-are-exported-34).
- **Names stored as UTF-8 and merged** ([#49](https://github.com/intermezzo-chess/scidtopgn/issues/49)). scidtopgn rewrites names that aren't UTF-8 when it writes a database, as [above](#names-that-arent-utf-8). SCID keeps their bytes. scidtopgn's names are Rust strings, which must be valid UTF-8, and SCID vs. PC itself stores names as UTF-8. Once converted, a name stored both ways reads the same twice, and SCID can't open a name file that holds the same name twice, so the two are merged. PGN text that isn't UTF-8 is read as Windows-1252 too ([#42](https://github.com/intermezzo-chess/scidtopgn/issues/42)).
- **Writes that are safe against partial failure** ([#110](https://github.com/intermezzo-chess/scidtopgn/issues/110)). SCID rewrites the index and name files in place, so a failure part way leaves a database neither program can open. scidtopgn writes new files and swaps them in, as [above](#if-a-write-fails). Checked by `test_a_failed_write_of_either_file_leaves_the_database_as_it_was` in `crates/scidtopgn/src/database.rs`. This has limits that SCID's in-place writes don't:
  - The new files need disk space beside the old ones, and the database's directory must be writable.
  - A read-only or symbolically linked `.si4` or `.sn4` file is refused with an error before anything is written, because replacing it would ignore its protection or leave the file it links to out of date (`test_flush_refuses_a_read_only_name_or_index_file` and `test_flush_refuses_a_linked_name_or_index_file`).
  - Replacing the files breaks hard links to them, and keeps only their permissions, not their owner, access control lists, or extended attributes.
  - A killed process can leave a directory named `.scidtopgn-compact-*` beside the database. If it holds `old.*` files, they are the original files, and a database file can be missing: move them back to open the database as it was.
  - This doesn't protect against a power loss.
- **Search patterns count Unicode characters.** In a header search pattern, `?` matches one Unicode character, as in the Tcl 8.6.13 that SCID vs. PC 4.27 for macOS bundles. SCID built with Tcl 8.5 counts each byte of a character above U+FFFF, such as an emoji, as a character. scidtopgn follows the newer Tcl (`matches_patterns_as_tcl` in `crates/scidtopgn/src/search.rs`).
- **Variation limits** ([#4](https://github.com/intermezzo-chess/scidtopgn/issues/4)). A game with variations nested more than 100 levels deep, or with more than 255 variations of one move, is rejected, whether it is read from PGN or from a database, so that deep nesting can't overflow the stack. SCID has no limit on nesting, and loses variations after the 255th. See [General differences](PGN_IMPORT.md#general-differences).

## Limits shared with SCID

These lose information, but SCID loses it the same way.

- **Event dates** ([#75](https://github.com/intermezzo-chess/scidtopgn/issues/75)). A database stores a game's event date only in its index entry, which holds an event date within 3 years of the game's year. An event date with an unknown year, more than 3 years from the game's year, or in a game whose year is unknown isn't stored (`test_event_date_is_stored_only_in_the_index` in `crates/scidtopgn/src/tests/test_database.rs`).
- **Record size.** A game's record can be at most 131,071 bytes, the most an index entry can record. A longer game can't be stored: import skips it with a warning, or stops with `--on-error=abort` (`test_add_game_rejects_a_game_too_large_to_store` and `test_cli_import_warns_about_games_too_large_to_store`).
- **NAGs and tags.** A move keeps at most 7 NAGs, and a tag longer than 255 bytes is dropped. [General differences](PGN_IMPORT.md#general-differences) has these and other limits of PGN import.

## Out of scope

scidtopgn reads and writes SCID databases. It doesn't offer SCID's application features, such as board and material search, opening tree statistics, ECO classification, sorting, editing or correcting names, and spellchecking. Its header search and twin deletion select games as SCID does, so that games can be exported or deleted.

## Using a database with SCID

Don't write to a database with scidtopgn while SCID vs. PC has it open. SCID reads the index and names into memory when it opens a database, so it doesn't see changes made after that, and changes it writes later can overwrite them. scidtopgn doesn't detect that SCID has the database open. This is untested.

## How compatibility is checked

- **Tests.** The tests named above run on every pull request, on Linux, macOS, and Windows. Most compare scidtopgn with fixtures that SCID vs. PC 4.26 or the C++ reference converter wrote, described in [CONTRIBUTING.md](../CONTRIBUTING.md#test-data).
- **The compatibility check** ([#105](https://github.com/intermezzo-chess/scidtopgn/issues/105)). `compat/run` builds SCID vs. PC 4.26 from source and checks that it reads the databases scidtopgn writes in each scenario, including SCID-written databases scidtopgn changes. It runs on every pull request and on each release candidate. [SCID_COMPAT.md](SCID_COMPAT.md) describes its scenarios and how to run it.

Neither tests SCID vs. PC's GUI, or any other version of SCID.
