# Game file (.sg4)

The game file holds each game's record: the tags the [index](SI4_FORMAT.md) has no field for, the starting position if it isn't the standard one, the moves with their variations and NAGs, and the comments. It has no header.

[SCID_FORMAT.md](SCID_FORMAT.md) states which version this describes, its terms, and [how each rule was checked](SCID_FORMAT.md#sources-and-verification). "SCID" means SCID vs. PC 4.26.

## Blocks, offsets, and record boundaries

```text
offset 0                                131072                   262144
┌──────────┬──────────┬──────┬─────────┬──────────┬─────────────┬───────────
│ record   │ record   │ ...  │ zeros   │ record   │ zeros       │ record ...
└──────────┴──────────┴──────┴─────────┴──────────┴─────────────┴───────────
└────────────── block 0 ───────────────┘└────── block 1 ───────┘└─ last block, which ends
                                                                   after its last record
```

- The file is divided into blocks of 131,072 bytes. The last block can be shorter.
- A record is wholly inside one block. A writer that can't fit a record in the rest of the last block fills the block with zeros and starts the record at the next one.
- An [index entry](SI4_FORMAT.md#game-entries-and-packed-fields) gives a record's offset, in 32 bits, and its length, 1 to 131,071 bytes. Nothing in the game file marks where a record starts, and records can be in any order.
- Bytes that no entry refers to are unused: the padding, and the old record of a game that was replaced. They stay until the database is compacted.

**A record's length has to come from the index.** A record can't be found by scanning for a byte: 0 and 15, which end its parts, can also be part of a string, a NAG, or a flag, and comments come after the end of the moves.

**SCID's behavior.** SCID reads a record only if its first and last bytes are in the same block, and reports the game as corrupt otherwise. A record of length 0 is corrupt too. Bytes after the last comment inside a record's length are ignored.

| Rule | SCID vs. PC 4.26 | Evidence | scidtopgn |
| --- | --- | --- | --- |
| Blocks and padding | [`GF_BLOCKSIZE`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/gfile.h#l25), [`GFile::AddGame`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/gfile.cpp#l265), [`GFile::Flush`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/gfile.cpp#l169) | Probe `blocks`: 8 records in 5 blocks, with the padding checked | [`add_game`](../crates/scidtopgn/src/gfile.rs) |
| Reading a record | [`GFile::ReadGame`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/gfile.cpp#l240) | Probes `cross`, `zero`, `truncated`, `after` | [`read_game`](../crates/scidtopgn/src/gfile.rs). Differs: [reads a record that crosses a block](COMPATIBILITY.md#files-scid-refuses-or-misreads) |
| Replaced records | [`gfile->AddGame`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/tkscid.cpp#l8877) | [`update-*` scenarios](SCID_COMPAT.md#scenarios) | [The same](COMPATIBILITY.md#update) |

## Tags and starting positions

```text
┌─────────────────────┬────┬───────┬───────────────────┬───────────────────┬───────────────┐
│ tags                │ 0  │ flags │ FEN and a NUL,    │ moves, NAGs, and  │ comments, a   │
│                     │ 1  │ 1     │ if flags bit 0    │ markers, ending 15│ NUL after each│
└─────────────────────┴────┴───────┴───────────────────┴───────────────────┴───────────────┘
```

A record has no padding between its parts, and each is read from the start, because the length of a part isn't stored.

### Tags

Each tag starts with a byte that is also its name's length or code:

| First byte | What follows |
| --- | --- |
| 0 | Nothing: the tags end |
| 1 to 240 | The name, of that many bytes, then the value's length (1 byte) and the value |
| 241 to 250 | The value's length and the value. The code stands for the name, below |
| 251 to 254 | Spare. SCID writes none, and skips a length and a value after one, but see below |
| 255 | An event date in 3 bytes, big-endian, as [packed in the index](SI4_FORMAT.md#dates-and-event-dates) (20 bits). SCID reads it but never writes it |

| Code | Name | | Code | Name |
| --- | --- | --- | --- | --- |
| 241 `f1` | `WhiteCountry` | | 246 `f6` | `Opening` |
| 242 `f2` | `BlackCountry` | | 247 `f7` | `Variation` |
| 243 `f3` | `Annotator` | | 248 `f8` | `Setup` |
| 244 `f4` | `PlyCount` | | 249 `f9` | `Source` |
| 245 `f5` | `EventDate`, as text | | 250 `fa` | `SetUp` |

A value is 0 to 255 bytes. Names and values aren't terminated, and no encoding is recorded. When SCID imports PGN, what the index holds isn't stored as a tag: the names, the date, the result, the ECO code, the event date, and each player's rating. Once a player has a rating other than 0, a further rating tag of theirs is stored as a tag, such as a `WhiteElo` after a `WhiteUSCF` ([`GetWhiteElo`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/pgnparse.cpp#l457), [Player names and ratings](COMPATIBILITY.md#player-names-and-ratings)), as is any other tag, apart from `FEN`, `ScidFlags`, and, with `pgnscid`, `PlyCount`.

`tests/data/one.sg4`, offset 0:

```text
0a "WhiteTitle"  02 "GM"          a name of 10 bytes, a value of 2
0a "BlackTitle"  02 "GM"
f6               08 "Sicilian"    Opening
f7               2c "accelerated fianchetto, Gurgenidze variation"
0b "WhiteFideId" 08 "10200649"
0b "BlackFideId" 08 "10200010"
00                                end of the tags, at 0x7e
```

**SCID's behavior.** In SCID 4.26, reading a tag with a value of 255 bytes overruns a buffer. What happens is undefined: in the build the probe used, the tag's name was lost. Values up to 254 bytes are safe. SCID writes a longer value with a length that has wrapped around, and all its bytes. The bytes past that length are then read as more tags, so decoding fails, or succeeds with other tags. Its importers drop a value longer than 255 bytes before converting it to UTF-8, which can make it longer: from 200 Latin-1 `é`s, `pgnscid` wrote a record that SCID can't decode, and from another 254 bytes, which became 256, one that it decodes with an empty value and a tag that wasn't in the PGN. Decoding a spare code (251 to 254) with tags stored crashes SCID. SCID holds 40 tags, and checks this only with an assertion, which its normal build leaves out: reading a record with 41 tags, or importing a game with 41, crashed the builds the probe used. Skipping tags, as its searches do, handles all three.

### Flags and starting position

```text
  7  6  5  4  3    2              1           0
┌───────────────┬──────────────┬───────────┬──────────────────┐
│ 0             │ underpromo-  │ promotion │ nonstandard start│
│               │ tion         │           │                  │
└───────────────┴──────────────┴───────────┴──────────────────┘
```

The bits mean the same as [bits 0 to 2 of the index entry's flags](SI4_FORMAT.md#flags). SCID writes the other bits as 0, and ignores them.

If bit 0 is set, a FEN follows, ending with a NUL. SCID writes all six fields, and reads a FEN without its last fields too. Without it, the game starts from the standard position.

| Rule | SCID vs. PC 4.26 | Evidence | scidtopgn |
| --- | --- | --- | --- |
| Record parts | [`Game::Encode`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4868), [`Game::Decode`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l5097) | Every probe game | [`encode_game`](../crates/scidtopgn/src/database.rs), [`decode`](../crates/scidtopgn/src/game.rs) |
| Which tags are stored | [`PgnParser::ExtractPgnTag`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/pgnparse.cpp#l312), [`AddIgnoredTag`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/pgnscid.cpp#l188) | `pgnscid` and `sc_base import` run on such tags | Differs: [`ScidFlags`, `PlyCount`](COMPATIBILITY.md#known-differences-outside-these-cases) |
| Tags | [`encodeTags`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4694), [`Game::DecodeTags`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4729), [`skipTags`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4774), [`commonTags`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4668) | Probes `tags`, `random`, `compactdate` | [`encode_tags`](../crates/scidtopgn/src/database.rs), [`decode_tags`](../crates/scidtopgn/src/game.rs) |
| 255-byte value | [`value`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4733) | Probe `tag255` | Differs: [reads it](COMPATIBILITY.md#files-scid-refuses-or-misreads) |
| Spare codes | [`AddPgnTag`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4753) | Probe `spare` | Differs: [drops the value](COMPATIBILITY.md#files-scid-refuses-or-misreads) |
| Most tags | [`MAX_TAGS`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.h#l105), [`Game::AddPgnTag`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l763) | Probes `tags40`, `tags41`, `manytags` | [`add_tag`](../crates/scidtopgn/src/game.rs). Differs: [keeps the first 40](COMPATIBILITY.md#files-scid-refuses-or-misreads) |
| Flags and FEN | [`flags`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4879), [`gflags`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l5113), [`Position::PrintFEN`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/position.cpp#l3013), [`Position::ReadFromFEN`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/position.cpp#l2850) | Probes `random`, `edge`, `flags8` | [`record_flags`](../crates/scidtopgn/src/database.rs), [`decode_start_position`](../crates/scidtopgn/src/game.rs). Differs: [FEN fields that don't fit the position are dropped](COMPATIBILITY.md#results-and-the-start-position) |

## Piece numbering and position tracking

A move names its piece by number, so a reader has to track each side's numbered pieces through the game, and through each variation.

### Numbers in the standard position

| Number | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 to 15 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| White | Ke1 | Ra1 | Nb1 | Bc1 | Qd1 | Bf1 | Ng1 | Rh1 | Pawns a2 to h2 |
| Black | Ke8 | Ra8 | Nb8 | Bc8 | Qd8 | Bf8 | Ng8 | Rh8 | Pawns a7 to h7 |

### Numbers in a position from a FEN

Number each side's pieces from 0 in FEN order: a8 to h8, then a7 to h7, and so on to h1. When the king comes, give it number 0, and give its number to the piece that had 0. Each side has one king and at most 16 pieces.

```text
rank 8 of a FEN: r3k2r      as read      a8 rook 0, e8 king 1, h8 rook 2
                            the king     e8 king 0, a8 rook 1, h8 rook 2
```

### Changes

| Event | Change |
| --- | --- |
| Move | The piece keeps its number |
| Capture | The captured piece's number goes to the piece of its side with the **highest** number, and the side has one piece fewer. If the captured piece has the highest number, no other changes |
| En passant | The same, for the captured pawn, which isn't on the destination square |
| Promotion | The new piece keeps the pawn's number |
| Castling | The king and the rook keep their numbers |
| Null move | None |

```text
number   ...  10   11   ...  14   15            number   ...  10   11   ...  14
Black    ...  c7   d5   ...  g7   h7     →      Black    ...  c7   h7   ...  g7
              White captures on d5
```

A variation starts from the position **before** the move it replaces, with the numbers the pieces had then. A reader can copy the position before each move, or undo the move and its renumbering exactly, as SCID does.

A number at or above the side's number of pieces is invalid. SCID doesn't check it: its list can still hold the square of a piece that was captured or renumbered.

| Rule | SCID vs. PC 4.26 | Evidence | scidtopgn |
| --- | --- | --- | --- |
| Standard position | [`Position::StdStart`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/position.cpp#l620) | Every probe game from it | [`standard`](../crates/scidtopgn/src/position.rs) |
| FEN | [`Position::AddPiece`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/position.cpp#l700), [`AddPiece`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/position.cpp#l2898) | Probes `random` (200 games), `edge` | [`from_board`](../crates/scidtopgn/src/position.rs) |
| Changes | [`Position::DoSimpleMove`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/position.cpp#l1804) | Probe `random`: 600 games with captures, en passant, promotions, and castling | [`PieceList`](../crates/scidtopgn/src/position.rs) |
| Variations | [`Game::AddVariation`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l1036), [`Position::UndoSimpleMove`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/position.cpp#l1956) | Probes `random`, `comments`, `edge` | [`decode_variation`](../crates/scidtopgn/src/game.rs) |
| Invalid number | [`from`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4499) | Probe `stalepiece` | [`decode_move`](../crates/scidtopgn/src/mov.rs). Differs: [refused](COMPATIBILITY.md#files-scid-refuses-or-misreads) |

## Piece-specific move encoding

```text
  7        4 3        0
┌───────────┬──────────┐
│ piece     │ value    │      a second byte follows only for a diagonal queen move
│ number    │          │
└───────────┴──────────┘
```

Squares are numbered by rank from a1 = 0, b1 = 1, to h8 = 63: `square = rank × 8 + file`, both from 0. The value depends on the kind of piece that has the number now, so a promoted pawn's value is that of its new kind.

**King** (always number 0, so the byte is the value):

| Value | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11–15 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| To − from | 0 | −9 | −8 | −7 | −1 | +1 | +7 | +8 | +9 | −2 | +2 | Not moves |
| Meaning | Null move | SW | S | SE | W | E | NW | N | NE | O-O-O | O-O | [Markers](#variations-comments-and-annotations) |

**Knight:**

| Value | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9–15 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| To − from | Invalid | −17 | −15 | −10 | −6 | +6 | +10 | +15 | +17 | Invalid |

**Rook:** 0 to 7 is the destination's file, on the same rank. 8 to 15 is the destination's rank plus 8, on the same file.

**Bishop:** bits 0 to 2 are the destination's file. With *d* = destination file − from file, the destination is from + 9 × *d* if bit 3 is 0 (up-right or down-left), and from − 7 × *d* if it is 1 (up-left or down-right).

**Queen:** as a rook, except that a value equal to the queen's own file, 0 to 7, means a diagonal move. The next byte is then the destination plus 64, 64 to 127.

**Pawn:** White adds the difference, and Black subtracts it.

| Value | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Difference | 7 | 8 | 9 | 7 | 8 | 9 | 7 | 8 | 9 | 7 | 8 | 9 | 7 | 8 | 9 | 16 |
| Promotion | | | | Q | Q | Q | R | R | R | B | B | B | N | N | N | |

So value 0 captures toward the a-file for White and toward the h-file for Black. An en passant capture is a pawn capture onto an empty square.

`tests/data/one.sg4`, offset 0x80, the start of the moves. Each line gives the moving side's piece:

```text
cf     1. e4       piece 12, pawn e2, value 15: 12 + 16 = 28
af     1... c5     piece 10, pawn c7, value 15: 50 - 16 = 34
67     2. Nf3      piece 6, knight g1, value 7: 6 + 15 = 21
22     2... Nc6    piece 2, knight b8, value 2: 57 - 15 = 42
bf     3. d4       piece 11, pawn d2
a0     3... cxd4   piece 10, pawn c5, value 0: 34 - 7 = 27
                   White's number 11 goes to its number 15, the pawn h2
65     4. Nxd4     piece 6, knight f3, value 5: 21 + 6 = 27
                   Black's number 10 goes to its number 15, the pawn h7
...
43 60  11... Qa5   at 0x95: piece 4, queen d8, value 3, its own file: to 0x60 - 64 = 32, a5
40 4b  14... Qxd2+ at 0x9c: piece 4, queen a5, value 0, its own file: to 0x4b - 64 = 11, d2
```

The piece number is part of the byte, so 1. e4 is `cf` and 1. d4 is `bf`. `6f` isn't a pawn move: number 6 is a knight, for which 15 is invalid.

**Damaged records.** A king value over 10, an invalid knight value, or a second queen byte outside 64 to 127 makes SCID report a decoding error, and so does a bishop destination off the board. SCID checks nothing else: a move that isn't legal is played as it is written. `18` at the start of a game, a rook's move to its own square, is played as a move from a1 to a1.

| Rule | SCID vs. PC 4.26 | Evidence | scidtopgn |
| --- | --- | --- | --- |
| Move byte | [`makeMoveByte`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4137), [`decodeMove`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4492) | Every probe game; `e4`, `d4` | [`encode_move`](../crates/scidtopgn/src/mov.rs), [`decode_move`](../crates/scidtopgn/src/mov.rs) |
| King | [`encodeKing`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4147), [`decodeKing`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4179) | Probe `random`; `castling`, `comments` | [`KING_SQDIFF`](../crates/scidtopgn/src/mov.rs) |
| Knight | [`encodeKnight`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4199), [`decodeKnight`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4224) | Probes `random`, `knight15` | [`KNIGHT_SQDIFF`](../crates/scidtopgn/src/mov.rs) |
| Rook | [`encodeRook`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4238), [`decodeRook`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4262) | Probe `random` | [`rook_destination`](../crates/scidtopgn/src/mov.rs) |
| Bishop | [`encodeBishop`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4277), [`decodeBishop`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4299) | Probe `random` | [`decode_move`](../crates/scidtopgn/src/mov.rs) |
| Queen | [`encodeQueen`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4317), [`decodeQueen`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4360) | Probes `random`, `queen` | [`decode_move`](../crates/scidtopgn/src/mov.rs) |
| Pawn | [`encodePawn`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4383), [`decodePawn`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4427) | Probe `random`, `edge` | [`PAWN_SQDIFF`](../crates/scidtopgn/src/mov.rs), [`PAWN_PROMO`](../crates/scidtopgn/src/mov.rs) |
| Illegal moves | [`decodeMove`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4492) | Probe `illegal` | Differs: [refused](COMPATIBILITY.md#files-scid-refuses-or-misreads) |

## Variations, comments, and annotations

These bytes are king values that aren't moves, so they can't be confused with one:

| Byte | Meaning | What follows |
| --- | --- | --- |
| 11 `0b` | NAG of the line's last move before it | The NAG, 1 byte |
| 12 `0c` | The line's last move before it has a comment, or the line's start does if no move comes before it | Nothing: the text is after the moves |
| 13 `0d` | A variation starts | Its moves, ending with 14 |
| 14 `0e` | The variation ends | The next variation, or the line it belongs to |
| 15 `0f` | The game ends | The comments |

SCID writes a move, its NAGs, its comment marker, its variations, and then the next move of its line. A variation replaces **the move it follows**. Variations can have variations, and one can be empty of moves.

```text
                  ┌────────────── 2. Nf3 ──────────────┐
1. e4 e5 2. Nf3 ( 2. f4 exf4 ( 2... d5 3. exd5 Qxd5 ) 3. Nf3 ) ( 2. Nc3 ...
cf    cf 67    0d df    c0  0d bf      c0      4c    0e 67    0e 0d 28
                            └── replaces 2... exf4 ─┘
```

That is `tests/data/edge.sg4` from offset 2.

**NAGs.** A NAG is 1 to 255. What each means isn't part of the format; 1 to 6 are `!`, `?`, `!!`, `??`, `!?`, and `?!`.

**Comments** follow the end of the game, each ending with a NUL. There is one text for each move, or start of a line, that has at least one marker. A marker before the first move of a line is for the start of that line, and its text is the comment before the line's first move. No encoding is recorded. SCID's PGN import converts comments that aren't UTF-8 to UTF-8 ([`PgnParser::ConvertComments`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/pgnparse.cpp#l1388)).

The texts are in the order of the game's tree, not of the markers in the bytes: the start of a line, then each of its moves in turn, and after each move its variations, each in the same way. A writer that puts a move's marker before its variations, as SCID does, gives the two the same order. A reader has to use the tree, because SCID also reads a marker after the variations, as for this record:

```text
00 00                           no tags, flags
cf                              1. e4
   0d bf 0c 0e                  (1. d4, with a marker)
0c                              a marker for 1. e4, after its variation
0f
"main" 00 "variation" 00        1. e4 {main} (1. d4 {variation})
```

`tests/data/annotated.sg4`, offset 0:

```text
00                              no tags
00                              flags
0c                              comment before the first move      "Intro"
cf 0b 01                        1. e4 $1
cf 0c                           1... e5                            "after e5"
67 0b 02                        2. Nf3 $2
   0d df 0b 05 0c               (2. f4 $5                          "gambit"
      c0 0d bf 0b 01 0c 0e      exf4 (2... d5 $1)                  "counter"
      67 0e                     3. Nf3)
   0d 28 0c 0e                  (2. Nc3)                           "quiet and solid"
22 0b 0e 0b 24                  2... Nc6 $14 $36
59 0b 01 0c                     3. Bb5 $1                          "Spanish"
   0d 0c 5a 52 0b 0a 0e         (comment, 3. Bc4 Bc5 $10)          "Or"
81 0f                           3... a6, end of the game
"Intro" 00 "after e5" 00 "gambit" 00 "counter" 00 "quiet and solid" 00 "Spanish" 00 "Or" 00
```

Illustrative pseudocode for a line. `fail` means that the record is damaged, and reading past the record's length fails too. A reader that recurses like this has to limit the depth, which the format doesn't:

```text
read_line(position, depth):
    before = none
    loop:
        b = read_uint(1)
        if b == 15 and depth == 0: return          the game ends
        if b == 14 and depth > 0: return           the variation ends
        if b == 14 or b == 15: fail
        if b == 11: add NAG read_uint(1) to the last move of this line, if any
        else if b == 12: note that the last move of this line, or its start, has a comment
        else if b == 13:
            if before is none: fail                a variation replaces a move
            read_line(copy of before, depth + 1)
        else:
            before = copy of position
            play the move that starts with b on position

after the outermost read_line: read_comments(the main line)

read_comments(line):
    for the start of the line, then each move of it in order:
        if it was noted as having a comment, once or more: read a text up to a NUL
        for each variation of the move, in order: read_comments(the variation)
```

**SCID's behavior.** SCID keeps a move's first 7 NAGs that aren't 0 and ignores the rest. It drops a NAG before a line's first move. It writes a move's NAGs and at most one marker before the move's variations, but reads them after the variations too, and reads repeated markers as one. SCID counts a move's variations in 1 byte, so it can hold 255.

| Rule | SCID vs. PC 4.26 | Evidence | scidtopgn |
| --- | --- | --- | --- |
| Markers | [`ENCODE_NAG`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4460) | Every probe game | [`ENCODE_NAG`](../crates/scidtopgn/src/mov.rs) |
| Structure | [`encodeVariation`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4564), [`Game::DecodeVariation`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4617) | Probes `random`, `comments`, `varfirst`, `emptyvar`, `endvar`, `noend` | [`encode_moves`](../crates/scidtopgn/src/database.rs), [`decode_variation`](../crates/scidtopgn/src/game.rs). Differs: [variation limits](COMPATIBILITY.md#deliberate-differences) |
| NAG limit | [`Game::AddNag`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.h#l560), [`MAX_NAGS`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.h#l132) | Probes `nag0`, `nag9`, `nagfirst`, `nagafter`, `comments` | [`add_nag`](../crates/scidtopgn/src/game.rs) |
| Comments | [`encodeComments`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4802), [`decodeComments`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.cpp#l4831) | Probes `comments`, `annotated`, `twocomments`, `commentafter`, `commentorder`, `nocomment` | [`encode_comments`](../crates/scidtopgn/src/database.rs), [`decode_comments_recursive`](../crates/scidtopgn/src/game.rs) |
| Variations of one move | [`numVariations`](https://sourceforge.net/p/scidvspc/code/3570/tree/src/game.h#l147) | `pgnscid` run on 255 and 256 | [Refused above 255](COMPATIBILITY.md#general-differences) |
