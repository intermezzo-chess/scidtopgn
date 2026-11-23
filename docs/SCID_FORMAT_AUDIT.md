# Audit of the SCID database format reference

This records the audit of `SCID_DATABASE_FORMAT.md` for [#145](https://github.com/intermezzo-chess/scidtopgn/issues/145), made in November 2026. It covers every section, table, and code block or diagram of [the document at `8c49478`](https://github.com/intermezzo-chess/scidtopgn/blob/8c49478/SCID_DATABASE_FORMAT.md). [SCID_FORMAT.md](SCID_FORMAT.md) and the documents it links to replace it.

- [Summary](#summary)
- [Verification](#verification)
- [Remaining uncertainties](#remaining-uncertainties)
- [Suspected scidtopgn defects](#suspected-scidtopgn-defects)
- [Inventory](#inventory)

## Summary

The old document has 184 items: 86 confirmed, 61 corrected, 26 historical, 11 qualified. None is unresolved; [Remaining uncertainties](#remaining-uncertainties) lists what the new documents leave open.

The errors that would have made a reader wrong:

| Old | Correct |
| --- | --- |
| A section with fewer than 256 names has 1-byte IDs (in code) | 2 bytes below 65,536 names, then 3 |
| A name record has ID, frequency, length, and data | A prefix length follows the length, except in each section's first record |
| Front coding subtracts and slices without checks | The prefix is at most the length and the previous name's length |
| Counts of variations, comments, and NAGs are numbers | Codes: 11 means 13 to 17, 15 means 45 or more |
| Bytes 33–36 are the material signature | The top byte is the stored-line code |
| Flags: 5 bits, "TBD" | 16 bits and 6 custom flags |
| Ratings to 4,095; type "FIDE" | 4,000; 7 types, none FIDE |
| 2022.12.19 is 1,036,691 | 1,035,667 |
| A date with unknown parts is invalid (in code) | 0 is unknown |
| SCID reads a record that crosses a block | It reports it as corrupt |
| A record has custom flags; a move has 1–3 bytes; a comment's text follows its marker; 15 ends the record | Three flag bits; 1 or 2 bytes; comments follow the moves, after 15 |
| A variation follows a later move than the one it replaces | It follows the move it replaces |
| `6f` is any pawn's double step | Number 6 is a knight. 1. e4 is `cf` |
| En passant is found by checking legality | A pawn capture onto an empty square |
| When the king comes in a FEN, the other pieces move along | Only the piece with number 0 moves, to the king's number |
| A description is cut at trailing NULs | It ends at its first NUL; SCID leaves other data after that |
| Base type: flags. Auto-load: the game's number. Timestamp: modification time | A number. The game's number plus 1. Unused |
| Black in the test database is "Cheparinov, I" | "Murshed, N" |
| `game.cpp` lines 59111–59348, `gfile.h` 250–270, `sc_game.cpp` | `game.cpp` has 5,148 lines, `gfile.h` 88; `sc_game.cpp` doesn't exist |

Removed as historical: progress reports, success rates, APIs and paths that no longer exist, advice on memory and I/O, and opening names. scidtopgn's own rules are in [COMPATIBILITY.md](COMPATIBILITY.md). The statements #143 corrected are kept.

## Verification

### Source

SourceForge's revision 3570 was compared with `scid_vs_pc-4.26.tgz`: all 84 `.cpp` and `.h` files in `src` (not its subdirectories) are byte-identical. Revisions 3580, 3585, 3590, 3600, and 3790 aren't. `scid-cpp/scid`, labeled revision 3585, differs from 4.26 in `bytebuf.cpp`, `game.cpp`, `game.h`, `myassert.cpp`, `naglatex.h`, `pgnparse.cpp`, `pgnparse.h`, and `stored.h`. None of those differences changes how the files are encoded, and this audit doesn't rely on that copy.

Each C++ link in the format documents was checked to name something on its line in 4.26, and each Rust link a function, constant, or type in its file.

### Probes

`format_probe.cpp` uses SCID 4.26's code, and `format_probe.py` only the rules in the format documents. From the repository root, after `compat/run` or `compat/build-scidvspc` has built SCID, on macOS:

```sh
V=$PWD/target/compat/scidvspc
objs=$(ls $V/obj/*.o | grep -v -E '/(cql_.*|tcscid|pgnscid|tree|filter|pbook|crosstab|spellchk|probe|optable|engine|recog)\.o')
c++ -std=c++11 -O1 -w -fsigned-char -I$V/scid_vs_pc-4.26/src -I$V/scid_vs_pc-4.26/src/cql \
    scid-cpp/tools/format_probe.cpp $objs -framework Tcl -lz -o format_probe

mkdir -p probes/pgn probes/db probes/damage
python3 -I scid-cpp/tools/format_probe_inputs.py probes/pgn
cp tests/data/*.pgn probes/pgn/
cp compat/data/names.pgn probes/pgn/compatnames.pgn
: > probes/pgn/empty.pgn
(cd probes/db && for f in ../pgn/*.pgn; do $V/bin/pgnscid -f $f $(basename $f .pgn); done)
python3 -I scid-cpp/tools/format_probe_damage.py tests/data probes/damage

./format_probe probes/db/counts > a.txt
python3 -I scid-cpp/tools/format_probe.py probes/db/counts > b.txt
cmp a.txt b.txt
```

`meta` is `five` after `compat/tcl/meta.tcl set`. `flags` is `counts` with user flag *n* set on game *n*:

```tcl
set base [sc_base open [lindex $argv 0]]
sc_base switch $base
set game 0
foreach flag {D W B M E N P T K Q ! ? U 1 2 3 4 5 6} { sc_game flag $flag [incr game] 1 }
sc_base close $base
```

It was run on macOS on Apple silicon.

### Results: databases SCID wrote

`fixtures` is the 13 databases in `tests/data`. On all of them the two programs print the same.

Of the 37 databases `pgnscid` and `tcscid` 4.26 wrote, they print the same for 33, which have 198,520 games:

| Database | What it checks | Games |
| --- | --- | --- |
| `names255`, `names256`, `names65535`, `names65536` | That many events, and that frequency for two players, a site, and a round | 131,582 |
| `names66000` | IDs above 65,535 of every kind of name | 66,000 |
| `namecases` | 255 bytes, prefixes that end inside a character, order, control characters | 6 |
| `dates` | Unknown parts, year 2047 and 2048, event dates −5 to +5 years, game years 1 to 3 | 22 |
| `ratings` | Rating types, 3,999 to 4,096, ECO codes | 23 |
| `counts` | 0 to 55 variations, comments, and NAGs | 56 |
| `moves` | 0, 1, 255, 256, 1,023, 1,024, 1,025, and 2,049 half-moves | 8 |
| `comments`, `tags` | Places for comments and NAGs; tags to 254 bytes | 12 |
| `blocks`, `size131071` | Records that fill blocks, and the longest | 10 |
| `random` | Random legal games from the standard position (400) and 5 FENs (200), with variations, NAGs, and comments | 600 |
| `flags`, `meta`, `empty` | User and custom flags, header fields, no games | 61 |
| `compatnames`, and the PGN of the fixtures apart from `notation` | | 140 |

`format_probe.py` also derives each entry's start and promotion flags, counts, material, half-moves, and home pawns from its game, and they match in every one.

The other four differ, as SCID itself misreads or can't write them:

| Database | Result |
| --- | --- |
| `notation` | Game 41 castles without a rook, which SCID writes as a [corrupt game](COMPATIBILITY.md#castling). The 40 before it match |
| `tag255` | SCID overruns its buffer when it reads a tag value of 255 bytes, and loses the tag's name |
| `size131072` | SCID writes a record of 131,072 bytes with length 0, and can't read it |
| `size131073` | `pgnscid` stops with "Fatal error encoding game!" |

In `blocks.sg4`, the padding after the records of blocks 0 to 3 is all zeros.

### Results: damaged files

SCID's error numbers are from `error.h`: 109 and 110 are versions, 152 is corrupt, 402 and 406 are decoding errors, and 602 is reading past the end. "Reads" means that SCID decodes the game.

| Database | What is wrong | SCID 4.26 | scidtopgn `validate` |
| --- | --- | --- | --- |
| `v300`, `v399` | Version | Refuses (109) | Refuses |
| `v401` | Version | Refuses (110) | Refuses |
| `trailing` | 3 bytes after the last entry | Reads | No damage |
| `elo4095` | Rating | Reads it as 4,000 | No damage |
| `badname` | Name ID 9 of 2 | `tcscid` refuses to open it | No damage |
| `order`, `duplicate` | Two names | Refuses (152) | No damage |
| `signed` | Signed order | Reads. Built with unsigned `char`: refuses (152) | No damage |
| `unsigned` | Unsigned order | Refuses (152). Built with unsigned `char`: reads | No damage |
| `firstbyte` | Order of first bytes | Reads, with "namebase is denormalized" | No damage |
| `maxfreq` | Maximum frequency 256, frequencies 1 in 2 bytes | Reads | No damage |
| `cross` | Record crosses a block | Game corrupt (152) | No damage |
| `truncated` | Record past the end of the file | Game corrupt (602) | Damaged |
| `zero` | Length 0 | Game corrupt (152) | Damaged |
| `compactdate` | Event date with code 255 | Reads 2021.3.7 | No damage |
| `spare` | Tag code 251 | Crashes | No damage |
| `flags8` | Unused flag bits | Reads | No damage |
| `e4`, `d4` | Nothing: 1. e4 and 1. d4 | Reads | No damage |
| `knight15`, `queen` | Invalid value, invalid second byte | Error 406 | Damaged |
| `illegal` | Rook to its own square | Reads | Damaged |
| `nag0` | NAG 0 | Reads without it | No damage |
| `nag9` | 9 NAGs | Reads the first 7 | No damage |
| `twocomments` | Two markers, one text | Reads one comment | No damage |
| `commentafter` | Marker after a variation | Reads | No damage |
| `nocomment` | Marker without text | Error 602 | Damaged |
| `varfirst` | Variation before a move | Error 402 | Damaged |
| `emptyvar` | Variation without moves | Reads | No damage |
| `endvar` | Variation end in the main line | Error 406 | Damaged |
| `noend` | No end | Error 602 | Damaged |
| `after` | Bytes after the comments | Reads | No damage |

For the builds with unsigned `char`, the 17 sources the probe needs were compiled with `-funsigned-char`. Such a build refuses `tests/data/accents` too.

### Fixtures

`CONTRIBUTING.md` says that `pgnscid` was built from the untracked `scidvspc/`, and `SCID_COMPATIBILITY.md` said that every fixture uses 4.26. What `pgnscid` 4.26 writes from each fixture's PGN was compared with the fixture:

| Fixtures | Writer | Compared with 4.26's files |
| --- | --- | --- |
| `accents`, `derived`, `long`, `stored`, `twins` | `pgnscid` | All three files byte-identical |
| `search` | `pgnscid`, then `search.tcl` | `.sn4` and `.sg4` byte-identical. `.si4` differs in the flags `search.tcl` sets |
| `one`, `five` | Not recorded | `.sn4` and `.sg4` byte-identical. `.si4` differs in the base type and, in `five`, in bytes after a text's NUL |
| `annotated`, `castling`, `edge`, `latin1`, `mixed` | The `scid-cpp` reference converter | `.sg4` byte-identical |

So the conflict doesn't affect them as evidence. No rule rests only on `one` or `five`.

## Remaining uncertainties

- **Versions.** Only SCID vs. PC 4.26 was checked. Other releases, the original SCID, and other index versions weren't.
- **Source only.** These were read in SCID's source and not exercised: the most games, names, and tags; that SCID doesn't lower a maximum frequency and counts frequencies when it opens a database; that a NUL ends a name; lookups in a file with first bytes out of order; piece numbers above the count; more than 255 variations of a move.
- **Undefined behavior.** What SCID does after the buffer overrun of `tag255` and with `spare` depends on the build.
- **Base types** are numbers to the format. What SCID's interface calls them wasn't audited.
- **Stored lines.** The codes' values come from a table in SCID's source, which the probes print but don't derive. `tests/data/stored` has 12 cases.
- **Links** to SourceForge depend on that site. The tarball's SHA-256 identifies the source without it.

## Suspected scidtopgn defects

Found by the probes above, and not changed here. Issues for them need a maintainer's approval.

1. **`validate` reports no damage in databases that SCID refuses.** Reproduce with `scidtopgn validate` on the `badname`, `order`, `duplicate`, and `cross` databases.
2. **Import stores tag values of 255 bytes, which SCID 4.26 can't read safely.** Reproduce by importing `tag255.pgn`. SCID's own import stores them too.

`GFile::read_game`'s comment said that SCID reads a record that crosses a block. The comment is corrected; the behavior is unchanged.

## Inventory

**Lines** are those of the old document. Bold items are sections, and an item's outcome is for its own text. **Evidence:** "S" is the source linked from the new location, and names are the databases above; every rule is also checked by the `fixtures`.

- **Confirmed:** right, though it may be reworded.
- **Corrected:** wrong or incomplete, and corrected in the new location.
- **Qualified:** right only within limits that are now stated.
- **Historical:** not about the format as it is. Not kept, except where a new location is given.

| Lines | Item | Outcome | Notes | Evidence | New location |
| --- | --- | --- | --- | --- | --- |
| 1 | **SCID Database Format - Complete Technical Specification** | Corrected | Title and subtitle claim a "Complete" and "Definitive" specification "verified against SCID source code", without saying which version or how | S, all databases | [SCID_FORMAT](SCID_FORMAT.md#overview-scope-and-terminology) |
| 10 | **Table of Contents** | Corrected | Lists "Validation and Testing", which doesn't exist | S, all databases | [SCID_FORMAT](SCID_FORMAT.md#overview-scope-and-terminology) |
| 23 | **Overview and Architecture** | Qualified | "Proprietary", "data integrity": SCID is free software, and the files have no checksum | S, all databases | [SCID_FORMAT](SCID_FORMAT.md#overview-scope-and-terminology) |
| 27 | **Three-File Architecture** | Qualified | "Exactly three files": a `.stc` cache can exist too | S, all databases | [SCID_FORMAT](SCID_FORMAT.md#three-file-architecture-and-relationships) |
| 31–35 | Table | Confirmed | Extensions and sizes | S, all databases | [SCID_FORMAT](SCID_FORMAT.md#three-file-architecture-and-relationships) |
| 37 | **Why This Architecture?** | Qualified | Design reasons, not in the source. "Parallel access" and "modify components without rebuilding" aren't properties of the format; the files can't be written independently. The header-only search is kept | S, all databases | [SCID_FORMAT](SCID_FORMAT.md#three-file-architecture-and-relationships) |
| 47 | **Data Flow and Relationships** | Confirmed |  | S, all databases | [SCID_FORMAT](SCID_FORMAT.md#three-file-architecture-and-relationships) |
| 49–61 | text block | Corrected | The site and round IDs were missing | S, all databases | [SCID_FORMAT](SCID_FORMAT.md#three-file-architecture-and-relationships) |
| 65 | **Index File (.si4) - Complete Specification** | Confirmed |  | `fixtures` | [SI4_FORMAT](SI4_FORMAT.md) |
| 69 | **File Structure Overview** | Confirmed |  | `fixtures` | [SI4_FORMAT](SI4_FORMAT.md) |
| 71–83 | text block | Confirmed |  | `fixtures` | [SI4_FORMAT](SI4_FORMAT.md) |
| 85 | **SI4 Header Structure (182 bytes)** | Confirmed |  | S, `fixtures`, `meta` | [SI4_FORMAT](SI4_FORMAT.md#file-header-and-database-metadata) |
| 87–95 | Table | Corrected | Base type is a number for a kind, not flags. Auto-load's value is the game number plus 1. No text encoding is recorded, and custom flag names are 8 bytes and a NUL. Byte order: confirmed | S, `fixtures`, `meta` | [SI4_FORMAT](SI4_FORMAT.md#file-header-and-database-metadata) |
| 99 | **Example Header Parsing** | Corrected |  | S, `fixtures`, `meta` | [SI4_FORMAT](SI4_FORMAT.md#file-header-and-database-metadata) |
| 101–118 | rust block | Corrected | The description was cut only at trailing NULs, so `five.si4` gives unrelated bytes; it ends at the first NUL. The code also borrows a temporary, and doesn't compile. Replaced by a byte dump | S, `fixtures`, `meta` | [SI4_FORMAT](SI4_FORMAT.md#file-header-and-database-metadata) |
| 120 | **Game Index Entry Structure (47 bytes)** | Confirmed |  | S, `counts`, `ratings`, `flags`, `names66000`, `blocks` | [SI4_FORMAT](SI4_FORMAT.md#game-entries-and-packed-fields) |
| 124–144 | Table | Corrected | Byte 33 holds the stored-line code as well as the material. The rest is confirmed | S, `counts`, `ratings`, `flags`, `names66000`, `blocks` | [SI4_FORMAT](SI4_FORMAT.md#game-entries-and-packed-fields) |
| 146 | **Critical Field Specifications** | Confirmed |  | S, `counts`, `ratings`, `flags`, `names66000`, `blocks` | [SI4_FORMAT](SI4_FORMAT.md#game-entries-and-packed-fields) |
| 148 | **Game Data Length (17-bit value)** | Confirmed |  | S, `counts`, `ratings`, `flags`, `names66000`, `blocks` | [SI4_FORMAT](SI4_FORMAT.md#game-entries-and-packed-fields) |
| 152–157 | rust block | Confirmed | Added: custom flag bits, and SCID's 131,072-byte records | S, `counts`, `ratings`, `flags`, `names66000`, `blocks` | [SI4_FORMAT](SI4_FORMAT.md#game-entries-and-packed-fields) |
| 159 | **Name ID Extraction (Packed Format)** | Confirmed |  | S, `counts`, `ratings`, `flags`, `names66000`, `blocks` | [SI4_FORMAT](SI4_FORMAT.md#game-entries-and-packed-fields) |
| 163–172 | rust block | Confirmed | Shifts checked with IDs above 65,535 of every kind | S, `counts`, `ratings`, `flags`, `names66000`, `blocks` | [SI4_FORMAT](SI4_FORMAT.md#game-entries-and-packed-fields) |
| 174 | **Variation Counts and Result (16-bit packed field)** | Corrected |  | S, `counts`, `ratings`, `flags`, `names66000`, `blocks` | [SI4_FORMAT](SI4_FORMAT.md#game-entries-and-packed-fields) |
| 176–188 | rust block | Corrected | The counts are codes, not numbers. Result values and bit positions: confirmed | S, `counts`, `ratings`, `flags`, `names66000`, `blocks` | [SI4_FORMAT](SI4_FORMAT.md#game-entries-and-packed-fields) |
| 190 | **ELO Ratings (12-bit values + 4-bit type)** | Confirmed |  | S, `counts`, `ratings`, `flags`, `names66000`, `blocks` | [SI4_FORMAT](SI4_FORMAT.md#game-entries-and-packed-fields) |
| 192–200 | rust block | Corrected | The rating's range is 0 to 4,000: SCID reads higher as 4,000. The types were "Elo, FIDE, etc."; FIDE isn't one | S, `counts`, `ratings`, `flags`, `names66000`, `blocks` | [SI4_FORMAT](SI4_FORMAT.md#game-entries-and-packed-fields) |
| 202 | **Half-Move Count (10-bit value split across fields)** | Corrected |  | S, `moves`, all games derived | [SI4_FORMAT](SI4_FORMAT.md#search-fields-and-derived-game-metadata) |
| 204–209 | rust block | Corrected | "Maximum: 1023" is the field's; a longer game is stored modulo 1,024. Bit positions: confirmed | S, `moves`, all games derived | [SI4_FORMAT](SI4_FORMAT.md#search-fields-and-derived-game-metadata) |
| 211 | **Date Field - The Most Critical Component** | Confirmed | The emphasis is removed | S, `dates` | [SI4_FORMAT](SI4_FORMAT.md#dates-and-event-dates) |
| 217 | **Date Field Bit Layout** | Confirmed |  | S, `dates` | [SI4_FORMAT](SI4_FORMAT.md#dates-and-event-dates) |
| 219–224 | text block | Corrected | The marks didn't line up with the fields | S, `dates` | [SI4_FORMAT](SI4_FORMAT.md#dates-and-event-dates) |
| 226 | **Game Date Encoding (20 bits, absolute)** | Confirmed |  | S, `dates` | [SI4_FORMAT](SI4_FORMAT.md#dates-and-event-dates) |
| 230–234 | text block | Corrected | 0 means unknown in each part | S, `dates` | [SI4_FORMAT](SI4_FORMAT.md#dates-and-event-dates) |
| 239–242 | rust block | Corrected | The decimal value is 1,035,667, not 1,036,691. The hexadecimal is right | S, `dates` | [SI4_FORMAT](SI4_FORMAT.md#dates-and-event-dates) |
| 245–252 | rust block | Confirmed |  | S, `dates` | [SI4_FORMAT](SI4_FORMAT.md#dates-and-event-dates) |
| 254 | **Event Date Encoding (12 bits, relative)** | Confirmed |  | S, `dates` | [SI4_FORMAT](SI4_FORMAT.md#dates-and-event-dates) |
| 258–262 | text block | Corrected | The year's code is 1 to 7 for a date, and 0 for none | S, `dates` | [SI4_FORMAT](SI4_FORMAT.md#dates-and-event-dates) |
| 265–273 | text block | Qualified | Right, but for game years 1 and 2 SCID stores no event date | S, `dates` | [SI4_FORMAT](SI4_FORMAT.md#dates-and-event-dates) |
| 276–279 | rust block | Confirmed | 0x90A | S, `dates` | [SI4_FORMAT](SI4_FORMAT.md#dates-and-event-dates) |
| 281 | **Complete Date Field Parsing** | Confirmed |  | S, `dates` | [SI4_FORMAT](SI4_FORMAT.md#dates-and-event-dates) |
| 283–313 | rust block | Confirmed | Duplicates the rules above; replaced by a worked value | S, `dates` | [SI4_FORMAT](SI4_FORMAT.md#dates-and-event-dates) |
| 315 | **Game Flags Field** | Corrected |  | S, `counts`, `ratings`, `flags`, `names66000`, `blocks` | [SI4_FORMAT](SI4_FORMAT.md#game-entries-and-packed-fields) |
| 319–329 | rust block | Corrected | Bit 2 was missing, and bits 6–15 were "TBD" | S, `counts`, `ratings`, `flags`, `names66000`, `blocks` | [SI4_FORMAT](SI4_FORMAT.md#game-entries-and-packed-fields) |
| 333 | **Name File (.sn4) - Complete Specification** | Confirmed | "Fast access" and "sophisticated" removed | `fixtures` | [SN4_FORMAT](SN4_FORMAT.md) |
| 337 | **SN4 Header Structure (36 bytes)** | Confirmed |  | S, `names*`, `maxfreq` | [SN4_FORMAT](SN4_FORMAT.md#header-sections-ids-and-frequencies) |
| 339–350 | Table | Corrected | The timestamp isn't a modification time: `pgnscid` writes 0, and nothing uses it | S, `names*`, `maxfreq` | [SN4_FORMAT](SN4_FORMAT.md#header-sections-ids-and-frequencies) |
| 352 | **Name Storage Format** | Confirmed |  | S, `names*`, `maxfreq` | [SN4_FORMAT](SN4_FORMAT.md#header-sections-ids-and-frequencies) |
| 354 | **Front-Coding Compression Algorithm** | Confirmed |  | S, `accents`, `namecases`, order files | [SN4_FORMAT](SN4_FORMAT.md#front-coding-and-name-ordering) |
| 364–370 | text block | Corrected | The names weren't in the required order ("Carlsen, Magnus" before "Carlsen, Henrik"). Replaced by fixture bytes | S, `accents`, `namecases`, order files | [SN4_FORMAT](SN4_FORMAT.md#front-coding-and-name-ordering) |
| 372 | **Name Order** | Qualified | Right, and tested with both kinds of `char`. Platform dependence made explicit. "Can't look its names up" is from source only | S, `accents`, `namecases`, order files | [SN4_FORMAT](SN4_FORMAT.md#front-coding-and-name-ordering) |
| 381–383 | text block | Confirmed | The same order as `accents.sn4` | S, `accents`, `namecases`, order files | [SN4_FORMAT](SN4_FORMAT.md#front-coding-and-name-ordering) |
| 387 | **Name Record Structure** | Corrected |  | S, `names*`, `maxfreq` | [SN4_FORMAT](SN4_FORMAT.md#header-sections-ids-and-frequencies) |
| 391–396 | text block | Corrected | The prefix length was missing from the diagram and the field list | S, `names*`, `maxfreq` | [SN4_FORMAT](SN4_FORMAT.md#header-sections-ids-and-frequencies) |
| 413 | **Variable-Length Integer Encoding** | Corrected |  | S, `names*`, `maxfreq` | [SN4_FORMAT](SN4_FORMAT.md#header-sections-ids-and-frequencies) |
| 417–427 | rust block | Corrected | The helper gave 1-byte IDs to sections with fewer than 256 names. IDs and frequencies now have their own rules | S, `names*`, `maxfreq` | [SN4_FORMAT](SN4_FORMAT.md#header-sections-ids-and-frequencies) |
| 429 | **Name Section Organization** | Confirmed |  | `fixtures` | [SN4_FORMAT](SN4_FORMAT.md) |
| 438 | **Front-Coding Implementation** | Corrected |  | S, `accents`, `namecases`, order files | [SN4_FORMAT](SN4_FORMAT.md#front-coding-and-name-ordering) |
| 440–479 | rust block | Corrected | Used the wrong ID width, and didn't check the prefix against the length or the previous name. The byte reconstruction from #143 is kept. Replaced by checked pseudocode | S, `accents`, `namecases`, order files | [SN4_FORMAT](SN4_FORMAT.md#front-coding-and-name-ordering) |
| 481 | **Text Encoding and Character Handling** | Qualified | #143's statements are kept. "SCID can't open a name file that holds the same name twice": confirmed for names in a row | S, `namecases`, `controls` | [SN4_FORMAT](SN4_FORMAT.md#text-encoding-and-legacy-names) |
| 491 | **Game File (.sg4) - Complete Specification** | Confirmed |  | `fixtures` | [SG4_FORMAT](SG4_FORMAT.md) |
| 495 | **File Organization** | Confirmed |  | S, `blocks`, `cross` | [SG4_FORMAT](SG4_FORMAT.md#blocks-offsets-and-record-boundaries) |
| 497 | **Block-Based Structure** | Corrected | "SCID reads a record at any offset and length" is wrong: it refuses a record that crosses a block. scidtopgn doesn't | S, `blocks`, `cross` | [SG4_FORMAT](SG4_FORMAT.md#blocks-offsets-and-record-boundaries) |
| 501–514 | text block | Corrected | Padding and the short last block were missing | S, `blocks`, `cross` | [SG4_FORMAT](SG4_FORMAT.md#blocks-offsets-and-record-boundaries) |
| 518 | **Game Record Structure** | Corrected |  | S, `tags`, `random` | [SG4_FORMAT](SG4_FORMAT.md#tags-and-starting-positions) |
| 522–544 | text block | Corrected | The flags byte has no custom flags; moves take 1 or 2 bytes, not 1 to 3; the comments' text comes after the end marker, not among the moves; the FEN and the tags' end were missing | S, `tags`, `random` | [SG4_FORMAT](SG4_FORMAT.md#tags-and-starting-positions) |
| 546 | **Multiple Games Per SG4 File** | Confirmed |  | S, `blocks`, `cross` | [SG4_FORMAT](SG4_FORMAT.md#blocks-offsets-and-record-boundaries) |
| 550 | **Game Organization in SG4 File** | Corrected |  | S, `blocks`, `cross` | [SG4_FORMAT](SG4_FORMAT.md#blocks-offsets-and-record-boundaries) |
| 553–560 | cpp block | Corrected | Not the source quoted: `gfile.h` has 88 lines, and the block check, the point of the function, was left out | S, `blocks`, `cross` | [SG4_FORMAT](SG4_FORMAT.md#blocks-offsets-and-record-boundaries) |
| 563–574 | cpp block | Historical | `sc_game.cpp` doesn't exist in 4.26, and the loop isn't from its source | None needed | Removed |
| 576 | **Game Boundary Detection** | Corrected | The comments were missing from the order of parts | S, `tags`, `random` | [SG4_FORMAT](SG4_FORMAT.md#tags-and-starting-positions) |
| 585 | **SCID's Multi-Game Reading Strategy** | Historical |  | None needed | Removed |
| 588–608 | cpp block | Historical | As 563–574 | None needed | Removed |
| 610 | **Common Implementation Error** | Confirmed |  | S, `blocks`, `cross` | [SG4_FORMAT](SG4_FORMAT.md#blocks-offsets-and-record-boundaries) |
| 613–621 | rust block | Historical | Wrong on purpose; replaced by prose | S, `blocks`, `cross` | [SG4_FORMAT](SG4_FORMAT.md#blocks-offsets-and-record-boundaries) |
| 624–629 | rust block | Corrected | Unchecked slice, and a function that doesn't exist now | S, `blocks`, `cross` | [SG4_FORMAT](SG4_FORMAT.md#blocks-offsets-and-record-boundaries) |
| 631 | **Real-World Examples** | Corrected |  | `one` | [SCID_FORMAT](SCID_FORMAT.md#worked-binary-decoding-examples) |
| 634–639 | text block | Corrected | Said nothing specific; replaced by bytes | `one` | [SCID_FORMAT](SCID_FORMAT.md#worked-binary-decoding-examples) |
| 642–645 | text block | Corrected | The same | `one` | [SCID_FORMAT](SCID_FORMAT.md#worked-binary-decoding-examples) |
| 647 | **Index File Coordination** | Corrected | "Derive boundaries by parsing structure inside a linear scan" isn't possible in general: padding and unused records are indistinguishable | S, `blocks`, `cross` | [SG4_FORMAT](SG4_FORMAT.md#blocks-offsets-and-record-boundaries) |
| 650–662 | cpp block | Corrected | Not SCID's structure | S, all databases | [SCID_FORMAT](SCID_FORMAT.md#three-file-architecture-and-relationships) |
| 666 | **Tag Parsing and Game Structure** | Confirmed |  | S, `tags`, `random` | [SG4_FORMAT](SG4_FORMAT.md#tags-and-starting-positions) |
| 670 | **Tag Section End Detection** | Confirmed |  | S, `tags`, `random` | [SG4_FORMAT](SG4_FORMAT.md#tags-and-starting-positions) |
| 673–697 | cpp block | Corrected | A mix of `skipTags` and `DecodeTags`, cited at the wrong lines | S, `tags`, `random` | [SG4_FORMAT](SG4_FORMAT.md#tags-and-starting-positions) |
| 701 | **Game Record Parsing Sequence** | Confirmed |  | S, `tags`, `random` | [SG4_FORMAT](SG4_FORMAT.md#tags-and-starting-positions) |
| 704–735 | cpp block | Confirmed | Cited at the wrong lines; linked instead | S, `tags`, `random` | [SG4_FORMAT](SG4_FORMAT.md#tags-and-starting-positions) |
| 739 | **Tag Encoding Rules** | Qualified | The limits are confirmed; scidtopgn's handling is moved to COMPATIBILITY.md. Added: SCID 4.26's overrun at 255 bytes and its crash on spare codes | S, `tags`, `random` | [SG4_FORMAT](SG4_FORMAT.md#tags-and-starting-positions) |
| 742–753 | text block | Confirmed | Codes 241 to 250 | S, `tags`, `random` | [SG4_FORMAT](SG4_FORMAT.md#tags-and-starting-positions) |
| 758–760 | text block | Confirmed |  | S, `tags`, `random` | [SG4_FORMAT](SG4_FORMAT.md#tags-and-starting-positions) |
| 765–767 | text block | Confirmed | Big-endian; not written by SCID | S, `tags`, `random` | [SG4_FORMAT](SG4_FORMAT.md#tags-and-starting-positions) |
| 778 | **Common Implementation Error** | Confirmed |  | S, `comments`, `random`, `annotated` | [SG4_FORMAT](SG4_FORMAT.md#variations-comments-and-annotations) |
| 781–786 | rust block | Historical | Wrong on purpose | None needed | Removed |
| 789–794 | rust block | Confirmed |  | S, `tags`, `random` | [SG4_FORMAT](SG4_FORMAT.md#tags-and-starting-positions) |
| 796 | **Special Byte Contexts** | Confirmed |  | S, `comments`, `random`, `annotated` | [SG4_FORMAT](SG4_FORMAT.md#variations-comments-and-annotations) |
| 800–804 | Table | Qualified | 11 to 15 can also be a tag's name length, a NAG, or a FEN or comment byte | S, `comments`, `random`, `annotated` | [SG4_FORMAT](SG4_FORMAT.md#variations-comments-and-annotations) |
| 808 | **Implementation Requirements** | Corrected |  | S, `tags`, `random` | [SG4_FORMAT](SG4_FORMAT.md#tags-and-starting-positions) |
| 812–853 | rust block | Corrected | Not runnable, and left out the comments. Replaced by tables and checked pseudocode | S, `tags`, `random` | [SG4_FORMAT](SG4_FORMAT.md#tags-and-starting-positions) |
| 856–863 | rust block | Confirmed | The constants 240 and 11 to 15 | S, `comments`, `random`, `annotated` | [SG4_FORMAT](SG4_FORMAT.md#variations-comments-and-annotations) |
| 865 | **Game Data Elements** | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-specific-move-encoding) |
| 867 | **Move Encoding (Piece-Specific Binary Format)** | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-specific-move-encoding) |
| 872–875 | text block | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-specific-move-encoding) |
| 882 | **Piece-Specific Move Values** | Corrected | Every cited line, 59111 to 59348, is past the end of `game.cpp`, which has 5,148 | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-specific-move-encoding) |
| 888–899 | cpp block | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-specific-move-encoding) |
| 902–915 | Table | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-specific-move-encoding) |
| 924–931 | cpp block | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-specific-move-encoding) |
| 934–945 | Table | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-specific-move-encoding) |
| 953–974 | cpp block | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-specific-move-encoding) |
| 977–994 | Table | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-specific-move-encoding) |
| 1004–1013 | cpp block | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-specific-move-encoding) |
| 1016–1019 | Table | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-specific-move-encoding) |
| 1025–1036 | cpp block | Confirmed | The check that the destination is on the board was left out | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-specific-move-encoding) |
| 1039–1042 | Table | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-specific-move-encoding) |
| 1049–1054 | Table | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-specific-move-encoding) |
| 1060–1074 | cpp block | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-specific-move-encoding) |
| 1077–1081 | Table | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-specific-move-encoding) |
| 1088 | **Special Game Elements** | Confirmed |  | S, `comments`, `random`, `annotated` | [SG4_FORMAT](SG4_FORMAT.md#variations-comments-and-annotations) |
| 1091–1094 | text block | Confirmed |  | S, `comments`, `random`, `annotated` | [SG4_FORMAT](SG4_FORMAT.md#variations-comments-and-annotations) |
| 1097–1100 | text block | Corrected | A comment marker isn't followed by its text | S, `comments`, `random`, `annotated` | [SG4_FORMAT](SG4_FORMAT.md#variations-comments-and-annotations) |
| 1103–1105 | text block | Corrected | 15 ends the moves; the comments follow | S, `comments`, `random`, `annotated` | [SG4_FORMAT](SG4_FORMAT.md#variations-comments-and-annotations) |
| 1107 | **Variation Tree Structure** | Corrected |  | S, `comments`, `random`, `annotated` | [SG4_FORMAT](SG4_FORMAT.md#variations-comments-and-annotations) |
| 1111–1116 | text block | Corrected | Didn't say which move a variation replaces | S, `comments`, `random`, `annotated` | [SG4_FORMAT](SG4_FORMAT.md#variations-comments-and-annotations) |
| 1119–1127 | text block | Corrected | A variation was shown after a later move than the one it replaces, and continued with the alternative as a main-line move. Replaced by fixture bytes | S, `comments`, `random`, `annotated` | [SG4_FORMAT](SG4_FORMAT.md#variations-comments-and-annotations) |
| 1129 | **Comment and NAG Integration** | Qualified | The two parts are confirmed. "UTF-8": no encoding is recorded | S, `comments`, `random`, `annotated` | [SG4_FORMAT](SG4_FORMAT.md#variations-comments-and-annotations) |
| 1136–1148 | cpp block | Confirmed | Shortened; linked instead | S, `comments`, `random`, `annotated` | [SG4_FORMAT](SG4_FORMAT.md#variations-comments-and-annotations) |
| 1151–1160 | text block | Confirmed |  | S, `tags`, `random` | [SG4_FORMAT](SG4_FORMAT.md#tags-and-starting-positions) |
| 1163–1165 | text block | Confirmed |  | S, `comments`, `random`, `annotated` | [SG4_FORMAT](SG4_FORMAT.md#variations-comments-and-annotations) |
| 1166–1173 | Table | Confirmed | NAGs 1 to 6 | S, `comments`, `random`, `annotated` | [SG4_FORMAT](SG4_FORMAT.md#variations-comments-and-annotations) |
| 1175 | **Important: Opening Diversity** | Corrected | `6f` isn't "any pawn double forward": number 6 is a knight, and the value is invalid for one. 1. e4 is `cf` and 1. d4 is `bf`. The advice not to assume a first move is kept as the piece number's meaning | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-specific-move-encoding) |
| 1179–1185 | Table | Historical | Opening names, not the format | None needed | Removed |
| 1194–1201 | text block | Confirmed | Duplicate | S, `comments`, `random`, `annotated` | [SG4_FORMAT](SG4_FORMAT.md#variations-comments-and-annotations) |
| 1203 | **Position-Aware Move Parsing** | Corrected | En passant needs no legality check: it is a pawn capture onto an empty square. "CF-byte ambiguities" don't exist | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-numbering-and-position-tracking) |
| 1207–1236 | rust block | Historical | An API sketch; a move's legality isn't part of the format | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-numbering-and-position-tracking) |
| 1240 | **Critical Implementation Details** | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-numbering-and-position-tracking) |
| 1242 | **Piece List Management - The Foundation of Move Decoding** | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-numbering-and-position-tracking) |
| 1246 | **Core Data Structures (from SCID `position.h`)** | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-numbering-and-position-tracking) |
| 1248–1253 | cpp block | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-numbering-and-position-tracking) |
| 1259 | **Standard Starting Position Piece Numbers** | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-numbering-and-position-tracking) |
| 1263–1280 | Table | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-numbering-and-position-tracking) |
| 1282 | **FEN Position Initialization** | Corrected | Only the piece with number 0 moves when the king comes, to the king's number; the others keep theirs | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-numbering-and-position-tracking) |
| 1286–1302 | cpp block | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-numbering-and-position-tracking) |
| 1306 | **The Capture Swap Algorithm - CRITICAL** | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-numbering-and-position-tracking) |
| 1310–1320 | cpp block | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-numbering-and-position-tracking) |
| 1323–1340 | text block | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-numbering-and-position-tracking) |
| 1344 | **Promotion Handling** | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-numbering-and-position-tracking) |
| 1348–1358 | cpp block | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-numbering-and-position-tracking) |
| 1362 | **Castling Updates** | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-numbering-and-position-tracking) |
| 1366–1370 | cpp block | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-numbering-and-position-tracking) |
| 1374 | **Implementation Checklist** | Corrected | Left out en passant, null moves, and variations, which start from the position before a move | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-numbering-and-position-tracking) |
| 1386 | **Endianness - The Most Critical Specification** | Confirmed | The emphasis and the "experiments framework" are removed | S, all databases | [SCID_FORMAT](SCID_FORMAT.md#overview-scope-and-terminology) |
| 1402–1410 | rust block | Confirmed |  | S, all databases | [SCID_FORMAT](SCID_FORMAT.md#overview-scope-and-terminology) |
| 1413–1421 | rust block | Confirmed | 36,865 and 327,680 are right | S, all databases | [SCID_FORMAT](SCID_FORMAT.md#overview-scope-and-terminology) |
| 1423 | **Date Parsing - Critical Implementation Pattern** | Confirmed |  | S, `dates` | [SI4_FORMAT](SI4_FORMAT.md#dates-and-event-dates) |
| 1435–1474 | rust block | Confirmed | Duplicate of 283–313 | S, `dates` | [SI4_FORMAT](SI4_FORMAT.md#dates-and-event-dates) |
| 1476 | **Memory and Performance Considerations** | Historical | Advice for programs, not the format | None needed | Removed |
| 1478 | **Index File Memory Usage** | Historical |  | None needed | Removed |
| 1481–1488 | text block | Historical | The arithmetic is right | `fixtures` | [SI4_FORMAT](SI4_FORMAT.md) |
| 1496 | **I/O Optimization Patterns** | Historical |  | None needed | Removed |
| 1498–1514 | rust block | Historical | Doesn't compile: `?` in a function that returns a `Vec` | None needed | Removed |
| 1516 | **Error Handling and Validation** | Qualified |  | S, damaged files | [SCID_FORMAT](SCID_FORMAT.md#format-limits-and-validation-rules) |
| 1518 | **File Integrity Validation** | Qualified |  | S, damaged files | [SCID_FORMAT](SCID_FORMAT.md#format-limits-and-validation-rules) |
| 1520–1564 | rust block | Corrected | The 50,000,000 limit can't be reached by 3 bytes, and SCID doesn't check the file's size. Replaced by a list of checks | S, damaged files | [SCID_FORMAT](SCID_FORMAT.md#format-limits-and-validation-rules) |
| 1566 | **Graceful Error Recovery** | Historical | A program's policy | None needed | Removed |
| 1568–1608 | rust block | Corrected | Treated a date with unknown parts as invalid; checked for 28 bytes before reading a 29th. Not kept | S, `dates` | [SI4_FORMAT](SI4_FORMAT.md#dates-and-event-dates) |
| 1612 | **Complete Working Examples** | Corrected |  | `one` | [SCID_FORMAT](SCID_FORMAT.md#worked-binary-decoding-examples) |
| 1614 | **Example 1: Reading Game Metadata** | Corrected |  | `one` | [SCID_FORMAT](SCID_FORMAT.md#worked-binary-decoding-examples) |
| 1616–1695 | rust block | Corrected | Not runnable: its functions don't exist. Indexed names by ID without a check. The fields' shifts are right | `one` | [SCID_FORMAT](SCID_FORMAT.md#worked-binary-decoding-examples) |
| 1697 | **Example 2: Position-Aware Move Parsing (Streaming)** | Historical |  | None needed | Removed |
| 1699–1727 | rust block | Historical | An API that no longer exists | None needed | Removed |
| 1731 | **Multi-Byte Move Parsing - FULLY IMPLEMENTED ✅** | Historical | A progress report | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-specific-move-encoding) |
| 1737 | **Queen Diagonal Moves - Complete Implementation** | Historical |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-specific-move-encoding) |
| 1741 | **Technical Details** | Confirmed |  | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-specific-move-encoding) |
| 1745–1765 | cpp block | Confirmed | Duplicate of 1060–1074 | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-specific-move-encoding) |
| 1773 | **Implementation Architecture** | Historical |  | None needed | Removed |
| 1777–1799 | rust block | Historical | Code that no longer exists | None needed | Removed |
| 1803–1842 | rust block | Historical | Code that no longer exists | None needed | Removed |
| 1844 | **Performance Impact** | Historical | Success rates of an old program | None needed | Removed |
| 1856 | **Validation Results** | Historical |  | None needed | Removed |
| 1866–1872 | text block | Historical | 74.5% of moves decoded: an old program's result. The probes decode every move | None needed | Removed |
| 1874 | **Technical Implementation** | Historical | Files that no longer exist | None needed | Removed |
| 1888 | **References and Verification** | Corrected |  | S | [SCID_FORMAT](SCID_FORMAT.md#sources-and-verification) |
| 1890 | **Primary Source Code Analysis** | Corrected | Linked to `scidvspc/`, a directory that isn't in the repository | S | [SCID_FORMAT](SCID_FORMAT.md#sources-and-verification) |
| 1909 | **Verification Methodology** | Historical | Paths that no longer exist | S | [SCID_FORMAT](SCID_FORMAT.md#sources-and-verification) |
| 1924 | **Verification Results** | Corrected | `test/data/five.si4` is `tests/data/five.si4` | S | [SCID_FORMAT](SCID_FORMAT.md#sources-and-verification) |
| 1928–1937 | Table | Corrected | Black's ID 1 is "Murshed, N", not "Cheparinov, I", whose ID is 9. The other values are right | `one` | [SCID_FORMAT](SCID_FORMAT.md#worked-binary-decoding-examples) |
| 1939 | **Implementation Status** | Historical | Paths and claims of an old program | None needed | Removed |
