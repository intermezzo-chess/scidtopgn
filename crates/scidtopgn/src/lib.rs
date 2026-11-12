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
//! # How a database is stored
//!
//! A SCID database is three files that share a path without the extension:
//!
//! - The index file, `.si4`, has a header and one fixed-size entry per game,
//!   which [`Database::get_entry`] returns as an [`IndexEntry`]. An entry
//!   gives the game's date, result, ratings, ECO code, and flags, including
//!   the logical delete flag, the IDs of its names, and where its record is
//!   in the game file.
//! - The name file, `.sn4`, stores the player, event, site, and round names
//!   that index entries refer to by ID.
//! - The game file, `.sg4`, stores each game's record: its other tags, its
//!   starting position, its moves, and its comments.
//!
//! [`Database::search_header`] selects games by what the index and name
//! files hold, without reading their records. Chess rules come from
//! [`shakmaty`], which is re-exported.
//!
//! # Threads
//!
//! Every public type is [`Send`] and [`Sync`]. A [`Database`] takes
//! `&mut self` to read games, so to share one between threads, put it
//! behind a [`Mutex`](std::sync::Mutex).
//!
//! Games are numbered from 0 in this API and from 1 in error messages. The
//! [file format description] gives the binary layout of each file.
//!
//! [file format description]: https://github.com/intermezzo-chess/scidtopgn/blob/main/SCID_DATABASE_FORMAT.md

#![warn(missing_docs)]
#![warn(unreachable_pub)]

// Each of a database's files has its own module, and `database` ties them
// together: `index` reads and writes the index file, `nfile` and `namebase`
// the name file, and `gfile` the game file. `game` decodes a game record
// into a `Game`, using SCID's move encoding from `mov` and the piece
// numbering in `position`; `database` encodes one. `bytebuf` reads and
// writes a record's bytes, and `date` SCID's packed dates. `search` chooses
// the games `Database::search_header` selects, `twins` the twin games
// `Database::delete_twins` deletes, and `stored_line` the stored-line code
// an index entry records. `replace` writes files beside a database and
// moves them into place. `pgn` reads PGN, and `error` defines the errors.
mod bytebuf;
mod common;
mod database;
mod date;
mod error;
mod game;
mod gfile;
mod index;
mod mov;
mod namebase;
mod nfile;
mod pgn;
mod position;
mod replace;
mod search;
mod stored_line;
#[cfg(test)]
mod tests;
mod twins;

pub use common::{GameResult, RatingType};
pub use database::{Database, GameIterator};
pub use date::{Date, YEAR_MAX};
pub use error::{CorruptGameKind, DatabaseFile, Error, InvalidMoveByte, Result};
pub use game::{nag_to_symbol, Game, Marker, MoveNode, Tag, MAX_TAGS};
pub use index::{Eco, IndexEntry, MAX_ELO, MAX_GAMES, SCID_VERSION};
pub use mov::GameMove;
pub use namebase::NameType;
pub use pgn::{parse_pgn, parse_single_game, PgnParser, PgnWarning, PgnWarningKind};
pub use search::{FlagMatch, HeaderFlags, HeaderSearch, DEFAULT_MAX_LENGTH};
/// The chess library that provides the rules, positions, and moves.
pub use shakmaty;
pub use twins::{DeleteTwin, TwinOptions};

/// This crate's version, such as `0.1.0`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Every public type is `Send` and `Sync`, as the crate documentation
// promises, and errors are also `'static`, so they can be stored, for example
// in `anyhow::Error`. Losing either is a breaking change.
const _: fn() = || {
    fn send_sync<T: Send + Sync>() {}
    fn send_sync_static<T: Send + Sync + 'static>() {}

    send_sync::<Database>();
    send_sync::<GameIterator<'_>>();
    send_sync::<Game>();
    send_sync::<MoveNode>();
    send_sync::<Marker>();
    send_sync::<Tag>();
    send_sync::<GameMove>();
    send_sync::<Date>();
    send_sync::<Eco>();
    send_sync::<IndexEntry>();
    send_sync::<GameResult>();
    send_sync::<RatingType>();
    send_sync::<NameType>();
    send_sync::<FlagMatch>();
    send_sync::<HeaderFlags>();
    send_sync::<HeaderSearch>();
    send_sync::<DeleteTwin>();
    send_sync::<TwinOptions>();
    send_sync::<PgnParser>();
    send_sync::<PgnWarning>();
    send_sync::<PgnWarningKind>();
    send_sync_static::<Error>();
    send_sync_static::<DatabaseFile>();
    send_sync_static::<CorruptGameKind>();
    send_sync_static::<InvalidMoveByte>();
};
