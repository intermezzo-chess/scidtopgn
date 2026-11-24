# Index file (.si4)

The index file holds a header with the database's settings, then one fixed-size entry for each game. An entry locates the game's record in the [game file](SG4_FORMAT.md), gives the IDs of its names in the [name file](SN4_FORMAT.md), and holds its date, result, ratings, ECO code, flags, and the fields SCID's searches use.

[SCID_FORMAT.md](SCID_FORMAT.md) states which version this describes, its terms, and [how each rule was checked](SCID_FORMAT.md#sources-and-verification). Every integer is unsigned and big-endian. "SCID" means SCID vs. PC 4.26.

```text
offset 0                                     182                229
┌───────────────────────────────────────────┬──────────────────┬─────┬──────────────────┐
│ header, 182 bytes                         │ game 1, 47 bytes │ ... │ game N, 47 bytes │
└───────────────────────────────────────────┴──────────────────┴─────┴──────────────────┘
```

Game *n*, counted from 1, has its entry at offset 182 + 47 × (*n* − 1).

## File header and database metadata

```text
 0               8    10       14     17     20                        128              182
┌───────────────┬─────┬────────┬──────┬──────┬─────────────────────────┬────────────────┐
│ magic         │ ver │ type   │games │ auto │ description             │ 6 custom flag  │
│ 8             │ 2   │ 4      │ 3    │ 3    │ 108                     │ names, 9 each  │
└───────────────┴─────┴────────┴──────┴──────┴─────────────────────────┴────────────────┘
```

| Offset | Bytes | Field | Values |
| --- | --- | --- | --- |
| 0 | 8 | Magic | `Scid.si` and a NUL: `53 63 69 64 2e 73 69 00` |
| 8 | 2 | Version | 400 (`01 90`) |
| 10 | 4 | Base type | A number for the kind of database, such as 1 in `tests/data/one.si4`; 0 if unset. SCID's interface chooses the number, and the file format doesn't interpret it |
| 14 | 3 | Number of games | 0 to 16,777,214, counting games marked as deleted |
| 17 | 3 | Auto-load game | 0 or 2: game 1. 1: none. Otherwise the game's number plus 1 |
| 20 | 108 | Description | At most 107 bytes and a NUL |
| 128 | 54 | Custom flag names | Six of 9 bytes, for custom flags 1 to 6: at most 8 bytes and a NUL each |
| **Total** | **182** | | |

A text field ends at its first NUL, and the bytes after that have no meaning. SCID doesn't clear all of them: a new header's fields start with only their first byte set, setting a description clears all but the field's last byte, and setting a custom flag's name clears its field. So they can hold unrelated data: in `tests/data/five.si4`, the description is empty but its last byte, at offset 127, is `65`, and the custom flag names hold pieces of a script. The format doesn't record a text encoding.

`tests/data/one.si4`, offset 0:

```text
53 63 69 64 2e 73 69 00   magic
01 90                     version 400
00 00 00 01               base type 1
00 00 01                  1 game
00 00 02                  auto-load game 1
00 00 ...                 no description
```

**SCID's behavior.** SCID 4.26 opens only version 400: it rejects a lower version as out of date and a higher one as too new. It doesn't check the file's size: extra bytes after the last entry are ignored.

| Rule | SCID vs. PC 4.26 | Evidence | scidtopgn |
| --- | --- | --- | --- |
| Header layout | [`Index::WriteHeader`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.cpp#l910), [`Index::Open`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.cpp#l936) | Probes `fixtures`, `meta`, `empty` | [`IndexHeader`](../crates/scidtopgn/src/index.rs) |
| Sizes and limits | [`INDEX_HEADER_SIZE`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.h#l70), [`MAX_GAMES`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.h#l36) and [`MAX_GAMES`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.cpp#l1268), [`SCID_DESC_LENGTH`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.h#l42), [`CUSTOM_FLAG_DESC_LENGTH`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.h#l43) | Probes `fixtures`, `meta`; the maximum number of games, `limits` | [`INDEX_HEADER_SIZE`](../crates/scidtopgn/src/index.rs), [`MAX_GAMES`](../crates/scidtopgn/src/index.rs) |
| Auto-load numbering | [`GetAutoLoad`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.h#l675) | Probe `meta`: setting game 3 stores 4 | Kept as stored ([Append](COMPATIBILITY.md#append)) |
| Text ends at a NUL, and what is cleared | [`Index::Init`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.cpp#l822), [`Index::SetDescription`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.cpp#l1237), [`SetCustomFlagDesc`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.h#l669) | `tests/data/five.si4`, probe `meta` | Kept as stored |
| Version check | [`SCID_VERSION`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.cpp#l977), [`SCID_VERSION`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/common.h#l53), [`SCID_OLDEST_VERSION`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/common.h#l54) | Probes `v300`, `v399`, `v401` | [`IndexHeader`](../crates/scidtopgn/src/index.rs): the same, and [other versions are rejected](COMPATIBILITY.md#supported-format) |

## Game entries and packed fields

```text
 0       4     6   7     9    10    12    14    15    17    19    21
┌───────┬─────┬───┬─────┬────┬─────┬─────┬─────┬─────┬─────┬─────┐
│offset │len  │len│flags│W B │W    │B    │E S R│E    │S    │R    │
│       │low  │hi │     │high│low  │low  │high │low  │low  │low  │
│ 4     │ 2   │ 1 │ 2   │ 1  │ 2   │ 2   │ 1   │ 2   │ 2   │ 2   │
└───────┴─────┴───┴─────┴────┴─────┴─────┴─────┴─────┴─────┴─────┘
 21    23    25      29    31    33       37   38                 47
┌─────┬─────┬───────┬─────┬─────┬────────┬────┬──────────────────┐
│count│ECO  │dates  │W    │B    │material│half│home pawns and    │
│+ res│     │       │Elo  │Elo  │+ line  │move│half-move high    │
│ 2   │ 2   │ 4     │ 2   │ 2   │ 4      │ 1  │ 9                │
└─────┴─────┴───────┴─────┴─────┴────────┴────┴──────────────────┘
```

| Offset | Bytes | Field |
| --- | --- | --- |
| 0 | 4 | Offset of the game's record in the game file |
| 4 | 2 | [Record length](#record-length-and-custom-flags), low 16 bits |
| 6 | 1 | [Record length's high bit, and custom flags](#record-length-and-custom-flags) |
| 7 | 2 | [Flags](#flags) |
| 9 | 1 | [White and Black name IDs](#name-ids), high bits |
| 10 | 2 | White name ID, low 16 bits |
| 12 | 2 | Black name ID, low 16 bits |
| 14 | 1 | [Event, site, and round name IDs](#name-ids), high bits |
| 15 | 2 | Event name ID, low 16 bits |
| 17 | 2 | Site name ID, low 16 bits |
| 19 | 2 | Round name ID, low 16 bits |
| 21 | 2 | [Result, and counts of NAGs, comments, and variations](#result-and-counts) |
| 23 | 2 | [ECO code](#eco-code) |
| 25 | 4 | [Date and event date](#dates-and-event-dates) |
| 29 | 2 | [White rating and its type](#ratings) |
| 31 | 2 | Black rating and its type |
| 33 | 4 | [Final material and stored-line code](#final-material-and-stored-line-code) |
| 37 | 1 | [Half-moves](#half-moves-and-home-pawns), low 8 bits |
| 38 | 9 | [Half-moves' high bits, and home pawns](#half-moves-and-home-pawns) |
| **Total** | **47** | |

SCID's own structure holds the same fields in another order; the table gives the order in the file.

### Record length and custom flags

```text
 byte 6                            bytes 4-5
  7    6    5  4  3  2  1  0       15                              0
┌────┬────┬──────────────────┐    ┌─────────────────────────────────┐
│ L16│ –  │ custom 6 ... 1   │    │ L15 ... L0                      │
└────┴────┴──────────────────┘    └─────────────────────────────────┘
```

The record length is 17 bits: bit 7 of byte 6, then bytes 4 and 5. Its range is 1 to 131,071. Bits 5 to 0 of byte 6 are custom flags 6 to 1. SCID doesn't use bit 6, and keeps it.

```text
length = (byte6 & 0x80) << 9 | u16(bytes 4-5)
```

Two SCID limits differ from the 17 bits. SCID's encoder accepts a record of exactly 131,072 bytes, whose length is then stored as 0, so SCID can't read the game back. A record of 131,073 bytes or more is refused.

### Flags

| Bit | Mask | Meaning | SCID's letter |
| --- | --- | --- | --- |
| 0 | `0001` | Starts from a position other than the standard one | |
| 1 | `0002` | Has a promotion in the main line | |
| 2 | `0004` | Has a promotion to a rook, bishop, or knight in the main line | |
| 3 | `0008` | Marked as deleted | `D` |
| 4 | `0010` | White opening | `W` |
| 5 | `0020` | Black opening | `B` |
| 6 | `0040` | Middlegame | `M` |
| 7 | `0080` | Endgame | `E` |
| 8 | `0100` | Novelty | `N` |
| 9 | `0200` | Pawn structure | `P` |
| 10 | `0400` | Tactics | `T` |
| 11 | `0800` | Kingside play | `K` |
| 12 | `1000` | Queenside play | `Q` |
| 13 | `2000` | Brilliancy or good play | `!` |
| 14 | `4000` | Blunder or bad play | `?` |
| 15 | `8000` | User-defined | `U` |

SCID derives bits 0 to 2 from the game; the rest are set by the user. Custom flags 1 to 6, letters `1` to `6`, are in [byte 6](#record-length-and-custom-flags).

### Name IDs

```text
 byte 9                          byte 14
  7  6  5  4    3  2  1  0        7  6  5   4  3  2   1  0
┌─────────────┬─────────────┐   ┌─────────┬─────────┬───────┐
│ White 19-16 │ Black 19-16 │   │ event   │ site    │ round │
│             │             │   │ 18-16   │ 18-16   │ 17-16 │
└─────────────┴─────────────┘   └─────────┴─────────┴───────┘
```

| ID | Bits | High bits | Low 16 bits | Most names of the kind |
| --- | --- | --- | --- | --- |
| White, Black (player) | 20 | Byte 9, bits 7–4 and 3–0 | Bytes 10–11 and 12–13 | 1,048,575 |
| Event | 19 | Byte 14, bits 7–5 | Bytes 15–16 | 524,287 |
| Site | 19 | Byte 14, bits 4–2 | Bytes 17–18 | 524,287 |
| Round | 18 | Byte 14, bits 1–0 | Bytes 19–20 | 262,143 |

An ID is valid if it is less than the number of names of its kind in the [name file](SN4_FORMAT.md#header-sections-ids-and-frequencies). SCID refuses to open a database with an invalid one.

### Result and counts

```text
 15      12 11       8 7        4 3        0
┌──────────┬──────────┬──────────┬──────────┐
│ result   │ NAGs     │ comments │variations│
└──────────┴──────────┴──────────┴──────────┘
```

Results: 0 `*`, 1 `1-0`, 2 `0-1`, 3 `1/2-1/2`. SCID writes no other value.

Each count is a code, not the count itself. Variations at every depth are counted, and comments and NAGs in variations too.

| Count | 0 to 10 | 11–12 | 13–17 | 18–24 | 25–34 | 35–44 | 45 or more |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Code | The count | 10 | 11 | 12 | 13 | 14 | 15 |
| SCID reads | The count | 10 | 15 | 20 | 30 | 40 | 50 |

### ECO code

0 means none. Otherwise, for a code such as `B36f`, with letter *L* (`A` = 0 to `E` = 4), number *n* (0 to 99), and subcode:

```text
code = (L × 100 + n) × 131 + s + 1

s = 0                                    no subcode, as in B36
s = 1 + (letter - 'a') × 5               one letter, as in B36f
s = 1 + (letter - 'a') × 5 + digit       letter and digit 1 to 4, as in A00a1
```

`A00` is 1, `A00a` is 2, `A00a1` is 3, `A00z4` is 131, `A01` is 132, and `E99z4` is 65,500, the highest.

### Ratings

```text
 15      12 11                           0
┌──────────┬──────────────────────────────┐
│ type     │ rating                       │
└──────────┴──────────────────────────────┘
```

The rating has 12 bits, 0 to 4,095; 0 means none. SCID's PGN import stores at most 4,000, and SCID reads a higher stored rating as 4,000. Its other ways to save a game don't limit it: `sc_game tags set -whiteElo 4095` and `sc_game save` store 4,095. Types: 0 Elo, 1 Rating, 2 Rapid, 3 ICCF, 4 USCF, 5 DWZ, 6 ECF. SCID names no other type.

### Sources

| Rule | SCID vs. PC 4.26 | Evidence | scidtopgn |
| --- | --- | --- | --- |
| Entry layout | [`IndexEntry::Read`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.cpp#l173), [`IndexEntry::Write`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.cpp#l239), [`INDEX_ENTRY_SIZE`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.h#l579) | Probe `fixtures`, every probe database | [`IndexEntry`](../crates/scidtopgn/src/index.rs) |
| Record length | [`GetLength`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.h#l263), [`SetLength`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.h#l354) | Probes `long`, `blocks`, `size131071` | [`get_length`](../crates/scidtopgn/src/index.rs) |
| Length 131,072 and over | [`MAX_GAME_LENGTH`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.h#l28), [`MAX_GAME_LENGTH`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4961) | Probes `size131072`, `size131073` | Differs: [refuses both](COMPATIBILITY.md#limits-shared-with-scid) |
| Flags | [`IDX_FLAG_START`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.h#l87), [`GetFlag`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.h#l295), [`IndexEntry::CharToFlag`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.cpp#l295) | Probe `flags`, which sets each user flag; `tests/data/derived` | [`get_flag`](../crates/scidtopgn/src/index.rs), [`get_custom_flag`](../crates/scidtopgn/src/index.rs) |
| Name IDs | [`IndexEntry::GetWhite`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.h#l484) to [`IndexEntry::GetRound`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.h#l558), [`NAME_MAX_ID`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/namebase.h#l46) | Probes `names66000`, `limits` | [`get_white`](../crates/scidtopgn/src/index.rs) to [`get_round`](../crates/scidtopgn/src/index.rs) |
| Invalid ID | [`IndexEntry::Verify`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.cpp#l95), [`VerifyFile`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/tkscid.cpp#l1038) | Probe `badname` | Differs: [reads the name as `?`](COMPATIBILITY.md#files-scid-refuses-or-misreads) |
| Counts and result | [`EncodeCount`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.h#l329), [`DecodeCount`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.h#l338), [`GetResult`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.h#l286) | Probes `counts` (0 to 55 of each), `fixtures` | [`encode_count`](../crates/scidtopgn/src/index.rs), [`get_result`](../crates/scidtopgn/src/index.rs) |
| ECO code | [`eco_FromString`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/misc.cpp#l71), [`eco_ToString`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/misc.cpp#l113) | Probe `ratings` | [`eco_from_string`](../crates/scidtopgn/src/index.rs) |
| Ratings | [`MAX_ELO`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/pgnparse.cpp#l463) (import), [`sc_game_tags_set`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/tkscid.cpp#l9928) and [`SetWhiteElo`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.h#l376) (no limit), [`MAX_ELO`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.cpp#l205) (reading), [`MAX_ELO`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.h#l46), [`ratingTypeNames`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l179) | Probes `ratings`, `elo4095` | [`MAX_ELO`](../crates/scidtopgn/src/index.rs), [`RatingType`](../crates/scidtopgn/src/common.rs) |

## Dates and event dates

Bytes 25–28 hold the event date in the top 12 bits and the game's date in the low 20.

```text
 31    29 28     25 24      20 19               9 8      5 4       0
┌────────┬─────────┬──────────┬──────────────────┬────────┬─────────┐
│ event  │ event   │ event    │ year             │ month  │ day     │
│ year   │ month   │ day      │                  │        │         │
│ 3      │ 4       │ 5        │ 11               │ 4      │ 5       │
└────────┴─────────┴──────────┴──────────────────┴────────┴─────────┘
```

**Date.** The year is 0 to 2047, the month 0 to 12, and the day 0 to 31. 0 means the part is unknown. The parts aren't checked against each other, so a date such as February 31 can be stored.

```text
date = year << 9 | month << 5 | day
2022.12.19 = 0xFCD93 = 1,035,667
```

**Event date.** The month and day are stored as they are. The year is stored as a 3-bit code relative to the game's year:

```text
code = event year - game year + 4        1 to 7, for -3 to +3 years
event year = game year + code - 4
```

Code 0 means that there is no event date. So an event date whose year is unknown or more than three years from the game's year can't be stored, and a writer stores 0 in all 12 bits. An event date's month and day can be unknown.

SCID computes `game year - 3` as an unsigned number, so it also stores no event date for a game whose year is unknown, 1, or 2.

`tests/data/one.si4`, offset 207 (entry offset 25):

```text
98 ff cd 93 = 1001 1000 1111 1111 1100 1101 1001 0011

event year code  100   = 4   2022 + 4 - 4 = 2022
event month     1100   = 12
event day      01111   = 15                        2022.12.15
year     11111100110   = 2022
month           1100   = 12
day            10011   = 19                        2022.12.19
```

**PGN import.** How PGN date text becomes these parts isn't part of the format. SCID's reader stores a year over 2047 as 0, a month over 12 as 0 with the day, and a day over 31 as 0.

| Rule | SCID vs. PC 4.26 | Evidence | scidtopgn |
| --- | --- | --- | --- |
| Packing | [`DATE_MAKE`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/date.h#l44), [`YEAR_MAX`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/date.h#l42), [`GetDate`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.h#l280) | Probe `dates` | [`Date`](../crates/scidtopgn/src/date.rs), [`get_date`](../crates/scidtopgn/src/index.rs) |
| Event date | [`IndexEntry::GetEventDate`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.cpp#l120), [`IndexEntry::SetEventDate`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.cpp#l137) | Probe `dates`: 2015 to 2025 for a game in 2020, and game years 0 to 3 | [`get_event_date`](../crates/scidtopgn/src/index.rs), [`set_event_date`](../crates/scidtopgn/src/index.rs) |
| PGN dates | [`date_EncodeFromString`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/date.cpp#l63) | Probe `dates` | [`from_string`](../crates/scidtopgn/src/date.rs). Differs: [dates that aren't all digits, and months above 12](COMPATIBILITY.md#known-differences-outside-these-cases) |

## Search fields and derived game metadata

SCID computes these fields from the main line when it stores a game, and its searches use them to skip games. Nothing in the game file depends on them.

### Final material and stored-line code

```text
 31      24 23 22 21 20 19 18 17 16 15     12 11 10 9  8 7  6 5  4 3      0
┌──────────┬─────┬─────┬─────┬─────┬─────────┬─────┬────┬────┬────┬────────┐
│ stored   │ WQ  │ WR  │ WB  │ WN  │ WP      │ BQ  │ BR │ BB │ BN │ BP     │
│ line     │     │     │     │     │         │     │    │    │    │        │
└──────────┴─────┴─────┴─────┴─────┴─────────┴─────┴────┴────┴────┴────────┘
```

The low 24 bits count the pieces at the end of the main line. Queens, rooks, bishops, and knights count at most 3 each, and a higher count is stored as 3. Pawns have 4 bits.

The top 8 bits are the code of a stored line: one of 254 openings in SCID's source, numbered from 1. It is the longest line whose moves all start the game, comparing from- and to-squares, and the first of those with the same length. 0 means that no line matches or that the game doesn't start from the standard position.

In `tests/data/one.si4`, offset 215, `9c 29 72 87` is stored line 156, and 2 rooks, 2 bishops, 1 knight, and 7 pawns against 2 rooks, 2 bishops, and 7 pawns.

### Half-moves and home pawns

```text
 byte 37            byte 38                        byte 39           bytes 40-46
                      7  6     5 ... 0               7..4    3..0
┌──────────────┐   ┌────────┬──────────────────┐  ┌───────┬───────┐ ┌─────────────┐
│ half-moves   │   │ bits   │ number of        │  │ 1st   │ 2nd   │ │ 3rd to 16th │
│ bits 7-0     │   │ 9-8    │ pawns listed     │  │ pawn  │ pawn  │ │ pawn        │
└──────────────┘   └────────┴──────────────────┘  └───────┴───────┘ └─────────────┘
```

**Half-moves** are the moves of the main line, stored modulo 1,024. 1,025 half-moves are stored as 1.

**Home pawns** list the pawns in the order they leave their starting squares, by moving or by being captured. Each has 4 bits, and unused ones are 0:

| Value | 15 | 14 | 13 | 12 | 11 | 10 | 9 | 8 | 7 | 6 | 5 | 4 | 3 | 2 | 1 | 0 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Pawn | a2 | b2 | c2 | d2 | e2 | f2 | g2 | h2 | a7 | b7 | c7 | d7 | e7 | f7 | g7 | h7 |

A game that doesn't start from the standard position lists none, and its promotion flags and half-moves are still set from its moves.

In `tests/data/one.si4`, offset 219, `25 09 b5 c1 d4 ae 70` is 37 half-moves and 9 pawns: e2, c7, d2, g7, c2, d7, f2, b2, a7. In `tests/data/derived.si4`, offset 219, `2e 43` is 0x12E = 302 half-moves and 3 pawns.

| Rule | SCID vs. PC 4.26 | Evidence | scidtopgn |
| --- | --- | --- | --- |
| Material | [`SHIFT_WQ`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/matsig.h#l47), [`matsig_Make`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/matsig.cpp#l92) | Every probe game, derived again from its moves | [`material_sig`](../crates/scidtopgn/src/index.rs) |
| Stored lines | [`storedLineText`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/stored.cpp#l49), [`storedLineCode`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4930) | All 254 lines, each imported by `pgnscid`; `tests/data/stored` | [`code`](../crates/scidtopgn/src/stored_line.rs) |
| Half-moves | [`NumHalfMoves`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.cpp#l218), [`NumHalfMoves`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/index.cpp#l279) | Probe `moves`: 0 to 2,049 | [`MAX_HALF_MOVES`](../crates/scidtopgn/src/index.rs) |
| Home pawns | [`Game::MakeHomePawnList`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l1379), [`Position::GetHPSig`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/position.cpp#l536) | Every probe game, derived again from its moves | [`HomePawnList`](../crates/scidtopgn/src/index.rs) |
