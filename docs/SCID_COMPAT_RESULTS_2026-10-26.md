# SCID compatibility of databases written by scidtopgn

Tested 2026-10-26 on `main` at `73e532e`, macOS 26 (arm64). No project code changed. Everything here lives in the git-ignored `.docs/scid-compat/`.

## Summary

Databases that scidtopgn creates or changes stay readable by SCID's game decoder. Every game reads back with the same PGN as SCID's own converter gives, and delete, undelete, and update behave as SCID expects. Two new problems make the databases not fully SCID-compatible:

1. **Games written by scidtopgn are invisible to SCID's board search and tree.** scidtopgn leaves the search fields of each index entry at 0. SCID's own search code finds 0 of 10,000 such games at any position after the start.
2. **SCID can't open a database with non-ASCII names after scidtopgn writes it.** scidtopgn sorts names in a different order than SCID requires. This happens even when scidtopgn only deletes a game in a database that SCID created.

The test also confirms the known bug #55. It happens on delete and update as well as on append.

## How it was tested

| Tool | Source | Role |
| --- | --- | --- |
| `scid-reference` | `scid-cpp` (built as in #44) | SCID → PGN reader. A database passes if this exits 0, gives no warnings, and exports the expected games |
| `scid-dump` | new, built from `scid-cpp/scid` sources | Prints every index entry and name, then checks the database with SCID's code: each record decodes; SCID's `Game::Encode` gives the same record bytes and derived index fields; each name's count equals its uses; `FindExactName` finds each name |
| `pgnscid` | SCID vs. PC 4.26 (`scidvspc/src`) | Writes the reference database for each scenario |
| `tcscid` | SCID vs. PC 4.26, Tcl only, no Tk | Opens databases and runs SCID's board search (`sc_search board`) and tree search (`sc_tree search`) |
| `dbtool` | new Rust crate using `crates/scidtopgn` | Runs `delete_game`, `undelete_game`, and `update_game`, which the CLI doesn't expose |

`scid-dump` was checked first against SCID-written databases. Every `pgnscid` database passed every check.

Each scenario compared:
- the PGN that scid-cpp reads from scidtopgn's database with the PGN it reads from `pgnscid`'s database for the same final PGN;
- the index entries;
- the names and their counts.

Test data:
- the `tests/data` fixtures;
- Caissabase 2022-12-24 chunks of 10,000 games each: 1 (ASCII), 2, 106 (UTF-8 only in team tags), and 109 (UTF-8 player names).

## Results

"Fields zeroed" means finding 1.

| # | Scenario | Result |
| --- | --- | --- |
| 1 | Import each fixture into a new database | Readable. PGN and name file identical to `pgnscid`'s. Fields zeroed |
| 2 | Import Caissabase chunk 1 (ASCII) | Readable, 10,000/10,000. PGN and name file identical. Fields zeroed. SCID board search and tree find 0 games after 1.e4 (SCID-written: 4,705) |
| 2u | Import chunk 106 (UTF-8 only outside names) | Same as 2 |
| 2n | Import chunk 109 (UTF-8 player names) | **Fails.** SCID can't open it: `Error opening name file.` |
| 3 | Import a PGN with no games | Byte-identical to `pgnscid`'s empty database |
| 4 | Append to a scidtopgn database | Readable. Matches `pgnscid` except the zeroed fields |
| 5a | Append to a SCID-written database (`five`) | Readable. The original entries are unchanged |
| 5b | Append chunk 2 to a SCID-written chunk 1 | Readable, 20,000/20,000. 13 original entries lose half-move counts above 255 (#55). The appended games are invisible to SCID's searches: 4,705 found after 1.e4, against 9,318 when SCID writes both chunks |
| 6 | `import --overwrite` over an existing database | Matches `pgnscid` for the new PGN. Fields zeroed |
| 7 | Delete the first, a middle, and the last game (scidtopgn and SCID databases, `five` and Caissabase) | SCID skips exactly the deleted games. No other entry changes. Name and game files unchanged. On SCID-written Caissabase: the 13 counts above 255 are cut (#55). With non-ASCII names: **fails**, see finding 2 |
| 8 | Undelete those games | Index byte-identical to the original, except entries already cut by #55 |
| 9 | Update a game: same players, new moves | Matches `pgnscid` on the spliced PGN |
| 10 | Update a game: new player, event, site, and round names | Matches. Unused old names stay with count 0 until compaction (#72), as in SCID |
| 11 | Update with a 72 KB game | Matches |
| 12 | Update in SCID-written databases (`five`, Caissabase) | Matches, apart from the zeroed fields of the new game. Caissabase also loses its 13 counts above 255 (#55) |
| 13 | Update the same game twice | Matches. Each update adds a record to `.sg4`, as in SCID |
| 14 | SCID-written Caissabase: delete 2, update 1, append 5, reopen | PGN exactly as expected. Name counts consistent. The 6 new records have zeroed fields, and 13 counts are cut (#55) |
| 15 | Open and close without changes (6 databases) | All three files byte-identical |

## Findings

### 1. New: scidtopgn doesn't compute the index fields SCID uses for searches

`Database::add_game` and `update_game` never set these fields. The setters exist in `index.rs`, but `database.rs` doesn't call them. SCID's `Game::Encode` sets all of them:

| Field | scidtopgn | SCID, Caissabase chunk 1 |
| --- | --- | --- |
| Half-move count (#55) | 0 | 9,943 of 10,000 games nonzero |
| Home-pawn data | all 0 | 9,941 nonzero |
| Final material signature | 0 | 9,912 nonzero |
| Promotion and underpromotion flags (index and record) | 0 | 955 and 24 set |
| Variation, comment, and NAG counts | 0 | set in the annotated fixtures |
| Stored-line code | 0 | e.g. 156 |

**Effect**, confirmed by running SCID vs. PC's own search code in `tcscid`:

| Position | SCID-written chunk 1: board / tree | scidtopgn-written chunk 1: board / tree |
| --- | --- | --- |
| Start | 10,000 / 10,000 | 10,000 / 10,000 |
| 1.e4 | 4,705 / 4,705 | 0 / 0 |
| 1.e4 e5 2.Nf3 | 929 / 929 | 0 / 0 |
| 1.d4 d5 2.c4 | 569 / 569 | 0 / 0 |
| Open Sicilian, 5.Nc3 | 362 / 362 | 0 / 0 |

Why:
- `sc_tree_search` (`tkscid.cpp:15250`) skips any game with 0 half-moves for every position except the start.
- `sc_tree_search`, `sc_search_board`, and `sc_game_novelty` skip any game whose home-pawn list can't reach the target position. An all-zero list reaches nothing past the start.
- The material signature, the promotion flags, and the comment, variation, and NAG counts feed material search, endgame classification, and the game list's markers. Their effects were found in SCID's code but not tested, because the UI was out of scope.

SCID's compaction copies each record without re-encoding it (`sc_compact_games`), so it doesn't repair these fields.

### 2. New: name files with non-ASCII names are written in an order SCID rejects

- **Cause:** SCID's `NameBase::ReadNameFile` adds each name with `StrTree::AddLast`. That returns `ERROR_Corrupt` unless names are in strictly increasing `strCompare` order. `strCompare` compares C `char`s, which are signed on x86 and Apple arm64, so a byte of 0x80 or above sorts before every ASCII byte. scidtopgn sorts by Rust `String` order, which compares unsigned bytes (`namebase.rs:255`). Example from chunk 109: SCID writes `Blübaum, Matthias` before `Blagojević, Dragisa`; scidtopgn writes it after.
- **Effect:** both scid-cpp ("cannot read name file") and SCID vs. PC (`sc_base open`: "Error opening name file.") reject the file. Every operation that writes the name file triggers it, including import, append, update, and delete, because `delete_game` also rewrites `.sn4`. A user's own SCID database with one accented player name becomes unopenable in SCID after scidtopgn deletes one game. scidtopgn itself still reads it.
- **Order is the only problem:** `snorder.py` shows SCID's file is in strictly increasing signed order and scidtopgn's in unsigned order; only the 69 player names with non-ASCII bytes are out of place. `snresort.py` rewrote the broken file from the one-game delete in signed order, keeping IDs and counts. SCID vs. PC then opened it, and scid-cpp read exactly the expected 9,999 games.
- **Untested:** only player names were tested. Event, site, and round names go through the same code.
- **Aside:** where `char` is unsigned (for example Linux arm64 builds of SCID), SCID's order would match scidtopgn's. Signed order is what SCID uses on x86 Windows, Linux, and macOS.

### 3. Known, #55: any write cuts existing half-move counts above 255

13 of 10,000 games in SCID-written chunk 1 lost their top 2 bits (282 → 26, 258 → 2) after a delete, an update, or an append. #55 describes only append. Delete and update trigger it too, because each rewrites the whole index.

### 4. Cosmetic: records encode tags differently

Moves, comments, and variations are byte-identical to SCID's encoding in all 10,000 games. The records differ only before the moves:
- scidtopgn stores `EventDate` as a tag in the record; SCID keeps it only in the index.
- scidtopgn writes common tags such as `Opening` and `Variation` as full names; SCID uses one-byte codes.
- The game-flags byte lacks the promotion bits (see finding 1).

As a result `.sg4` is about 9% larger (11 or 20 bytes per game in Caissabase). SCID reads both forms.

## Not covered

- The SCID UI: game list columns, markers, and opening the files in the GUI.
- User and custom flags set by SCID, then kept through an update.
- Windows-1252 PGN input, and non-ASCII event, site, and round names.
- Stored-line codes can't be recomputed with scid-cpp, whose `StoredLine` is a stub. They were compared with `pgnscid` instead.

## Rerunning

All paths are under `.docs/scid-compat/`:
- `lib.sh`: the check and compare helpers.
- `scid-dump.cpp`, `dbtool/`, `build-tcscid.sh`: the tools.
- `search.tcl`, `open.tcl`: the SCID searches.
- `recdiff.py`, `splice.py`, `dropgames.py`: comparison scripts.

Build scid-cpp tools with the `c++` command from #44. For `pgnscid`, use a copy of `stored.cpp` with `tkscid.h` replaced by `timer.h` (in `shim/`), plus `universalchardet/*.cpp` and `-framework Tcl`. Build the Rust crate with `--offline` and the repo's `Cargo.lock`. Scenario outputs are in `s1/`–`s15/`, `s2u/`, and `ref/`.
