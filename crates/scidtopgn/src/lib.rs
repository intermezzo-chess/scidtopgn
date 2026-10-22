pub mod bytebuf;
pub mod common;
pub mod database;
pub mod date;
pub mod error;
pub mod game;
pub mod gfile;
pub mod index;
pub mod mov;
pub mod namebase;
pub mod nfile;
pub mod pgn;
pub mod position;
pub mod private;

pub use bytebuf::ByteBuffer;
pub use common::GameResult;
pub use database::{Database, GameIterator};
pub use date::{
    date_decode_to_string, date_encode_from_string, date_get_day, date_get_month, date_get_year,
    date_make, date_valid_string, Date, YEAR_MAX,
};
pub use error::{Error, Result};
pub use game::{
    nag_to_symbol, Game, Marker, MoveNode, Tag, GAME_DECODE_ALL, GAME_DECODE_COMMENTS,
    GAME_DECODE_NONE, GAME_DECODE_TAGS, MAX_NAGS, MAX_TAGS,
};
pub use gfile::{FileMode, GFile, GFILE_SUFFIX, GF_BLOCKSIZE};
pub use index::{
    Index, IndexEntry, IndexHeader, INDEX_ENTRY_SIZE, INDEX_HEADER_SIZE, INDEX_MAGIC, INDEX_SUFFIX,
    MAX_GAMES, SCID_VERSION,
};
pub use mov::{
    decode_move, encode_move, make_move_byte, parse_move_byte, GameMove, ENCODE_COMMENT,
    ENCODE_END_GAME, ENCODE_END_MARKER, ENCODE_FIRST, ENCODE_NAG, ENCODE_START_MARKER,
};
pub use namebase::{NameBase, NameEntry, NameType, NAMEBASE_MAGIC, NUM_NAME_TYPES};
pub use nfile::NFile;
pub use pgn::{parse_pgn, parse_single_game, PgnParser};
pub use position::{PieceList, ScidPosition};
/// The chess library that provides the rules, positions, and moves.
pub use shakmaty;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
