//! Read and write [SCID](https://scid.sourceforge.net/) chess databases, and
//! convert between them and PGN.
//!
//! [`Database`] opens or creates a database and reads and adds its games.
//! [`Game`] holds a game's tags and moves, and [`Game::to_pgn`] writes it as
//! PGN. [`parse_pgn`] and [`PgnParser`] read games from PGN. Fallible
//! functions return [`Error`].
//!
//! # Examples
//!
//! Export a database to PGN:
//!
//! ```
//! use scidtopgn::Database;
//!
//! # let dir = tempfile::tempdir()?;
//! # let path = dir.path().join("games");
//! # let mut db = Database::create(&path)?;
//! # db.add_game(&scidtopgn::parse_single_game("1. e4 e5 *")?)?;
//! # db.close()?;
//! let mut db = Database::open(&path)?;
//! for game in db.games() {
//!     print!("{}\n", game?.to_pgn());
//! }
//! # Ok::<(), scidtopgn::Error>(())
//! ```
//!
//! Import PGN into a new database, keeping the games that parse:
//!
//! ```
//! use scidtopgn::{Database, PgnParser};
//!
//! # let dir = tempfile::tempdir()?;
//! # let path = dir.path().join("games");
//! let pgn = "[Event \"Example\"]\n\n1. e4 e5 2. Nf3 Nc6 *\n";
//!
//! let mut db = Database::create(&path)?;
//! for result in PgnParser::new(pgn) {
//!     match result {
//!         Ok(game) => {
//!             db.add_game(&game)?;
//!         }
//!         Err(err) => eprintln!("skipped: {err}"),
//!     }
//! }
//! db.close()?;
//! # assert_eq!(Database::open(&path)?.num_games(), 1);
//! # Ok::<(), scidtopgn::Error>(())
//! ```
//!
//! # Architecture
//!
//! A SCID database is three files that share a path without the extension.
//! Each has its own module, and [`database`] ties them together:
//!
//! - The index file, `.si4` ([`index`]), has a header and one fixed-size
//!   [`IndexEntry`] per game. An entry gives the position and length of the
//!   game's record in the game file, the IDs of its names, and its date,
//!   result, ratings, ECO code, and flags, including the logical delete flag.
//! - The name file, `.sn4` ([`nfile`] and [`namebase`]), stores the player,
//!   event, site, and round names that index entries refer to by ID.
//! - The game file, `.sg4` ([`gfile`]), stores each game's record: its other
//!   tags, its starting position, its moves, and its comments.
//!
//! A game record is decoded into a [`Game`] by [`game`], using SCID's move
//! encoding from [`mov`] and the piece numbering in [`position`].
//! [`bytebuf`] reads and writes the record's bytes, and [`date`] SCID's packed
//! dates. Chess rules come from [`shakmaty`], which is re-exported. [`pgn`]
//! reads PGN, and [`error`] defines the errors.
//!
//! Games are numbered from 0 in this API and from 1 in error messages. The
//! [file format description] gives the binary layout of each file.
//!
//! [file format description]: https://github.com/intermezzo-chess/scidtopgn/blob/main/SCID_DATABASE_FORMAT.md

#![warn(missing_docs)]

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
mod replace;

pub use bytebuf::ByteBuffer;
pub use common::GameResult;
pub use database::{Database, GameIterator};
pub use date::{
    date_decode_to_string, date_encode_from_string, date_get_day, date_get_month, date_get_year,
    date_make, date_valid_string, Date, YEAR_MAX,
};
pub use error::{CorruptGameKind, DatabaseFile, Error, InvalidMoveByte, Result};
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
pub use pgn::{parse_pgn, parse_single_game, PgnParser, PgnWarning, PgnWarningKind};
pub use position::{PieceList, ScidPosition};
/// The chess library that provides the rules, positions, and moves.
pub use shakmaty;

/// This crate's version, such as `0.1.0`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
