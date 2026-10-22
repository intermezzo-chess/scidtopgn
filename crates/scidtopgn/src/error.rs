use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
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

    #[error("Buffer error: {0}")]
    Buffer(String),

    #[error("Name not found: {name}")]
    NameNotFound { name: String },

    #[error("Database error: {0}")]
    Database(String),

    #[error("Encoding error: {0}")]
    Encoding(String),

    #[error("Corrupt data: {0}")]
    CorruptData(String),

    #[error("Buffer full")]
    BufferFull,

    #[error("Buffer read error")]
    BufferRead,

    #[error("End of move list")]
    EndOfMoveList,

    #[error("Decode error: {0}")]
    Decode(String),
}

pub type Result<T> = std::result::Result<T, Error>;
