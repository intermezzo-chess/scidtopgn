//! The errors the library returns.
//!
//! Every fallible function returns [`Result`], whose error is [`Error`](enum@Error).
//! Errors that wrap another error, such as [`Error::File`] and
//! [`Error::Game`], expose it through [`std::error::Error::source`], so the
//! whole chain can be reported.

use std::fmt;

use shakmaty::Square;
use thiserror::Error;

use crate::game::MAX_VARIATION_DEPTH;
use crate::index::MAX_GAMES;
use crate::namebase::NameType;

/// An error reading or writing a SCID database, or parsing PGN.
///
/// # Examples
///
/// An error opening a database names the file that failed, with the cause as
/// its source:
///
/// ```
/// use scidtopgn::{Database, DatabaseFile, Error};
///
/// let err = Database::open("no/such/database").err().unwrap();
/// match &err {
///     Error::File { file, source } => {
///         assert_eq!(*file, DatabaseFile::Index);
///         assert!(matches!(**source, Error::Io(_)));
///     }
///     other => panic!("unexpected error: {other}"),
/// }
/// assert_eq!(err.to_string(), "SCID index file (.si4)");
/// ```
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// An I/O error from the operating system.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// PGN text that can't be read, or a date with a part out of range.
    /// `line` is the PGN line where the problem was found, or 0 for a date.
    #[error("Parse error at line {line}: {message}")]
    Parse {
        /// The line, counting from 1, or 0 when there is no line.
        line: u32,
        /// What is wrong.
        message: String,
    },

    /// A PGN move that can't be read or played in its position.
    #[error("Invalid move {san:?} at line {line}")]
    PgnMove {
        /// The line the move is on, counting from 1.
        line: u32,
        /// The move as written.
        san: String,
        /// Why the move can't be read or played.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// A game in a PGN file that failed to parse. Games are numbered from 1.
    #[error("PGN game {number}, starting at line {line}")]
    PgnGame {
        /// The game's number, counting from 1.
        number: usize,
        /// The line the game starts on, counting from 1.
        line: u32,
        /// Why the game failed.
        #[source]
        source: Box<Error>,
    },

    /// An error reading or creating one of a SCID database's files.
    #[error("SCID {file}")]
    File {
        /// The file that failed.
        file: DatabaseFile,
        /// Why it failed.
        #[source]
        source: Box<Error>,
    },

    /// A game in a SCID database that failed to read. Games are numbered
    /// from 1, so game 1 is [`Database::get_game(0)`](crate::Database::get_game).
    #[error("SCID game {number}")]
    Game {
        /// The game's number, counting from 1.
        number: u64,
        /// Why the game failed.
        #[source]
        source: Box<Error>,
    },

    /// A game number past the end of the database. Games are numbered from 1.
    #[error("No game {number} in the database")]
    GameNotFound {
        /// The game's number, counting from 1.
        number: u64,
    },

    /// A game record that can't be decoded. `offset` is the position of the
    /// byte where the problem was found, from the start of the record.
    #[error("Damaged game data at byte {offset}: {kind}")]
    CorruptGame {
        /// The byte's position, counting from 0.
        offset: usize,
        /// What is wrong.
        kind: CorruptGameKind,
    },

    /// An index entry whose game record extends past the end of the game
    /// file.
    #[error("Game data at byte {offset}, {length} bytes long, is past the end of the game file")]
    GameOutOfRange {
        /// Where the record starts in the game file.
        offset: u32,
        /// The record's length in bytes.
        length: u32,
    },

    /// A file that doesn't start with the magic number of its SCID file type.
    #[error("Not a SCID file: wrong magic number")]
    InvalidMagic,

    /// A file that ends before the data its header describes.
    #[error("The file ends early")]
    Truncated,

    /// A number that isn't one of the four [`NameType`]s.
    #[error("Invalid name type: {value}")]
    InvalidNameType {
        /// The number.
        value: usize,
    },

    /// A front-coded name that shares more bytes with the previous name than
    /// the name has.
    #[error("A {name_type} name shares {prefix} bytes with the previous name but is {length} bytes long")]
    NamePrefixTooLong {
        /// The kind of name.
        name_type: NameType,
        /// How many bytes the name claims to share.
        prefix: u8,
        /// The previous name's length in bytes.
        length: u8,
    },

    /// A name in the name file that isn't valid UTF-8.
    #[error("A {name_type} name isn't valid UTF-8")]
    NameNotUtf8 {
        /// The kind of name.
        name_type: NameType,
    },

    /// A name ID in an index entry that the name file has no name for.
    #[error("No {name_type} name has ID {id}")]
    NameNotFound {
        /// The kind of name.
        name_type: NameType,
        /// The ID.
        id: u32,
    },

    /// A variation nested too deeply to read or write without risking a
    /// stack overflow.
    #[error("Variation nested more than {MAX_VARIATION_DEPTH} levels deep")]
    VariationTooDeep,

    /// SCID counts a move's variations in a byte, so it can't read back a
    /// move with more than 255.
    #[error("More than 255 variations of one move")]
    TooManyVariations,

    /// A variation added to a game that has no current move.
    #[error("No current move")]
    NoCurrentMove,

    /// A game added to a database that already holds [`MAX_GAMES`] games.
    #[error("The index is full: a SCID database holds at most {MAX_GAMES} games")]
    IndexFull,

    /// A name added when the name file already holds as many names of its
    /// kind as SCID allows.
    #[error("Too many {name_type} names: a SCID database holds at most {}", name_type.max_id())]
    NameBaseFull {
        /// The kind of name.
        name_type: NameType,
    },

    /// A game whose record is too long to store.
    #[error("Game is {length} bytes long; at most {max} bytes fit")]
    GameTooLarge {
        /// The record's length in bytes.
        length: usize,
        /// The most bytes a record can have.
        max: usize,
    },

    /// A tag whose value is too long to store in a game record.
    #[error("Tag {name} has a value {length} bytes long; at most {max} bytes fit")]
    TagTooLong {
        /// The tag's name.
        name: String,
        /// The value's length in bytes.
        length: usize,
        /// The most bytes a value can have.
        max: usize,
    },

    /// A game file used after it was closed.
    #[error("File not open")]
    FileNotOpen,

    /// A change to a database or game file opened read-only, with
    /// [`Database::open_read_only`](crate::Database::open_read_only) or
    /// [`GFile::open_read_only`](crate::GFile::open_read_only). The files
    /// themselves may be writable.
    #[error("Database is open read-only")]
    ReadOnly,

    /// A PGN game for a chess variant other than standard chess, named by
    /// its `Variant` tag.
    #[error("Unsupported variant: {variant}")]
    UnsupportedVariant {
        /// The value of the `Variant` tag.
        variant: String,
    },

    /// A move that is illegal in its position, or a null move in check.
    #[error("Invalid move: {move_str}")]
    InvalidMove {
        /// The move and the position, described in words.
        move_str: String,
    },

    /// A FEN that can't be read, or a position SCID can't store.
    #[error("Invalid FEN: {fen}")]
    InvalidFen {
        /// The FEN.
        fen: String,
        /// Why it is invalid.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// A write to a [`ByteBuffer`](crate::ByteBuffer) made from existing
    /// data, which is read-only.
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

/// Names the file and its extension, such as `index file (.si4)`.
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
    /// The record ends before the game does.
    #[error("unexpected end of data")]
    UnexpectedEnd,

    /// A move byte that doesn't describe a move in the position, given as a
    /// FEN.
    #[error("invalid move byte {byte:#04x} ({reason}) in {fen}")]
    InvalidMove {
        /// The move byte.
        byte: u8,
        /// Why it doesn't describe a move.
        reason: InvalidMoveByte,
        /// The position, as a FEN.
        fen: String,
    },

    /// A variation that starts before the first move of its line.
    #[error("variation before any move")]
    VariationBeforeMove,

    /// The end of a variation where no variation is open.
    #[error("end of a variation outside any variation")]
    UnmatchedEndMarker,

    /// The end of the game while a variation is still open.
    #[error("end of the game inside a variation")]
    UnclosedVariation,

    /// A starting position that can't be read or that SCID can't number.
    /// `reason` describes the problem.
    #[error("invalid starting position {fen:?}: {reason}")]
    InvalidStartPosition {
        /// The starting position as stored.
        fen: String,
        /// What is wrong with it.
        reason: String,
    },
}

/// Why a move byte doesn't describe a move.
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidMoveByte {
    /// The side to move has no piece with the byte's piece number.
    #[error("no such piece")]
    NoSuchPiece,

    /// The piece's square is empty.
    #[error("no piece on its square")]
    EmptySquare,

    /// The move would leave the board.
    #[error("off the board")]
    OffBoard,

    /// The value isn't one of the king's moves.
    #[error("not a king move")]
    NotKingMove,

    /// The value isn't one of the knight's moves.
    #[error("not a knight move")]
    NotKnightMove,

    /// A diagonal queen move whose second byte isn't a square.
    #[error("bad diagonal queen move")]
    BadQueenMove,

    /// A null move while the side to move is in check.
    #[error("null move in check")]
    NullMoveInCheck,

    /// A move from `from` to `to` that isn't legal in the position.
    #[error("{from}{to} is illegal")]
    Illegal {
        /// The square the piece moves from.
        from: Square,
        /// The square it moves to.
        to: Square,
    },
}

/// A [`std::result::Result`] whose error is [`Error`](enum@Error).
pub type Result<T> = std::result::Result<T, Error>;

// Errors can cross threads and be stored, for example in `anyhow::Error`.
const _: fn() = || {
    fn assert_send_sync_static<T: Send + Sync + 'static>() {}
    assert_send_sync_static::<Error>();
};
