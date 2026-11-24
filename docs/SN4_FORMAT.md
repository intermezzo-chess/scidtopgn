# Name file (.sn4)

The name file holds the names of a database's players, events, sites, and rounds, each once. [Index entries](SI4_FORMAT.md#name-ids) refer to them by ID.

[SCID_FORMAT.md](SCID_FORMAT.md) states which version this describes, its terms, and [how each rule was checked](SCID_FORMAT.md#sources-and-verification). Every integer is unsigned and big-endian. "SCID" means SCID vs. PC 4.26.

```text
offset 0          36
┌────────────────┬─────────┬────────┬───────┬────────┐
│ header, 36     │ players │ events │ sites │ rounds │   each section: its name records
└────────────────┴─────────┴────────┴───────┴────────┘
```

The sections follow each other without padding or offsets, so a reader has to read each record to find the next section.

## Header, sections, IDs, and frequencies

```text
 0               8          12     15     18     21     24     27     30     33     36
┌───────────────┬──────────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┬──────┐
│ magic         │timestamp │player│event │site  │round │player│event │site  │round │
│ 8             │ 4        │count │count │count │count │max   │max   │max   │max   │
└───────────────┴──────────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┴──────┘
                             3 bytes each: number of names, then maximum frequency
```

| Offset | Bytes | Field | Values |
| --- | --- | --- | --- |
| 0 | 8 | Magic | `Scid.sn` and a NUL: `53 63 69 64 2e 73 6e 00` |
| 8 | 4 | Timestamp | Not used. `pgnscid` writes 0 |
| 12 | 3 | Number of players | 0 to 1,048,575 |
| 15 | 3 | Number of events | 0 to 524,287 |
| 18 | 3 | Number of sites | 0 to 524,287 |
| 21 | 3 | Number of rounds | 0 to 262,143 |
| 24 | 3 | Maximum player frequency | 0 to 16,777,215 |
| 27 | 3 | Maximum event frequency | 0 to 16,777,215 |
| 30 | 3 | Maximum site frequency | 0 to 16,777,215 |
| 33 | 3 | Maximum round frequency | 0 to 16,777,215 |
| **Total** | **36** | | |

There is no version field. The index file's version covers all three files.

### Name records

```text
┌─────────┬───────────┬────────┬───────────────┬─────────────────┐
│ ID      │ frequency │ length │ prefix length │ suffix          │
│ 2 or 3  │ 1, 2 or 3 │ 1      │ 1, not in the │ length − prefix │
│         │           │        │ first record  │ length          │
└─────────┴───────────┴────────┴───────────────┴─────────────────┘
```

| Field | Bytes | Decided by |
| --- | --- | --- |
| ID | 2 if the section has fewer than 65,536 names, otherwise 3 | The section's number of names |
| Frequency | 1 if the section's maximum frequency is at most 255, 2 if it is at most 65,535, otherwise 3 | The header's maximum frequency, even if no name in the section has it |
| Length | 1 | The whole name's length in bytes, 0 to 255 |
| Prefix length | 1, or 0 in the first record of each section | How many bytes to take from the start of the previous name |
| Suffix | Length minus prefix length | The rest of the name's bytes |

A section with fewer than 256 names still has 2-byte IDs: only frequencies can take 1 byte.

| Names in the section | ID bytes | | Maximum frequency | Frequency bytes |
| --- | --- | --- | --- | --- |
| 255 | 2 | | 255 | 1 |
| 256 | 2 | | 256 | 2 |
| 65,535 | 2 | | 65,535 | 2 |
| 65,536 | 3 | | 65,536 | 3 |

`tests/data/one.sn4`, the whole file:

```text
0x00  53 63 69 64 2e 73 6e 00                          magic
0x08  00 00 00 00                                      timestamp
0x0c  00 00 02  00 00 01  00 00 01  00 00 01           2 players, 1 event, 1 site, 1 round
0x18  00 00 01  00 00 01  00 00 01  00 00 01           maximum frequencies
      ID     freq len prefix
0x24  00 00  01   0d         "Hossain, Enam"           first player: no prefix length
0x35  00 01  01   0a  00     "Murshed, N"
0x44  00 00  01   19         "47th ch-Bangahbandhu 2022"
0x61  00 00  01   09         "Dhaka BAN"
0x6e  00 00  01   03         "5.6"
```

**IDs** number a section's names from 0. Each is less than the number of names, and no two are the same. They don't follow the order of the records: SCID gives a name the next ID when it is added, and a new name's record can come anywhere. `tests/data/five.sn4` lists its players with IDs 2, 9, 0, 6, 7, 3, 1, 5, 8, 4.

**Frequencies** count the uses of a name by index entries, counting deleted games, and counting a player twice for a game against itself. A name no game uses can stay, with frequency 0.

**SCID's behavior.** SCID doesn't rely on the stored frequencies: it counts them again when it opens a database, unless a fast open is requested. The maximum frequency rises with a name's frequency, but SCID doesn't lower it when that frequency falls. SCID doesn't check that a frequency fits in 3 bytes: at 16,777,216, its maximum wraps to 0 in the header while the records are written with 3-byte frequencies, and reading the file back is undefined. SCID saved a game of a player against themselves to a database of 8,388,607 such games without an error, and then crashed when it opened the database. scidtopgn [writes the same](COMPATIBILITY.md#append).

| Rule | SCID vs. PC 4.26 | Evidence | scidtopgn |
| --- | --- | --- | --- |
| Header layout | [`NameBase::WriteHeader`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/namebase.cpp#l74), [`NameBase::OpenNameFile`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/namebase.cpp#l99) | Probe `fixtures`, every probe database | [`read_sn4`](../crates/scidtopgn/src/namebase.rs), [`write_sn4`](../crates/scidtopgn/src/namebase.rs) |
| Record layout | [`NameBase::ReadNameFile`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/namebase.cpp#l149), [`NameBase::WriteNameFile`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/namebase.cpp#l259) | Every probe database | The same functions |
| ID width | [`numNames`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/namebase.cpp#l182), [`numNames`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/namebase.cpp#l282) | Probes `names255`, `names256`, `names65535`, `names65536`, `names66000` | The same functions |
| Frequency width | [`maxFrequency`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/namebase.cpp#l191), [`maxFrequency`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/namebase.cpp#l289) | The same probes, with those numbers as frequencies; `maxfreq` for a maximum no name has | The same functions |
| IDs | [`numNames`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/namebase.cpp#l384) | Every probe database | [`add_name`](../crates/scidtopgn/src/namebase.rs). Differs: [names can get new IDs](COMPATIBILITY.md#names-that-arent-utf-8) |
| Most names | [`NAME_MAX_ID`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/namebase.h#l46), [`NAME_MAX_ID`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/namebase.cpp#l361) | Probe `limits` | [`max_id`](../crates/scidtopgn/src/namebase.rs) |
| Frequencies | [`NameBase::IncFrequency`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/namebase.h#l254), [`recalcNameFrequencies`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/tkscid.cpp#l310), [`fastOpen`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/tkscid.cpp#l1047) | Probes `names*`, `limits`, `frequency`; opening `tests/data/castling`, whose frequencies are 0, with and without `-fast` | [`increment_frequency`](../crates/scidtopgn/src/namebase.rs). Differs: [doesn't count again](COMPATIBILITY.md#append) |
| Timestamp | [`SetTimeStamp`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/pgnscid.cpp#l256) | Every probe database has 0 | Kept as stored |

## Front coding and name ordering

Each name is stored as the bytes it shares with the previous name of its section, and the rest. The prefix is of the previous name's **bytes as stored**, so it can end inside a character, and only the whole name's bytes can be decoded as text.

Illustrative pseudocode, which checks the bounds a damaged file can break:

```text
previous = empty
for i in 0 .. number of names - 1:
    id        = read_uint(3 if number of names >= 65536 else 2)
    frequency = read_uint(3 if maximum >= 65536 else 2 if maximum >= 256 else 1)
    length    = read_uint(1)
    prefix    = read_uint(1) if i > 0 else 0
    fail if id >= number of names, or an earlier name has it
    fail if prefix > length or prefix > length of previous
    name      = first `prefix` bytes of previous, then read_bytes(length - prefix)
    previous  = name
```

`read_uint` and `read_bytes` fail at the end of the file. SCID's reader doesn't check any of these bounds.

`tests/data/accents.sn4`, offset 0x73, has a prefix that ends inside a character. `ü` is `c3 bc`, `Ä` is `c3 84`, and `É` is `c3 89`:

```text
offset  ID     freq len prefix suffix
0x73    00 04  01   0c  00     c3 84 "rger, Hans"    Ärger, Hans
0x84    00 08  01   06  01     89 "mile"             Émile: the prefix is c3
```

### Order

A section's names are in this order:

1. By first byte, as an unsigned number. An empty name's first byte is 0.
2. Among names with the same first byte, by comparing bytes as **signed** numbers, with a 0 after the last byte of each name.

So in a name after its first byte, a byte of `80` or more comes before the end of a shorter name, which comes before ASCII. But a name that starts with such a byte comes after every name that starts with ASCII. `tests/data/accents.sn4` lists its players in this order:

```text
Abc, Bé, Blübaum, Matthias, Bl, Blagojević, Dragisa, Bz, Zed, Ärger, Hans, Émile, Øst, Ole
```

Two names of a section can't be the same.

**Platform.** The second rule is C++ `char` comparison on a platform where `char` is signed, such as x86 and Apple arm64. Built with an unsigned `char`, the default for example on Linux on arm64, SCID compares the bytes as unsigned and refuses a file in the order above. A file is portable between the two only if its names don't depend on the difference. scidtopgn and the compatibility check use signed comparison everywhere.

**SCID's behavior.** SCID refuses to open a file whose names break the second rule, or that has the same name twice in a row ("Error opening name file"). A file that breaks only the first rule opens, with the warning "namebase is denormalized". SCID's name lookups then search the wrong part of the list.

| Rule | SCID vs. PC 4.26 | Evidence | scidtopgn |
| --- | --- | --- | --- |
| Front coding | [`nameStr`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/namebase.cpp#l208), [`strPrefix`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/misc.cpp#l386) | Probe `namecases`, `tests/data/accents` | [`read_sn4`](../crates/scidtopgn/src/namebase.rs), [`compute_prefix`](../crates/scidtopgn/src/namebase.rs) |
| Order within a first byte | [`StrTree<C>::AddLast`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/strtree.h#l485), [`strCompare_INLINE`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/misc.h#l193) | Probes `order`, `signed`, `unsigned`, `duplicate`, with both kinds of `char` | [`scid_order`](../crates/scidtopgn/src/namebase.rs). Differs when reading: [any order is accepted](COMPATIBILITY.md#files-scid-refuses-or-misreads) |
| Order of first bytes | [`StrTree<C>::MakeList`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/strtree.h#l273), [`StrTree<C>::MakeTree`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/strtree.h#l340) | Probes `firstbyte`, `limits` | [`scid_order`](../crates/scidtopgn/src/namebase.rs) |

## Text encoding and legacy names

- A name is a sequence of 0 to 255 bytes. The file doesn't record or check an encoding.
- `pgnscid` and SCID's PGN import convert PGN text that isn't UTF-8 to UTF-8, with the encoding they detect. A name that is then longer than 255 bytes is cut there, inside a character if need be, so even their files can hold a name that isn't valid UTF-8. Other programs can store other encodings, and one file can hold several. The `latin1` and `mixed` fixtures, written by the reference converter in `scid-cpp`, hold such names.
- Names are read as stored. Control characters are kept, and nothing is filtered or trimmed, so names that differ only by a control character or a space are different names.
- A name can be empty.
- SCID holds names as C strings, so a byte of 0 in a name ends it when SCID reads it. SCID never writes one.

How a program shows or converts bytes that aren't UTF-8 isn't part of the format. scidtopgn's rules are in [COMPATIBILITY.md](COMPATIBILITY.md#names-that-arent-utf-8). How PGN tags become names, including trimming, is in [Player names and ratings](COMPATIBILITY.md#player-names-and-ratings).

| Rule | SCID vs. PC 4.26 | Evidence | scidtopgn |
| --- | --- | --- | --- |
| Bytes kept as stored | [`readString`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/namebase.cpp#l218), [`writeString`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/namebase.cpp#l303) | Probes `namecases`, `controls`, `latin1` and `mixed` fixtures | [`read_sn4`](../crates/scidtopgn/src/namebase.rs). Differs: [bytes that aren't UTF-8 are converted](COMPATIBILITY.md#reading) |
| Conversion on import | [`PgnParser::DoCharsetConversion`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/pgnparse.cpp#l1350), [`PgnParser::ConvertToUTF8`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/pgnparse.cpp#l1310) | `pgnscid` run on Latin-1 names, one of which it cut inside a character | Differs: [read as Windows-1252](COMPATIBILITY.md#known-differences-outside-these-cases) |
| Length limit | [`strlen`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/namebase.cpp#l374) | Probe `namecases`: 255 bytes | [`stored_name`](../crates/scidtopgn/src/namebase.rs). Differs: [a cut never splits a character](COMPATIBILITY.md#general-differences) |
| NUL ends a name | [`nameStr`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/namebase.cpp#l221) | A name with a NUL that scidtopgn wrote, read by SCID | Differs: [the whole name is read](COMPATIBILITY.md#reading) |
