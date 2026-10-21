# scidtopgn - SCID to PGN Converter

A standalone C++ CLI tool that converts SCID chess database files to PGN format and vice versa.

## Features

- ✅ Reads SCID database files (.si4, .sn4, .sg4)
- ✅ Converts all games to standard PGN format
- ✅ Writes PGN files to SCID database format
- ✅ Includes all PGN tags (Event, Site, Date, Round, White, Black, Result, ELO, ECO, etc.)
- ✅ Handles variations (with parentheses)
- ✅ Includes comments (in braces)
- ✅ Supports NAGs (Numeric Annotation Glyphs) like !, ?, !?
- ✅ Handles special positions (FEN tags for non-standard starts)
- ✅ Skips deleted games
- ✅ Uses original SCID encoding/decoding logic (100% compatible)

## Usage

### Read SCID to PGN:
```bash
scidtopgn <database_path>
```

Where `<database_path>` is base name of SCID database (without file extensions).

### Write PGN to SCID:
```bash
scidtopgn <database_path> <pgn_file>
```

### Save to File:
```bash
# Read SCID to PGN
scidtopgn mygames > output.pgn

# Write PGN to SCID
scidtopgn newdb games.pgn
```

## Example Output

```pgn
[Event "World Championship"]
[Site "London"]
[Date "2023.04.15"]
[Round "1"]
[White "Magnus Carlsen"]
[Black "Fabiano Caruana"]
[Result "1-0"]
[WhiteElo "2853"]
[BlackElo "2820"]
[ECO "C50"]

1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 4. O-O Nf6 5. d3 d6 6. c3 a6 7. a4 Ba7
8. Re1 h6 9. h3 O-O 10. Nbd2 Be6 11. Bxe6 fxe6 12. b4 Nd7 13. Qb3 Nb6 14. a5 Nc4 Bxc4 16. d4 exd4 17. cxd4 Qxc4 18. Rxa7 Rxd4 19. Rxc6 Rxc4 20. Rxa6 1-0
```

## Building

### Initial Setup (One-time)

From the project root directory:

```bash
# Download doctest (single-header test framework)
cd scid-cpp/tests
curl -L https://github.com/doctest/doctest/releases/download/v2.4.11/doctest.h -o doctest.h
```

### Build with CMake

```bash
cd scid-cpp
mkdir -p build
cd build
cmake ..
make
```

### Clean Build

```bash
cd scid-cpp
rm -rf build
mkdir -p build
cd build
cmake ..
make clean
make
```

### Install (Optional)

```bash
cd scid-cpp/build
sudo make install
```

This installs `scidtopgn` executable to `/usr/local/bin`.

## Requirements

- C++17 or later
- CMake 3.15 or later
- SCID source files (in scid/ directory)
- zlib (for compression support)
- doctest (for unit tests, optional)

## Project Structure

```
scid-cpp/
├── src/              # Tool source code
│   └── main.cpp      # CLI entry point
├── scid/             # SCID source files
│   ├── game.cpp/h    # Game decoding and PGN output
│   ├── index.cpp/h   # Index file handling
│   ├── namebase.cpp/h # Name file handling
│   └── ...          # Other SCID files
├── tests/            # Unit tests
│   ├── CMakeLists.txt
│   ├── README.md         # Test documentation
│   ├── doctest.h        # Test framework
│   ├── run_tests.sh     # Test runner
│   ├── test_bytebuf.cpp
│   ├── test_date.cpp
│   ├── test_namebase.cpp
│   └── test_pgn.cpp
├── CMakeLists.txt   # Build configuration
└── README.md           # This file
```

## Testing

### Run All Tests

```bash
cd scid-cpp/tests
./run_tests.sh
```

### Run Specific Test

```bash
cd scid-cpp/tests
./test_bytebuf
./test_date
./test_namebase
./test_pgn
```

### Run with CMake Test

```bash
cd scid-cpp
mkdir -p build && cd build
cmake ..
make
ctest --verbose
```

## PGN Features

The tool outputs full PGN format including:

### Standard Tags
- Event, Site, Date, Round, White, Black, Result
- WhiteElo, BlackElo
- ECO code
- Additional custom tags if present

### Move Notation
- Standard Algebraic Notation (SAN)
- Move numbers (1. e4 e5 2. Nf3...)
- Check and mate symbols (+ and #)

### Annotations
- Comments in braces: {excellent move}
- NAGs: !, ?, !!, ??, !?, $1-$255
- Variations in parentheses: (1. e4 e5 2. Nf3 (2. d4) Nc6)

### Special Cases
- Castling: O-O, O-O-O
- Pawn promotions: e8=Q
- En passant: exd6
- Non-standard starting positions: [FEN "..."]

## Unit Tests

The project includes a comprehensive unit test suite covering:

### Test Files

| Test File | Description | Test Count |
|-----------|-------------|------------|
| `test_bytebuf.cpp` | ByteBuffer operations (PutByte, GetByte, buffer boundaries) | ~40 |
| `test_date.cpp` | Date encoding/decoding (absolute and relative formats) | ~20 |
| `test_namebase.cpp` | Name management (add, lookup, duplicates, special chars) | ~25 |
| `test_pgn.cpp` | PGN format validation (tags, moves, annotations) | ~20 |

### Test Coverage

- **Critical buffer operations** (ByteBuffer read/write, boundaries, overflow)
- **Date encoding accuracy** (absolute dates, leap years, month/day ranges)
- **Name management** (adding, lookup, case sensitivity, special characters)
- **PGN format validation** (tags, moves, castling, promotions, annotations)

### Running Tests

```bash
# Build tests
cd scid-cpp
mkdir -p build && cd build
cmake ..
make

# Run tests
cd ../tests
./run_tests.sh

# Run with CMake
cd build
ctest --verbose
```

### Test Categories

1. **ByteBuffer Tests**
   - Basic read/write operations
   - Buffer boundary checking
   - Multi-byte operations (Put2Bytes, Put3Bytes, Put4Bytes)
   - String operations (PutTerminatedString, PutFixedString)
   - Buffer navigation (BackToStart, Skip)
   - Memory operations (CopyTo, CopyFrom)

2. **Date Tests**
   - Absolute date encoding (year << 9 | month << 5 | day)
   - Date bounds checking (valid ranges)
   - Leap year handling
   - Month validation (1-12)
   - Day validation (1-31)
   - Year range (0-2047)

3. **NameBase Tests**
   - Name adding (players, events, sites, rounds)
   - Name lookup by ID
   - Duplicate name detection
   - Special characters (hyphens, apostrophes)
   - Case sensitivity
   - Unicode handling

4. **PGN Tests**
   - PGN header tags parsing
   - Move notation (SAN format)
   - Castling (O-O, O-O-O)
   - Pawn promotions (e8=Q, etc.)
   - Annotations (comments, NAG symbols)
   - Variations (move alternatives)
   - Round numbers
   - Check and mate symbols
   - SCID file extensions

### Dependencies

- **Required**: C++17, CMake 3.15, zlib
- **Optional**: doctest (for unit tests)

## CI/CD Integration

For continuous integration, the test suite can be integrated with GitHub Actions or other CI systems.

### Test Output

All tests should pass with no failures. The test suite provides:

- ✅ Comprehensive coverage of core SCID operations
- ✅ Validation of date/time encoding logic
- ✅ Name management accuracy
- ✅ PGN format compliance
- ✅ Buffer operation reliability
- ✅ Edge case handling

## Troubleshooting

### "cannot open index file" Error

Ensure all three database files exist:
```bash
ls -l mygames.si4 mygames.sn4 mygames.sg4
```

### "cannot read name file" Error

Check that all three files have the same base name.

### Compilation Errors

Ensure you have C++17 support:
```bash
g++ --version # Should show version 7.0 or later
```

### "undefined reference" Errors

Ensure all SCID files are in the scid/ directory:
```bash
ls scid/*.cpp scid/*.h
```

## License

The `scid/` directory holds source files from [Scid vs. PC](https://scidvspc.sourceforge.net/) (Subversion revision 3585). Most are unchanged; a few have small changes for this tool, and `charset.h` and `stored.h` are stubs written for it. The files keep their upstream copyright notices. Scid is distributed under the GNU General Public License without a version number, as stated in [`scid/COPYING`](scid/COPYING). Section 9 of GPL version 2 permits distributing such a program under any version, so this project distributes them under GPL version 3 or later.

The rest of `scid-cpp/` is part of scidtopgn and licensed under GPL-3.0-or-later. See the root [`LICENSE`](../LICENSE).

## Acknowledgments

- SCID: Shane's Chess Information Database
- PGN format: Portable Game Notation Standard
- doctest: Lightweight C++ test framework
- CMake: Cross-platform build system
- zlib: Compression library

### Using CMake (Recommended):
```bash
cd scid-cpp
mkdir -p build && cd build
cmake ..
make
```

### Clean Build:
```bash
cd scid-cpp
rm -rf build
mkdir -p build && cd build
cmake ..
make clean
make
```

## Testing

### Run Unit Tests:
```bash
cd scid-cpp/tests
mkdir -p build && cd build
cmake ..
make
./run_tests.sh
```

### Test with Sample PGN:
```bash
# Create test PGN file
cat > /tmp/test.pgn << 'EOF'
[Event "Test Game"]
[Site "Test Site"]
[Date "2024.01.01"]
[Round "1"]
[White "Alice"]
[Black "Bob"]
[Result "1-0"]

1. e4 e5 2. Nf3 Nc6 1-0
EOF

# Write to SCID database
./scidtopgn /tmp/testdb /tmp/test.pgn

# Read back to verify
./scidtopgn /tmp/testdb
```

## Requirements

- C++17 or later
- CMake 3.15 or later (for build system)
- zlib (for compression support)
- SCID source files (scid/ directory)

## Project Structure

```
scid-cpp/
├── scid/                    # SCID library source files (45 files)
│   ├── bytebuf.cpp/h     # Buffer operations
│   ├── date.cpp/h          # Date encoding/decoding
│   ├── game.cpp/h          # Game encoding/decoding
│   ├── index.cpp/h         # Index file handling
│   ├── namebase.cpp/h       # Name file handling
│   ├── gfile.cpp/h          # Game file I/O
│   ├── position.cpp/h       # Chess position logic
│   └── ...                # Other SCID modules
├── src/                     # Tool source code
│   └── main.cpp             # CLI entry point
├── tests/                   # Unit tests
│   ├── test_bytebuf.cpp     # ByteBuffer tests
│   ├── test_date.cpp        # Date encoding tests
│   ├── test_namebase.cpp     # Name management tests
│   ├── test_pgn.cpp         # PGN format tests
│   ├── CMakeLists.txt       # Test build config
│   ├── README.md             # Test documentation
│   ├── run_tests.sh          # Test runner script
│   └── doctest.h            # Test framework
├── build/                   # Build output (created by CMake)
├── CMakeLists.txt          # Build configuration
├── README.md                 # This file
```

## Unit Tests

Comprehensive unit test suite covering:

### Test Categories:
1. **ByteBuffer Operations** (`test_bytebuf.cpp`)
   - Basic read/write operations
   - Buffer boundary checking
   - Multi-byte operations (Put2Bytes, Put3Bytes, Put4Bytes)
   - String operations
   - Memory operations

2. **Date Encoding** (`test_date.cpp`)
   - Absolute date encoding/decoding
   - Date bounds validation
   - Leap year handling
   - Month/day range checking

3. **NameBase Operations** (`test_namebase.cpp`)
   - Name adding and lookup
   - Duplicate detection
   - Special character handling
   - Case sensitivity
   - Prefix compression simulation

4. **PGN Format** (`test_pgn.cpp`)
   - PGN header parsing
   - Move notation (SAN)
   - Castling, promotions, annotations
   - Variations
   - SCID file format validation

### Running Tests:
```bash
cd scid-cpp/tests
./run_tests.sh
```

## Troubleshooting

### "cannot open index file" Error
Ensure all three database files exist:
```bash
ls -l mygames.si4 mygames.sn4 mygames.sg4
```

### "cannot read name file" Error
Check that all three files have same base name.

### Compilation Errors
Ensure you have C++17 support:
```bash
g++ --version  # Should show version 7.0 or later
```

### "undefined reference" Errors
Ensure all SCID files are in the scid/ directory.

### Buffer Overflow Errors
If you see ERROR_BufferFull errors during write mode, it means the game data is too large.

### "no such file" for CMake
Install CMake 3.15 or later:
```bash
# Ubuntu/Debian
sudo apt-get install cmake
# macOS
brew install cmake
```

## Implementation Notes

### Write Mode Limitations:
- Game data length: Maximum 131,071 bytes per game (2^17 - 1)
- Maximum games: ~16,777,214 games (2^(3*8) - 1)
- Name limits: 1,048,575 players, etc. per type

### Character Encoding:
- All text is UTF-8 encoded
- Special characters are supported (hyphens, apostrophes, etc.)
- Unicode names are fully supported

### Performance:
- Block-based game file organization (128KB blocks)
- Front-coding compression for names
- Efficient binary encoding for moves

### Compatibility:
- Compatible with SCID 4.0 database format
- Reads SCID 3.x and 4.x databases
- Writes SCID 4.0 format databases
- PGN output matches SCID's PGN generation

## License

The `scid/` directory holds source files from [Scid vs. PC](https://scidvspc.sourceforge.net/) (Subversion revision 3585). Most are unchanged; a few have small changes for this tool, and `charset.h` and `stored.h` are stubs written for it. The files keep their upstream copyright notices. Scid is distributed under the GNU General Public License without a version number, as stated in [`scid/COPYING`](scid/COPYING). Section 9 of GPL version 2 permits distributing such a program under any version, so this project distributes them under GPL version 3 or later.

The rest of `scid-cpp/` is part of scidtopgn and licensed under GPL-3.0-or-later. See the root [`LICENSE`](../LICENSE).

## Acknowledgments

- SCID: Shane's Chess Information Database by Shane Hudson
- PGN format: Portable Game Notation Standard
- doctest: Test framework
