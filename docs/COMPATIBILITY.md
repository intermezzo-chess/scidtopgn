# Compatibility with SCID

scidtopgn is file-compatible with SCID vs. PC 4. It reads the databases SCID vs. PC writes, and it writes, appends to, and changes databases so that SCID vs. PC still reads them. It isn't a replacement for the SCID application: it converts between SCID databases and PGN, and changes a database's games, but doesn't offer SCID's other features.

This document says what that compatibility covers, what scidtopgn keeps and changes when it writes a database, and where it deliberately differs from SCID vs. PC, and why. Each statement names what checks it: a test in this repository, a scenario of the [compatibility check](SCID_COMPAT.md), or neither, in which case it says the statement is untested. "SCID" below means SCID vs. PC unless it says otherwise.

[PGN import compared with SCID](#pgn-import-compared-with-scid) lists each case of PGN notation, and has its own definition of "SCID" and [list of tests](#tests). [SCID_COMPAT.md](SCID_COMPAT.md) is a different document: it describes how the compatibility check works and how to run it.

## Contents

- [Supported format](#supported-format)
- [Reading](#reading)
- [Writing](#writing)
- [Deliberate differences](#deliberate-differences)
- [Limits shared with SCID](#limits-shared-with-scid)
- [Out of scope](#out-of-scope)
- [Using a database with SCID](#using-a-database-with-scid)
- [How compatibility is checked](#how-compatibility-is-checked)
- [PGN import compared with SCID](#pgn-import-compared-with-scid)
  - [Summary of differences](#summary-of-differences)
  - [General differences](#general-differences)
  - [Warnings](#warnings)
  - [Already accepted by both](#already-accepted-by-both)
  - [Move notation](#move-notation)
  - [Castling](#castling)
  - [Annotation symbols](#annotation-symbols)
  - [Results and the start position](#results-and-the-start-position)
  - [Text before and between games](#text-before-and-between-games)
  - [Comments, tag values, results in variations, and stray brackets](#comments-tag-values-results-in-variations-and-stray-brackets)
  - [Player names and ratings](#player-names-and-ratings)
  - [Known differences outside these cases](#known-differences-outside-these-cases)
  - [Export and round trips](#export-and-round-trips)
  - [How SCID's behavior was checked](#how-scids-behavior-was-checked)
  - [Tests](#tests)

## Supported format

scidtopgn reads and writes SCID vs. PC 4 databases: an index file (`.si4`), a name file (`.sn4`), and a game file (`.sg4`), with index format version 400. It is tested with SCID vs. PC 4.26, which the compatibility check uses. The fixtures `pgnscid` wrote are [the files 4.26 writes](SCID_FORMAT_AUDIT.md#fixtures). [SCID_FORMAT.md](SCID_FORMAT.md) describes the format. Other 4.x releases of SCID vs. PC are untested.

- **Other format versions are rejected.** Opening an index file of any other version, such as 300, fails with an error naming both versions ([#107](https://github.com/intermezzo-chess/scidtopgn/issues/107)). Checked by `test_open_rejects_other_versions` and `test_unsupported_version_names_both_versions` in `crates/scidtopgn/src/tests/test_errors.rs`, and `test_cli_rejects_other_versions` in `crates/scidtopgn-cli/tests/integration.rs`.
- **Other SCID programs and formats aren't supported**, including the original SCID and its later `.si5` format, which scidtopgn can't open. If the original SCID wrote a database with index version 400, scidtopgn opens it, but this is untested.

## Reading

scidtopgn reads the databases SCID vs. PC 4.26 writes. The claims below are checked against fixtures in `tests/data`, most written by SCID vs. PC's PGN converter, `pgnscid`: `long`, `accents`, `derived`, `twins`, `search`, and `stored`. [CONTRIBUTING.md](../CONTRIBUTING.md#test-data) describes each fixture.

- **Games, index fields, and names.** Importing the PGN of `long`, `accents`, `derived`, `twins`, or `stored` gives files byte-identical to the fixture (`test_imports_match_scid_byte_for_byte` in `crates/scidtopgn/src/tests/test_interop.rs`), and the index fields SCID derives from the moves read as SCID wrote them (`test_index_derived_fields_match_scid`). scidtopgn's header search and twin deletion select the same games as SCID's on the `search` and `twins` fixtures (`crates/scidtopgn/src/tests/test_search.rs` and `test_twins.rs`).
- **Records over 64 KiB.** A game record can be up to 131,071 bytes, more than a 16-bit length holds. The second game of `long` is 72,015 bytes, and reads back as its PGN (`test_reads_a_scid_written_record_longer_than_64_kib`).
- **Names that aren't UTF-8.** SCID vs. PC stores names as UTF-8, converting the PGN it imports. Older versions of SCID, and the C++ reference converter in `scid-cpp`, store the bytes of a Latin-1 or Windows-1252 PGN file's names as they are. scidtopgn reads the bytes of such a name that aren't valid UTF-8 as Windows-1252, and warns about each name ([#49](https://github.com/intermezzo-chess/scidtopgn/issues/49)). The `latin1` and `mixed` fixtures, written by the reference converter, hold such names. Checked by `test_reading_converts_names_that_arent_utf8` in `crates/scidtopgn/src/tests/test_names_not_utf8.rs` and `test_cli_reads_names_that_arent_utf8` in `crates/scidtopgn-cli/tests/integration.rs`. A name that the conversion makes longer than 255 bytes is cut to 255, ending before a character that the cut would split, as every name longer than 255 bytes is cut when it is added; checked by `test_read_cuts_a_converted_name_to_255_bytes` in `crates/scidtopgn/src/namebase.rs`.
- **Control characters in names.** A name is read as stored, control characters included, as SCID stores names ([#142](https://github.com/intermezzo-chess/scidtopgn/issues/142)). Reading removes no character and trims nothing, names that differ only by control characters stay separate, and a write never changes a name it didn't add. The exceptions are the names that aren't UTF-8, in the bullet above. A NUL in a name is kept too, but SCID reads the name only up to it. Keeping a name doesn't mean that its export can be imported again: see [Export and round trips](#export-and-round-trips). Checked by `test_read_keeps_control_characters_in_names` in `crates/scidtopgn/src/namebase.rs`, and `test_import_keeps_control_characters_in_names_through_writes` and `test_names_differing_only_by_control_characters_stay_separate_through_writes` in `crates/scidtopgn/tests/integration.rs`.

The compatibility check's append, merge, delete, and update scenarios also start by reading the databases `pgnscid` writes from each of its inputs.

### Files SCID refuses or misreads

scidtopgn reads some files that SCID vs. PC 4.26 doesn't, and the other way around. SCID's behavior, and scidtopgn's where no test is named, was checked by the [probes of the format audit](SCID_FORMAT_AUDIT.md#results-damaged-files), not by the compatibility check.

| File | SCID | scidtopgn |
| --- | --- | --- |
| A [record that crosses a block](SG4_FORMAT.md#blocks-offsets-and-record-boundaries) | Reports the game as corrupt | Reads it (`test_gfile_read_across_blocks` in `crates/scidtopgn/src/gfile.rs`) |
| A tag with a [spare code](SG4_FORMAT.md#tags), 251 to 254 | Crashes when it decodes the tags | Drops the value (`test_decode_drops_spare_common_tags` in `crates/scidtopgn/src/game.rs`) |
| A tag value of 255 bytes | Overruns a buffer. Values up to 254 bytes are safe | Reads and writes it (`test_tag_values_up_to_255_bytes_round_trip` in `crates/scidtopgn/src/tests/test_database.rs`) |
| A record with more than 40 tags | Crashes when it decodes the tags | Keeps the first 40, without a warning. Import does the same, where `pgnscid` crashes |
| Names out of [order](SN4_FORMAT.md#order), or the same name twice | Refuses to open the database, or can't look names up | Reads them. A write puts them in order, and [merges](#names-that-arent-utf-8) the same names |
| An entry with a name ID that no name has | Refuses to open the database | Reads the name as `?` |
| An illegal move that has a destination | Plays it as written | Refuses the game (`rejects_illegal_bytes` in `crates/scidtopgn/src/mov.rs`) |

`scidtopgn validate` reports damage only in games scidtopgn can't read, so it finds only the last of these.

## Writing

Import, append, update, delete, and merge are each a [scenario](SCID_COMPAT.md#scenarios) of the compatibility check. In each, SCID's own code must read every game and name of the database scidtopgn writes, and the scenario compares the database with one SCID wrote, as each section below says.

### Import into a new database

- **Matches SCID:** the files are byte-identical to those `pgnscid` writes from the same PGN, for every input of the compatibility check (`import` scenario), and for `long`, `accents`, `derived`, `twins`, and `stored` (`test_imports_match_scid_byte_for_byte`). `import --overwrite` replaces an existing database's files with the same result (`overwrite` scenario).
- **PGN differences:** where scidtopgn reads PGN differently from SCID, the games stored differ too, as [PGN import compared with SCID](#pgn-import-compared-with-scid) describes. The check leaves out the two fixtures with such games.

### Append

`scidtopgn import` onto an existing database, and `Database::add_game`.

- **Kept:** the description, base type, auto-load game, custom flag names, and each game's flags, which SCID sets on the database before the append. Each existing game's index entry stays byte-identical (`append` scenario).
- **Changed:** the game count, and the name file, which gains the new names.
- **Matches SCID:** the appended entries and the names match the database `pgnscid` writes from both PGN files (`append` scenario).

### Update

`Database::update_game`, which replaces a game and keeps its number. The CLI doesn't offer it.

- **Kept:** the game's number, the flags the user sets, the delete flag, and the custom flags (`test_update_game_keeps_user_flags_and_sets_the_start_flag` in `crates/scidtopgn/src/tests/test_database.rs`). Every other game's index entry stays byte-identical (`test_changes_keep_scids_other_entries`). The index header keeps the base type, auto-load game, description, and custom flag names (`test_compact_games_keeps_the_index_header` in `crates/scidtopgn/src/tests/test_compact.rs`, which updates a game after compacting). SCID reads the same header fields, and the same flags of the first three games, after an update as before it (`update-*` scenarios).
- **Changed:** the start, promotion, and underpromotion flags follow the new game, with the other fields derived from its moves (`test_update_game_sets_the_derived_fields` in `crates/scidtopgn/src/tests/test_database.rs`). The new record is added at the end of the game file. The old record stays there, unused, until the games are compacted. Names the old game used and the new one doesn't stay in the name file with their frequency lowered, to 0 if no game uses them, until the names are compacted, as SCID does ([#72](https://github.com/intermezzo-chess/scidtopgn/issues/72)). SCID's tree cache is deleted (see [Tree cache](#tree-cache)).
- **Matches SCID:** the games read back, the index entries, and the names match `pgnscid`'s database from the PGN with that game replaced, apart from game file offsets and unused names (`update-same-names`, `update-new-names`, `update-long`, and `update-twice` scenarios).

### Delete and undelete

`Database::delete_game` and `undelete_game`. The CLI doesn't offer them.

- **Kept:** the game's record and number, and every name and its frequency, until the games are compacted (`test_delete_and_undelete_game`). Every other game's index entry stays byte-identical (`test_changes_keep_scids_other_entries`), and the name file keeps SCID's order (`test_delete_keeps_scids_name_order`).
- **Changed:** only the game's delete flag.
- **Matches SCID:** deleting the first, middle, and last games gives files byte-identical to those SCID writes when it deletes the same games, and undeleting them gives back the original files (`delete` and `undelete` scenarios). SCID sets a description, base type, auto-load game, custom flag names, and flags on the first three games before the delete, and reads them back unchanged apart from the deleted games' delete flags.

### Merge

`scidtopgn merge`, and `Database::copy_game`.

- **Kept:** each game's record and flags, copied as stored, including logically deleted games, and its index entry apart from its name IDs and game file offset (`test_copy_game_copies_the_record_and_entry_as_stored` and `test_copy_game_keeps_custom_and_user_flags`). The sources don't change.
- **Changed:** names are rewritten as UTF-8, with new IDs and with frequencies that count the new database's games. The new database has no description and no custom flag names, as with SCID's `scmerge` (`test_cli_merge_gives_the_output_a_default_header`).
- **Matches SCID:** the entries, names, and games read back match `pgnscid`'s database from both PGN files (`merge` scenario).

### Names that aren't UTF-8

Any write to a database with names that aren't UTF-8 rewrites every name as UTF-8, as SCID vs. PC stores the names of PGN it imports. If a name was stored both ways, such as `Müller, Hans` once as Latin-1 and once as UTF-8, the two become one name, because SCID can't open a name file that holds the same name twice. The names then get new IDs, and every game keeps its names (`test_appending_writes_the_names_as_utf8` and `test_appending_merges_names_stored_two_ways` in `crates/scidtopgn/src/tests/test_names_not_utf8.rs`). SCID reads the result after an append, a delete, and an update, and the delete and the update keep the header fields and flags SCID set (`append`, `delete`, and `update` scenarios on `latin1` and `mixed`).

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

- **PGN import and export.** scidtopgn reads some PGN SCID rejects or misreads, rejects some PGN SCID stores as a corrupt game, and reports more warnings. It exports PGN in a layout of its own, close to the PGN standard's export format, as [Export and round trips](#export-and-round-trips) describes. [PGN import compared with SCID](#pgn-import-compared-with-scid) lists every case, with its reason.
- **NAG export symbols** ([#45](https://github.com/intermezzo-chess/scidtopgn/issues/45)). PGN export writes a NAG as a symbol only when both SCID and scidtopgn read that symbol back as the same NAG, and as `$n` otherwise. SCID writes some NAGs as symbols it can't read back, such as `→` for `$40`, so they are lost when a game SCID exports is imported again. With scidtopgn, every NAG survives export and import (`test_every_nag_survives_export_and_import` in `crates/scidtopgn/src/game.rs`). See [How NAGs are exported](#how-nags-are-exported-34).
- **Names stored as UTF-8 and merged** ([#49](https://github.com/intermezzo-chess/scidtopgn/issues/49)). scidtopgn rewrites names that aren't UTF-8 when it writes a database, as [above](#names-that-arent-utf-8). SCID keeps their bytes. scidtopgn's names are Rust strings, which must be valid UTF-8, and SCID vs. PC itself stores names as UTF-8. Once converted, a name stored both ways reads the same twice, and SCID can't open a name file that holds the same name twice, so the two are merged. PGN text that isn't UTF-8 is read as Windows-1252 too ([#42](https://github.com/intermezzo-chess/scidtopgn/issues/42)).
- **Writes that are safe against partial failure** ([#110](https://github.com/intermezzo-chess/scidtopgn/issues/110)). SCID rewrites the index and name files in place, so a failure part way leaves a database neither program can open. scidtopgn writes new files and swaps them in, as [above](#if-a-write-fails). Checked by `test_a_failed_write_of_either_file_leaves_the_database_as_it_was` in `crates/scidtopgn/src/database.rs`. This has limits that SCID's in-place writes don't:
  - The new files need disk space beside the old ones, and the database's directory must be writable.
  - Writing changes to an existing database, such as an append, refuses a read-only or symbolically linked `.si4` or `.sn4` file with an error instead of replacing it, because replacing it would ignore its protection or leave the file it links to out of date (`test_flush_refuses_a_read_only_name_or_index_file` and `test_flush_refuses_a_linked_name_or_index_file`). The new games' records may already be at the end of the game file then, as unused bytes. `import --overwrite` and `merge --overwrite` don't refuse such files: they replace them.
  - Replacing the files breaks hard links to them, and doesn't keep their owner, access control lists, or extended attributes. Writing changes to an existing database, and compacting it, keep their permissions. `import --overwrite` and `merge --overwrite` don't: the new files get default permissions, which can make a private database readable by others ([#151](https://github.com/intermezzo-chess/scidtopgn/issues/151)).
  - A killed process can leave a temporary directory beside the database: `.scidtopgn-compact-*` from writing changes to a database or compacting it, or `.scidtopgn-new-*` from `import` into a new database, `import --overwrite`, or `merge`. If it holds `old.*` files, they are the original files, and a database file can be missing: move them back to open the database as it was.
  - This doesn't protect against a power loss.
- **Search patterns count Unicode characters.** In a header search pattern, `?` matches one Unicode character, as in Tcl 8.6.13. SCID built with Tcl 8.5 or an early 8.6, such as 8.6.0, counts each byte of a character above U+FFFF, such as an emoji, as a character. scidtopgn follows Tcl 8.6.13 (`matches_patterns_as_tcl` in `crates/scidtopgn/src/search.rs`).
- **Variation limits** ([#4](https://github.com/intermezzo-chess/scidtopgn/issues/4)). A game with variations nested more than 100 levels deep, or with more than 255 variations of one move, is rejected, whether it is read from PGN or from a database, so that deep nesting can't overflow the stack. SCID has no limit on nesting, and counts a move's variations modulo 256. See [General differences](#general-differences).

## Limits shared with SCID

These lose information, but SCID loses it the same way.

- **Event dates** ([#75](https://github.com/intermezzo-chess/scidtopgn/issues/75)). A database stores a game's event date only in its index entry, which holds an event date within 3 years of the game's year. An event date with an unknown year, more than 3 years from the game's year, or in a game whose year is unknown, 1, or 2 isn't stored (`test_event_date_is_stored_only_in_the_index` in `crates/scidtopgn/src/tests/test_database.rs`).
- **Years.** A year is at most 2047. Import reads a date's later year as unknown, and keeps its month and day, up to 12 and 31.
- **Record size.** A game's record can be at most 131,071 bytes, the most an index entry can record. A longer game can't be stored: import skips it with a warning, or stops with `--on-error=abort` (`test_add_game_rejects_a_game_too_large_to_store` and `test_cli_import_warns_about_games_too_large_to_store`).
- **NAGs and tags.** A move keeps at most 7 NAGs, a tag longer than 255 bytes is dropped, and a game keeps at most 40 tags besides those with fields of their own. scidtopgn drops the 41st and later ones without a warning ([#155](https://github.com/intermezzo-chess/scidtopgn/issues/155)). SCID vs. PC 4.26 holds 40 too, but crashes at the 41st. [General differences](#general-differences) has these and other limits of PGN import.

## Out of scope

scidtopgn reads and writes SCID databases. It doesn't offer SCID's application features, such as board and material search, opening tree statistics, ECO classification, sorting, editing or correcting names, and spellchecking. Its header search and twin deletion select games as SCID does, so that games can be exported or deleted.

## Using a database with SCID

Don't write to a database with scidtopgn while SCID vs. PC has it open. SCID reads the index and names into memory when it opens a database, so it doesn't see changes made after that, and changes it writes later can overwrite them. scidtopgn doesn't detect that SCID has the database open. This is untested.

## How compatibility is checked

- **Tests.** The tests named above run on every pull request, on Linux, macOS, and Windows. Most compare scidtopgn with fixtures that SCID vs. PC 4.26 or the C++ reference converter wrote, described in [CONTRIBUTING.md](../CONTRIBUTING.md#test-data).
- **The compatibility check** ([#105](https://github.com/intermezzo-chess/scidtopgn/issues/105)). `compat/run` builds SCID vs. PC 4.26 from source and checks that it reads the databases scidtopgn writes in each scenario, including SCID-written databases scidtopgn changes. It runs on every pull request and on each release candidate. [SCID_COMPAT.md](SCID_COMPAT.md) describes its scenarios and how to run it.

Neither tests SCID vs. PC's GUI, or any other version of SCID.

## PGN import compared with SCID

This section lists how `scidtopgn` reads PGN notation that isn't standard, and compares each case with SCID. It uses three PGN terms: SAN (standard algebraic notation) is how moves are written, such as `Nf3` or `exd5`. FEN is the notation for a position, used by the `FEN` tag to set a game's start position. A NAG (numeric annotation glyph) is an annotation written as `$n`, such as `$1` for a good move, or as a symbol, such as `!`. In this section, "SCID" means the SCID vs. PC sources in `scid-cpp/scid` (`pgnparse.cpp`, `Position::ReadMove` in `position.cpp`, and `game_parseNag` in `game.cpp`), together with the C++ reference converter in `scid-cpp/src/main.cpp`.

Each case is marked **Same** or **Differs** in its **Comparison** column. "Same" means `scidtopgn` reads the input and stores the game the way SCID does, apart from the [general differences](#general-differences). **It doesn't mean that the game comes back unchanged when `scidtopgn` exports it to PGN.** Export writes moves in canonical SAN and can drop parts of the game, as [Export and round trips](#export-and-round-trips) explains. Each difference also says what changes for someone moving from SCID, and why `scidtopgn` behaves that way.

The case numbers match [#37](https://github.com/intermezzo-chess/scidtopgn/issues/37). Most cases are games in [`tests/data/notation.pgn`](../tests/data/notation.pgn), where each game's `Event` tag starts with the case number. Both products' behavior was checked by running them on that file (see [How SCID's behavior was checked](#how-scids-behavior-was-checked)).

### Summary of differences

`scidtopgn` reads input that SCID reads incorrectly or rejects:

- **#8d** `a8b`, a lowercase bishop promotion without `=`, is read as `a8=B`. SCID rejects the move.
- **#9** `a8=n` promotes to a knight. SCID promotes to a queen and warns.
- **#15** `00` and `000` are castling. SCID drops them as move numbers, without a message.
- **#22** A piece move with `-+` or `-/+` attached, such as `Bb5-+`, is read with its annotation. SCID drops the move, and usually fails at the next one.
- **#22c** `e4--+` and `e4->` are read with the symbol written, $21 and $41. SCID reads `e4--+` as $19 and fails at `e4->`.
- **#23** `e4~` is read with its annotation, `$13`. SCID fails at the move.
- **#28** A game whose moves don't end with a result is imported with a warning, with the `Result` tag's result, or `*` without one. SCID skips it.
- A result inside a variation is skipped with a warning, and the whole game is imported. SCID ends the game at the result and reads the rest of the moves as text before the next game ([#60](https://github.com/intermezzo-chess/scidtopgn/issues/60)).
- `\"` and `\\` in a tag value are read as `"` and `\`, as the PGN standard specifies. SCID ends the value at its first `"`, and keeps `\\` as two backslashes (#60).
- A move right after a stray `]`, as in `]Bc5`, is read. SCID drops it (#60).
- A rating at the end of a player name is read even when SCID's tidying of the name's spacing changes its length, as in `Carlsen,Magnus (2850)`. SCID misses the rating then ([#43](https://github.com/intermezzo-chess/scidtopgn/issues/43)).

`scidtopgn` rejects or changes input where SCID guesses or produces a corrupt game:

- **A5b** A null move while in check is rejected. SCID plays it.
- **#2** A pawn capture without a rank that fits more than one capture is rejected as ambiguous. SCID plays the capture on the lowest rank.
- **#18** Castling with no rook on the board is rejected. SCID moves the king as if castling and produces a corrupt game.
- **#29** A game with a move that can't be read is skipped and reported. SCID keeps the game up to that move and adds an `{Error reading move: ...}` comment.
- **#31** Castling rights and en passant squares in a `FEN` that don't fit the position are dropped, with a warning. SCID keeps them. An en passant square where no capture is possible is dropped from the exported `FEN`, without a warning.
- **#32** A game with a `Variant` tag other than standard chess is rejected. SCID reads its moves as standard chess.
- A negative rating, such as `[WhiteElo "-5"]`, is no rating. SCID stores 4,000 (#43).

`scidtopgn` reports more:

- **#20** Playing `O-O` as `O-O-O` gives a warning. SCID doesn't warn.
- **#25** Every unknown symbol gives a warning. SCID warns about most, but not about a word that starts with `D`, such as `Diagram`.
- **#27c** A result after the moves gives no warning when the game has no `Result` tag. SCID warns.
- **#27d** An unrecognized `Result` tag, such as `1-0 (time)`, gives a warning only when the result after the moves isn't `*`. SCID reads the tag as `1-0` and warns when the results differ.
- **#33** The CLI prints every warning. The SCID converter never prints them.
- A NAG after a move's first 7 is dropped with a warning. SCID drops it without a message ([#57](https://github.com/intermezzo-chess/scidtopgn/issues/57)).

PGN export:

- **#34** Each NAG is exported as the symbol SCID writes when both SCID and `scidtopgn` read it back as the same NAG, and as `$n` otherwise. SCID writes some NAGs as symbols it can't read back, such as `→` for $40.
- **#38** Variations, NAGs, and comments are exported where SCID exports them, with the spacing of the PGN standard's export format, such as `1. e4 ! (1. d4)`. SCID writes `1.e4! ( 1.d4 )`.
- `"` and `\` in tag values are exported as `\"` and `\\`, as the PGN standard specifies. SCID imports such a game, but reads `\\` as two backslashes and ends the value at `\"` (#60).

Text before and between games ([#41](https://github.com/intermezzo-chess/scidtopgn/issues/41)):

- Text before a game, including moves with no tags, is kept as the comment before the game's first move, as in SCID. `;`, `%`, and `{...}` lines between games are skipped, where SCID keeps them in the text. A line starting with `[` that isn't a tag is reported as a failed game, where SCID logs an error and ignores it. A `[` after a result on the same line is text, where SCID reads it as a tag. See [Text before and between games](#text-before-and-between-games).

Other known differences, which these cases don't cover, are listed in [Known differences outside these cases](#known-differences-outside-these-cases).

### General differences

These apply to every case below.

- **A move that can't be read.** SCID keeps the game up to that move and adds the comment `{Error reading move: ...}`. `scidtopgn` skips the game and reports which game and move failed (#29). When this section says that SCID rejects a move, SCID still keeps the game up to that move.
- **Warnings.** SCID's parser logs warnings, but the SCID converter never prints them. `scidtopgn` prints each warning (#33). See [Warnings](#warnings).
- **Canonical SAN.** `scidtopgn` stores and exports the SAN it generates, with `O-O`, `=` before a promotion piece, `x` for a capture, and check and mate signs only when they are correct. SCID does the same.
- **Where annotations attach (#38).** As in SCID, a NAG or comment attaches to the move it follows, in the main line and in variations. An annotation symbol attaches the same way as the `$n` NAG it stands for. Both products also handle these cases the same way:
  - A NAG or comment after a variation's `)` attaches to the move the variation replaces. In `1. e4 (1. d4) $1`, `$1` is on `1. e4`.
  - A comment before the first move of the game, or right after a variation's `(`, is kept at the start of the game or variation, and exported there.
  - Several comments after one move are joined into one, with a space between them unless the first is empty or ends in whitespace. `1. e4 {a} {b}` stores `a b`.
  - A `;` comment on a line between tags is ignored.
  - **A NAG before the first move of the game or of a variation is dropped, without a warning.** In `$1 1. e4 ($2 1. d4)`, neither NAG is kept. SCID keeps such a NAG in memory, but it neither saves it to the database nor exports it, so the result is the same.
  - `$0` means no annotation, and isn't stored.
  - **A move keeps at most 7 NAGs ([#57](https://github.com/intermezzo-chess/scidtopgn/issues/57)).** SCID holds a move's NAGs in an array of 8 that ends with a 0, so it keeps the first 7 and drops the rest. `scidtopgn` does the same, and warns about each NAG it drops. NAGs from `$n`, from symbols attached to the move or written on their own, and after a variation's `)` count toward one limit, in the order they appear. In `1. e4 $1 $2 $3 $4 $5 $6 $7 $8 $9`, `1. e4` keeps NAGs 1 to 7, with a warning each for `$8` and `$9`. `$0` isn't counted or warned about. A game read from a SCID database also keeps the first 7 NAGs of each move, without a warning, as SCID does. The library's `MoveNode::nags` documents the limit.
- **A `{...}` comment between tags.** `scidtopgn` ignores it and reads one game. SCID reads the tags before it as a game without moves, logs `PGN header '[' seen inside game`, and starts another game at the next tag, so the tags are split between two games. The reference converter skips the first of them, and `pgnscid` 4.26 stores it.
- **Variation limits ([issue 4](https://github.com/intermezzo-chess/scidtopgn/issues/4)).** A game with variations nested more than 100 levels deep, or with more than 255 variations of one move, is rejected with an error, whether it is read from PGN or from a SCID database. Reading and writing a game recurse once per level of nesting, so deeper nesting could overflow the stack. SCID has no limit on nesting. It counts a move's variations in a byte, which wraps: of 256 variations it keeps none, and of 257, one.
- **Tag and name lengths ([#53](https://github.com/intermezzo-chess/scidtopgn/issues/53), [#82](https://github.com/intermezzo-chess/scidtopgn/issues/82)).** As in SCID, a tag whose name or value is longer than 255 bytes is dropped, whatever tag it is, and the rest of the game is imported. A dropped `White`, `Black`, `Event`, `Site`, or `Round` tag leaves `?`, and a dropped duplicate leaves the earlier value. SCID logs `Error reading tag`, and `scidtopgn` warns. When the game is stored, a tag name longer than 240 bytes is cut to 240, as SCID cuts it. Lengths count the bytes of the UTF-8 that is stored, after escapes are read. Two things differ from SCID:
  - Text read as Windows-1252 (#42) takes more bytes in UTF-8, so it can pass the limit where SCID, which stores the bytes unchanged, keeps it.
  - A cut that would split a character ends before that character. SCID cuts in the middle of it.
  - A player name is [tidied](#player-names-and-ratings), which adds a space after a comma that has none, so it is checked again once tidied and without its rating. A name over 255 bytes then is dropped in the same way. SCID writes the tidied name into its 256-byte buffer without checking its length (#43).

### Warnings

A warning doesn't stop a game from being imported. The CLI prints each warning to stderr, in both `--on-error` modes, as `Warning: PGN game <number>, line <line>: <message>`. A game with warnings isn't counted as skipped, and warnings don't change the exit status. In the library, a game's warnings are in `Game::warnings`, as `PgnWarning` values with the game number, the line, and a `PgnWarningKind`.

A game that fails to parse is reported by its error only, without its warnings.

| Warning | When | Case |
| --- | --- | --- |
| `skipped unknown text "zz"` | Text in the moves that isn't a move, an annotation symbol, or a result, such as `zz`, `Diagram`, `B` on its own, or `nf3`. Also text after the annotation symbols attached to a move, as in `e4!x`, and each `}` or `]` outside a comment. | #12, #25, #60 |
| `skipped result "1-0" inside a variation` | A result inside a variation, at any depth. The game goes on. | #60 |
| `"O-O" castles without the castling right` | Castling after the king or rook has moved and returned, or when the `FEN` gives no castling right. | #16 |
| `read "O-O" as O-O-O because only queenside castling is legal` | `O-O` when only queenside castling is legal. | #20 |
| `dropped castling rights "KQkq" from the FEN because they don't fit the position` | Castling rights in the `FEN` without the king or rook on its original square. | #31 |
| `dropped en passant square e3 from the FEN because no pawn could have just moved past it` | An en passant square in the `FEN` that no pawn could have just moved past. | #31 |
| `the result 0-1 after the moves replaces the Result tag 1-0` | The result after the moves differs from the `Result` tag. A tag `scidtopgn` doesn't recognize, such as `1-0 (time)`, counts as `*`, so it gives this warning only when the moves end with another result. | #27 |
| `the moves don't end with a result, so the Result tag 1-0 is used` | The moves end at the next game's tags, or at the end of the input, without a result. | #28a, #28b |
| `the moves don't end with a result, and there is no Result tag` | The same, in a game with no `Result` tag. The game's result is `*`. | #28c |
| `dropped tag "Annotator" because its value is 300 bytes, more than 255` | A tag whose value is longer than 255 bytes, or a player name longer than 255 bytes once tidied and without its rating. The tag is dropped, as in SCID. | #53, #43 |
| `dropped a tag because its name is 300 bytes, more than 255` | A tag whose name is longer than 255 bytes. The tag is dropped, as in SCID. | #53 |
| `read rating "4500" in tag "WhiteElo" as 4000, the highest rating SCID stores` | A rating above 4,000 in a rating tag, or at the end of a player name, for which the warning names the `White` or `Black` tag. 4,000 is used, as in SCID. | #43 |
| `dropped NAG $8 because a move keeps at most 7` | A NAG after the first 7 on a move. The NAG is dropped, as in SCID, which drops it without a message. | #57 |
| `read text that isn't UTF-8 as Windows-1252` | The game's text, including text before it that is kept as its first comment, has bytes that aren't valid UTF-8. Those bytes are read as Windows-1252. Given once per game, on the first line with such bytes. In the library, only `PgnParser::from_bytes` and `PgnParser::from_reader` read such input; the CLI uses `from_reader`. | #42 |

### Already accepted by both

`scidtopgn` already read these the same way as SCID before #37, except for a null move while in check (A5b).

| # | Case | Example | SCID | scidtopgn | Comparison |
| --- | --- | --- | --- | --- | --- |
| A1 | Castling with zeros or lowercase letters | `0-0`, `o-o` | `O-O` | `O-O` | Same |
| A2 | `P` before a pawn move | `Pe4` | `e4` | `e4` | Same |
| A3 | A missing or extra capture sign | `ed5`, `Nxc3` | `exd5`, `Nc3` | `exd5`, `Nc3` when only one move fits | Same |
| A4 | Pawn moves in coordinates | `e2e4` | `e4` | `e4` | Same |
| A5 | Null moves, when the side to move isn't in check | `--`, `Z0`, `null` | Null move | Null move | Same |
| A5b | A null move while in check | `1. --` from `k7/8/8/8/8/8/4r3/4K3 w - -` | Plays it | Rejects the move | **Differs** |
| A6 | Wrong check or mate signs | `Qh5+` | Corrected | Corrected | Same |
| A7 | Move numbers without dots | `1 e4 e5 2 Nf3` | Ignored | Ignored | Same |
| A8 | `++` for check | `Qh5++` | Corrected | Corrected | Same |

**A5b: a null move while in check.** *What changes:* a game that SCID imports is skipped and reported. *Why:* after a null move in check, the side that moves next could capture the king, so the position can't continue as a game of chess. shakmaty can't represent it, and [#20](https://github.com/intermezzo-chess/scidtopgn/issues/20) chose to reject it.

### Move notation

| # | Case | Example | SCID | scidtopgn | Comparison |
| --- | --- | --- | --- | --- | --- |
| 1a, 1b | Pawn capture without a rank | `ed`, `exd` | `exd5` | `exd5` | Same |
| 1c | Pawn capture without a rank, en passant | `ed` | `exd6` | `exd6` | Same |
| 2 | Pawn capture without a rank that fits two captures | `ed` with pawns on e3 and e5 and pieces on d4 and d6 | Plays `exd4`, the lowest rank, without a warning | Rejects the move as ambiguous | **Differs** |
| 3 | `:` for a capture | `e:d5`, `Q:d5` | `exd5`, `Qxd5` | `exd5`, `Qxd5` | Same |
| 4 | Hyphen between squares | `e2-e4`, `Ng1-f3` | `e4`, `Nf3` | `e4`, `Nf3` | Same |
| 5 | Piece moves in coordinates | `g1f3` | `Nf3` | `Nf3` | Same |
| 6a, 6b | King moves two squares | `Kg1`, `e1g1` | `O-O` | `O-O` | Same |
| 6c | King moves two squares without the castling right | `Kg1` after `Ke2` and `Ke1` | Rejects the move | Rejects the move | Same |
| 7a, 7b | Promotion with no piece | `a8`, `axb8` | `a8=Q`, `axb8=Q` | `a8=Q`, `axb8=Q` | Same |
| 7c | Coordinate promotion with no piece | `a7a8` | Rejects the move | Rejects the move | Same |
| 8a, 8b, 8c, 8e | Lowercase promotion piece | `a8q`, `a7a8q`, `abn`, `a7a8b` | `a8=Q`, `a8=Q`, `axb8=N`, `a8=B` | Same moves | Same |
| 8d | Lowercase bishop promotion without `=` | `a8b` | Rejects the move | `a8=B` | **Differs** |
| 9 | Lowercase piece after `=` | `a8=n` | `a8=Q`, and warns `Invalid annotation symbol: =n` | `a8=N` | **Differs** |
| 10a, 10b | Capture-promotion without a rank | `ab=Q`, `abQ` | `axb8=Q` | `axb8=Q` | Same |
| 11a, 11b | En passant marker | `exd6ep`, `exd6 e.p.` | Ignores the marker | Ignores the marker | Same |
| 12 | Lowercase piece letter | `nf3` | Skips it and logs `Unexpected symbol: nf` | Skips it with a warning | Same |
| 13 | Lowercase `b` for a bishop | `bc4` | Rejects the move | Rejects the move | Same |

**#2: ambiguous pawn capture without a rank.** *What changes:* a game that SCID imports with SCID's guess is skipped and reported. *Why:* SCID tries each rank from 1 to 8 and plays the first capture that fits, which can be the wrong move. `scidtopgn` already rejects other ambiguous moves, such as a missing capture sign that fits two moves.

**#8d: `a8b`.** *What changes:* a game that SCID truncates at `a8b` is imported in full. *Why:* SCID reads the `b` as a file, since its reader allows a second file after a pawn's first. A pawn on the last rank must promote, so `a8b` can only mean `a8=B`. SCID already reads `a7a8b` as `a8=B`.

**#9: `a8=n`.** *What changes:* the pawn promotes to the piece written instead of a queen. *Why:* SCID only reads an uppercase letter after `=`. It reads `=n` as an unknown annotation, then promotes to a queen by default. `scidtopgn` reads lowercase promotion letters everywhere else, so it reads them after `=` too.

### Castling

| # | Case | Example | SCID | scidtopgn | Comparison |
| --- | --- | --- | --- | --- | --- |
| 14 | No dashes | `OO`, `OOO` | `O-O`, `O-O-O` | `O-O`, `O-O-O` | Same |
| 15 | Zeros and no dashes | `00`, `000` | Dropped as move numbers, without a message | `O-O`, `O-O-O` | **Differs** |
| 16a, 16b | After the king or rook has moved and returned | `4. Ke2 Ke7 5. Ke1 Ke8 6. O-O` | Castles and logs `illegal castling (O-O)` | Castles with a warning | Same |
| 16c | With no castling right in the `FEN` | `O-O-O` from `r3k2r/8/8/8/8/8/8/R3K2R w - -` | Castles and logs `illegal castling` | Castles with a warning | Same |
| 16d | Without the castling right, in a variation | `6. d3 (6. O-O)` | Castles and logs `illegal castling` | Castles with a warning | Same |
| 17 | Exporting a SCID database that contains #16 | | Exports `O-O` | Exports `O-O` | Same |
| 18a, 18b | No rook on the board | `O-O` from `4k3/8/8/8/8/8/8/4K3 w - -` | Moves the king to g1 as if castling and logs `illegal castling`. The game is corrupt. | Rejects the move | **Differs** |
| 19 | Through check | `O-O` with a rook on g8 | Rejects the move | Rejects the move | Same |
| 20a | `O-O` when only queenside castling is legal | `O-O` with a knight on g1 | Plays `O-O-O`, without a warning | Plays `O-O-O`, with a warning | **Differs** |
| 20b | `O-O-O` when only kingside castling is legal | `O-O-O` with a knight on b1 | Rejects the move | Rejects the move | Same |

Castling without the castling right (#16) still needs the king and rook on their original squares, the squares between them empty, and the king not in check or passing through an attacked square. Afterward, that side has no castling rights left, as after any castle. Like SCID, `scidtopgn` reads castling in this order:

- `O-O`: kingside, then queenside (#20), then kingside without the castling right (#16).
- `O-O-O`: queenside, then queenside without the castling right.

Only castling written with `O`, `o`, or `0` can castle without the castling right. Like SCID, `Kg1` and `e1g1` castle only when castling is legal (#6c).

Before #37, `scidtopgn` couldn't export a SCID database containing such a castle (#17). It failed with `invalid move byte 0x0a (e1g1 is illegal)`.

**#15: `00` and `000`.** *What changes:* a game written with `00` keeps its castling move. With SCID, the castle disappears, so the game usually fails at the next move. If the next move happens to be legal for the other side, the game is silently wrong. *Why:* SCID reads any token made only of digits as a move number before it checks for castling. `scidtopgn` reads `0-0`, `0-0-0`, `OO`, and `OOO` as castling, so it treats `00` and `000` the same way.

**#18: castling with no rook.** *What changes:* a game that SCID imports in a corrupt state is skipped and reported. *Why:* when SCID retries castling without the castling right, it checks the squares between the king and rook, but not that the rook is there. It moves the king, and the game can't continue correctly. `scidtopgn` lets shakmaty check castling with the right restored, which requires the rook.

**#20: `O-O` played as `O-O-O`.** *What changes:* a warning names the move. *Why:* SCID guesses that the writer meant queenside castling. `scidtopgn` follows SCID, but says so, because the guess changes the move that was written.

### Annotation symbols

`scidtopgn` reads the symbols SCID reads, with the NAGs from SCID's `game_parseNag`, whether the symbol is attached to the move or written separately.

| Symbols | NAG |
| --- | --- |
| `!` `?` `!!` `??` `!?` `?!` | 1, 2, 3, 4, 5, 6 |
| `=` | 10 |
| `~` | 13 |
| `+=` `+/=` | 14 |
| `=+` `=/+` | 15 |
| `+/-` `-/+` | 16, 17 |
| `+-` `-+` | 18, 19 |
| `+--` `--+` | 20, 21 |
| `+>` `->` | 40, 41 |
| `=/&` `~=` | 44 |
| `N` on its own | 146 (novelty) |
| `D` on its own | 201 (diagram) |
| `R` on its own | 144 (various moves) |

`--` is a null move and `o-o` is castling, as in SCID, although `game_parseNag` also lists them as symbols. `--+` is NAG 21, as in SCID. Before #37, `scidtopgn` read `--+` as a null move with a check sign. A null move can have symbols attached, as in `Z0!`, but a word that only starts like one, such as `nullify`, is skipped with a warning (#25c). Several symbols can follow a move, as in `e4!!+-`, which gives NAGs 3 and 18. Check and mate signs among them are ignored. Before #37, `scidtopgn` dropped these symbols without a message, or rejected the game when the symbol started with `=` or `-`.

| # | Case | Example | SCID | scidtopgn | Comparison |
| --- | --- | --- | --- | --- | --- |
| 21a | Attached to the move | `e4! e5? Nf3!! Nc6?? Bb5!? a6?!` | $1 to $6 | $1 to $6 | Same |
| 21b, 21c | After the move | `e4 += e5 =+`, `Bb5 +> a6 ->` | $14, $15, $40, $41 | $14, $15, $40, $41 | Same |
| 21d | After a mate sign | `Qxf7#!` | $1 | $1 | Same |
| 22a | Attached by `=` or `-` to a pawn move | `e4=+`, `Nf3=` | $15, $10 | $15, $10 | Same |
| 22a | Attached by `-` to a piece move | `Bb5-+`, `Ba4-/+` | Logs `Unexpected symbol: Bb5` and drops the move. The next move then usually fails. | `Bb5` with $19, `Ba4` with $17 | **Differs** |
| 22b | Several attached | `e4!!+-` | $3, $18 | $3, $18 | Same |
| 22c | `--+` or `->` attached | `e4--+`, `Nf3->` | `e4` with $19, the symbol after the first `-`. `Nf3->` logs `Unexpected symbol: Nf3` and drops the move. | `e4` with $21, `Nf3` with $41 | **Differs** |
| 23 | `~` attached | `e4~` | Logs `Unexpected symbol: e4`, drops the move, and then fails at the next move | `e4` with $13 | **Differs** |
| 24 | Letters on their own | `N`, `D`, `R` | $146, $201, $144 | $146, $201, $144 | Same |
| 25a | Unknown symbols | `zz`, `/\`, `Diagram` | Skips them; logs `Unexpected symbol` for `zz` and `/\` | Skips them with a warning for each | **Differs** (warning) |
| 25b | A piece letter on its own | `B` | Skips it and logs `Invalid annotation symbol: B` | Skips it with a warning | Same |
| 25c | A word that starts like a null move | `nullify` | Skips it and logs `Unexpected symbol: nullify` | Skips it with a warning | Same |

**#22: a piece move with `-+` attached.** *What changes:* the move and its annotation are kept, instead of a game that breaks after the move. *Why:* SCID splits `-+` and `-/+` from a pawn move but not from a piece move, where its reader rejects the whole token. `scidtopgn` splits them the same way for every move.

**#22c: `--+` or `->` attached.** *What changes:* `e4--+` keeps the symbol written, $21, instead of $19, and `Nf3->` keeps the move and $41 instead of breaking the game. *Why:* SCID ignores a `-` inside a move unless `+` or `/` follows it, so it drops the first `-` of `--+`, and its reader rejects the `>` of `->`. `scidtopgn` treats a `-` as part of the move only between squares, as in `e2-e4`, or at its end, so every symbol reads the same attached or separate.

**#23: `e4~`.** *What changes:* the game is imported with the annotation, instead of being cut at the next move. *Why:* SCID's reader doesn't allow `~` directly after a move. `scidtopgn` reads every symbol either attached or separate.

**#25: `Diagram`.** *What changes:* a warning names the skipped word. *Why:* SCID reads a word that starts with `D` as an annotation, finds no NAG for it, and drops it without a message. `scidtopgn` warns about any text it skips.

### Results and the start position

| # | Case | Example | SCID | scidtopgn | Comparison |
| --- | --- | --- | --- | --- | --- |
| 26a to 26d | Other result notation | `1:0`, `0:1`, `1/2`, `1/2:1/2` | Results | Results | Same |
| 27a | Result after the moves differs from the `Result` tag | `[Result "1-0"]`, moves end `0-1` | Uses `0-1` and logs `Result did not match the header result` | Uses `0-1`, with a warning | Same |
| 27b | `*` after the moves with a `Result` tag | `[Result "1-0"]`, moves end `*` | Uses `*` and logs the mismatch | Uses `*`, with a warning | Same |
| 27c | Result after the moves with no `Result` tag | moves end `1-0` | Uses `1-0` and logs the mismatch | Uses `1-0`, without a warning | **Differs** (warning) |
| 27d | `*` after the moves with an unrecognized `Result` tag | `[Result "1-0 (time)"]`, moves end `*` | Reads the tag as `1-0`, uses `*`, and logs the mismatch | Uses `*`, without a warning, since it reads the tag as `*` | **Differs** (warning) |
| 28a | No result before the next game | `1. e4 e5` followed by `[Event ...]` | Logs `PGN header '[' seen inside game (result missing ?)` and skips the game | Imports the game with the `Result` tag's result, with a warning | **Differs** |
| 28b | No result at the end of the input | `1. e4 e5` at the end of the file | Logs `End of input reached in game (result missing ?)` and skips the game | Imports the game with the `Result` tag's result, with a warning | **Differs** |
| 28c | No result after the moves and no `Result` tag | `1. e4 e5` followed by `[Event ...]` | Skips the game, as in #28a | Imports the game with the result `*`, with a warning | **Differs** |
| 29 | A move that can't be read | `2. Nf6` | Keeps `1. e4 e5 {Error reading move: Nf6}` | Skips the game and reports it | **Differs** |
| 30 | `[SetUp "1"]` without `FEN` | | Standard start, without a warning | Standard start, without a warning | Same |
| 31a | `FEN` castling rights that don't fit | `4k3/8/8/8/8/8/8/4K3 w KQkq -` | Keeps `KQkq` | Drops them, with a warning | **Differs** |
| 31b | `FEN` en passant square that doesn't fit | `4k3/8/8/8/8/8/8/4K3 w - e3` | Keeps `e3` | Drops it, with a warning | **Differs** |
| 31c | `FEN` en passant square with no capture possible | `4k3/8/8/8/4P3/8/8/4K3 b - e3` | Keeps `e3` | Keeps it while reading, but exports the `FEN` without it, without a warning | **Differs** |
| 32 | `Variant` tag other than standard chess | `[Variant "Chess960"]` | Reads the moves as standard chess | Rejects the game | **Differs** |

A `FEN` tag sets the start position with or without `SetUp`, in both products.

**#27c: no `Result` tag.** *What changes:* no warning. *Why:* without a `Result` tag there is nothing to contradict, so the result after the moves is simply the game's result.

**#27d: an unrecognized `Result` tag.** *What changes:* no warning when the moves end with `*`. *Why:* SCID reads a `Result` tag by its first characters, so `1-0 (time)` is `1-0`. `scidtopgn` reads only the four PGN results, so it treats the tag as `*`, which agrees with the moves. Reading tags like SCID does is listed in [Known differences outside these cases](#known-differences-outside-these-cases).

**#28: no result after the moves.** *What changes:* games that SCID skips are imported, with a warning naming the game. The game's result is the `Result` tag's, or `*` if there is no tag. *Why:* the moves were read without error. Skipping the whole game loses them, and the `Result` tag, if present, still gives the result.

**#29: a move that can't be read.** *What changes:* SCID imports the start of the game and marks the bad move with a comment. `scidtopgn` imports none of the game, and reports the game number, its first line, and the move. With `--on-error abort`, the import stops. *Why:* [#26](https://github.com/intermezzo-chess/scidtopgn/issues/26) chose to report such games rather than store part of a game that looks complete.

**#31a, #31b: castling rights and en passant squares that don't fit.** *What changes:* the stored and exported `FEN` doesn't have them, and a warning says what was dropped. *Why:* shakmaty can't represent a position with castling rights for a rook that isn't there, or an en passant square no pawn just moved past. Keeping them could also make an illegal castle or en passant capture legal.

**#31c: en passant square with no capture possible.** *What changes:* the exported `FEN` has `-` instead of the en passant square. *Why:* `scidtopgn` writes an en passant square only when an en passant capture is legal, so the same position always has the same `FEN`. Nothing is lost: a square no capture can use doesn't affect the game. There is no warning, for that reason.

**#32: `Variant` tag.** *What changes:* games in other variants, such as Chess960, are skipped and reported instead of being read as standard chess. *Why:* reading them as standard chess gives wrong moves or fails partway through. Chess960 is [#14](https://github.com/intermezzo-chess/scidtopgn/issues/14). `Variant` values that mean standard chess are accepted: `Standard`, `Chess`, `Normal`, and Lichess's `From Position`.

### Text before and between games

As in SCID, a game starts at its first tag line: a line whose first character other than whitespace is `[`. Everything before that line, from the start of the input or from the previous game's result, is text, and the text is kept as the comment before the game's first move. These cases were added by [#41](https://github.com/intermezzo-chess/scidtopgn/issues/41).

| Case | Example | SCID | scidtopgn | Comparison |
| --- | --- | --- | --- | --- |
| Text before the first game | `Downloaded from example.org.` above `[Event ...]` | Keeps it as the comment before the first move | Keeps it as the comment before the first move | Same |
| Text between games | `Round 2 starts here.` after a game's result, on the same line or below it | Keeps it as the comment before the next game's first move | Keeps it as the comment before the next game's first move | Same |
| Moves with no tags | `1. c4 c5 *` above `[Event ...]` | Keeps them as text, as above | Keeps them as text, as above | Same |
| Text or moves after the last game | `The end.` at the end of the input | Drops it, without a message | Drops it, without a message | Same |
| Text and a comment before the first move | `Notes` above the tags, then `{Intro} 1. e4` | `Notes` and `Intro` on separate lines | `Notes Intro` | **Differs** |
| Blank lines and spaces around the text | Blank lines after the text, spaces at the end of a line | Drops blank lines before the text, keeps the spaces and one line break at the end | Drops blank lines before and after the text, and the spaces at the end of each line and before text after a result | **Differs** |
| Long text | Text of 16,000 bytes or more | Keeps lines while the text stays under 16,000 bytes, and drops the others without a message. Reads a longer line in pieces | Keeps all of it | **Differs** |
| Comments between games | `; note`, `% note`, or `{note}` on its own line | Keeps each line in the text as written, with its `;`, `%`, or braces | Skips them | **Differs** |
| A line starting with `[` that isn't a tag | `[Note: see below]` above `[Event ...]` | Logs `Error reading tag` and reads the next tags into the same game | Reports a failed game, so the next game's number is one higher | **Differs** |
| A `[` after a result or a comment on the same line | `1-0 [Round 2 notes]` | Reads it as a tag. If it isn't one, logs `Error reading tag` | Keeps it as text, like other text after a result | **Differs** |

Lines inside the text, including blank lines, are joined by line breaks. Text that is only whitespace isn't stored. Text after a game that fails to parse is skipped with that game, so it isn't attached to the next game. Before #41, `scidtopgn` read text before a game as the moves of a game, which failed and was reported as an extra game. It also imported moves with no tags as a game, so a file of games without tags now imports no games, as in SCID.

**Text and a comment before the first move.** *What changes:* the text and the comment are joined with a space instead of a line break. *Why:* `scidtopgn` joins every comment it stores in one place the same way, as [Where annotations attach](#general-differences) describes.

**Blank lines and spaces around the text.** *What changes:* the stored comment has no blank lines or spaces at its ends, and no spaces at the end of its lines. *Why:* they come from the layout of the file, not from the text.

**Long text.** *What changes:* text that SCID would cut short is kept in full. Exporting it can still lose text that contains `}` ([Export and round trips](#export-and-round-trips)). *Why:* SCID reads the text into buffers of `MAX_COMMENT_SIZE`, 16,000 bytes, in `pgnparse.cpp`. It adds a line if the text so far, with its line breaks, and the line are together under 16,000 bytes, and drops it otherwise. A line of more than 15,999 bytes is read in pieces, each treated as a line, which can split a UTF-8 character: from a line of 8,100 `é`s, `pgnscid` 4.26 stored only U+FFFD. `scidtopgn` reads the text without a fixed buffer, so it doesn't drop part of it without saying so.

**Comments between games.** *What changes:* they aren't kept in the text. *Why:* `;` and `{...}` are comments in PGN, and a `%` line is an escape for other programs. `scidtopgn` skipped them between games before #41 and still does. Reading a brace comment as a comment also keeps a line starting with `[` inside it from starting a game.

**A line starting with `[` that isn't a tag.** *What changes:* the line is reported as a failed game instead of being logged and ignored. *Why:* #41 only changed text that doesn't start with `[`. A malformed tag can't be told apart from a game whose tags are broken.

**A `[` after a result or a comment on the same line.** *What changes:* it is text, so `1-0 [Event "B"]` doesn't start a game, and `1-0 [Round 2 notes]` isn't reported as a failed game. *Why:* `scidtopgn` starts a game only at a tag line, as it does where a game without a result ends and where it resumes after a failed game. Before #41, `scidtopgn` read the `[` as a tag, as SCID does.

### Comments, tag values, results in variations, and stray brackets

These cases were added by [#60](https://github.com/intermezzo-chess/scidtopgn/issues/60).

| Case | Example | SCID | scidtopgn | Comparison |
| --- | --- | --- | --- | --- |
| Whitespace in a comment | `{  line one⏎   line two  }` | `line one line two` | `line one line two` | Same |
| `\"` in a tag value | `[Event "a \"b\" c"]` | `a \` | `a "b" c` | **Differs** |
| `\\` in a tag value | `[Site "x\\y"]` | `x\\y` | `x\y` | **Differs** |
| `\` before another character in a tag value | `[Site "C:\games"]` | `C:\games` | `C:\games` | Same |
| A tag value that ends with `\"` | `[Annotator "a \"]` | `a \` | Skips the game and reports `Unterminated string` | **Differs** |
| A result inside a variation | `1. e4 (1. d4 1-0) 1... e5 2. Nf3 *` | Ends the game with `1. e4 (1. d4) 1-0`, without saying that the result was inside a variation. `) 1... e5 2. Nf3 *` becomes the comment before the next game's first move. | Skips `1-0` with a warning, and imports `1. e4 (1. d4) e5 2. Nf3 *` | **Differs** |
| A result inside a variation that isn't closed | `1. e4 (1. d4 1-0` followed by `[Event ...]` | Keeps `1. e4 (1. d4) 1-0` | Skips the game and reports `Unexpected '['` | **Differs** |
| A long comment in the moves | 17,000 `a`s in braces | Keeps the first 15,999 bytes, without a message | Keeps all of it, up to the [record size](#limits-shared-with-scid) | **Differs** |
| A `}` outside a comment | `1. e4 } e5` | Skips it and logs `"}" seen outside a comment`, once per game | Skips it, with a warning for each | Same |
| A `]` in the moves | `1. e4 ] e5` | Skips it and logs `Unexpected symbol: ]` | Skips it with a warning | Same |
| A move right after a `]` | `]Bc5` | Drops the move and logs `Unexpected symbol: ]Bc5` | Skips the `]` with a warning, and plays `Bc5` | **Differs** |

In a comment, each run of whitespace, including line breaks, becomes one space, and whitespace at the ends is dropped, as in SCID. Whitespace means what C's `isspace` matches: space, tab, line feed, carriage return, vertical tab, and form feed. Other characters, such as a no-break space, are kept. A comment of only whitespace is empty, as `{}` is. This applies to comments in the moves and after the tags, not to [text before a game](#text-before-and-between-games), which keeps its line breaks. Before #60, `scidtopgn` kept a comment's whitespace as written.

**`\"` and `\\` in a tag value.** *What changes:* `\"` is read as `"`, so the value doesn't end there, and `\\` is read as one backslash. *Why:* these are the PGN standard's escapes, and other PGN programs write them. SCID ends the value at its first `"`, so it cuts off `a "b" c` after `a \`. A `\` before any other character is kept, as in SCID, so a value such as `C:\games`, written without escapes, reads the same in both. Before #60, `scidtopgn` dropped that `\`.

**A tag value that ends with `\"`.** *What changes:* a game that SCID imports is skipped and reported. *Why:* under the PGN standard, `\"` doesn't end the value, so the value has no end. `scidtopgn` doesn't guess where it ends.

**A result inside a variation.** *What changes:* the game is imported in full, with the result after the main line, and a warning names the skipped result. *Why:* SCID ends the game at the first result at any depth. It drops the rest of the main line and keeps it as text before the next game, so both games change. A result can't end a variation, so skipping it loses nothing. Before #60, `scidtopgn` skipped the game with `Unterminated variation`.

**A result inside a variation that isn't closed.** *What changes:* the game is skipped and reported. *Why:* skipping the result leaves the variation open, so the game fails at the next game's tags as any unclosed variation does.

**A long comment in the moves.** *What changes:* a comment that SCID would cut short is kept in full. *Why:* SCID reads it into a buffer of `MAX_COMMENT_SIZE` bytes. `scidtopgn` has no fixed buffer.

**A move right after a `]`.** *What changes:* the move is played instead of dropped. *Why:* SCID reads the `]` and the text after it as one unknown symbol. `scidtopgn` reads a `]` on its own, as it reads a `}`, so the move after it is read either way.

### Player names and ratings

These cases were added by [#43](https://github.com/intermezzo-chess/scidtopgn/issues/43). Before #43, `scidtopgn` stored player names as written, read a rating only from `WhiteElo` and `BlackElo`, and exported every rating as `WhiteElo` or `BlackElo`.

As SCID's `standardPlayerName` does, the value of a `White` or `Black` tag is tidied: spaces at the start are dropped, each comma that something follows gets exactly one space after it, and spaces and tabs at the end are dropped. Other characters, including other spaces, are kept. This gives the PGN standard's `Surname, Given name` form, so `Carlsen,Magnus` and `Carlsen, Magnus` are one player. Other tags aren't tidied.

Names that the tidying above leaves keep their control characters other than NUL, as in SCID ([#142](https://github.com/intermezzo-chess/scidtopgn/issues/142)): a control character inside a name, one at its start, and one at its end are stored as written, and export writes them as SCID does. The exception is a tab at the end, which that tidying drops with the trailing spaces: a `White` value of `A` followed by a tab is stored as `A`, and a name that is only a tab is stored as an empty string. A name whose remaining characters are all control characters is stored as written, and names that differ only by control characters stay different players. A NUL in a tag value is kept, where SCID drops the tag and logs `Error reading tag`, so the player is `?`. A tag value in a PGN file can't contain a line feed, which is an error in both. This is the tidying of a `White` or `Black` tag read from a PGN file: names read from a stored `.sn4` name file are kept as the file stores them, with no filtering or trimming, as [Reading](#reading) describes. The same holds for `Event`, `Site`, and `Round` names, which import stores as written.

A rating is read from three places, as SCID's `ExtractPgnTag` reads it:

- **A rating tag**: `White` or `Black` followed by one of SCID's kinds of rating, `Elo`, `Rating`, `Rapid`, `ICCF`, `USCF`, `DWZ`, or `ECF`, such as `WhiteUSCF`. The name must match exactly.
- **`WhiteElo` and `BlackElo`**, which are rating tags of the kind `Elo`.
- **The end of a player name**: a space, `(`, four ASCII digits, and `)`, after at least one other character, as in `Carlsen, Magnus (2850)`. It is removed from the name, and spaces and tabs left at the end are dropped. Its kind is `Elo`.

The first rating tag with a rating sets the player's rating and its kind. Later rating tags are kept as ordinary tags. A rating in a name replaces any rating read before it. A tag's value is read as SCID's `strtoul` reads it: after any whitespace and a `+`, the digits up to the first other character. A value of 0, or with no digits, such as `?`, sets no rating, and the tag is dropped. A rating above 4,000 is stored as 4,000, with a warning. The index entry stores each rating's kind, and export writes the rating with it, such as `[WhiteUSCF "2400"]`, as SCID does. A rating tag kept as an ordinary tag is exported too, unless reading the export back would change the player's rating with it: when it has the same name as the player's exported rating tag, or when the player has no rating.

| Case | Example | SCID | scidtopgn | Comparison |
| --- | --- | --- | --- | --- |
| Spacing in a player name | `Carlsen,Magnus`, ` Doe,John`, `Anand,  V` | `Carlsen, Magnus`, `Doe, John`, `Anand, V` | The same | Same |
| A space before a comma, or a final comma | `Doe , John`, `Carlsen,` | Kept | Kept | Same |
| Dutch name prefixes | `van der Wiel`, `Van Der Sterren`, `van Wely` | `Van der Wiel`, `Van der Sterren`, `Van Wely` | Kept as written | **Differs** |
| Rating in a player name | `Carlsen, Magnus (2850)` | `Carlsen, Magnus`, Elo 2850 | The same | Same |
| Names that don't end in a rating | `Smith (GM)`, `(123)`, `X (123)`, `(2850)`, `Y (２８５０)` | Kept as written | Kept as written | Same |
| `(0000)` in a name | `B (0000)` | `B`, with no rating | The same | Same |
| Rating in a name after tidying changes its length | `Carlsen,Magnus (2850)`, `Kasparov (2851) ` | `Carlsen, Magnus (2850)` and `Kasparov (2851)`, with no rating | `Carlsen, Magnus` and `Kasparov`, Elo 2850 and 2851 | **Differs** |
| Two spaces before a rating in a name | `Carlsen  (2850)` | `Carlsen ` with a space at the end, Elo 2850 | `Carlsen`, Elo 2850 | **Differs** |
| A rating tag | `[WhiteUSCF "2400"]` | USCF 2400, exported as `WhiteUSCF` | The same | Same |
| Rating tags SCID doesn't know | `[WhiteUscf "2400"]`, `[BlackFIDE "2400"]` | Ordinary tags | Ordinary tags | Same |
| A second rating tag | `[BlackUSCF "2400"]` then `[BlackElo "2500"]` | USCF 2400, and `BlackElo` as an ordinary tag | The same | Same |
| A rating tag after a rating in the name | `[White "Kasparov (2851)"]` then `[WhiteElo "2700"]` | Elo 2851, and `WhiteElo` as an ordinary tag, exported as a second `WhiteElo` | Elo 2851, and `WhiteElo` as an ordinary tag, which isn't exported | **Differs** |
| A rating tag before `(0000)` in the name | `[WhiteUSCF "2400"]`, `[WhiteElo "2500"]`, then `[White "A (0000)"]` | No rating, and `WhiteElo` as an ordinary tag, exported, so reading the export back gives Elo 2500 | No rating, and `WhiteElo` as an ordinary tag, which isn't exported | **Differs** |
| A rating in the name after a rating tag | `[WhiteElo "2700"]` then `[White "X (2600)"]` | Elo 2600 | Elo 2600 | Same |
| A value with no rating | `[WhiteDWZ "?"]` then `[WhiteRating "2100"]` | Drops `WhiteDWZ`, and reads 2100 as the rating | The same | Same |
| Text after the digits, or whitespace and `+` before them | `"2100x"`, `" +2400"` | 2100, 2400 | 2100, 2400 | Same |
| A rating above 4,000 | `[BlackICCF "4001"]`, `A (4500)` | 4,000, and logs `rating too large` | 4,000, with a warning | Same |
| A negative rating | `[WhiteElo "-5"]` | 4,000, and logs `rating too large` | No rating | **Differs** |
| A rating of 2³² or more | `[WhiteElo "4294967297"]` | Wraps around, here to 1 without a message, where `unsigned long` has 64 bits. Below | 4,000, with a warning | **Differs** |

**Dutch name prefixes.** *What changes:* a name starting with `van`, `Van Der`, `Van Den`, or `Van De` keeps its capital letters as written. *Why:* SCID's `standardDutchName` writes these prefixes as `Van der`, `Van den`, and `Van de`, so that one player isn't stored under several spellings. The PGN standard doesn't ask for it, and it changes how the file spells the name, so `scidtopgn` doesn't do it.

**Rating in a name after tidying changes its length.** *What changes:* the rating is read from the name, and the name is stored without it. *Why:* SCID tidies the name and then looks for the rating where it would end in the name's length before tidying. When tidying added or removed a space, it looks in the wrong place and keeps the rating in the name. `scidtopgn` looks for the rating in the tidied name.

**Two spaces before a rating in a name.** *What changes:* the stored name has no space at its end. *Why:* SCID tidies the name before removing the rating, so a space before the rating is left at the end. `scidtopgn` drops it, as tidying drops any other space at the end of a name.

**A rating tag after a rating in the name, and a rating tag before `(0000)` in the name.** *What changes:* the rating tag kept as an ordinary tag isn't exported. *Why:* SCID exports it. After a rating in the name, that gives the game two `WhiteElo` tags, which the PGN standard doesn't allow. When `(0000)` has cleared the rating, reading the export back makes the tag the player's rating. The tag is still stored with the game.

**A negative rating, and a rating of 2³² or more.** *What changes:* a negative rating is no rating, and a rating of 2³² or more is 4,000, with a warning. *Why:* SCID takes these steps:

1. `strtoul` reads the value as an `unsigned long`. A value too large for it, with or without a `-`, gives its largest number. Otherwise a `-` makes it wrap around.
2. Its low 32 bits are kept.
3. 0 is no rating, and more than 4,000 is stored as 4,000, with `rating too large` in the log.

With a 64-bit `unsigned long`, as on macOS, where these were run:

| Value | Low 32 bits | Stored |
| --- | --- | --- |
| `-5` | 4,294,967,291 | 4,000, logged |
| `-4294967295` | 1 | 1 |
| `-4294967296` | 0 | No rating |
| `-18446744073709551616` (too large) | 4,294,967,295 | 4,000, logged |
| `4294967296` (2³²) | 0 | No rating |
| `4294967297` | 1 | 1 |
| `4294971297` | 4,001 | 4,000, logged |
| `18446744073709551616` (2⁶⁴, too large) | 4,294,967,295 | 4,000, logged |

Where `unsigned long` has 32 bits, as on Windows, every value from 2³² on is too large, so SCID would store 4,000. This wasn't run. `scidtopgn` reads a negative value as no rating, and any value above 4,000 as 4,000.

### Known differences outside these cases

#37 doesn't change these. The first and the last were checked against SCID's sources in `scid-cpp/scid` and `scidtopgn` as of #37, and the others by importing the examples with `pgnscid` 4.26 and `scidtopgn` ([#145](https://github.com/intermezzo-chess/scidtopgn/issues/145)).

| Case | Example | SCID | scidtopgn |
| --- | --- | --- | --- |
| `Result` tag with more text | `[Result "1-0 (time)"]` | Reads `1-0` | Reads it as `*`. The result after the moves is used, with a warning only when that result isn't `*` (#27d). |
| A date part that isn't only digits | `2024x.05.06`, `20a0.01.01` | Reads the digits it starts with: 2024, 20 | The part is unknown |
| A date with another separator | `2020-05-06` | Reads the year, 2020 | Unknown |
| A month above 12 | `2020.13.01` | Month and day unknown | Month unknown, day 1 |
| An empty name tag | `[White ""]` | Ignores the tag, so the name is `?` or that of an earlier tag | Stores an empty name |
| Several tags on one line | `[Event "E"] [Site "S"]` | Reads the first, and ignores the rest of the line | Reads each |
| Text that isn't UTF-8 | A Latin-1 `é` in a tag | Stores the bytes unchanged | Reads each byte that isn't valid UTF-8 as Windows-1252, stores UTF-8, and warns (#42). Valid UTF-8 is unchanged, even on the same line, so a name written in Latin-1 and in UTF-8 is stored once. Latin-1 bytes that happen to be valid UTF-8, such as `Ã©`, are read as UTF-8, without a warning. The five bytes Windows-1252 leaves undefined are read as the control characters U+0081, U+008D, U+008F, U+0090, and U+009D. |

### Export and round trips

The cases above compare how a game is imported and stored. Exporting the stored game to PGN is a separate step, and it doesn't always give back what was imported. Variations, NAGs, and comments are exported where they were imported (#38). These are the losses:

- **A NAG before the first move** of the game or of a variation is dropped on import, as in SCID. See [Where annotations attach](#general-differences).
- **A NAG after a move's first 7** is dropped on import, as in SCID (#57). In the library, a move's `nags` can be set directly to more than 7 NAGs, or to NAG 0. The game is then stored and exported with only the first 7 that aren't 0, the NAGs SCID can hold, and the index's NAG count counts only those, as SCID's encoder does. So SCID reads back each NAG `scidtopgn` stores.
- **The en passant square** of a `FEN` is exported only when an en passant capture is legal (#31c).
- **A rating tag kept as an ordinary tag** isn't exported when it has the same name as the player's rating tag, such as a second `WhiteElo`, or when the player has no rating (#43). See [Player names and ratings](#player-names-and-ratings).
- **Tags after a game's 40th** are dropped on import, without a warning ([#155](https://github.com/intermezzo-chess/scidtopgn/issues/155)). See [Limits shared with SCID](#limits-shared-with-scid).
- **A comment that contains `}`**, which text before a game can give and SCID databases can hold, is exported as it is, so the comment ends there when the export is read. The rest is skipped with a warning, or read as moves or a result, which can change the game ([#152](https://github.com/intermezzo-chess/scidtopgn/issues/152)). SCID's export has the same text.
- **A name or other tag value with a line feed**, which PGN can't give but a database can hold, is exported over two lines. Reading the export skips the game up to the next tag line, and reads the rest as a game without what was skipped, such as its players or result. If none of its tag lines follows, as when the value is the last tag's, **the whole game is lost** ([#153](https://github.com/intermezzo-chess/scidtopgn/issues/153)).
- **Spaces at either end of a player name, and tabs at its end**, which PGN can't give but a database can hold, are dropped when the export is imported, by the [tidying of player names](#player-names-and-ratings).
- **Some dates with unknown parts** are stored in full but exported wrongly, and event dates too. With an unknown year, an unknown month or day is written as `00`. With a known year and an unknown month, a known day is written as `??` and lost. SCID writes `??` for each unknown part and keeps the known ones ([#154](https://github.com/intermezzo-chess/scidtopgn/issues/154)).
- **An ECO code that SCID doesn't know**, such as `BAD`, isn't stored, as in SCID. A game that was parsed and not stored still exports it.
- **A player name that still ends in a rating** once its rating is removed, such as `X (2850)` from `X (2850) (1234)`, is exported as it is. Reading it back removes ` (2850)` from the name and makes it the rating, as SCID also does with its own export (#43).

Tag values are exported with `"` written as `\"` and `\` as `\\`, as the PGN standard specifies, so `scidtopgn` reads a value with them back unchanged, apart from the losses above. SCID imports the exported game, with all its moves, but reads these escapes as written: `[Site "C:\\games"]` comes back from SCID as `C:\\games`, and `[Event "a \"b\" c"]` as `a \`. Before #60, `scidtopgn` wrote tag values without escapes, so it couldn't read back a value with `"`, and read `\` in one as an escape.

The moves themselves are exported as canonical SAN, so `ed` comes back as `exd5` and `0-0` as `O-O`. This is a change in notation, not a loss. The movetext has the spacing of the PGN standard's export format: a space after each move number, no space inside a variation's parentheses, and a move number before a Black move that starts a variation or follows a comment or a variation. The export isn't in that format in full:

- It writes some NAGs as symbols ([below](#how-nags-are-exported-34)), where the standard allows only `$n`.
- It writes the tags after the seven standard ones in an order of its own, such as ratings and ECO before other tags, where the standard sorts them by name.

Import reads both back. The standard allows tags in any order on import. Symbols other than `!`, `?`, `!!`, `??`, `!?`, and `?!`, such as `+=`, are extensions that SCID and `scidtopgn` read, and other programs may not.

For example, importing and exporting

```text
1. e4 e5 (1... c5 2. Nf3) 2. Nf3 $14 {Develops} Nc6 +-- 3. Bb5 *
```

gives this from `scidtopgn`:

```text
1. e4 e5 (1... c5 2. Nf3) 2. Nf3 += {Develops} 2... Nc6 +-- 3. Bb5
*
```

The SCID converter gives this:

```text
1.e4 e5 ( 1...c5 2.Nf3 ) 2.Nf3 += {Develops} 2...Nc6 +-- 3.Bb5 *
```

#### How NAGs are exported (#34)

| | SCID | scidtopgn | Comparison |
| --- | --- | --- | --- |
| A NAG with a symbol both read back | The symbol | The symbol | Same |
| Any other NAG | A symbol from `evalNagsRegular` in `scid-cpp/scid/nagtext.h`, such as `→` for $40, `forced` for $7, or `RR` for $145, or `$n` where it has none | `$n` | **Differs** |

The symbols both read back are `!` `?` `!!` `??` `!?` `?!` (NAGs 1 to 6), `=` (10), `~` (13), `+=` (14), `=+` (15), `+/-` (16), `-/+` (17), `+-` (18), `-+` (19), `+--` (20), `--+` (21), `=/&` (44), `R` (144), `N` (146), and `D` (201). Every NAG from 1 to 255 can be imported again from `scidtopgn`'s export.

**#34: NAGs without a symbol both read back.** *What changes:* `scidtopgn` writes `$40` where SCID writes `→`, for example. *Why:* many of SCID's symbols are Unicode or words that neither SCID nor `scidtopgn` reads, so a game exported with them loses those annotations when it is imported again. `$n` is the standard PGN form, and every PGN reader reads it. Before #37, `scidtopgn` exported most NAGs, such as $20 and $41, as nothing, and others as Unicode symbols, such as `⩲`, that it couldn't import again.

### How SCID's behavior was checked

[`scid-cpp/tools/pgn_probe.cpp`](../scid-cpp/tools/pgn_probe.cpp) runs SCID's PGN parser from `scid-cpp/scid` on a PGN file. For each game, it prints:

- whether SCID would skip the game
- the game as SCID would store it, without tags whose value is `?`
- the messages SCID logged for the game

Build and run it from the repository root with a C++17 compiler and zlib:

```sh
c++ -std=c++17 -Wno-register -w -Iscid-cpp/scid \
    scid-cpp/tools/pgn_probe.cpp \
    scid-cpp/scid/{bytebuf,date,dstring,game,gfile,index,matsig,mfile,misc,movelist,myassert,namebase,pgnparse,position,stralloc,textbuf}.cpp \
    -lz -o pgn_probe
./pgn_probe tests/data/notation.pgn
```

`scid-cpp/CMakeLists.txt` doesn't build `pgn_probe`, so these commands call the compiler directly. To build the reference converter itself, for example to write a test database such as `tests/data/castling`, follow [The C++ reference converter](../CONTRIBUTING.md#the-c-reference-converter-scid-cpp) in `CONTRIBUTING.md`. Then run it from the repository root as `scid-cpp/build/scidtopgn <database> <pgn-file>`.

To compare with `scidtopgn`, run `cargo test -p scidtopgn --test test_pgn test_notation_fixture`, which checks every game in `tests/data/notation.pgn`. To check another file, run `scidtopgn import <file> -o <database>`, which prints each warning and skipped game. The cases in [Known differences outside these cases](#known-differences-outside-these-cases) were checked the same way, with the example inputs shown. So were the cases in [Text before and between games](#text-before-and-between-games), in [Player names and ratings](#player-names-and-ratings), and in [Comments, tag values, results in variations, and stray brackets](#comments-tag-values-results-in-variations-and-stray-brackets), where `pgn_probe` was also run on `scidtopgn`'s export of games with escaped tag values. `pgn_probe` joins the lines of its output with spaces, so where the text's line breaks and spaces matter, SCID's stored comment was printed with `Game::GetMoveComment` instead.

<details>
<summary>SCID's output for <code>tests/data/notation.pgn</code></summary>

```text
game 1: OK | [Event "A1 castling with zeros or lowercase"] [Result "*"] 1.e4 e5 2.Nf3 Nf6 3.Bc4 Bc5 4.O-O O-O *
game 2: OK | [Event "A2 pawn prefix"] [Result "*"] 1.e4 e5 *
game 3: OK | [Event "A3 missing or extra capture sign"] [Result "*"] 1.e4 d5 2.exd5 Qxd5 3.Nc3 *
game 4: OK | [Event "A4 pawn coordinates"] [Result "*"] 1.e4 e5 *
game 5: OK | [Event "A5 null moves"] [Result "*"] 1.e4 -- 2.-- -- 3.d4 *
game 6: OK | [Event "A5b null move in check"] [Result "*"] [FEN "k7/8/8/8/8/8/4r3/4K3 w - - 0 1"] 1.-- *
game 7: OK | [Event "A6 check and mate signs"] [Result "*"] 1.e4 e5 2.Bc4 Nc6 3.Qh5 Nf6 4.Qxf7# *
game 8: OK | [Event "A7 move numbers without dots"] [Result "*"] 1.e4 e5 2.Nf3 *
game 9: OK | [Event "A8 double check sign"] [Result "*"] 1.e4 f6 2.d4 g5 3.Qh5# *
game 10: OK | [Event "1a pawn capture without a rank"] [Result "*"] 1.e4 d5 2.exd5 *
game 11: OK | [Event "1b pawn capture without a rank, with x"] [Result "*"] 1.e4 d5 2.exd5 *
game 12: OK | [Event "1c pawn capture without a rank, en passant"] [Result "*"] [FEN "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 2"] 2.exd6 *
game 13: OK | [Event "2 ambiguous pawn capture without a rank"] [Result "*"] [FEN "4k3/8/3n4/4P3/3n4/4P3/8/4K3 w - - 0 1"] 1.exd4 *
game 14: OK | [Event "3 colon for a capture"] [Result "*"] 1.e4 d5 2.exd5 Qxd5 *
game 15: OK | [Event "4 hyphen between squares"] [Result "*"] 1.e4 e5 2.Nf3 Nc6 *
game 16: OK | [Event "5 coordinate piece moves"] [Result "*"] 1.Nf3 Nf6 *
game 17: OK | [Event "6a king moves two squares"] [Result "*"] 1.e4 e5 2.Nf3 Nf6 3.Bc4 Bc5 4.O-O O-O *
game 18: OK | [Event "6b king coordinate castling"] [Result "*"] 1.e4 e5 2.Nf3 Nf6 3.Bc4 Bc5 4.O-O O-O *
game 19: OK | [Event "6c king moves two squares without the castling right"] [Result "*"] 1.e4 e5 2.Nf3 Nf6 3.Bc4 Bc5 4.Ke2 Ke7 5.Ke1 Ke8 {Error reading move: Kg1} *
    logged: (game 19, line 77) Error reading move in game ? - ?, 0: Kg1
game 20: OK | [Event "7a promotion with no piece"] [Result "*"] [FEN "4k3/P7/8/8/8/8/8/4K3 w - - 0 1"] 1.a8=Q+ *
game 21: OK | [Event "7b capture promotion with no piece"] [Result "*"] [FEN "1n2k3/P7/8/8/8/8/8/4K3 w - - 0 1"] 1.axb8=Q+ *
game 22: OK | [Event "7c coordinate promotion with no piece"] [Result "*"] [FEN "4k3/P7/8/8/8/8/8/4K3 w - - 0 1"] {Error reading move: a7a8}  *
    logged: (game 22, line 92) Error reading move in game ? - ?, 0: a7a8
game 23: OK | [Event "8a lowercase promotion piece"] [Result "*"] [FEN "4k3/P7/8/8/8/8/8/4K3 w - - 0 1"] 1.a8=Q+ *
game 24: OK | [Event "8b lowercase coordinate promotion"] [Result "*"] [FEN "4k3/P7/8/8/8/8/8/4K3 w - - 0 1"] 1.a8=Q+ *
game 25: OK | [Event "8c lowercase capture promotion without a rank"] [Result "*"] [FEN "1n2k3/P7/8/8/8/8/8/4K3 w - - 0 1"] 1.axb8=N *
game 26: OK | [Event "8d lowercase bishop promotion"] [Result "*"] [FEN "4k3/P7/8/8/8/8/8/4K3 w - - 0 1"] {Error reading move: a8b}  *
    logged: (game 26, line 112) Error reading move in game ? - ?, 0: a8b
game 27: OK | [Event "8e lowercase coordinate bishop promotion"] [Result "*"] [FEN "4k3/P7/8/8/8/8/8/4K3 w - - 0 1"] 1.a8=B *
game 28: OK | [Event "9 lowercase piece after equals sign"] [Result "*"] [FEN "4k3/P7/8/8/8/8/8/4K3 w - - 0 1"] 1.a8=Q+ *
    logged: (game 28, line 122) Warning: Invalid annotation symbol: =n
game 29: OK | [Event "10a capture promotion without a rank"] [Result "*"] [FEN "1n2k3/P7/8/8/8/8/8/4K3 w - - 0 1"] 1.axb8=Q+ *
game 30: OK | [Event "10b capture promotion without a rank or equals sign"] [Result "*"] [FEN "1n2k3/P7/8/8/8/8/8/4K3 w - - 0 1"] 1.axb8=Q+ *
game 31: OK | [Event "11a en passant marker attached"] [Result "*"] [FEN "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 2"] 2.exd6 *
game 32: OK | [Event "11b en passant marker after the move"] [Result "*"] [FEN "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 2"] 2.exd6 *
game 33: OK | [Event "12 lowercase piece letter"] [Result "*"] 1.e4 e5 *
    logged: (game 33, line 146) Error: Unexpected symbol: nf
game 34: OK | [Event "13 lowercase b for a bishop"] [Result "*"] 1.e4 e5 {Error reading move: bc4} *
    logged: (game 34, line 150) Error reading move in game ? - ?, 0: bc4
game 35: OK | [Event "14 castling without dashes"] [Result "*"] [FEN "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1"] 1.O-O-O O-O *
game 36: OK | [Event "15 castling with zeros and no dashes"] [Result "*"] [FEN "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1"] *
game 37: OK | [Event "16a castling after the king moved"] [Result "*"] 1.e4 e5 2.Nf3 Nf6 3.Bc4 Bc5 4.Ke2 Ke7 5.Ke1 Ke8 6.O-O O-O *
    logged: (game 37, line 164) Warning: illegal castling (O-O) in game ? - ?, 0
    logged: (game 37, line 164) Warning: illegal castling (O-O) in game ? - ?, 0
game 38: OK | [Event "16b castling after the rook moved"] [Result "*"] 1.e4 e5 2.Nf3 Nf6 3.Bc4 Bc5 4.Rg1 Rg8 5.Rh1 Rh8 6.O-O *
    logged: (game 38, line 168) Warning: illegal castling (O-O) in game ? - ?, 0
game 39: OK | [Event "16c queenside castling without the right in the FEN"] [Result "*"] [FEN "r3k2r/8/8/8/8/8/8/R3K2R w - - 0 1"] 1.O-O-O *
    logged: (game 39, line 173) Warning: illegal castling (O-O-O) in game ? - ?, 0
game 40: OK | [Event "16d castling without the right in a variation"] [Result "*"] 1.e4 e5 2.Nf3 Nf6 3.Bc4 Bc5 4.Ke2 Ke7 5.Ke1 Ke8 6.d3 ( 6.O-O ) 6...d6 *
    logged: (game 40, line 177) Warning: illegal castling (O-O) in game ? - ?, 0
game 41: OK | [Event "18a castling with no rook"] [Result "*"] [FEN "4k3/8/8/8/8/8/8/4K3 w - - 0 1"] 1.O-O *
    logged: (game 41, line 182) Warning: illegal castling (O-O) in game ? - ?, 0
game 42: OK | [Event "18b queenside castling with no rook"] [Result "*"] [FEN "4k3/8/8/8/8/8/8/4K3 w - - 0 1"] 1.O-O-O *
    logged: (game 42, line 187) Warning: illegal castling (O-O-O) in game ? - ?, 0
game 43: OK | [Event "19 castling through check"] [Result "*"] [FEN "4k1r1/8/8/8/8/8/8/4K2R w K - 0 1"] {Error reading move: O-O}  *
    logged: (game 43, line 192) Error reading move in game ? - ?, 0: O-O
game 44: OK | [Event "20a kingside castling when only queenside is legal"] [Result "*"] [FEN "r3k2r/8/8/8/8/8/8/R3K1NR w KQkq - 0 1"] 1.O-O-O *
game 45: OK | [Event "20b queenside castling when only kingside is legal"] [Result "*"] [FEN "r3k2r/8/8/8/8/8/8/RN2K2R w KQkq - 0 1"] {Error reading move: O-O-O}  *
    logged: (game 45, line 202) Error reading move in game ? - ?, 0: O-O-O
game 46: OK | [Event "21a annotation symbols attached"] [Result "*"] 1.e4 $1 e5 $2 2.Nf3 $3 Nc6 $4 3.Bb5 $5 a6 $6 *
game 47: OK | [Event "21b annotation symbols after the move"] [Result "*"] 1.e4 $1 e5 $2 2.Nf3 $14 Nc6 $15 3.Bb5 $16 a6 $17 4.Ba4 $18 Nf6 $19 5.O-O $10  Be7 $13 *
game 48: OK | [Event "21c other annotation symbols"] [Result "*"] 1.e4 $14 e5 $15 2.Nf3 $20 Nc6 $21 3.Bb5 $40 a6 $41 4.Ba4 $44 Nf6 $44 *
game 49: OK | [Event "21d annotation symbols after check"] [Result "*"] 1.e4 e5 2.Bc4 Nc6 3.Qh5 Nf6 4.Qxf7# $1 *
game 50: OK | [Event "22a annotation symbols attached by equals or hyphen"] [Result "*"] 1.e4 $15 e5 2.Nf3 $10 Nc6 $19 {Error reading move: a6} *
    logged: (game 50, line 222) Error: Unexpected symbol: Bb5
    logged: (game 50, line 222) Error reading move in game ? - ?, 0: a6
    logged: (game 50, line 222) Error: Unexpected symbol: Ba4
game 51: OK | [Event "22b several annotation symbols attached"] [Result "*"] 1.e4 $3 $18 e5 *
game 52: OK | [Event "22c symbols starting with two hyphens or an arrow attached"] [Result "*"] 1.e4 $19 e5 {Error reading move: Nc6} *
    logged: (game 52, line 230) Error: Unexpected symbol: Nf3
    logged: (game 52, line 230) Error reading move in game ? - ?, 0: Nc6
game 53: OK | [Event "23 tilde attached"] [Result "*"] {Error reading move: e5}  *
    logged: (game 53, line 234) Error: Unexpected symbol: e4
    logged: (game 53, line 234) Error reading move in game ? - ?, 0: e5
game 54: OK | [Event "24 letter symbols"] [Result "*"] 1.e4 $146 e5 $201 2.Nf3 $144 Nc6 *
game 55: OK | [Event "25a unknown symbols"] [Result "*"] 1.e4 e5 2.Nf3 Nc6 *
    logged: (game 55, line 242) Error: Unexpected symbol: zz
    logged: (game 55, line 242) Error: Unexpected symbol: /\
game 56: OK | [Event "25b piece letter on its own"] [Result "*"] 1.e4 e5 *
    logged: (game 56, line 246) Warning: Invalid annotation symbol: B
game 57: OK | [Event "25c word that starts like a null move"] [Result "*"] 1.e4 e5 *
    logged: (game 57, line 250) Error: Unexpected symbol: nullify
game 58: OK | [Event "26a result 1:0"] [Result "1-0"] 1.e4 e5 1-0
game 59: OK | [Event "26b result 0:1"] [Result "0-1"] 1.e4 e5 0-1
game 60: OK | [Event "26c result 1/2"] [Result "1/2-1/2"] 1.e4 e5 1/2-1/2
game 61: OK | [Event "26d result 1/2:1/2"] [Result "1/2-1/2"] 1.e4 e5 1/2-1/2
game 62: OK | [Event "27a result differs from the tag"] [Result "0-1"] 1.e4 e5 0-1
    logged: (game 62, line 275) Result did not match the header result
game 63: OK | [Event "27b unknown result with a result tag"] [Result "*"] 1.e4 e5 *
    logged: (game 63, line 280) Result did not match the header result
game 64: OK | [Event "27c result with no result tag"] [Result "1-0"] 1.e4 e5 1-0
    logged: (game 64, line 284) Result did not match the header result
game 65: OK | [Event "27d unrecognized result tag and unknown result"] [Result "*"] 1.e4 e5 *
    logged: (game 65, line 289) Result did not match the header result
game 66: ERROR (SCID skips this game) | [Event "28a no result before the next game"] [Result "1-0"] 1.e4 e5 1-0
    logged: (game 66, line 296) PGN header '[' seen inside game (result missing ?)
game 67: ERROR (SCID skips this game) | [Event "28c no result and no result tag"] [Result "*"] 1.e4 e5 *
    logged: (game 67, line 300) PGN header '[' seen inside game (result missing ?)
game 68: OK | [Event "29 move that can't be read"] [Result "*"] 1.e4 e5 {Error reading move: Nf6} *
    logged: (game 68, line 302) Error reading move in game ? - ?, 0: Nf6
game 69: OK | [Event "30 SetUp without FEN"] [Result "*"] [SetUp "1"] 1.e4 *
game 70: OK | [Event "31a FEN castling rights that don't fit"] [Result "*"] [FEN "4k3/8/8/8/8/8/8/4K3 w KQkq - 0 1"] 1.Kd1 *
game 71: OK | [Event "31b FEN en passant square that doesn't fit"] [Result "*"] [FEN "4k3/8/8/8/8/8/8/4K3 w - e3 0 1"] 1.Kd1 *
game 72: OK | [Event "31c FEN en passant square with no capture"] [Result "*"] [FEN "4k3/8/8/8/4P3/8/8/4K3 b - e3 0 1"] 1...Kd7 *
game 73: OK | [Event "32 Variant tag"] [Result "*"] [Variant "Chess960"] 1.e4 *
game 74: ERROR (SCID skips this game) | [Event "28b no result at the end of the input"] [Result "0-1"] 1.e4 e5 0-1
    logged: (game 74, line 333) End of input reached in game (result missing ?)
```

</details>

### Tests

- **`test_notation_fixture`** in `crates/scidtopgn/tests/test_pgn.rs` checks how `scidtopgn` reads each game in `tests/data/notation.pgn`: the moves, NAGs, result, warnings, or error.
- **`test_annotation_symbols_match_nags`** checks that each annotation symbol gives the same game as its `$n` NAG.
- **`test_player_names_are_tidied`, `test_rating_in_a_player_name`, `test_rating_tags`, `test_ratings_above_the_maximum`, `test_ratings_are_exported_with_their_kind`, `test_rating_tags_that_would_change_the_rating_are_not_exported`, `test_a_name_still_ending_in_a_rating_loses_it_on_reimport`, and `test_player_names_are_limited_to_255_bytes_after_tidying`** in `test_pgn.rs` check the cases in [Player names and ratings](#player-names-and-ratings) and their export. `test_rating_types_are_stored_and_exported` in `test_database.rs` checks that the index keeps each rating's kind.
- **The other tests in `test_pgn.rs`** check that warnings name the game and line, and that a rejected game reports only its error.
- **Unit tests in `crates/scidtopgn/src/pgn.rs`** cover each notation, the symbol table, and how tokens are split.
- **`tests/data/castling`** was written by the reference converter from `tests/data/castling.pgn`. `crates/scidtopgn/src/tests/test_interop.rs` checks that `scidtopgn` reads it (#17) and writes the same moves for `castling.pgn`. `position.rs` and `mov.rs` test castling without the castling right on its own.
- **`tests/data/annotated`** was written by the reference converter from `tests/data/annotated.pgn`, which has NAGs and comments in the main line and in variations. `test_interop.rs` checks that `scidtopgn` reads and exports it with each annotation where SCID stores it, and writes the same moves and annotations for `annotated.pgn` (#38).
- **`crates/scidtopgn/src/tests/test_variations.rs`** checks where annotations attach in variations nested six levels deep, after PGN import and after `scidtopgn` writes them to a SCID database and reads them back, and checks both variation limits from PGN and from a SCID database. It doesn't run SCID ([issue 4](https://github.com/intermezzo-chess/scidtopgn/issues/4)).
- **`test_comment_whitespace_is_single_spaced`**, **`test_tag_value_escapes`**, **`test_tag_values_with_quotes_and_backslashes_round_trip`**, **`test_a_result_inside_a_variation_is_skipped`**, and **`test_stray_closing_brackets_give_warnings`** in `test_pgn.rs` check the cases added by #60.
- **`test_every_nag_survives_export_and_import`** in `crates/scidtopgn/src/game.rs` exports and imports each NAG from 1 to 255 (#34).
- **`test_a_move_keeps_at_most_7_nags`, `test_nags_from_symbols_and_after_variations_count_toward_the_limit`, and `test_each_move_has_its_own_nag_limit`** in `test_pgn.rs` check the NAG limit on import and its warnings. **`test_decoding_keeps_the_first_7_nags_of_a_move`** in `test_move_encoding.rs` checks it when reading a SCID record. **`test_a_move_with_more_than_7_nags_round_trips_as_scid_reads_it`** and **`test_nags_set_directly_are_stored_as_scid_holds_them`** in `test_database.rs` check what is stored, counted in the index, and exported (#57).
- **`test_cli_import_prints_warnings_and_keeps_the_game`** in `crates/scidtopgn-cli/tests/integration.rs` checks how the CLI prints warnings in both `--on-error` modes (#33).
- **The pre-game text tests in `test_pgn.rs`**, such as `test_text_before_a_game_is_its_first_comment`, check each case in [Text before and between games](#text-before-and-between-games). **`test_cli_import_keeps_text_before_games`** in `crates/scidtopgn-cli/tests/integration.rs` checks that the CLI numbers the games correctly in both `--on-error` modes, and that the text is stored and exported as each game's first comment (#41).
