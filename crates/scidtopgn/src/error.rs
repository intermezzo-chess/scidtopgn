use std::fmt;

use shakmaty::Square;
use thiserror::Error;

use crate::game::MAX_VARIATION_DEPTH;
use crate::index::MAX_GAMES;
use crate::namebase::NameType;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Parse error at line {line}: {message}")]
    Parse { line: u32, message: String },

    /// A PGN move that can't be read or played in its position.
    #[error("Invalid move {san:?} at line {line}")]
    PgnMove {
        line: u32,
        san: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// A game in a PGN file that failed to parse. Games are numbered from 1.
    #[error("PGN game {number}, starting at line {line}")]
    PgnGame {
        number: usize,
        line: u32,
        #[source]
        source: Box<Error>,
    },

    /// An error reading or creating one of a SCID database's files.
    #[error("SCID {file}")]
    File {
        file: DatabaseFile,
        #[source]
        source: Box<Error>,
    },

    /// A game in a SCID database that failed to read. Games are numbered
    /// from 1, so game 1 is `Database::get_game(0)`.
    #[error("SCID game {number}")]
    Game {
        number: u64,
        #[source]
        source: Box<Error>,
    },

    /// A game number past the end of the database. Games are numbered from 1.
    #[error("No game {number} in the database")]
    GameNotFound { number: u64 },

    /// A game record that can't be decoded. `offset` is the position of the
    /// byte where the problem was found, from the start of the record.
    #[error("Damaged game data at byte {offset}: {kind}")]
    CorruptGame {
        offset: usize,
        kind: CorruptGameKind,
    },

    /// An index entry whose game record extends past the end of the game
    /// file.
    #[error("Game data at byte {offset}, {length} bytes long, is past the end of the game file")]
    GameOutOfRange { offset: u32, length: u32 },

    /// A file that doesn't start with the magic number of its SCID file type.
    #[error("Not a SCID file: wrong magic number")]
    InvalidMagic,

    /// A file that ends before the data its header describes.
    #[error("The file ends early")]
    Truncated,

    #[error("Invalid name type: {value}")]
    InvalidNameType { value: usize },

    /// A front-coded name that shares more bytes with the previous name than
    /// the name has.
    #[error("A {name_type} name shares {prefix} bytes with the previous name but is {length} bytes long")]
    NamePrefixTooLong {
        name_type: NameType,
        prefix: u8,
        length: u8,
    },

    #[error("A {name_type} name isn't valid UTF-8")]
    NameNotUtf8 { name_type: NameType },

    #[error("Variation nested more than {MAX_VARIATION_DEPTH} levels deep")]
    VariationTooDeep,

    /// SCID counts a move's variations in a byte, so it can't read back a
    /// move with more than 255.
    #[error("More than 255 variations of one move")]
    TooManyVariations,

    /// A variation added to a game that has no current move.
    #[error("No current move")]
    NoCurrentMove,

    #[error("The index is full: a SCID database holds at most {MAX_GAMES} games")]
    IndexFull,

    #[error("Too many {name_type} names: a SCID database holds at most {}", name_type.max_id())]
    NameBaseFull { name_type: NameType },

    /// A game whose record is too long to store.
    #[error("Game is {length} bytes long; at most {max} bytes fit")]
    GameTooLarge { length: usize, max: usize },

    #[error("File not open")]
    FileNotOpen,

    #[error("File is read-only")]
    ReadOnly,

    #[error("Unsupported variant: {variant}")]
    UnsupportedVariant { variant: String },

    #[error("Invalid move: {move_str}")]
    InvalidMove { move_str: String },

    #[error("Invalid FEN: {fen}")]
    InvalidFen {
        fen: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Buffer full")]
    BufferFull,
}

impl Error {
    /// Adds the file the error happened in. Reading past the end of the
    /// file becomes [`Error::Truncated`].
    pub(crate) fn in_file(self, file: DatabaseFile) -> Self {
        let source = match self {
            Error::Io(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Error::Truncated,
            other => other,
        };
        Error::File {
            file,
            source: Box::new(source),
        }
    }
}

/// One of the three files of a SCID database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DatabaseFile {
    /// The index file, `.si4`.
    Index,
    /// The name file, `.sn4`.
    Name,
    /// The game file, `.sg4`.
    Game,
}

impl fmt::Display for DatabaseFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            DatabaseFile::Index => "index file (.si4)",
            DatabaseFile::Name => "name file (.sn4)",
            DatabaseFile::Game => "game file (.sg4)",
        })
    }
}

/// What is wrong with a damaged game record.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CorruptGameKind {
    #[error("unexpected end of data")]
    UnexpectedEnd,

    /// A move byte that doesn't describe a move in the position, given as a
    /// FEN.
    #[error("invalid move byte {byte:#04x} ({reason}) in {fen}")]
    InvalidMove {
        byte: u8,
        reason: InvalidMoveByte,
        fen: String,
    },

    #[error("variation before any move")]
    VariationBeforeMove,

    #[error("end of a variation outside any variation")]
    UnmatchedEndMarker,

    #[error("end of the game inside a variation")]
    UnclosedVariation,

    /// A starting position that can't be read or that SCID can't number.
    /// `reason` describes the problem.
    #[error("invalid starting position {fen:?}: {reason}")]
    InvalidStartPosition { fen: String, reason: String },
}

/// Why a move byte doesn't describe a move.
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidMoveByte {
    #[error("no such piece")]
    NoSuchPiece,

    #[error("no piece on its square")]
    EmptySquare,

    #[error("off the board")]
    OffBoard,

    #[error("not a king move")]
    NotKingMove,

    #[error("not a knight move")]
    NotKnightMove,

    #[error("bad diagonal queen move")]
    BadQueenMove,

    #[error("null move in check")]
    NullMoveInCheck,

    #[error("{from}{to} is illegal")]
    Illegal { from: Square, to: Square },
}

pub type Result<T> = std::result::Result<T, Error>;

// Errors can cross threads and be stored, for example in `anyhow::Error`.
const _: fn() = || {
    fn assert_send_sync_static<T: Send + Sync + 'static>() {}
    assert_send_sync_static::<Error>();
};
