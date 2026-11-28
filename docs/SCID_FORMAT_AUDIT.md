# Audit of the SCID database format reference

This records the audit of `SCID_DATABASE_FORMAT.md` for [#145](https://github.com/intermezzo-chess/scidtopgn/issues/145), made in November 2026. It covers every section, table, and code block or diagram of [the document at `8c49478`](https://github.com/intermezzo-chess/scidtopgn/blob/8c49478/SCID_DATABASE_FORMAT.md). [SCID_FORMAT.md](SCID_FORMAT.md) and the documents it links to replace it.

- [Summary](#summary)
- [Verification](#verification)
- [Remaining uncertainties](#remaining-uncertainties)
- [Follow-up issues](#follow-up-issues)
- [Audit of COMPATIBILITY.md](#audit-of-compatibilitymd)
- [Inventory](#inventory)

## Summary

The old document has 184 items: 85 confirmed, 62 corrected, 26 historical, 11 qualified. None is unresolved; [Remaining uncertainties](#remaining-uncertainties) lists what the new documents leave open.

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
| `game.cpp` lines 59111–59348, `gfile.h` 250–270, `sc_game.cpp` | In 4.26, `game.cpp` has 5,150 lines, `gfile.h` 90; `sc_game.cpp` doesn't exist |

Removed as historical: progress reports, success rates, APIs and paths that no longer exist, advice on memory and I/O, and opening names. scidtopgn's own rules are in [COMPATIBILITY.md](COMPATIBILITY.md). The statements #143 corrected are kept.

## Verification

### Source

SourceForge's revision 3570 was compared with `scid_vs_pc-4.26.tgz`: all 84 `.cpp` and `.h` files in `src` (not its subdirectories) are byte-identical. Revisions 3580, 3585, 3590, 3600, and 3790 aren't. `scid-cpp/scid`, labeled revision 3585, differs from 4.26 in `bytebuf.cpp`, `game.cpp`, `game.h`, `myassert.cpp`, `naglatex.h`, `pgnparse.cpp`, `pgnparse.h`, and `stored.h`. Two of the differences change what the copy writes. Its `stored.h` is a stub, so it writes 0 as every entry's stored-line code. And the headers its `pgnparse.cpp` includes replace SCID's character set detector and converter with stubs, so it stores a Latin-1 PGN file's text as it is, where 4.26 converts it to UTF-8: `latin1.sn4` stores `Müller` as `4d fc`, and 4.26 as `4d c3 bc`. The name files of the five fixtures the reference converter wrote also have every frequency 0, because its `main.cpp` doesn't count them. This audit doesn't rely on that copy.

Each C++ link in the format documents was checked to name something on its line in 4.26, and each Rust link a function, constant, or type in its file.

### Probes

`format_probe.cpp` uses SCID 4.26's code, and `format_probe.py` only the rules in the format documents. From the repository root, after `compat/run` or `compat/build-scidvspc` has built SCID, on macOS:

These commands are for `bash` or `sh`: `zsh` doesn't split `$objs` into words.

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

`limits` is [`format_probe_limits.cpp`](../scid-cpp/tools/format_probe_limits.cpp), built as `format_probe.cpp` is, which calls SCID's code with counts too large to reach with a file. It is run as `./format_probe_limits probes/big probes/damage/firstbyte`, and passes.

`meta` is `five` after `compat/tcl/meta.tcl set`. `flags` is `counts` with user flag *n* set on game *n*:

```tcl
set base [sc_base open [lindex $argv 0]]
sc_base switch $base
set game 0
foreach flag {D W B M E N P T K Q ! ? U 1 2 3 4 5 6} { sc_game flag $flag [incr game] 1 }
sc_base close $base
```

`frequency` is a database of 8,388,607 games of player A against themselves, which [`format_probe_frequency.py`](../scid-cpp/tools/format_probe_frequency.py) writes. SCID opens it, and saves one more such game, with a frequency of 16,777,216. Opening the result then crashes SCID, with and without `-fast`. Importing `self.pgn` onto the first database with scidtopgn writes the same name file. Importing 32,769 copies of `self.pgn` onto the first database instead, with `sc_base import` or scidtopgn, gives a database that both open, whose player has the frequency 65,536:

```sh
printf '[Event "E"]\n[Site "S"]\n[White "A"]\n[Black "A"]\n[Round "1"]\n\n1. e4 *\n' > self.pgn
$V/bin/pgnscid -f self.pgn self
python3 -I scid-cpp/tools/format_probe_frequency.py self frequency
printf 'sc_base open frequency\nsc_game load 1\nputs [sc_game save 0]\nsc_base close\nexit 0\n' > save.tcl
printf 'puts [sc_base open frequency]\nexit 0\n' > open.tcl
$V/bin/tcscid open.tcl    # opens
$V/bin/tcscid save.tcl    # saves
$V/bin/tcscid open.tcl    # crashes
```

The probes were run on macOS on Apple silicon.

### Results: databases SCID wrote

`fixtures` is the 13 databases in `tests/data`. On all of them the two programs print the same.

Of the 39 databases `pgnscid` and `tcscid` 4.26 wrote, or tried to, they print the same for 34, which have 198,521 games:

| Database | What it checks | Games |
| --- | --- | --- |
| `names255`, `names256`, `names65535`, `names65536` | That many events, and that frequency for two players, a site, and a round | 131,582 |
| `names66000` | IDs above 65,535 of every kind of name | 66,000 |
| `namecases` | 255 bytes, prefixes that end inside a character, order, control characters | 6 |
| `dates` | Unknown parts, year 2047 and 2048, event dates −5 to +5 years, game years 1 to 3 | 22 |
| `ratings` | Rating types, 3,999 to 4,096, ECO codes | 23 |
| `counts` | 0 to 55 variations, comments, and NAGs | 56 |
| `moves` | 0, 1, 255, 256, 1,023, 1,024, 1,025, and 2,049 half-moves | 8 |
| `comments`, `tags`, `tags40` | Places for comments and NAGs; tags to 254 bytes; 40 tags | 13 |
| `blocks`, `size131071` | Records that fill blocks, and the longest | 10 |
| `random` | Random legal games from the standard position (400) and 5 FENs (200), with variations, NAGs, and comments | 600 |
| `flags`, `meta`, `empty` | User and custom flags, header fields, no games | 61 |
| `compatnames`, and the PGN of the fixtures apart from `notation` | | 140 |

`format_probe.py` also derives each entry's start and promotion flags, counts, material, half-moves, and home pawns from its game, and they match in every one.

The other five differ, as SCID itself misreads or can't write them:

| Database | Result |
| --- | --- |
| `notation` | Game 41 castles without a rook, which SCID writes as a [corrupt game](COMPATIBILITY.md#castling). The 40 before it match |
| `tag255` | SCID overruns its buffer when it reads a tag value of 255 bytes, and loses the tag's name |
| `tags41` | `pgnscid` crashes at the 41st tag |
| `size131072` | SCID writes a record of 131,072 bytes with length 0, and can't read it |
| `size131073` | `pgnscid` stops with "Fatal error encoding game!" |

In `blocks.sg4`, the padding after the records of blocks 0 to 3 is all zeros.

### Results: damaged files

SCID's error numbers are from `error.h`: 109 and 110 are versions, 152 is corrupt, 402 and 406 are decoding errors, and 602 is reading past the end. "Reads" means that SCID decodes the game. The two programs print the same for `elo4095`, `signed`, `maxfreq`, `e4`, `d4`, `compactdate`, `flags8`, `emptyvar`, `nagfirst`, `nagafter`, and the three with comments that SCID reads, whose entries fit their games.

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
| `truncated` | Record past the end of the file | Error 602 | Damaged |
| `zero` | Length 0 | Game corrupt (152) | Damaged |
| `compactdate` | Event date with code 255 | Reads 2021.3.7 | No damage |
| `spare` | Tag code 251 | Crashes | No damage |
| `manytags` | 41 tags, each with its own name | Crashes | No damage |
| `flags8` | Unused flag bits | Reads | No damage |
| `e4`, `d4` | Nothing: 1. e4 and 1. d4 | Reads | No damage |
| `knight15`, `queen` | Invalid value, invalid second byte | Error 406 | Damaged |
| `illegal` | Rook to its own square | Reads | Damaged |
| `stalepiece` | A piece number its side no longer has | Reads, as a move of the piece that had it | Damaged |
| `staleempty` | The same, after that piece has moved | Error 406 | Damaged |
| `knightoff` | Knight from g1 to 11 squares before a1 | Reads, as a move to square 245 | Damaged |
| `nag0` | NAG 0 | Reads without it | No damage |
| `nag9` | 9 NAGs | Reads the first 7 | No damage |
| `twocomments` | Two markers, one text | Reads one comment | No damage |
| `commentafter` | Marker after a variation | Reads | No damage |
| `commentorder` | Markers in another order than the tree | Reads the texts in the tree's order | No damage |
| `nagafter` | NAG after a variation | Reads it as the move's | No damage |
| `nagfirst` | NAG before the first move | Reads it, but doesn't write or export it | No damage |
| `nocomment` | Marker without text | Error 602 | Damaged |
| `varfirst` | Variation before a move | Error 402 | Damaged |
| `emptyvar` | Variation without moves | Reads | No damage |
| `endvar` | Variation end in the main line | Error 406 | Damaged |
| `noend` | No end | Error 602 | Damaged |
| `after` | Bytes after the comments | Reads | No damage |

For the builds with unsigned `char`, the SCID sources the probe links were compiled with `-funsigned-char`. Such a build refuses `tests/data/accents` too.

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
- **Component runs.** The most games and names were tried by calling SCID's code (`limits`), not with a database of that size. A frequency of 2²⁴ was tried both ways, but not by importing that many games with `pgnscid`.
- **Undefined behavior.** What SCID does after the buffer overrun of `tag255`, with `spare`, with `knightoff`, and with more than 40 tags depends on the build.
- **Base types** are numbers to the format. What SCID's interface calls them wasn't audited.
- **`COMPATIBILITY.md`** was audited [separately](#audit-of-compatibilitymd), with its own uncertainties.
- **Platforms.** All runs were on macOS on Apple silicon. The unsigned `char` results are from a build with that option.
- **Links** to SourceForge depend on that site. The tarball's SHA-256 identifies the source without it.

## Follow-up issues

Found by the probes above, and not changed here.

1. **`validate` reports no damage in databases that SCID refuses** ([#149](https://github.com/intermezzo-chess/scidtopgn/issues/149)). It is documented to check only that each game's record can be read, so this is a possible improvement. Reproduce with `scidtopgn validate` on the `badname`, `order`, `duplicate`, `cross`, and `firstbyte` databases.
2. **SCID 4.26 can't read a tag value of 255 bytes safely** ([#150](https://github.com/intermezzo-chess/scidtopgn/issues/150)). The bug is SCID's: `Game::DecodeTags` writes 1 byte past a 255-byte buffer, which AddressSanitizer reports for `tag255` written by `pgnscid` and by scidtopgn. Both programs import such a value without a warning, and write the same record. The issue is to decide whether scidtopgn should.

The [audit of `COMPATIBILITY.md`](#audit-of-compatibilitymd) led to six more, all bugs: #151 to #156. Its PGN section's audits led to three more, #157, #158, and #161.

`GFile::read_game`'s comment said that SCID reads a record that crosses a block. The comment is corrected; the behavior is unchanged.

## Audit of COMPATIBILITY.md

A reviewer audited `docs/COMPATIBILITY.md` at `ec9696c`, all 351 paragraphs, table rows, and examples, against scidtopgn's source, `scid-cpp/scid`, and SCID vs. PC 4.26, and found these 17 corrections and gaps. Each was confirmed again before it was corrected, by importing and exporting the cases with scidtopgn, `pgnscid` 4.26, and the reference converter, or, for F01 and F16, in the source and file tree.

| | Finding | Correction |
| --- | --- | --- |
| F01 | An update recomputes three flags, not only the start flag | [Update](COMPATIBILITY.md#update) |
| F02 | Export isn't in the PGN standard's export format in full | [Export and round trips](COMPATIBILITY.md#export-and-round-trips) |
| F03 | SCID doesn't ignore a `{...}` comment between tags | [General differences](COMPATIBILITY.md#general-differences). Two test comments that said so are corrected |
| F04 | NUL is an exception to keeping control characters as SCID does | [Reading](COMPATIBILITY.md#reading), [Player names and ratings](COMPATIBILITY.md#player-names-and-ratings) |
| F05 | Tags after the 40th are dropped silently | [Limits shared with SCID](COMPATIBILITY.md#limits-shared-with-scid), [#155](https://github.com/intermezzo-chess/scidtopgn/issues/155) |
| F06 | A `}` in a comment loses text in a round trip. It can also lose moves and change the result | [Export and round trips](COMPATIBILITY.md#export-and-round-trips), [#152](https://github.com/intermezzo-chess/scidtopgn/issues/152) |
| F07 | Rating overflow depends on the width of `unsigned long`, and not every negative rating is 4,000 | [Player names and ratings](COMPATIBILITY.md#player-names-and-ratings) |
| F08 | SCID keeps none of 256 variations | [General differences](COMPATIBILITY.md#general-differences) |
| F09 | SCID's limit on text before a game counts bytes, and splits long lines | [Text before and between games](COMPATIBILITY.md#text-before-and-between-games) |
| F10 | SCID cuts long comments in the moves | [Comments](COMPATIBILITY.md#comments-tag-values-results-in-variations-and-stray-brackets) |
| F11 | Dates: game years 1 and 2, years over 2047, malformed dates, and partly unknown dates in exports | [Limits shared with SCID](COMPATIBILITY.md#limits-shared-with-scid), [Known differences](COMPATIBILITY.md#known-differences-outside-these-cases), [#154](https://github.com/intermezzo-chess/scidtopgn/issues/154). Export can also lose a known day |
| F12 | `--overwrite` doesn't keep permissions | [Deliberate differences](COMPATIBILITY.md#deliberate-differences), [#151](https://github.com/intermezzo-chess/scidtopgn/issues/151) |
| F13 | Empty name tags | [Known differences](COMPATIBILITY.md#known-differences-outside-these-cases) |
| F14 | Several tags on one line | [Known differences](COMPATIBILITY.md#known-differences-outside-these-cases) |
| F15 | An unknown ECO code isn't stored | [Export and round trips](COMPATIBILITY.md#export-and-round-trips) |
| F16 | Two test paths were wrong, and one test doesn't run SCID | [Tests](COMPATIBILITY.md#tests) |
| F17 | A line feed in a name or other tag value, and spaces and tabs at a player name's ends, don't survive a round trip. A line feed in the last tag loses the whole game | [Export and round trips](COMPATIBILITY.md#export-and-round-trips), [#153](https://github.com/intermezzo-chess/scidtopgn/issues/153) |

### Second review

At `1e33e31`, the reviewer checked all five documents against 4.26's source and runs of it, not against other documents. Each finding was reproduced before it was corrected.

| | Finding | Correction |
| --- | --- | --- |
| S1 | The PGN section described the reference converter, not 4.26. `pgnscid` stores games without a result, drops `PlyCount`, and converts text | The section now describes 4.26's parser and importers, with [a table of how they differ](COMPATIBILITY.md#pgn-import-compared-with-scid). #28 is no longer a difference |
| S2 | Names that SCID writes aren't always UTF-8 | [SN4](SN4_FORMAT.md#text-encoding-and-legacy-names), [Reading](COMPATIBILITY.md#reading) |
| S3 | SCID can store ratings above 4,000 | [Ratings](SI4_FORMAT.md#ratings) |
| S4 | A second rating tag is stored in the record | [Tags](SG4_FORMAT.md#tags) |
| S5 | `ScidFlags` sets flags in SCID | [Known differences](COMPATIBILITY.md#known-differences-outside-these-cases) |
| S6 | Frequencies and PGN dates weren't marked as differences | [SN4](SN4_FORMAT.md#header-sections-ids-and-frequencies), [SI4](SI4_FORMAT.md#dates-and-event-dates), [Append](COMPATIBILITY.md#append) |
| S7 | SCID clears some of the unused header bytes | [Header](SI4_FORMAT.md#file-header-and-database-metadata) |
| S8 | A failed write by SCID can leave a readable database | [Deliberate differences](COMPATIBILITY.md#deliberate-differences) |
| S9 | Citations that didn't establish the claim, and a claim about older versions of SCID | Citations added or replaced, and the claim removed |
| S10 | A frequency of 2²⁴ wraps | [SN4](SN4_FORMAT.md#header-sections-ids-and-frequencies) |

Reproducing them showed three more: `sc_base import` ignores text before games; neither importer skips a game with an error; and 4.26 writes a wrapped length when a tag value is over 255 bytes after conversion.

`COMPATIBILITY.md` had no links to SCID's source. It now links the code for each section's claims. The PGN tables' examples were run again with the unmodified 4.26 parser, which agreed with each, apart from S1. Its `pgnparse.cpp` differs from the copy in `scid-cpp/scid` only in which headers it includes, and `position.cpp` doesn't differ.

### Third review

At `81b5721`, the reviewer checked the corrections above. Each finding was reproduced before it was corrected.

| | Finding | Correction |
| --- | --- | --- |
| T1 | A record with a wrapped tag length can decode, with other tags. An `Annotator` of 66 `A`s, `=`, 59 `a`s, and a Latin-1 `é`, twice, is 254 bytes, and 256 after conversion | [Tags](SG4_FORMAT.md#tags), [Tag and name lengths](COMPATIBILITY.md#general-differences) |
| T2 | A further rating tag is stored as a tag only once the player's rating isn't 0. `[WhiteUSCF "0"]` and then `[WhiteElo "2400"]` stores no tag | [Tags](SG4_FORMAT.md#tags) |
| T3 | The reference converter's copy doesn't replace `DoCharsetConversion`, but the detector and converter | [PGN import](COMPATIBILITY.md#pgn-import-compared-with-scid) |
| T4 | Functions that read were cited for writing | [Reading](COMPATIBILITY.md#reading) |
| T5 | A frequency of 2²⁴ can be reached with a database | [SN4](SN4_FORMAT.md#header-sections-ids-and-frequencies), [Append](COMPATIBILITY.md#append). scidtopgn writes the same file, which was found while reproducing this ([#156](https://github.com/intermezzo-chess/scidtopgn/issues/156)) |

`README.md`, `CONTRIBUTING.md`, and doc comments made the claims about older versions of SCID and about SCID vs. PC 4.27 that had been removed from `COMPATIBILITY.md`. They are removed there too.

At `8724c38`, the reviewer found these resolved, and one more: a frequency above 16,777,215 doesn't always make the file unreadable (T5). It was reproduced and corrected.

### Fourth review

At `ca3a6aa`, a reviewer checked `SI4_FORMAT.md` alone, 119 claims, against the code it links to. Each finding was reproduced before it was corrected.

| | Finding | Correction |
| --- | --- | --- |
| U1 | A `FEN` tag of the standard position sets the start flag, and leaves the stored-line code 0 and the home pawns empty. scidtopgn does the same | [Flags](SI4_FORMAT.md#flags), [Search fields](SI4_FORMAT.md#search-fields-and-derived-game-metadata), [SG4](SG4_FORMAT.md#flags-and-starting-position) |
| U2 | `[ScidFlags "x"]` sets the start flag without a starting position | [Flags](SI4_FORMAT.md#flags), [Known differences](COMPATIBILITY.md#known-differences-outside-these-cases) |
| U3 | The limits of date parts apply after a conversion that wraps | [Dates](SI4_FORMAT.md#dates-and-event-dates), [Known differences](COMPATIBILITY.md#known-differences-outside-these-cases) |

At `25c2389`, the reviewer confirmed these, and then checked `SN4_FORMAT.md` in the same way. Each finding was reproduced before it was corrected.

| | Finding | Correction |
| --- | --- | --- |
| U4 | `AddName` doesn't terminate the name it cuts. A name of 300 bytes was kept as 257 after a function that filled its stack, and as 255 otherwise | [SN4](SN4_FORMAT.md#text-encoding-and-legacy-names), [Tag and name lengths](COMPATIBILITY.md#general-differences) |
| U5 | `sc_base import file` doesn't convert a file that starts with a byte order mark | [SN4](SN4_FORMAT.md#text-encoding-and-legacy-names), [Reading](COMPATIBILITY.md#reading), [PGN import](COMPATIBILITY.md#pgn-import-compared-with-scid) |
| U6 | Names out of order can open without a warning, and then not be found | [Order](SN4_FORMAT.md#order) |

Reproducing them showed one more: `pgnscid` reads the line with the mark as text before the game.

At `d907547`, the reviewer confirmed these. A check of `SG4_FORMAT.md` in the same way stopped before it was complete, with three findings. They were reproduced and corrected.

| | Finding | Correction |
| --- | --- | --- |
| U7 | Comments aren't converted from a file that starts with a byte order mark | [Comments](SG4_FORMAT.md#variations-comments-and-annotations) |
| U8 | Bytes `0b` to `0f` are markers, not king moves that are refused | [Damaged records](SG4_FORMAT.md#piece-specific-move-encoding) |
| U9 | A NAG before a line's first move is dropped when the game is written, not when it is read | [Annotations](SG4_FORMAT.md#variations-comments-and-annotations) |

Reproducing them showed two more: SCID refuses an invalid piece number whose square is empty, and doesn't refuse a king, knight, or pawn destination off the board.

At `81d22d4`, a new reviewer audited `SG4_FORMAT.md` in full, and found nothing. Another audited the PGN section of `COMPATIBILITY.md` in full, with both programs. Each finding was reproduced before it was corrected.

| | Finding | Correction |
| --- | --- | --- |
| P1 | After a line starting with `[` that isn't a tag, the next game is lost unless a blank line comes first | [Text before and between games](COMPATIBILITY.md#text-before-and-between-games) |
| P2 | Where a game without a result ends, only spaces and tabs may come before `[` | The same |
| P3 | A carriage return in a tag value is kept, where SCID drops the tag | [Player names and ratings](COMPATIBILITY.md#player-names-and-ratings) |
| P4 | SCID keeps all but one line break after the text before a game | [Text before and between games](COMPATIBILITY.md#text-before-and-between-games) |
| P5 | `--quiet` hides warnings, and `--log` writes them to a file | [Warnings](COMPATIBILITY.md#warnings) |
| P6 | A `}` or `]` before the first move gives no warning | The same |
| P7 | The 40 tags are those without a field of their own | [Export and round trips](COMPATIBILITY.md#export-and-round-trips) |
| P8 | SCID cuts a long tag name when it reads the tag, so it keeps two tags of the same long name | [General differences](COMPATIBILITY.md#general-differences) |
| P9 | `A,,B` gets one space, after the second comma | [Player names and ratings](COMPATIBILITY.md#player-names-and-ratings) |
| P10 | `scidtopgn` reads a `Result` tag of `½-½` | [Results](COMPATIBILITY.md#results-and-the-start-position) |

It also qualified the date export bug, a claim about every PGN reader, and the example of export. Checking the corrections, it qualified three of them again: the warning for a `}` or `]` before the moves start, which tags the 40-tag limit drops, and the library functions that read a game without tags. A second full audit of the section found seven more, which were reproduced and corrected: a NUL in a comment, which shifts the comments after it when the game is read back; a player name that only tidying makes too long, which SCID keeps; tag names, which SCID doesn't convert; spaces at the start of text before a game, which are kept; two summaries that left out the exceptions of #157 and #158; and the probe's filter, which can hide a game's moves.

A full audit of the rest of `COMPATIBILITY.md` found four more, which were reproduced and corrected: a failed swap whose file can't be moved back, which can leave a file missing; compaction, which replaces read-only and linked files; the 40-tag limit, which counts names, as SCID's does; and a test of links, which doesn't run on Windows.

A full audit of `SCID_FORMAT.md` found six more, which were reproduced and corrected: a header search for another tag needs the record; derived fields can show a mismatched file; the 40-tag limit counts names; the probes' output and checks were overstated; an example's home pawns were ambiguous; and two date limits are the index's.

A second full audit of `SI4_FORMAT.md` found two more, which were reproduced and corrected: for a game with a starting position, SCID doesn't compute its promotion flags, material, and half-moves again when it saves it, so editing the moves can leave them out of date; and a description of 107 bytes clears the field's last byte.

A second full audit of `SN4_FORMAT.md` found one more, which was corrected: the record whose prefix ends inside a character is at 0x84, not 0x73.

An audit of this record found ten more, which were corrected: the reference converter's copy writes stored-line code 0; two line counts were the copy's, not 4.26's; one inventory item was corrected, not confirmed, which changes the totals; the event dates SCID doesn't store; the list of databases both probes print the same; an error number's label; the 32,769 games' database; the number of sources rebuilt; an old example's compile error; and the build commands, which need `bash` or `sh`. Checking the corrections, it found that the copy's parser stubs change its output too.

A third full audit of the PGN section, by a new reviewer, found three more, which were reproduced and corrected: Unicode whitespace before the first tag line, which SCID doesn't read as whitespace; the event dates that the date export bug applies to; and the probe's filter, which can leave out only some of the moves.

### Not verified

Not verified by either:

- Ratings and dates where `unsigned long` has 32 bits, such as on Windows. The documents say so.
- Which Tcl SCID vs. PC 4.27 for macOS bundles. The document no longer says. How Tcl 8.6.13 and 8.6.0 match was read in their source, and 8.6.0's functions were run on their own, not in SCID.
- SCID's GUI, two programs writing at once, power loss, and every way a file operation can fail.
- What "Before #..." statements say about old versions, beyond the linked issues.
- Inputs beyond the cases tried. A claim that the sources and tests support can still have exceptions.

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
| 101–118 | rust block | Corrected | The description was cut only at trailing NULs, so `five.si4` gives unrelated bytes; it ends at the first NUL. The code doesn't compile. Replaced by a byte dump | S, `fixtures`, `meta` | [SI4_FORMAT](SI4_FORMAT.md#file-header-and-database-metadata) |
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
| 265–273 | text block | Qualified | Right, but for a game whose year is unknown, 1, or 2, SCID stores no event date | S, `dates` | [SI4_FORMAT](SI4_FORMAT.md#dates-and-event-dates) |
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
| 372 | **Name Order** | Qualified | Right, and tested with both kinds of `char`. Platform dependence made explicit. "Can't look its names up" is confirmed by `limits` | S, `accents`, `namecases`, order files | [SN4_FORMAT](SN4_FORMAT.md#front-coding-and-name-ordering) |
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
| 553–560 | cpp block | Corrected | Not the source quoted: `gfile.h` has 90 lines, and the block check, the point of the function, was left out | S, `blocks`, `cross` | [SG4_FORMAT](SG4_FORMAT.md#blocks-offsets-and-record-boundaries) |
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
| 882 | **Piece-Specific Move Values** | Corrected | Every cited line, 59111 to 59348, is past the end of `game.cpp`, which has 5,150 | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-specific-move-encoding) |
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
| 1248–1253 | cpp block | Corrected | `Count` is a `uint`, not a `byte` | S, `random` | [SG4_FORMAT](SG4_FORMAT.md#piece-numbering-and-position-tracking) |
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
