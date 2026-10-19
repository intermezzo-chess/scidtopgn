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
pub use common::{piece_color, piece_make, piece_type, Color, GameResult, Piece, Square};
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
    decode_bishop, decode_king, decode_knight, decode_pawn, decode_queen, decode_rook,
    encode_bishop, encode_king, encode_knight, encode_pawn, encode_queen, encode_rook,
    make_move_byte, parse_move_byte, SimpleMove, ENCODE_COMMENT, ENCODE_END_GAME,
    ENCODE_END_MARKER, ENCODE_FIRST, ENCODE_NAG, ENCODE_START_MARKER,
};
pub use namebase::{NameBase, NameEntry, NameType, NAMEBASE_MAGIC, NUM_NAME_TYPES};
pub use nfile::NFile;
pub use pgn::{parse_pgn, parse_single_game, PgnParser};
pub use position::{Board, PieceList, BK_CASTLE, BQ_CASTLE, WK_CASTLE, WQ_CASTLE};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
