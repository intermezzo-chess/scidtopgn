# Changelog

All notable changes to the `scidtopgn` library and the `scidtopgn-cli` crate are recorded here. Both crates share one version. Entries are generated from changesets by the Version workflow; see [docs/RELEASING.md](docs/RELEASING.md).

## 0.2.0 (2026-11-16)

### Breaking Changes

#### `scidtopgn info` needs the database's game file to exist

`info` still reads only the index and name files, so a damaged or read-only `.sg4` file works as before. A missing one is now an error, as it is for export.

#### Keep text before a PGN game as its first comment instead of reading it as a game

As in SCID, a PGN game now starts at its first tag line. Text before it, such as notes at the start of a file or between games, is kept as the comment before the game's first move. Text after the last game is dropped. Before, the text was read as a game, which failed: `scidtopgn import` reported an extra skipped game and numbered every later game one too high, and with `--on-error abort` it couldn't import the file.

Moves with no tags are now text too. Before the next game they become its first comment, and at the end of the input they are dropped, so a file of games without tags imports no games. `PgnParser::parse_game` and `parse_single_game` still read moves without tags.

#### Keep the library's public API to databases, games, and PGN

The modules are private, and every public item is at the crate root. `scidtopgn::index::MAX_ELO` is now `scidtopgn::MAX_ELO`, and `scidtopgn::search::DEFAULT_MAX_LENGTH` is `scidtopgn::DEFAULT_MAX_LENGTH`.

The types and functions that read and write SCID's files are no longer public: `ByteBuffer`, `GFile`, `FileMode`, `NFile`, `NameBase`, `NameEntry`, `Index`, `IndexHeader`, `ScidPosition`, `PieceList`, `encode_move`, `decode_move`, `make_move_byte`, `parse_move_byte`, `eco_to_string`, and `eco_from_string`, and the constants that describe the file formats, such as `ENCODE_*`, `IDX_FLAG_*`, `IDX_MASK_*`, `INDEX_*`, `GF_BLOCKSIZE`, `NAMEBASE_MAGIC`, and `GAME_DECODE_*`. `MAX_NAGS` is private too: `MoveNode::nags` states that a move keeps at most 7 NAGs.

- `Database::index`, `nfile`, and `gfile` are removed. The new `Database::converted_names` lists the names that weren't UTF-8, and `Database::name_count` counts the names of a kind.
- `IndexEntry` is read-only, and keeps the getters that need no knowledge of SCID's packing: the date, its parts, the result, the ratings, the ECO code as text, the half-move count, the start, promotion, and delete flags, and the variation, comment, and NAG counts. `get_date` returns a `Date`, replacing `get_date_obj`.
- `Game::decode`, `decode_start`, `decode_tags`, `initial_position`, `load_standard_tags`, and `add_variation`, and the `current_move` field, are private.
- `GameResult::to_byte` and `from_byte`, `RatingType::to_byte` and `from_byte`, `Date::from_raw`, `raw`, and `month_day`, `Eco::code`, and `NameType::as_usize` are private.
- `date_valid_string`, `date_make`, `date_get_year`, `date_get_month`, `date_get_day`, `date_decode_to_string`, `date_encode_from_string`, `Date::add_months`, and `GameResult::from_char` and `to_char` are removed or private. The library doesn't use them; `Date` reads, writes, and compares dates.
- `Error::BufferFull` and `Error::NoCurrentMove` are removed. Only the private `ByteBuffer` and `Game::add_variation` returned them.

#### Let option, game, and error types gain fields without a breaking change

`HeaderSearch`, `HeaderFlags`, `TwinOptions`, `DeleteTwin`, `Game`, `MoveNode`, and `PgnWarning` are `#[non_exhaustive]`, and so is each variant of `Error`, `CorruptGameKind`, and `InvalidMoveByte` that has fields. Start from `Default::default()` and set fields, instead of writing a struct expression, and add `..` to patterns that name a variant's fields, such as `Error::GameNotFound { number, .. }`.

#### Read ratings from player names and rating tags, tidy player names, and export each rating with its kind

PGN import now reads player names and ratings as SCID does, as [docs/PGN_IMPORT.md](https://github.com/intermezzo-chess/scidtopgn/blob/main/docs/PGN_IMPORT.md) describes:

- `White` and `Black` tag values are tidied: spaces at the start and at the end are dropped, and each comma that something follows gets one space after it, so `Carlsen,Magnus` is stored as `Carlsen, Magnus`. Before, names were stored as written. A name longer than 255 bytes once tidied is dropped with a `LongTagValue` warning, as a longer tag is.
- A rating at the end of a player name, such as `Carlsen, Magnus (2850)`, is removed from the name and becomes the player's Elo rating.
- A rating tag of one of SCID's kinds, `WhiteElo`, `WhiteRating`, `WhiteRapid`, `WhiteICCF`, `WhiteUSCF`, `WhiteDWZ`, or `WhiteECF`, or the same for Black, sets the player's rating and its kind. The first one with a rating is used, and later ones are kept as ordinary tags. Before, only `WhiteElo` and `BlackElo` were read, and the last one was used.
- A rating is read as SCID reads it: `2100x` is 2100, and a value with no digits, such as `?`, is no rating. A negative value is no rating.
- A rating above 4,000 is stored as 4,000, with the new `PgnWarningKind::RatingTooLarge` warning. Before, it was stored as 4,000 without a warning.

The new `RatingType` gives a rating's kind, and the new `Game::white_rating_type` and `Game::black_rating_type` fields hold it. A database stores it in the index, and reading a game sets it. PGN export writes each rating with its kind, as SCID does, such as `[WhiteUSCF "2400"]`, so games from SCID databases whose ratings aren't Elo ratings export differently. Before, every rating was exported as `WhiteElo` or `BlackElo`. A rating tag kept as an ordinary tag, such as a second rating tag, is now exported, unless it has the same name as the player's rating tag or the player has no rating, so that reading the export back doesn't change the rating. Before, an ordinary `WhiteElo` or `BlackElo` tag was never exported. A database written by 0.1.0's PGN import keeps rating tags other than `WhiteElo` and `BlackElo`, such as `WhiteUSCF`, as ordinary tags without a rating, so exporting it now drops them. Import the PGN again to read them as ratings.

Code that builds a `Game` with a struct expression must set the two new fields, or use `..Game::new()`.

#### Read the PGN notations SCID reads, and report what import changes

PGN import reads the notations SCID reads: a pawn capture without a rank (`ed`), `:` for a capture, `-` between squares, coordinates for any piece (`g1f3`, `e1g1`), a king moving two squares (`Kg1`), castling without dashes (`OO`, `00`), lowercase or missing promotion pieces (`a8q`, `a8`), en passant markers (`exd6 e.p.`), annotation symbols such as `!?`, `+=`, `N`, and `D` as NAGs, and results written `1:0`, `0:1`, or `1/2`. Like SCID, castling after the king or rook has moved and returned is played, and export reads SCID databases that contain such a castle. The result after the moves replaces the `Result` tag. `--+` is NAG 21, not a null move with a check sign.

Problems that don't stop a game from being imported are reported as warnings, which the CLI prints to stderr: castling without the castling right, `O-O` played as `O-O-O`, unknown text in the moves, castling rights and en passant squares dropped from a `FEN`, a result that replaces the `Result` tag, and moves that don't end with a result. Text that isn't a move, an annotation, or a result is skipped with a warning instead of being dropped silently or failing the game.

PGN export writes each NAG as the symbol SCID writes when SCID and `scidtopgn` both read it back as the same NAG, such as `!`, `+=`, or `N`, and as `$n` otherwise. Previously most NAGs, such as $20 and $41, were dropped from the export, and others were written as Unicode symbols, such as `⩲`, that couldn't be imported again. `nag_to_symbol` returns these symbols, or `""` for a NAG written as `$n`.

[docs/PGN_IMPORT.md](https://github.com/intermezzo-chess/scidtopgn/blob/main/docs/PGN_IMPORT.md) lists each notation and where `scidtopgn` differs from SCID.

Breaking changes to the library API:

- `Game` has a new public field, `warnings: Vec<PgnWarning>`.
- `PgnWarning` and `PgnWarningKind` are new.

#### Remove the `scidtopgn::private` module

`skip_tags`, `is_move_byte`, and `is_special_byte` were public only so that tests could use them, and the library didn't call them.

#### Require Rust 1.97 or later

Building `scidtopgn` or `scidtopgn-cli`, including with `cargo install --locked`, now requires Rust 1.97 or later instead of 1.70. The library's new chess rules dependency, shakmaty 0.30.2, requires it.

#### Report damaged SCID databases with structured errors instead of panicking or using messages

A damaged SCID database, or a game too large to store, no longer panics. These panicked before: a name in the `.sn4` file that claims to share more bytes with the previous name than it has, an index entry with zero length, and adding a game whose record is longer than a game file block. A game with more than 65,535 half-moves in its main line, such as a long run of null moves, no longer panics either; `Game::num_half_moves` stops at 65,535. A comment or starting FEN that ends without its terminator is now an error instead of being read as if it were complete.

In the library, `Error` is `#[non_exhaustive]`, and SCID errors are variants that callers can match on, such as `CorruptGame`, `GameOutOfRange`, `InvalidMagic`, `Truncated`, `NamePrefixTooLong`, `VariationTooDeep`, `TooManyVariations`, `IndexFull`, `NameBaseFull`, and `GameTooLarge`. `CorruptGame` gives the byte offset in the game record and a `CorruptGameKind`. This includes a null move in check and a starting position that can't be read, which returned `InvalidMove` and `InvalidFen` without an offset before. `Database::get_game` wraps its errors in `Error::Game`, which numbers games from 1, and returns `GameNotFound` for a game past the end. Opening or creating a database file wraps its errors in `Error::File`, which names the file as a `DatabaseFile`. `Error::Io` now displays only the I/O error.

The `Decode`, `Database`, `CorruptData`, `Encoding`, `Buffer`, `BufferRead`, `NameNotFound`, and `EndOfMoveList` variants are removed. `Database::add_game` rejects a game whose record is longer than 131,071 bytes, the most the index can record.

`scidtopgn export` names the database it can't open and the game it can't read, for example `SCID game 3` followed by `Damaged game data at byte 17: …`.

#### Use shakmaty for chess rules

The library now uses [shakmaty](https://crates.io/crates/shakmaty) 0.30 for legality, SAN, check and mate detection, and FEN, and keeps only SCID's move encoding and piece numbering.

- PGN export disambiguates moves correctly (for example `Nbd2`, `R1a3`, `Qh4e1`, and `Nd2` when the other knight is pinned) and writes `#` only for checkmate.
- Every move read from a SCID database or PGN file is checked for legality. An illegal move is an error instead of a corrupted game.
- PGN import stores the SAN it generates, so check and mate signs, unneeded disambiguation, castling written with zeros or lowercase letters (previously skipped without a warning), and promotions without `=` are corrected.

Breaking changes to the library API:

- `Board`, `SimpleMove`, `Square`, `Piece`, `Color`, `piece_make`, `piece_color`, `piece_type`, the castling flag constants, and the per-piece `encode_*` and `decode_*` functions are removed. Use the re-exported `scidtopgn::shakmaty` types and `GameMove`.
- `Game::start_board`, `current_board`, `non_standard_start`, and `var_depth` are replaced by `Game::start_position: Option<Chess>`. `Game::make_move_on_board` is removed.
- `MoveNode::move_data` is an `Option<GameMove>`, `None` for marker nodes.
- `Error::InvalidFen` has a `source`, and `Error::PgnMove`, `Error::PgnGame`, and `Error::UnsupportedVariant` are new.
- `private::MoveDecoder`, `private::DecodeElement`, and `private::decode_move` are removed.

### Features

#### Export selected games, show database info, and quiet the CLI

- `scidtopgn export --games 10-20` exports only the given games, numbered from 1. It takes numbers and ranges separated by commas, such as `1-5,8`, and still skips logically deleted games.
- `scidtopgn info` shows a database's total, deleted, and active games, its player, event, site, and round names, and the years of its active games. It reads only the index and name files.
- `--quiet` (`-q`) on export, import, and `info` writes nothing to stderr, not even errors. The exit status still shows whether the command failed.
- `--verbose` now reports progress at each 10% of the games, such as `Processed 50% (50/100 games)`, instead of every 100 games.
- The help and README now say that `--count` includes logically deleted games, so it can be more than the number of games exported.
- Options given before a subcommand, as in `scidtopgn --games 1-5 export mydb`, were accepted and ignored. They're now rejected with exit status 2, and the error shows the command with the options moved after the subcommand, or says how to fix it.

#### Write the CLI's warnings, errors, and progress to a log file

- `--log <path>` on export, import, and `info` appends the messages stderr gets without `--quiet` to a file: warnings and errors, and with `--verbose` progress and summaries too. Each message is one line starting with `error: `, `warning: `, or `info: `, and each run starts with a line giving the version and the arguments.
- `--quiet` and `--verbose` can now be combined, but only together with `--log`, so that a quiet run can still record progress.
- The log is opened before any other file. A log that can't be opened, or that is a file the command reads or writes, such as the `-o` output or the database, is an error, and nothing is written. If writing the log fails part way, the command finishes but exits with status 1.

#### Compact a database's games and names

`Database::compact_games` removes logically deleted games and the records that no game uses, such as those that `update_game` replaced, as SCID's Compact games command does. The remaining games are renumbered from 0 in their original order, and their records and index entries are copied as stored. The index header is kept, the name frequencies are counted again, and SCID's `.stc` tree cache is deleted.

`Database::compact_names` removes the names that no game uses, as SCID's Compact names command does, and gives the other names new IDs. As in SCID, a name that only deleted games use is kept, so run `compact_games` first. If every name is used, no file changes.

Both write any unwritten changes first. They write the new files in a temporary directory beside the database, and replace the database's files only when the new files are complete. If a method fails, the database's files are left as they were. A game whose record can't be read stops `compact_games`, and a game that refers to a name the name file lacks stops `compact_names`, with the new `Error::NameNotFound`.

#### Replace, delete, and undelete games in a database

`Database::update_game` replaces a game with another and keeps its number. As in SCID, the new record is added to the end of the `.sg4` file and the old one stays there unused, the game keeps its user-settable flags, including the delete flag, and the names it no longer uses lose 1 from their frequency but stay in the name file.

`Database::delete_game` sets a game's logical delete flag, and `Database::undelete_game` clears it. The game's record, its number, and the name frequencies don't change. Like adding a game, these changes are written by `flush` or `close`.

`Database::add_game` no longer changes name frequencies when it fails.

#### Find twin games and mark them deleted, as SCID does

`Database::delete_twins` finds twin games, such as the same game imported twice, and marks the copies SCID vs. PC's Delete Twin Games command (`sc_base duplicates`) would delete as logically deleted. It returns their numbers. `compact_games` then removes them.

`TwinOptions` selects what twins must have in common: the same players, colors, event, site, round, year, month, day, result, basic ECO code, and main-line moves up to 120 half-moves, with SCID's home pawn check first. It also selects whether to skip games shorter than 10 half-moves, which twin to delete (`DeleteTwin::Shorter`, `Older`, or `Newer`), whether games with comments or variations are kept, and whether ratings are copied to the kept game. Its defaults are those of SCID's Delete Twin Games window, and its documentation lists where the command's defaults differ. As in SCID, a game of at most 2 half-moves is the twin of a full game with the same year, site, and round, and pairs are compared in SCID's order, so the same games are deleted. Unlike the window, `delete_twins` doesn't undelete every game first.

Only delete flags change, and ratings if they are copied. If a game's record can't be read, nothing changes and the error is returned. SCID's results for games without an ECO code are undefined when ECO codes must match; here such games match.

#### Merge SCID databases

`scidtopgn merge db1 db2 -o merged` copies every game of the source databases, in order, into a new database, as SCID's `scmerge` does. Each game's record and index entry are copied as stored, without converting the game to PGN and back, so logically deleted games are copied and stay deleted. The new database holds each name its games use, stored as UTF-8, with frequencies that count its games, and has a new index header. The sources are opened read-only. `merge` refuses an output database that exists, unless `--overwrite` is given, and one that is also a source. It writes the new database beside the output path and moves it into place only once it is complete, so after an error no new database is left and an existing one is unchanged. It accepts `--verbose`, `--quiet`, and `--log`.

In the library, `Database::copy_game` copies a game from another database as stored.

When `import --overwrite` or `merge --overwrite` can't move one of the new database's files into place, the old database's files, and with `merge` its tree cache, are now put back. Before, the old database could be left with some of its files replaced. A directory at one of the output database's paths is now an error before any file is moved.

#### Read and write databases whose names aren't UTF-8

Older versions of SCID, and the C++ reference converter, store the names of a Latin-1 or Windows-1252 PGN file as they are. Export, append, and `info` failed for these databases, because their name files hold names that aren't valid UTF-8.

- Opening a database reads the bytes of such a name that aren't valid UTF-8 as Windows-1252, as `PgnParser::from_bytes` reads PGN, and the new `Database::converted_names` lists the names it converted. The CLI warns about each one.
- **Writing a database stores the converted names as UTF-8**, as SCID vs. PC stores the names of PGN it imports. If two names of a kind read the same, such as a name stored once as Latin-1 and once as UTF-8, `Database::flush`, `close`, `compact_games`, and `compact_names` merge them into one name, whose frequency is the sum of theirs, and give the names new IDs, because SCID can't open a name file that holds a name twice. `compact_names` writes converted names as UTF-8 even when it drops and merges none, and keeps the IDs. A merging `flush` writes the name and index files in a temporary directory and replaces both once they are complete, and returns `Error::Game` with `Error::NameNotFound` for a game that refers to a name the name file lacks.
- A name longer than 255 bytes once converted is cut to 255, as a name added to a database is, and spaces the cut leaves at the end are removed.
- A front-coded name now shares its prefix with the previous name as stored, rather than as read with control characters and surrounding spaces removed.
- `Error::NameNotUtf8` is removed, since no name fails to read for that reason.

#### Single-space PGN comments, escape tag values on export, and keep games with a result inside a variation

PGN import now treats four cases as [docs/PGN_IMPORT.md](https://github.com/intermezzo-chess/scidtopgn/blob/main/docs/PGN_IMPORT.md) describes:

- As in SCID, each run of whitespace in a comment, including line breaks, becomes one space, and whitespace at the ends is dropped.
- A result inside a variation is skipped with the new `PgnWarningKind::ResultInVariation` warning, and the game is imported. Before, the game was skipped with `Unterminated variation`. SCID ends the game at the result instead.
- A `}` or `]` outside a comment gives a `skipped unknown text` warning, as SCID also warns. Before, it was skipped without a warning.
- A `\` in a tag value before a character other than `"` or `\` is kept, as in SCID, so `C:\games` is no longer read as `C:games`.

PGN export writes `"` and `\` in tag values as `\"` and `\\`, as the PGN standard specifies, so `scidtopgn` reads its own export back unchanged. Before, a value with `"` gave PGN that couldn't be imported again. SCID imports the exported games, but reads `\\` as two backslashes and ends a value at `\"`.

#### Import and export games that start from a FEN

PGN import reads the `FEN` tag and plays the game from that position, and export writes `SetUp` and `FEN` tags and numbers the moves from the position, such as `40... Kd7`. A malformed FEN or a position that can't be played is an error. Invalid castling rights and en passant squares in a FEN are dropped. Games whose `Variant` tag names a variant other than standard chess, such as Chess960, are rejected.

#### Export a database that can't be written to

`scidtopgn` export and `--count` failed with `Permission denied` for a database whose `.sg4` game file couldn't be written, such as one marked read-only or on read-only media, though they only read it. They now open the database with the new `Database::open_read_only`, which needs no write permission on any of its files.

A database opened with `Database::open_read_only` can't be changed: `add_game`, `update_game`, `delete_game`, and `undelete_game` return `Error::ReadOnly` and change nothing, and `flush` and `close` write nothing. `Error::ReadOnly` now reads "Database is open read-only".

#### Import PGN files that aren't valid UTF-8

`scidtopgn import` now reads a Latin-1 or Windows-1252 PGN file instead of failing for the whole file with `stream did not contain valid UTF-8`. Each byte that isn't valid UTF-8 is read as Windows-1252 and stored as UTF-8, and each game with such bytes gets the warning `read text that isn't UTF-8 as Windows-1252`. Valid UTF-8 is read as before. In the library, `PgnParser::from_bytes` reads such input, and `PgnWarningKind::NotUtf8` is the new warning.

#### Report PGN games that import can't store, and leave the database unchanged when aborting

`scidtopgn import` now always warns about a game it parsed but can't add to the database, such as one whose record is longer than 131,071 bytes. The warning names the game by its number and first line, as for a game that fails to parse, and the game is counted as skipped. Before, the warning appeared only with `--verbose` and didn't name the game.

With `--on-error abort`, the error now names the game, and the database is left as it was. Before, the game file kept the records of games added before the failure, and `--overwrite` had already replaced the existing database. A new database, with or without `--overwrite`, is now written beside its path and moved into place when the import succeeds.

In the library, `PgnParser::game_number` and `PgnParser::game_line` give the number and first line of the game the parser last read. `Database::discard` closes a database without writing its changes, and cuts the game file back so that it holds no records that its index doesn't point to.

#### Search games by their header, as SCID does, and export the matches

`Database::search_header` returns the games that match a `HeaderSearch`, counting from 0, as SCID vs. PC's header search (`sc_search header`) selects them. It reads only the index and the names, never a game's record. `HeaderSearch` has SCID's criteria: White, Black, event, site, and round names, matched as text without case or spaces, or as a pattern in double quotes; a range of dates, results, White's and Black's ratings, and their difference, where an unrated player counts with their highest rating in the database; a range of ECO codes whose end includes its extensions, and whether games without one match; the number of half-moves; the side to move at the end; game numbers counting from 1, or from the end if negative; ignoring colors; and, in `HeaderFlags`, whether each of the 25 index flags must be set, as a `FlagMatch`. Its defaults are those of SCID's search window, which matches every game of at most 999 half-moves. The new `Eco` type parses and formats ECO codes such as `B20` and `B20a1`.

`scidtopgn export`, with or without the subcommand, exports only the games that match the new options `--white`, `--black`, `--event`, `--site`, `--round`, `--date`, `--result`, `--white-elo`, `--black-elo`, `--elo-diff`, `--eco`, `--eco-missing`, `--length`, `--to-move`, `--ignore-colors`, and `--flag`. Ranges are written `FROM..TO`, with either end optional. Combined with `--games`, a game must match both, and deleted games are still skipped. With search options, `--count` prints the number of games that match, including deleted ones, as SCID counts them.

#### Report every damaged game in a SCID database

`scidtopgn validate mydb` reads every stored game, including logically deleted games, and reports each one it can't read as a warning, such as `SCID game 4 (deleted): Game data at byte 710, 166 bytes long, is past the end of the game file`. A final line gives the number of games checked and the number damaged. It writes the report to stderr and to the log given with `--log`, and doesn't change the database. It exits with 0 if no game is damaged, 1 if any is, and 2 if the check couldn't finish, for example because the database couldn't be opened.

### Fixes

#### Keep NAGs and comments on the move they follow, and export variations

PGN import and SCID database reading now attach each NAG and comment to the move it follows, as SCID does, in the main line and in variations. Previously they attached to the move before, moved one move earlier each time a database was read back, and were dropped inside variations. A comment before the first move of the game or of a variation is kept, and several comments after one move are joined. A NAG before the first move is dropped, as in SCID. PGN export now writes variations.

#### State what counts as a breaking change from 1.0 on

`docs/RELEASING.md` says what the semantic versioning promise covers from 1.0.0 on, including the minimum supported Rust version and the shakmaty dependency. The library documentation states that every public type is `Send` and `Sync`.

#### Delete SCID's tree cache when a game is replaced or a database is overwritten

SCID uses a cached tree as long as the database has the same number of games, so a replaced game could show the old game's tree statistics. Writing the changes of `update_game` now deletes the database's `.stc` file, as SCID does when it saves a game, just before the new name and index files replace the old ones. A cache that exists but can't be deleted is reported as an error, and the changes stay unwritten so that `flush` can try again.

`Database::create` and `import --overwrite` now delete the `.stc` file of the database they replace too, because its new games could reach the old count. Because of this, `import` refuses a `--log` path that is the output's `.stc` file, as `merge` does.

Adding, deleting, and undeleting games, deleting twins, and compacting names still keep the cache.

#### Refuse a game that would start past 4 GiB in the game file

An index entry stores where its game's record starts in the game file in 32 bits. Adding a game whose record would start past that, about 4 GiB into the game file, now fails with the new `Error::GameFileFull`. Before, the offset wrapped around, and the index pointed at the wrong record.

#### Set the index fields SCID searches by, and keep half-move counts above 255

`Database::add_game` and `Database::update_game` now set the index fields SCID derives from a game's moves, as SCID does: the half-move count, the home pawn data, the final material signature, the promotion and underpromotion flags, and the variation, comment, and NAG counts. The promotion flags are also stored in the game record. Earlier versions left these fields at 0, so SCID's board search, tree, and novelty search couldn't find the games past the starting position.

`IndexEntry` now keeps half-move counts up to 1,023, as SCID stores them. Earlier versions lost everything above 255 when reading, setting, or writing a count, so any change to a database SCID wrote cut the counts of its games longer than 255 half-moves.

A database written by an earlier version keeps the empty fields until its games are replaced with `update_game` or imported again.

#### Keep at most 7 NAGs on a move, as SCID does

SCID keeps a move's first 7 NAGs and drops the rest. PGN import kept 8, so SCID dropped the 8th when it read the stored game. Import now keeps 7 and warns about each NAG it drops, with the new `PgnWarningKind::TooManyNags`. Reading a SCID database also keeps the first 7, without a warning, as SCID does.

A move whose `nags` are set directly to more than 7 NAGs, or to NAG 0, is now stored and exported with only the first 7 that aren't 0, and the index's NAG count counts only those, so SCID reads back each NAG stored.

#### Reject games with variations nested too deeply or too many variations of one move

A game whose variations are nested more than 100 levels deep, or that has more than 255 variations of one move, is now rejected with an error, whether it is read from PGN or from a SCID database. Deeper nesting overflowed the stack and aborted the whole process, even with `--on-error skip`. More than 255 variations of one move panicked in debug builds, and in release builds lost the game's comments when it was read back.

#### Keep games and databases readable when tags or names are too long to store

SCID stores the lengths of tag names, tag values, and player, event, site, and round names in one byte. Longer text used to make data that couldn't be read back: a game whose tag value was longer than 255 bytes was lost, and a name cut in the middle of a character, or two names that differ only after byte 255, made the database fail to open in `scidtopgn` or in SCID.

- PGN import drops a tag whose name or value is longer than 255 bytes, with a warning, and imports the rest of the game, as SCID does. This includes `White`, `Event`, and the other tags with their own fields, which then stay `?`.
- `Database::add_game` and `update_game` return the new `Error::TagTooLong` for a tag value longer than 255 bytes, and change nothing.
- A tag name longer than 240 bytes is cut to 240 when the game is stored, as in SCID. It used to be dropped.
- A player, event, site, or round name longer than 255 bytes is cut to 255 before it is looked up, as SCID does, so names that start with the same 255 bytes share one name.
- A cut ends before a character it would split.

#### Open a database created without games

A database created without adding a game, such as by `scidtopgn import` of a PGN file with no game it could read, had no `.sn4` name file and couldn't be opened. `Database::create` now writes all three files at once, so the database opens with 0 games. Replacing an existing database, as `import --overwrite` does, now replaces its name file too; previously, if no game was added, `scidtopgn info` still reported the old database's names.

#### Read back games from a new database before closing it

`Database::create` opened the new `.sg4` game file for writing only. Once a 128 KiB block of game data had been written to the file, `get_game` for a game in that block failed with an I/O error, and so did `add_game` after a read of an earlier block. The file is now opened for reading and writing, as SCID does, so every game reads back before the database is closed.

#### Read games whose SCID records are longer than 64 KiB

Reading a game whose record is between 65,536 and 131,071 bytes from a SCID database, such as a game with long comments, read only the first `length % 65536` bytes. Usually its comments were cut short without an error; otherwise reading it failed. This affected databases written by SCID too. A record that crosses a 128 KiB block boundary of the `.sg4` file is now read, as SCID reads it, instead of failing.

#### Read games whose tags include a spare common-tag code

Reading the tags of a game record with a common-tag code from 251 to 254 misread the tags after it, so `Database::get_game` failed or misread the game. SCID never writes these spare codes, but, like SCID, reading a game now skips the value that follows one. The value is dropped, because SCID has no tag name for it.

#### Reject index files that aren't SCID format version 400, as SCID vs. PC does

Opening a database whose index file is of a SCID format version other than 400 now fails with the new `Error::UnsupportedVersion`, which holds the version found, instead of reading the file as version 400. `Database::open` and `Database::open_read_only` return it wrapped in `Error::File` for `DatabaseFile::Index`. No file is changed.

The CLI's `export`, `info`, `validate`, `merge`, and `import` (appending to an existing database) report the database, the index file, the version found, and that only SCID vs. PC 4 databases (version 400) are supported.

#### Report PGN games that fail to import instead of dropping the rest

`scidtopgn import` no longer stops at the first game it can't parse. With `--on-error skip`, the default, it warns with the game's number, line, and reason, counts the game as skipped, and imports the games after it. With `--on-error abort`, it exits with an error that names the game, before changing the database.

In the library, `PgnParser` is an iterator over each game or its error, and resumes at the next game after an error. `parse_pgn` returns the first failed game's error instead of returning the games before it.

#### Document what SCID compatibility means

`docs/SCID_COMPATIBILITY.md` states what scidtopgn is compatible with: SCID vs. PC 4 databases, tested with SCID vs. PC 4.26. It says what each write keeps and changes, where scidtopgn deliberately differs from SCID and why, and which tests or compatibility checks back each statement. The README's opening describes the same scope.

#### Encode record tags as SCID does, so game files are the same size

`Database::add_game` and `Database::update_game` now write a game record's tags as SCID does. SCID's ten common tags, such as `Annotator`, `Opening`, and `SetUp`, are stored as one-byte codes instead of by name, and the event date is stored only in the index entry instead of also in the record. A database imported from PGN now has the same game records as one SCID vs. PC's `pgnscid` writes from the same PGN, and its game file is about 8% smaller than before.

As in SCID, an event date the index can't hold is dropped: one with an unknown year, one more than 3 years from the game's year, or any event date when the game's year is unknown. Earlier versions kept it in the record. Records written by earlier versions still read the same.

#### Set stored-line codes as SCID does

`Database::add_game` and `Database::update_game` now set each game's stored-line code as SCID vs. PC 4.26 does: the number of the longest of SCID's 254 stored opening lines that the game's main line starts with, or 0 if none matches or the game starts from a FEN. SCID vs. PC shows the line as the game list's opening moves, and its tree search uses the code to skip games without decoding them. Earlier versions stored 0, so the opening moves were blank and tree searches were slower. With this change, importing the PGN test files and two Caissabase chunks gives files byte-identical to those SCID vs. PC's `pgnscid` writes from the same PGN.

A database written by an earlier version keeps a code of 0 until its games are replaced with `update_game` or imported again.

#### Read and write variations and null moves like SCID

Games with variations no longer fail to load from a SCID database, and each variation starts from the position before the move it replaces. A move with more than one variation is written the way SCID writes it. Null moves (`--`, `Z0`, or `null` in PGN) are imported and exported as SCID null moves. A null move while in check is an error.

#### Write index and name changes safely, so a failed write can't damage the database

`Database::flush` and `close`, and so appending with `scidtopgn import`, used to truncate the database's `.si4` and `.sn4` files and write them again, so a failure part way left a database that couldn't be opened. They now write new files in a temporary directory beside the database, and replace the originals only once both are complete. After a failed `flush`, the `Database` keeps its changes, so calling `flush` again writes them.

Writing changes now needs a writable directory and disk space for the new files alongside the old ones. It also fails if the `.si4` or `.sn4` file is read-only or a symbolic link, instead of replacing it. `scidtopgn import` names the database when writing it fails.

#### Write names with non-ASCII bytes in the order SCID requires

The name file now lists names in SCID's order: by first byte, then by bytes compared as signed, as SCID does on x86 and Apple arm64. Earlier versions sorted names with a byte of 0x80 or above, such as accented names, in a different order. SCID then couldn't open the database ("Error opening name file"), even after only deleting a game in a database SCID wrote.

To repair a database written by an earlier version, make any change to it with this version, then `close` it, which rewrites the name file.

## 0.1.0 (not published)

0.1.0 was prepared but never published. 0.2.0 is the first release on crates.io and GitHub.

### Features

- Initial release.
- The `scidtopgn` library reads and writes SCID databases (`.si4`, `.sn4`, and `.sg4`) and parses PGN.
- The `scidtopgn` command exports a SCID database to PGN on stdout or to a file, with `--count` and `--overwrite`.
- `scidtopgn --version` prints the version.
- `scidtopgn import` imports PGN from a file or stdin into a new or existing SCID database. Invalid games are skipped with a warning, or abort the import with `--on-error=abort`.
- Export skips logically deleted games, matching the SCID reference converter.
- Locked builds (`cargo install scidtopgn-cli --locked`) support Rust 1.70 and later.
- Both crates are licensed under GPL-3.0-or-later.
