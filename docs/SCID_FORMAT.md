# SCID database format

A SCID database is three files with the same base name. This document says how they fit together; each file has its own:

- [SI4_FORMAT.md](SI4_FORMAT.md): the index file, `.si4`
- [SN4_FORMAT.md](SN4_FORMAT.md): the name file, `.sn4`
- [SG4_FORMAT.md](SG4_FORMAT.md): the game file, `.sg4`

They describe the files, not scidtopgn. Where scidtopgn behaves differently from SCID, they link to [COMPATIBILITY.md](COMPATIBILITY.md).

## Overview, scope, and terminology

These documents describe **index version 400 as SCID vs. PC 4.26 reads and writes it**. That is the only version they were checked against:

- Other releases of SCID vs. PC weren't checked. Version 400 is the number of the file format, not of the program: many releases share it.
- The original SCID is a different program. Whether its version-400 files are the same wasn't checked.
- Older index versions, such as 300 (`.si3`), and SCID's later `.si5` format aren't described.

| Term | Meaning |
| --- | --- |
| SCID | SCID vs. PC 4.26, unless it says otherwise |
| Record | A game's bytes in the game file |
| Entry | A game's 47 bytes in the index file |
| Main line | A game's moves outside variations |
| Half-move | One side's move |
| NAG | Numeric annotation glyph: a number for an annotation, such as 1 for `!` |
| FEN | The notation for a position |
| `pgnscid` | SCID vs. PC's program that imports PGN into a new database |
| SCID's behavior | What SCID 4.26 does with a file. It isn't a rule of the format |

**Numbers.** Every integer in the three files is unsigned and big-endian: its most significant byte comes first. Widths are 1 to 4 bytes. `0x` and bytes in `code` are hexadecimal. Bit 0 is the least significant. Offsets and lengths count bytes.

**Text** is bytes. No file records an encoding.

| Rule | SCID vs. PC 4.26 | Evidence | scidtopgn |
| --- | --- | --- | --- |
| Byte order, index | [`MFile::ReadTwoBytes`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/mfile.cpp#l305), [`MFile::ReadThreeBytes`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/mfile.cpp#l325), [`MFile::ReadFourBytes`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/mfile.cpp#l349) | Every probe database | [`read_u32_be`](../crates/scidtopgn/src/index.rs) |
| Byte order, names | [`readTwoBytes`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/misc.h#l293), [`readThreeBytes`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/misc.h#l318), [`readFourBytes`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/misc.h#l344) | Every probe database | [`read_three_bytes`](../crates/scidtopgn/src/namebase.rs) |
| Byte order, games: only the 3-byte event date | [`date`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4740) | Probe `compactdate` | [`decode_tags`](../crates/scidtopgn/src/game.rs) |

## Three-file architecture and relationships

```text
 .si4                              .sn4
┌──────────────────────┐          ┌──────────────────────────┐
│ header               │          │ header                   │
├──────────────────────┤          ├──────────────────────────┤
│ entry for game 1     │─ IDs ───▶│ players, events, sites,  │
│   name IDs           │          │ rounds: ID, frequency,   │
│   offset and length  │─┐        │ name                     │
│   date, result, ...  │ │        └──────────────────────────┘
├──────────────────────┤ │         .sg4
│ entry for game 2     │ │        ┌──────────────────────────┐
│ ...                  │ └───────▶│ record: tags, start      │
└──────────────────────┘          │ position, moves, comments│
                                  └──────────────────────────┘
```

- **The index comes first.** It says how many games there are and where each record is. A game's number is its entry's position.
- **Names** are found by ID. White and Black share the player section.
- **Records** are found by offset and length. A record alone doesn't give the names, date, result, ratings, or ECO code of its game, and an entry alone doesn't give the moves.
- **Finding games by their header** needs only the index and the names, for the fields those hold. Searching another tag, such as `Annotator`, needs the record too, which SCID's header search then reads ([`readGame`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/tkscid.cpp#l17680)). Decoding moves needs the record from its start, because a move's meaning depends on the position.

Nothing ties the three files together but these references: there is no shared identifier or checksum, so a file restored from another copy can't be detected reliably. A reference that doesn't fit can show it, and so can [derived fields](#format-limits-and-validation-rules) that don't match the game, as when an index entry's home pawns don't match its record's moves. The separate files don't make it safe for two programs to write to a database at once.

SCID can also keep a `.stc` file, a cache of its opening tree, which isn't needed to read a database. See [Tree cache](COMPATIBILITY.md#tree-cache).

## Format limits and validation rules

| What | Limit | Comes from |
| --- | --- | --- |
| Games | 16,777,214 | SCID's limit. The 3-byte count holds one more |
| Players, events, sites, rounds | 1,048,575, 524,287, 524,287, 262,143 | SCID's limit, from the [width of an ID](SI4_FORMAT.md#name-ids) |
| Name | 255 bytes | 1-byte length |
| Record | 131,071 bytes | 17-bit length, and the [block](SG4_FORMAT.md#blocks-offsets-and-record-boundaries) |
| Record offset | 4,294,967,295 | 4 bytes |
| Tag name | 240 bytes | [Higher bytes are codes](SG4_FORMAT.md#tags) |
| Tag value | 255 bytes | 1-byte length. SCID 4.26 [can't read 255 safely](SG4_FORMAT.md#tags) |
| Tag names of a game | 40 | Besides tags with fields of their own. SCID's limit: a later tag with one of those names replaces its value, and a 41st name overruns 4.26's array, which [crashed the builds tried](SG4_FORMAT.md#tags). The record can have more |
| Year in the index | 2047 | 11 bits |
| Event date in the index | 3 years from the game's year | [3 bits](SI4_FORMAT.md#dates-and-event-dates) |
| Rating | 4,095 | 12 bits. SCID reads more than 4,000 as 4,000, and its PGN import stores at most that |
| Half-moves | Stored modulo 1,024 | 10 bits. The record can have more |
| Counts of variations, comments, and NAGs | 50 | [4-bit codes](SI4_FORMAT.md#result-and-counts). The record can have more |
| NAGs of a move | 7 | SCID's limit. The record can have more |
| Pieces of a side | 16 | 4-bit piece number |
| Depth of variations | None | scidtopgn [limits it](COMPATIBILITY.md#deliberate-differences) |

A reader of files it didn't write should check:

1. **Index:** the magic, version 400, and that the file has 182 + 47 × games bytes. SCID ignores more.
2. **Names:** the magic, that each record fits in the file, the [prefix, length, and ID](SN4_FORMAT.md#front-coding-and-name-ordering), and the order if it looks names up by it.
3. **Entries:** that each name ID is below its section's count, the length isn't 0, the record is inside the game file, and it doesn't cross a block.
4. **Records:** that each part ends inside the record, each piece number exists, each value fits its piece, each destination is on the board, each variation follows a move, and each comment has its NUL. Legality needs the rules of chess.

A checked field isn't always right: [derived fields](SI4_FORMAT.md#search-fields-and-derived-game-metadata), counts, and frequencies can be recomputed and compared.

## Worked binary decoding examples

`tests/data/one` is a database of one game. `xxd tests/data/one.si4` shows these bytes.

**1. The index header** is 182 bytes. Its version is `01 90` = 400, and bytes 14–16 are `00 00 01`: one game.

**2. The entry** is at offset 182 (0xb6):

```text
offset  bytes                      field
+0      00 00 00 00                record offset 0
+4      00 a8                      record length, low bits: 168
+6      00                         length's high bit 0, no custom flags
+7      00 00                      no flags
+9      00  00 00  00 01           White ID 0, Black ID 1
+14     00  00 00  00 00  00 00    event ID 0, site ID 0, round ID 0
+21     30 00                      result 3 = 1/2-1/2, no NAGs, comments, or variations
+23     45 b3                      ECO 17,843 = B36f
+25     98 ff cd 93                date 2022.12.19, event date 2022.12.15
+29     09 44  09 73               Elo 2372 and 2419, type 0
+33     9c  29 72 87               stored line 156; 2R 2B 1N 7P against 2R 2B 7P
+37     25  09 b5 c1 d4 ae 70 00 00 00   37 half-moves; 9 pawns have left their home squares
```

B36f: 17,843 − 1 = 136 × 131 + 26, so `B36`, and 26 = 1 + 5 × 5, so `f`.

**3. The names.** [`one.sn4`](SN4_FORMAT.md#name-records) has player 0 "Hossain, Enam", player 1 "Murshed, N", event 0 "47th ch-Bangahbandhu 2022", site 0 "Dhaka BAN", and round 0 "5.6".

**4. The record** is bytes 0 to 167 of `one.sg4`. Its [tags](SG4_FORMAT.md#tags) end at 0x7e, its flags byte at 0x7f is 0, and its [moves](SG4_FORMAT.md#piece-specific-move-encoding) start at 0x80, with two two-byte queen moves. The last byte, at 0xa7, is `0f`: the game ends, with no comments.

A capture that renumbers pieces and a game with variations and comments are worked through in [SG4_FORMAT.md](SG4_FORMAT.md#changes).

## Sources and verification

**Sources.** Links go to SCID vs. PC's source at Subversion revision 3570 on SourceForge. All 84 `.cpp` and `.h` files in its `src` directory are byte-identical to those of `scid_vs_pc-4.26.tgz`, whose SHA-256 is `386eda599c058c3da9fef64de2f5248d51204e00715b94babeb09976c36afa87`, the one [`compat/build-scidvspc`](../compat/build-scidvspc) builds. scidtopgn links name the function or type that implements the rule.

**Evidence.** Two programs print what they decode from a database, and their output is compared: each entry's fields, each name, and each game's tags, flags, moves, and comments. They don't print every bit: both leave out the unused [bit 6 of entry byte 6](SI4_FORMAT.md#record-length-and-custom-flags), and print a rating above 4,000 as 4,000. Where SCID keeps one of two tags with the same name, the second probe prints both.

- [`format_probe.cpp`](../scid-cpp/tools/format_probe.cpp) reads it with SCID 4.26's own code.
- [`format_probe.py`](../scid-cpp/tools/format_probe.py) reads it as these documents describe, without code from SCID or scidtopgn. It also works out each entry's derived fields, except the stored line, from its game, and checks references, lengths, and record boundaries. It doesn't check every bound above: it accepts a FEN with more than 16 pieces of a side, which SCID refuses.

[SCID_FORMAT_AUDIT.md](SCID_FORMAT_AUDIT.md#verification) gives the commands, the databases, and the results. The names in the **Evidence** columns, such as `dates`, are its databases. Tests that only write and read with scidtopgn aren't used as evidence.

**Not resolved:**

- Other versions and programs, as above.
- Counts too large for a file that could be tried, such as the most games. They were tried by calling SCID's code.
- What base types mean, beyond the number: SCID's interface defines them.
