//! [`Database`], which reads and writes a SCID database's three files
//! together.

use std::fs;
use std::io::{self, BufWriter, Seek, Write};
use std::path::{Path, PathBuf};

use crate::bytebuf::ByteBuffer;
use crate::common::RatingType;
use crate::error::{DatabaseFile, Error, Result};
use crate::game::{Game, Marker, GAME_DECODE_ALL, MAX_TAG_VALUE_LEN};
use crate::gfile::{FileMode, GFile};
use crate::index::{
    material_sig, HomePawnList, Index, IndexEntry, IDX_MASK_PROMO, IDX_MASK_UPROMO,
};
use crate::mov::{encode_move, GameMove};
use crate::namebase::{NameBase, NameType, NewIds};
use crate::nfile::NFile;
use crate::position::{fen, ScidPosition};
use crate::replace::NewFiles;
use crate::search::{self, HeaderSearch};
use crate::twins::{self, TwinOptions};
use crate::Date;
use shakmaty::{Board, Position, Role};

/// A SCID database: an index file (`.si4`), a name file (`.sn4`), and a game
/// file (`.sg4`) that share a path without the extension.
///
/// Games are numbered from 0 in the order they were added. A game marked as
/// logically deleted keeps its record and its number, and the methods that
/// read games include it. See [Logically deleted
/// games](#logically-deleted-games).
///
/// Changes are written by [`flush`](Self::flush) and
/// [`close`](Self::close), which replace the name file and index file only
/// once new ones are complete, so a failed write leaves them as they were.
/// Dropping a `Database` with unwritten changes
/// loses them, though the records of games it added or replaced may stay
/// in the game file unused; [`discard`](Self::discard) removes those too.
/// A database opened with
/// [`open_read_only`](Self::open_read_only) can't be changed, and nothing
/// writes to its files.
///
/// # Examples
///
/// Import PGN into a new database, then read it back:
///
/// ```
/// use scidtopgn::{parse_pgn, Database};
///
/// let dir = tempfile::tempdir()?;
/// let path = dir.path().join("games");
///
/// let mut db = Database::create(&path)?;
/// for game in parse_pgn("[White \"Anand\"]\n\n1. e4 e5 *\n\n[White \"Carlsen\"]\n\n1. d4 *\n")? {
///     db.add_game(&game)?;
/// }
/// db.close()?;
///
/// let mut db = Database::open(&path)?;
/// assert_eq!(db.num_games(), 2);
/// for game in db.games() {
///     print!("{}", game?.to_pgn());
/// }
/// # Ok::<(), scidtopgn::Error>(())
/// ```
///
/// # Logically deleted games
///
/// SCID deletes a game by setting its delete flag, as
/// [`delete_game`](Self::delete_game) does. The game's record stays in the
/// database, and its number doesn't change, until
/// [`compact_games`](Self::compact_games) removes it, so
/// [`undelete_game`](Self::undelete_game) can restore it.
/// [`num_games`](Self::num_games), [`games`](Self::games), and
/// [`get_game`](Self::get_game) include deleted games, which is useful for
/// inspecting or recovering them. The `scidtopgn` command skips them when
/// it exports, as SCID does. To do the same, check each game's
/// [`IndexEntry::get_delete_flag`] before reading it, as the example for
/// [`get_entry`](Self::get_entry) shows.
///
/// # Names that aren't UTF-8
///
/// Older versions of SCID store the player, event, site, and round names of
/// a Latin-1 or Windows-1252 PGN file as they are, so a name file can hold
/// names that aren't valid UTF-8. When the name file is read, the bytes of
/// such a name that aren't valid UTF-8 are read as Windows-1252, and
/// [`converted_names`](Self::converted_names) lists the name. Games read with these
/// names converted.
///
/// **Writing the database stores the converted names as UTF-8**, as SCID
/// vs. PC stores the names of PGN it imports. Two names of a kind can then
/// be the same, such as a name stored once as Latin-1 and once as UTF-8,
/// and SCID can't open a name file with both. [`flush`](Self::flush),
/// [`compact_games`](Self::compact_games), and
/// [`compact_names`](Self::compact_names) merge them into one name, whose
/// frequency is the sum of theirs, and give the names of that kind new IDs.
/// Every game reads back with the same names as before.
pub struct Database {
    path: PathBuf,
    index: Index,
    gfile: GFile,
    nfile: NFile,
    modified: bool,
    /// The game file's length when the database was opened, created, or
    /// last flushed, which [`discard`](Self::discard) restores.
    saved_game_size: u64,
    /// Whether the unwritten changes put SCID's tree cache file out of date.
    tree_cache_stale: bool,
}

impl Database {
    /// Opens the database at `path`, given without an extension, for
    /// reading and changing games. To read a database whose files may not be
    /// writable, use [`open_read_only`](Self::open_read_only).
    ///
    /// # Errors
    ///
    /// Returns [`Error::File`] naming the file that couldn't be opened or is
    /// damaged. An index file of a SCID format version other than
    /// [`SCID_VERSION`](crate::index::SCID_VERSION) gives
    /// [`Error::UnsupportedVersion`] for [`DatabaseFile::Index`].
    ///
    /// # Examples
    ///
    /// ```
    /// use scidtopgn::{Database, DatabaseFile, Error};
    ///
    /// # let dir = tempfile::tempdir()?;
    /// # let path = dir.path().join("games");
    /// # let mut db = Database::create(&path)?;
    /// # for pgn in ["[White \"Anand\"]\n\n1. e4 e5 *", "[White \"Carlsen\"]\n\n1. d4 d5 *"] {
    /// #     db.add_game(&scidtopgn::parse_single_game(pgn)?)?;
    /// # }
    /// # db.close()?;
    /// let db = Database::open(&path)?;
    /// assert_eq!(db.num_games(), 2);
    ///
    /// let missing = Database::open(dir.path().join("missing"));
    /// assert!(matches!(
    ///     missing,
    ///     Err(Error::File { file: DatabaseFile::Index, .. })
    /// ));
    /// # Ok::<(), scidtopgn::Error>(())
    /// ```
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::open_with(path.as_ref(), FileMode::Both)
    }

    /// Opens the database at `path`, given without an extension, for reading
    /// only, so none of its files needs to be writable.
    ///
    /// Adding, replacing, deleting, or undeleting a game returns
    /// [`Error::ReadOnly`] and changes nothing. [`flush`](Self::flush) and
    /// [`close`](Self::close) write nothing.
    ///
    /// # Errors
    ///
    /// The same as [`open`](Self::open).
    ///
    /// # Examples
    ///
    /// ```
    /// use scidtopgn::{parse_single_game, Database, Error};
    ///
    /// # let dir = tempfile::tempdir()?;
    /// # let path = dir.path().join("games");
    /// # let mut db = Database::create(&path)?;
    /// # db.add_game(&parse_single_game("[White \"Anand\"]\n\n1. e4 e5 *")?)?;
    /// # db.close()?;
    /// let mut db = Database::open_read_only(&path)?;
    /// assert_eq!(db.get_game(0)?.white, "Anand");
    ///
    /// let game = parse_single_game("[White \"Carlsen\"]\n\n1. d4 d5 *")?;
    /// assert!(matches!(db.add_game(&game), Err(Error::ReadOnly)));
    /// assert_eq!(db.num_games(), 1);
    /// # Ok::<(), scidtopgn::Error>(())
    /// ```
    pub fn open_read_only<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::open_with(path.as_ref(), FileMode::ReadOnly)
    }

    /// Opens the database at `path`, with its game file open in
    /// `file_mode`, which is [`ReadOnly`](FileMode::ReadOnly) or
    /// [`Both`](FileMode::Both).
    fn open_with(path: &Path, file_mode: FileMode) -> Result<Self> {
        let path = path.to_path_buf();

        let si4_path = path.with_extension("si4");
        let index = Index::open(&si4_path)?;

        let gfile = GFile::open_file(&path, file_mode)?;

        let sn4_path = path.with_extension("sn4");
        let nfile = NFile::open(&sn4_path)?;

        let saved_game_size = gfile.file_size();
        Ok(Self {
            path,
            index,
            gfile,
            nfile,
            modified: false,
            saved_game_size,
            tree_cache_stale: false,
        })
    }

    /// Creates an empty database at `path`, given without an extension,
    /// replacing the files of any database already there.
    ///
    /// All three files are written at once, so the new database can be
    /// opened even if no game is ever added. SCID's tree cache file (`.stc`)
    /// is deleted first, as SCID does, because SCID would use it for the new
    /// database once that has as many games as the old one.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if the tree cache file can't be deleted, which
    /// leaves the other files as they were, and [`Error::File`] naming the
    /// file that couldn't be created.
    ///
    /// # Examples
    ///
    /// ```
    /// use scidtopgn::Database;
    ///
    /// let dir = tempfile::tempdir()?;
    /// let path = dir.path().join("games");
    ///
    /// Database::create(&path)?;
    ///
    /// assert!(path.with_extension("si4").exists());
    /// assert!(path.with_extension("sn4").exists());
    /// assert!(path.with_extension("sg4").exists());
    /// assert_eq!(Database::open(&path)?.num_games(), 0);
    /// # Ok::<(), scidtopgn::Error>(())
    /// ```
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        remove_tree_cache(&path)?;

        let si4_path = path.with_extension("si4");
        let index = Index::create(&si4_path)?;

        let gfile = GFile::create(&path)?;

        let mut nfile = NFile::create(&path)?;
        nfile.close().map_err(|e| e.in_file(DatabaseFile::Name))?;

        Ok(Self {
            path,
            index,
            gfile,
            nfile,
            modified: false,
            saved_game_size: 0,
            tree_cache_stale: false,
        })
    }

    /// Returns the number of games, including logically deleted games.
    ///
    /// # Examples
    ///
    /// ```
    /// use scidtopgn::Database;
    ///
    /// # let dir = tempfile::tempdir()?;
    /// # let path = dir.path().join("games");
    /// # let mut db = Database::create(&path)?;
    /// # for pgn in ["[White \"Anand\"]\n\n1. e4 e5 *", "[White \"Carlsen\"]\n\n1. d4 d5 *"] {
    /// #     db.add_game(&scidtopgn::parse_single_game(pgn)?)?;
    /// # }
    /// # db.close()?;
    /// let db = Database::open(&path)?;
    /// assert_eq!(db.num_games(), 2);
    /// # Ok::<(), scidtopgn::Error>(())
    /// ```
    pub fn num_games(&self) -> u32 {
        self.index.num_games()
    }

    /// Returns the index entry of game `game_num`, counting from 0, or `None`
    /// if there is no such game. Logically deleted games have entries.
    ///
    /// # Examples
    ///
    /// Read only the games that aren't logically deleted, in order:
    ///
    /// ```
    /// use scidtopgn::Database;
    ///
    /// # let dir = tempfile::tempdir()?;
    /// # let path = dir.path().join("games");
    /// # let mut db = Database::create(&path)?;
    /// # for pgn in ["[White \"Anand\"]\n\n1. e4 e5 *", "[White \"Carlsen\"]\n\n1. d4 d5 *"] {
    /// #     db.add_game(&scidtopgn::parse_single_game(pgn)?)?;
    /// # }
    /// # db.delete_game(0)?;
    /// # db.close()?;
    /// let mut db = Database::open(&path)?;
    ///
    /// let mut white = Vec::new();
    /// for n in 0..db.num_games() {
    ///     if db.get_entry(n).is_some_and(|entry| !entry.get_delete_flag()) {
    ///         white.push(db.get_game(n)?.white);
    ///     }
    /// }
    ///
    /// // Game 0 is deleted. Game 1 keeps its number.
    /// assert_eq!(white, ["Carlsen"]);
    /// assert_eq!(db.num_games(), 2);
    /// # Ok::<(), scidtopgn::Error>(())
    /// ```
    pub fn get_entry(&self, game_num: u32) -> Option<&IndexEntry> {
        self.index.get_entry(game_num)
    }

    /// Reads game `game_num`, counting from 0, including a game that is
    /// logically deleted.
    ///
    /// The names, date, result, ratings, and ECO code come from the index
    /// and name files, and the moves, comments, and other tags from the game
    /// file. A name that is missing from the name file is `?`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::GameNotFound`] if there is no such game, and
    /// [`Error::Game`], which gives the game's number counting from 1, if it
    /// can't be read.
    ///
    /// # Examples
    ///
    /// ```
    /// use scidtopgn::{Database, Error};
    ///
    /// # let dir = tempfile::tempdir()?;
    /// # let path = dir.path().join("games");
    /// # let mut db = Database::create(&path)?;
    /// # for pgn in ["[White \"Anand\"]\n\n1. e4 e5 *", "[White \"Carlsen\"]\n\n1. d4 d5 *"] {
    /// #     db.add_game(&scidtopgn::parse_single_game(pgn)?)?;
    /// # }
    /// # db.close()?;
    /// let mut db = Database::open(&path)?;
    /// assert_eq!(db.get_game(1)?.white, "Carlsen");
    /// assert!(matches!(db.get_game(2), Err(Error::GameNotFound { number: 3 })));
    /// # Ok::<(), scidtopgn::Error>(())
    /// ```
    pub fn get_game(&mut self, game_num: u32) -> Result<Game> {
        let number = u64::from(game_num) + 1;
        let entry = self
            .index
            .get_entry(game_num)
            .ok_or(Error::GameNotFound { number })?;
        let in_game = |source| Error::Game {
            number,
            source: Box::new(source),
        };

        let offset = entry.get_offset();
        let length = entry.get_length();

        let mut buf = ByteBuffer::new();
        self.gfile
            .read_game(&mut buf, offset, length)
            .map_err(in_game)?;
        buf.back_to_start();

        let mut game = Game::new();
        game.decode(&mut buf, GAME_DECODE_ALL).map_err(in_game)?;

        let white_id = entry.get_white();
        let black_id = entry.get_black();
        let event_id = entry.get_event();
        let site_id = entry.get_site();
        let round_id = entry.get_round();

        let nb = self.nfile.namebase();

        game.white = nb
            .get_name(NameType::Player, white_id)
            .unwrap_or("?")
            .to_string();
        game.black = nb
            .get_name(NameType::Player, black_id)
            .unwrap_or("?")
            .to_string();
        game.event = nb
            .get_name(NameType::Event, event_id)
            .unwrap_or("?")
            .to_string();
        game.site = nb
            .get_name(NameType::Site, site_id)
            .unwrap_or("?")
            .to_string();
        game.round = nb
            .get_name(NameType::Round, round_id)
            .unwrap_or("?")
            .to_string();

        game.date = entry.get_date();

        let event_date_raw = entry.get_event_date();
        if event_date_raw != 0 {
            game.event_date = Some(Date::from_raw(event_date_raw));
        }

        game.result = entry.get_result();

        let white_elo = entry.get_white_elo();
        if white_elo > 0 {
            game.white_elo = Some(white_elo);
        }

        let black_elo = entry.get_black_elo();
        if black_elo > 0 {
            game.black_elo = Some(black_elo);
        }
        game.white_rating_type = RatingType::from_byte(entry.get_white_rating_type());
        game.black_rating_type = RatingType::from_byte(entry.get_black_rating_type());

        let eco_code = entry.get_eco_code();
        if eco_code > 0 {
            if let Some(eco_str) = entry.get_eco_string() {
                game.eco = Some(eco_str);
            }
        }

        Ok(game)
    }

    /// Adds `game` and returns its number, counting from 0. The game is
    /// written to the files by [`flush`](Self::flush) or
    /// [`close`](Self::close).
    ///
    /// As in SCID, the game's event date is kept only if the index can hold
    /// it; see [`Game::event_date`]. As in SCID, a tag name longer than 240
    /// bytes is cut to 240, and a player, event, site, or round name longer
    /// than 255 bytes is cut to 255. Unlike SCID, a cut that would split a
    /// character ends before it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::GameTooLarge`] if the game's record is longer than
    /// 131,071 bytes, the most the index can record,
    /// [`Error::TagTooLong`] if a tag's value is longer than 255 bytes,
    /// [`Error::IndexFull`], [`Error::NameBaseFull`], or
    /// [`Error::GameFileFull`] if the database can't hold another game,
    /// name, or record, [`Error::InvalidMove`] or
    /// [`Error::InvalidFen`] if the game's moves or starting position can't
    /// be stored, [`Error::ReadOnly`] if the database was opened with
    /// [`open_read_only`](Self::open_read_only), and [`Error::Io`] if the
    /// game file can't be written.
    ///
    /// # Examples
    ///
    /// ```
    /// use scidtopgn::{parse_single_game, Database};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let mut db = Database::create(dir.path().join("games"))?;
    ///
    /// let game = parse_single_game("[White \"Anand\"]\n\n1. e4 e5 *")?;
    /// assert_eq!(db.add_game(&game)?, 0);
    /// assert_eq!(db.add_game(&game)?, 1);
    /// db.close()?;
    /// # Ok::<(), scidtopgn::Error>(())
    /// ```
    pub fn add_game(&mut self, game: &Game) -> Result<u32> {
        self.check_writable()?;
        let entry = self.store_game(game)?;
        let game_num = self.index.add_entry(entry)?;
        let entry = self
            .index
            .get_entry(game_num)
            .expect("the entry was just added");
        count_names(
            self.nfile.namebase_mut(),
            entry,
            NameBase::increment_frequency,
        );

        self.modified = true;

        Ok(game_num)
    }

    /// Copies game `game_num` of `source`, counting from 0, to the end of
    /// this database without decoding it, as SCID's `scmerge` does, and
    /// returns its number here. The game is written to the files by
    /// [`flush`](Self::flush) or [`close`](Self::close).
    ///
    /// The game's record and index entry are copied as stored. Only the
    /// entry's name IDs, which refer to this database's names, and the
    /// record's offset change, so a logically deleted game stays deleted.
    /// The game's names are added to this database's names if they are
    /// missing, and counted. `source` may be opened with
    /// [`open_read_only`](Self::open_read_only).
    ///
    /// # Errors
    ///
    /// Returns [`Error::GameNotFound`] if `source` has no such game, and
    /// [`Error::Game`] for the game if its entry refers to a name that
    /// `source`'s name file doesn't have or its record can't be read from
    /// `source`'s game file. Returns [`Error::IndexFull`],
    /// [`Error::NameBaseFull`], or [`Error::GameFileFull`] if this database
    /// can't hold another game, name, or record, [`Error::ReadOnly`] if it
    /// was opened with [`open_read_only`](Self::open_read_only), and
    /// [`Error::Io`] if its game file can't be written. After an error, the
    /// index and the name frequencies are unchanged, though a new name or
    /// the record may stay in the files unused.
    ///
    /// # Examples
    ///
    /// ```
    /// use scidtopgn::{parse_single_game, Database};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let mut source = Database::create(dir.path().join("source"))?;
    /// source.add_game(&parse_single_game("[White \"Anand\"]\n\n1. e4 e5 *")?)?;
    /// source.delete_game(0)?;
    ///
    /// let mut db = Database::create(dir.path().join("games"))?;
    /// assert_eq!(db.copy_game(&mut source, 0)?, 0);
    /// assert!(db.get_entry(0).unwrap().get_delete_flag());
    /// assert_eq!(db.get_game(0)?.white, "Anand");
    /// # Ok::<(), scidtopgn::Error>(())
    /// ```
    pub fn copy_game(&mut self, source: &mut Database, game_num: u32) -> Result<u32> {
        self.check_writable()?;
        let mut entry = source
            .index
            .get_entry(game_num)
            .ok_or(Error::GameNotFound {
                number: u64::from(game_num) + 1,
            })?
            .clone();

        // Read everything from `source` before changing this database.
        let mut names = Vec::with_capacity(5);
        for (name_type, id) in name_ids(&entry) {
            let name = source
                .nfile
                .namebase()
                .get_name(name_type, id)
                .ok_or(Error::NameNotFound { name_type, id })
                .map_err(|e| in_game(game_num, e))?;
            names.push((name_type, name.to_owned()));
        }
        let record = source
            .gfile
            .read_game_data(entry.get_offset(), entry.get_length())
            .map_err(|e| in_game(game_num, e))?;

        let nb = self.nfile.namebase_mut();
        let ids = names
            .iter()
            .map(|(name_type, name)| nb.add_name(*name_type, name))
            .collect::<Result<Vec<_>>>()?;
        entry.set_white(ids[0]);
        entry.set_black(ids[1]);
        entry.set_event(ids[2]);
        entry.set_site(ids[3]);
        entry.set_round(ids[4]);
        entry.set_offset(self.gfile.add_game_data(&record)?);

        let game_num = self.index.add_entry(entry)?;
        let entry = self
            .index
            .get_entry(game_num)
            .expect("the entry was just added");
        count_names(
            self.nfile.namebase_mut(),
            entry,
            NameBase::increment_frequency,
        );

        self.modified = true;

        Ok(game_num)
    }

    /// Replaces game `game_num`, counting from 0, with `game`. The game
    /// keeps its number, and the change is written to the files by
    /// [`flush`](Self::flush) or [`close`](Self::close).
    ///
    /// As in SCID, the new record is added to the end of the game file, and
    /// the old one stays there unused until
    /// [`compact_games`](Self::compact_games). The game keeps its
    /// user-settable flags, including the logical delete flag. The names the
    /// game no longer uses stay in the name file, even when no game uses
    /// them, until [`compact_names`](Self::compact_names).
    ///
    /// The old record isn't read, so a game that can't be read can still be
    /// replaced.
    ///
    /// Writing the change deletes SCID's tree cache file (`.stc`), as SCID
    /// does when it saves a game. The number of games stays the same, so SCID
    /// would otherwise use the old game's results.
    ///
    /// # Errors
    ///
    /// Returns [`Error::GameNotFound`] if there is no such game, and
    /// otherwise the same errors as [`add_game`](Self::add_game). After an
    /// error, the game's index entry and the name frequencies are unchanged,
    /// though a new name or the new record may stay in the files unused.
    ///
    /// # Examples
    ///
    /// ```
    /// use scidtopgn::{parse_single_game, Database};
    ///
    /// # let dir = tempfile::tempdir()?;
    /// # let path = dir.path().join("games");
    /// # let mut db = Database::create(&path)?;
    /// # for pgn in ["[White \"Anand\"]\n\n1. e4 e5 *", "[White \"Carlsen\"]\n\n1. d4 d5 *"] {
    /// #     db.add_game(&parse_single_game(pgn)?)?;
    /// # }
    /// # db.close()?;
    /// let mut db = Database::open(&path)?;
    /// db.update_game(0, &parse_single_game("[White \"Kasparov\"]\n\n1. c4 *")?)?;
    /// db.close()?;
    ///
    /// let mut db = Database::open(&path)?;
    /// assert_eq!(db.get_game(0)?.white, "Kasparov");
    /// assert_eq!(db.num_games(), 2);
    /// # Ok::<(), scidtopgn::Error>(())
    /// ```
    pub fn update_game(&mut self, game_num: u32, game: &Game) -> Result<()> {
        self.check_writable()?;
        let old = self
            .index
            .get_entry(game_num)
            .ok_or(Error::GameNotFound {
                number: u64::from(game_num) + 1,
            })?
            .clone();

        let mut entry = self.store_game(game)?;
        entry.copy_user_flags(&old);

        // The old names lose 1 first, so a name the game keeps ends at 1 or
        // more, even if its count was wrongly 0.
        let nb = self.nfile.namebase_mut();
        count_names(nb, &old, NameBase::decrement_frequency);
        count_names(nb, &entry, NameBase::increment_frequency);

        *self
            .index
            .get_entry_mut(game_num)
            .expect("the entry was found above") = entry;
        self.modified = true;
        self.tree_cache_stale = true;

        Ok(())
    }

    /// Marks game `game_num`, counting from 0, as logically deleted. Its
    /// record, its number, and the name frequencies don't change, and
    /// [`undelete_game`](Self::undelete_game) reverses it. The change is
    /// written to the files by [`flush`](Self::flush) or
    /// [`close`](Self::close). See [Logically deleted
    /// games](#logically-deleted-games).
    ///
    /// Deleting a game that is already deleted does nothing.
    ///
    /// # Errors
    ///
    /// Returns [`Error::GameNotFound`] if there is no such game, and
    /// [`Error::ReadOnly`] if the database was opened with
    /// [`open_read_only`](Self::open_read_only).
    ///
    /// # Examples
    ///
    /// ```
    /// use scidtopgn::Database;
    ///
    /// # let dir = tempfile::tempdir()?;
    /// # let path = dir.path().join("games");
    /// # let mut db = Database::create(&path)?;
    /// # db.add_game(&scidtopgn::parse_single_game("1. e4 e5 *")?)?;
    /// # db.close()?;
    /// let mut db = Database::open(&path)?;
    /// db.delete_game(0)?;
    /// db.close()?;
    ///
    /// let db = Database::open(&path)?;
    /// assert!(db.get_entry(0).unwrap().get_delete_flag());
    /// assert_eq!(db.num_games(), 1);
    /// # Ok::<(), scidtopgn::Error>(())
    /// ```
    pub fn delete_game(&mut self, game_num: u32) -> Result<()> {
        self.set_delete_flag(game_num, true)
    }

    /// Clears the logical delete flag of game `game_num`, counting from 0,
    /// reversing [`delete_game`](Self::delete_game). The change is written
    /// to the files by [`flush`](Self::flush) or [`close`](Self::close).
    ///
    /// Undeleting a game that isn't deleted does nothing.
    ///
    /// # Errors
    ///
    /// Returns [`Error::GameNotFound`] if there is no such game, and
    /// [`Error::ReadOnly`] if the database was opened with
    /// [`open_read_only`](Self::open_read_only).
    pub fn undelete_game(&mut self, game_num: u32) -> Result<()> {
        self.set_delete_flag(game_num, false)
    }

    /// Finds twin games, copies of a game, and marks the copies SCID vs. PC
    /// would delete as logically deleted, as its Delete Twin Games command
    /// (`sc_base duplicates`) does. Returns the numbers of the games it
    /// deleted, counting from 0, in order. The changes are written to the
    /// files by [`flush`](Self::flush) or [`close`](Self::close), and
    /// [`compact_games`](Self::compact_games) removes the deleted games.
    ///
    /// Two games that aren't deleted are twins if they match in every way
    /// `options` selects. A game of at most 2 half-moves is also the twin of
    /// a game that matches it otherwise and has the same year, site, and
    /// round, as SCID takes it to hold the place of the full game, if the
    /// higher-numbered of the two games has a White and an event other than
    /// `?`. Of each pair of twins, the one [`TwinOptions::delete`] chooses
    /// is deleted. Pairs are compared in SCID's order, and a game deleted
    /// earlier doesn't match later, so of three or more twins the same games
    /// are kept as in SCID.
    ///
    /// Only delete flags change, and ratings if
    /// [`TwinOptions::copy_ratings`] is set. Records, names, and game numbers
    /// stay the same. As in SCID, the half-move counts, home pawn lists, and
    /// comment and variation counts come from the index. Version 0.1.0 of
    /// this crate stored them as 0, so each game it added counts as having
    /// no moves, as SCID also counts it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ReadOnly`] if the database was opened with
    /// [`open_read_only`](Self::open_read_only), and [`Error::Game`] for a
    /// game whose record can't be read from the game file, while comparing
    /// its moves. After an error, nothing has changed. Moves are decoded as
    /// SCID's twin search decodes them, without checking that they are
    /// legal, and a record whose moves can't be decoded is compared up to
    /// the move that can't be.
    ///
    /// # Examples
    ///
    /// ```
    /// use scidtopgn::{parse_single_game, Database, TwinOptions};
    ///
    /// # let dir = tempfile::tempdir()?;
    /// # let path = dir.path().join("games");
    /// # let mut db = Database::create(&path)?;
    /// let short = "[White \"Anand\"]\n[Black \"Carlsen\"]\n\n\
    ///     1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 Nf6 5. O-O Be7 *";
    /// let long = "[White \"Anand\"]\n[Black \"Carlsen\"]\n\n\
    ///     1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 4. Ba4 Nf6 5. O-O Be7 6. Re1 b5 *";
    /// db.add_game(&parse_single_game(long)?)?;
    /// db.add_game(&parse_single_game(short)?)?;
    ///
    /// assert_eq!(db.delete_twins(&TwinOptions::default())?, [1]);
    /// db.compact_games()?;
    /// assert_eq!(db.num_games(), 1);
    /// # Ok::<(), scidtopgn::Error>(())
    /// ```
    pub fn delete_twins(&mut self, options: &TwinOptions) -> Result<Vec<u32>> {
        self.check_writable()?;
        let changed = twins::find(&self.index, self.nfile.namebase(), &mut self.gfile, options)?;

        let mut deleted = Vec::new();
        for game in changed {
            let entry = self
                .index
                .get_entry_mut(game.game)
                .expect("twins are games of the index");
            if game.deleted {
                entry.set_delete_flag(true);
                deleted.push(game.game);
            }
            if game.ratings_copied {
                let [(white, white_type), (black, black_type)] = game.ratings;
                entry.set_white_elo(white);
                entry.set_white_rating_type(white_type);
                entry.set_black_elo(black);
                entry.set_black_rating_type(black_type);
            }
            self.modified = true;
        }
        deleted.sort_unstable();
        Ok(deleted)
    }

    /// Returns the games that meet every criterion of `search`, counting
    /// from 0, in order, as SCID vs. PC's header search (`sc_search
    /// header`) selects them. [`HeaderSearch`] describes each criterion.
    ///
    /// Only the index and the names are read, never a game's record. As in
    /// SCID, a game whose index entry gives it no record never matches, and
    /// logically deleted games match unless
    /// [`HeaderFlags::deleted`](crate::HeaderFlags::deleted) excludes them.
    /// The game length, side to move, and promotion, comment, variation, and
    /// annotation flags come from the index. Version 0.1.0 of this crate
    /// stored them as 0 for each game it added.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NameNotFound`] for a game whose index entry refers
    /// to a name the name file lacks, if that name is one the search
    /// compares.
    ///
    /// # Examples
    ///
    /// ```
    /// use scidtopgn::{parse_single_game, Database, GameResult, HeaderSearch};
    ///
    /// # let dir = tempfile::tempdir()?;
    /// # let path = dir.path().join("games");
    /// # let mut db = Database::create(&path)?;
    /// for pgn in [
    ///     "[White \"Carlsen, Magnus\"]\n[Black \"Anand\"]\n[Result \"1-0\"]\n\n1. e4 e5 1-0",
    ///     "[White \"Anand\"]\n[Black \"Carlsen, Magnus\"]\n[Result \"0-1\"]\n\n1. d4 d5 0-1",
    /// ] {
    ///     db.add_game(&parse_single_game(pgn)?)?;
    /// }
    ///
    /// let wins_as_white = HeaderSearch {
    ///     white: Some("carlsen".into()),
    ///     results: vec![GameResult::White],
    ///     ..HeaderSearch::default()
    /// };
    /// assert_eq!(db.search_header(&wins_as_white)?, [0]);
    ///
    /// // With colors ignored, the wins as Black match too.
    /// let wins = HeaderSearch {
    ///     ignore_colors: true,
    ///     ..wins_as_white
    /// };
    /// assert_eq!(db.search_header(&wins)?, [0, 1]);
    /// # Ok::<(), scidtopgn::Error>(())
    /// ```
    pub fn search_header(&self, search: &HeaderSearch) -> Result<Vec<u32>> {
        search::search(&self.index, self.nfile.namebase(), search)
    }

    fn set_delete_flag(&mut self, game_num: u32, deleted: bool) -> Result<()> {
        self.check_writable()?;
        let entry = self
            .index
            .get_entry_mut(game_num)
            .ok_or(Error::GameNotFound {
                number: u64::from(game_num) + 1,
            })?;
        if entry.get_delete_flag() != deleted {
            entry.set_delete_flag(deleted);
            self.modified = true;
        }
        Ok(())
    }

    /// Returns [`Error::ReadOnly`] if the database was opened with
    /// [`open_read_only`](Self::open_read_only).
    fn check_writable(&self) -> Result<()> {
        if self.gfile.file_mode() == FileMode::ReadOnly {
            return Err(Error::ReadOnly);
        }
        Ok(())
    }

    /// Adds `game`'s record to the game file and its names to the name base,
    /// and returns its index entry. The names' frequencies don't change.
    fn store_game(&mut self, game: &Game) -> Result<IndexEntry> {
        let (encoded_data, summary) = self.encode_game(game)?;
        let buf = ByteBuffer::from_slice(&encoded_data);
        let offset = self.gfile.add_game(&buf)?;

        let mut entry = IndexEntry::new();
        entry.set_offset(offset);
        entry.set_length(encoded_data.len() as u32);
        entry.set_start_flag(game.start_position.is_some());
        summary.set_fields(&mut entry);
        entry.set_stored_line_code(crate::stored_line::code(game));

        let nb = self.nfile.namebase_mut();

        let white_id = nb.add_name(NameType::Player, &game.white)?;
        let black_id = nb.add_name(NameType::Player, &game.black)?;
        let event_id = nb.add_name(NameType::Event, &game.event)?;
        let site_id = nb.add_name(NameType::Site, &game.site)?;
        let round_id = nb.add_name(NameType::Round, &game.round)?;

        entry.set_white(white_id);
        entry.set_black(black_id);
        entry.set_event(event_id);
        entry.set_site(site_id);
        entry.set_round(round_id);

        entry.set_date(game.date.raw());

        if let Some(event_date) = game.event_date {
            entry.set_event_date(event_date.raw());
        }

        entry.set_result(game.result);

        if let Some(elo) = game.white_elo {
            entry.set_white_elo(elo);
        }

        if let Some(elo) = game.black_elo {
            entry.set_black_elo(elo);
        }
        entry.set_white_rating_type(game.white_rating_type.to_byte());
        entry.set_black_rating_type(game.black_rating_type.to_byte());

        if let Some(ref eco) = game.eco {
            let eco_code = crate::index::eco_from_string(eco);
            if eco_code > 0 {
                entry.set_eco_code(eco_code);
            }
        }

        Ok(entry)
    }

    /// Encodes `game`'s record, and returns it with what the index entry
    /// records about its moves.
    fn encode_game(&self, game: &Game) -> Result<(Vec<u8>, MoveSummary)> {
        let mut buf = ByteBuffer::new();

        self.encode_tags(&mut buf, game)?;

        buf.put_byte(0);

        // The game flags depend on the moves, so they are set below.
        let flags_pos = buf.len();
        buf.put_byte(0);
        if let Some(ref start) = game.start_position {
            buf.put_terminated_string(&fen(start));
        }

        let mut summary = MoveSummary::new(game);
        if let Some(start_idx) = game.first_move {
            Self::encode_moves(
                &mut buf,
                game,
                start_idx,
                game.initial_position()?,
                &mut summary,
                true,
            )?;
        }

        buf.put_byte(crate::mov::ENCODE_END_GAME);

        self.encode_comments(&mut buf, game, &mut summary.comments)?;

        let mut record = buf.as_slice().to_vec();
        record[flags_pos] = summary.record_flags(game);
        Ok((record, summary))
    }

    /// Encodes `game`'s tags as SCID's `encodeTags` does: a common tag by its
    /// code, and others by name. Like SCID, it doesn't store the event date,
    /// which only the index entry holds.
    ///
    /// A name longer than [`MAX_TAG_LEN`](crate::game::MAX_TAG_LEN) bytes is
    /// cut to it, as SCID's `AddPgnTag` does, but before a character the cut
    /// would split.
    fn encode_tags(&self, buf: &mut ByteBuffer, game: &Game) -> Result<()> {
        for tag in &game.tags {
            if tag.value.len() > MAX_TAG_VALUE_LEN {
                return Err(Error::TagTooLong {
                    name: tag.name.clone(),
                    length: tag.value.len(),
                    max: MAX_TAG_VALUE_LEN,
                });
            }
            if let Some(code) = crate::game::common_tag_code(&tag.name) {
                buf.put_byte(code);
            } else {
                let len = tag
                    .name
                    .floor_char_boundary(crate::game::MAX_TAG_LEN as usize);
                buf.put_byte(len as u8);
                buf.put_fixed_string(&tag.name, len);
            }
            buf.put_byte(tag.value.len() as u8);
            buf.put_fixed_string(&tag.value, tag.value.len());
        }

        Ok(())
    }

    /// Encodes the line that starts at `start_idx`, played from `position`,
    /// with its variations, and adds them to `summary`. `main_line` is set
    /// for the game's main line.
    fn encode_moves(
        buf: &mut ByteBuffer,
        game: &Game,
        start_idx: usize,
        mut position: ScidPosition,
        summary: &mut MoveSummary,
        main_line: bool,
    ) -> Result<()> {
        let mut current_idx = Some(start_idx);

        while let Some(idx) = current_idx {
            if idx >= game.moves.len() {
                break;
            }

            let node = &game.moves[idx];

            if matches!(node.marker, Marker::EndMarker | Marker::EndGame) {
                break;
            }

            // A variation replaces this node's move, so it starts from the
            // position before it.
            let before = position.clone();
            if let Some(mv) = node.move_data {
                encode_move(buf, &position, mv)?;
                position.play(mv)?;
                if main_line {
                    summary.add_main_line_move(mv, position.chess().board());
                }
            }

            // Only the NAGs SCID can hold, so that it reads back each one
            // written and counted.
            for nag in node.stored_nags() {
                buf.put_byte(crate::mov::ENCODE_NAG);
                buf.put_byte(nag);
                summary.nags += 1;
            }

            if node.comment.is_some() {
                buf.put_byte(crate::mov::ENCODE_COMMENT);
            }

            for var_start in game.variations(idx) {
                buf.put_byte(crate::mov::ENCODE_START_MARKER);
                summary.variations += 1;
                Self::encode_moves(buf, game, var_start, before.clone(), summary, false)?;
                buf.put_byte(crate::mov::ENCODE_END_MARKER);
            }

            current_idx = node.next;
        }

        if main_line {
            summary.final_material = material_sig(position.chess().board());
        }

        Ok(())
    }

    /// Encodes the game's comments, adding the number encoded to `count`.
    fn encode_comments(&self, buf: &mut ByteBuffer, game: &Game, count: &mut u32) -> Result<()> {
        if let Some(start_idx) = game.first_move {
            Self::encode_comments_recursive(buf, game, start_idx, count)?;
        }
        Ok(())
    }

    fn encode_comments_recursive(
        buf: &mut ByteBuffer,
        game: &Game,
        start_idx: usize,
        count: &mut u32,
    ) -> Result<()> {
        let mut current_idx = Some(start_idx);

        while let Some(idx) = current_idx {
            if idx >= game.moves.len() {
                break;
            }

            let node = &game.moves[idx];

            match node.marker {
                crate::game::Marker::EndMarker | crate::game::Marker::EndGame => {
                    break;
                }
                _ => {}
            }

            if let Some(ref comment) = node.comment {
                buf.put_terminated_string(comment);
                *count += 1;
            }

            for var_start in game.variations(idx) {
                Self::encode_comments_recursive(buf, game, var_start, count)?;
            }

            current_idx = node.next;
        }

        Ok(())
    }

    /// Writes the games added, replaced, deleted, or undeleted since the
    /// database was opened, created, or last flushed: the game file's
    /// pending data, then the name file and the index file. Does nothing,
    /// and touches no file, if no game changed.
    ///
    /// The name file and index file are written in a temporary directory
    /// beside the database, and replace the database's files, with the same
    /// permissions, only once both are complete. If that fails, the
    /// database's name file and index file are as they were, and it reads
    /// the same games as before.
    ///
    /// If a game was replaced, SCID's tree cache file (`.stc`) is deleted
    /// once the new files are complete, just before they replace the old
    /// ones, so that it is never left beside the changes. Adding, deleting,
    /// or undeleting games keeps it: SCID ignores results cached for another
    /// number of games, and keeps them itself when it deletes a game.
    ///
    /// Names are written as UTF-8, including those read from the name file
    /// as Latin-1 or Windows-1252; see [Names that aren't
    /// UTF-8](#names-that-arent-utf-8). If two names of a kind read the
    /// same, they are merged into one, **the names get new IDs**, and the
    /// name IDs in every index entry change to match.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if a file can't be written or replaced, if the
    /// tree cache file exists and can't be deleted, or if the name file or
    /// index file is read-only or a symbolic link. Replacing
    /// a read-only file would ignore its protection, and replacing a link
    /// would leave the file it links to out of date. When names are merged,
    /// it returns [`Error::Game`] with [`Error::NameNotFound`] for the first
    /// game that refers to a name the name file doesn't have.
    ///
    /// After an error, the `Database` keeps its changes, so calling `flush`
    /// again writes them. The new games' records may already be in the game
    /// file, as unused bytes at its end: no index entry refers to them, and
    /// the records before them don't change. [`discard`](Self::discard) or
    /// [`compact_games`](Self::compact_games) removes them. If the files
    /// couldn't be replaced, the tree cache file stays deleted, which is
    /// harmless: SCID works out the results again without it.
    ///
    /// # Limits
    ///
    /// - A process killed while writing the files leaves the database as it
    ///   was, and beside it the temporary directory, named
    ///   `.scidtopgn-compact-*`, which can be removed.
    /// - Each file is moved aside before its replacement moves into place. If
    ///   a process is killed during those few renames, or a rename fails and
    ///   a file can't be moved back, that directory stays, holding the
    ///   original files as `old.si4` and `old.sn4`, and a file of the database
    ///   can be missing. Moving them back over the database's files makes the
    ///   database as it was.
    /// - The new files need disk space alongside the old ones, and the
    ///   database's directory must be writable, even if its files are.
    /// - The new files are new: a hard link to an old one keeps the old
    ///   contents, and they keep only the old files' permissions, not their
    ///   owner, access control lists, or extended attributes.
    /// - Nothing is synced to disk, so this doesn't protect against a power
    ///   loss. Other programs must not have the database open for writing.
    pub fn flush(&mut self) -> Result<()> {
        self.flush_with(NewFiles::create)
    }

    /// Does what [`flush`](Self::flush) does, writing the name file and index
    /// file to what `create` returns for each, so that a test can make
    /// writing them fail.
    fn flush_with<W: Write + Seek>(
        &mut self,
        mut create: impl FnMut(&mut NewFiles, &'static str) -> Result<W>,
    ) -> Result<()> {
        if !self.modified {
            return Ok(());
        }

        self.gfile.flush()?;
        check_replaceable(&self.path)?;

        let merged = match self.nfile.namebase().merged() {
            Some((namebase, new_ids)) => Some((namebase, renumbered(&self.index, &new_ids)?)),
            None => None,
        };
        let (namebase, index) = match &merged {
            Some((namebase, index)) => (namebase, index),
            None => (self.nfile.namebase(), &self.index),
        };
        let mut new = NewFiles::new(&self.path)?;
        write_names(namebase, create(&mut new, "sn4")?)?;
        write_index(index, create(&mut new, "si4")?)?;
        if self.tree_cache_stale {
            remove_tree_cache(&self.path)?;
        }
        new.replace()?;

        match merged {
            Some((namebase, index)) => {
                self.index = index;
                *self.nfile.namebase_mut() = namebase;
            }
            None => self.nfile.namebase_mut().clear_converted(),
        }
        self.index.mark_written();
        self.modified = false;
        self.saved_game_size = self.gfile.file_size();
        self.tree_cache_stale = false;

        Ok(())
    }

    /// Writes any changes, as [`flush`](Self::flush) does, and closes the
    /// database, with the same guarantee and limits: if writing fails, the
    /// database's name file and index file are as they were.
    ///
    /// # Errors
    ///
    /// The same as [`flush`](Self::flush). The changes are then lost, because
    /// the `Database` is gone, and the new games' records may stay in the
    /// game file as unused bytes at its end. To be able to try again, or to
    /// [`discard`](Self::discard) them, call [`flush`](Self::flush) first.
    ///
    /// # Examples
    ///
    /// ```
    /// use scidtopgn::{parse_single_game, Database};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let path = dir.path().join("games");
    /// let mut db = Database::create(&path)?;
    /// db.add_game(&parse_single_game("1. e4 *")?)?;
    /// db.close()?;
    ///
    /// assert_eq!(Database::open(&path)?.num_games(), 1);
    /// # Ok::<(), scidtopgn::Error>(())
    /// ```
    pub fn close(mut self) -> Result<()> {
        self.flush()
    }

    /// Closes the database without writing the changes made since it was
    /// opened, created, or last flushed, so that its files are as they were
    /// then.
    ///
    /// [`add_game`](Self::add_game) and [`update_game`](Self::update_game)
    /// write each block of the game file as it fills, before
    /// [`flush`](Self::flush), and dropping the database writes the last
    /// one. `discard` cuts the game file back to its earlier length, which
    /// removes those records, including any left by a game that failed to be
    /// added. It writes nothing to a database opened with
    /// [`open_read_only`](Self::open_read_only).
    ///
    /// # Errors
    ///
    /// Returns [`Error::File`] for [`DatabaseFile::Game`] if the game file
    /// can't be cut back.
    ///
    /// # Examples
    ///
    /// Add games, and keep none of them if one fails:
    ///
    /// ```
    /// use scidtopgn::{parse_pgn, Database};
    ///
    /// # let dir = tempfile::tempdir()?;
    /// # let path = dir.path().join("games");
    /// # let mut db = Database::create(&path)?;
    /// # db.add_game(&scidtopgn::parse_single_game("[White \"Anand\"]\n\n1. e4 e5 *")?)?;
    /// # db.close()?;
    /// let before = std::fs::read(path.with_extension("sg4"))?;
    ///
    /// let mut db = Database::open(&path)?;
    /// let big = format!("[White \"Carlsen\"]\n\n1. d4 {{{}}} d5 *", "x".repeat(140_000));
    /// let games = parse_pgn(&format!("[White \"Kasparov\"]\n\n1. c4 *\n\n{big}"))?;
    /// match games.iter().try_for_each(|game| db.add_game(game).map(drop)) {
    ///     Ok(()) => db.close()?,
    ///     Err(_) => db.discard()?,
    /// }
    ///
    /// assert_eq!(std::fs::read(path.with_extension("sg4"))?, before);
    /// assert_eq!(Database::open(&path)?.num_games(), 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn discard(mut self) -> Result<()> {
        if self.gfile.file_mode() == FileMode::ReadOnly {
            return Ok(());
        }
        self.gfile
            .discard_after(self.saved_game_size)
            .map_err(|e| e.in_file(DatabaseFile::Game))
    }

    /// Removes the logically deleted games, and the records that no game
    /// uses, as SCID's Compact games command does.
    ///
    /// The games that aren't deleted are copied in order to a new index file
    /// and game file, which replace the old ones. **Games are renumbered**
    /// from 0, so a game's number changes if a game before it was deleted.
    /// Each record and index entry is copied as stored, apart from where the
    /// record is, so a record that was replaced by
    /// [`update_game`](Self::update_game) is removed too. The index header is
    /// kept, apart from the number of games. As in SCID, its auto-load game
    /// is a game number that isn't changed, so it may name a different game.
    ///
    /// The name frequencies are counted again from the games that are left.
    /// Names that no game uses stay until
    /// [`compact_names`](Self::compact_names). Names that read the same are
    /// merged and get new IDs, as [`flush`](Self::flush) merges them. SCID's
    /// tree cache file (`.stc`), whose game numbers would be out of date, is
    /// deleted.
    ///
    /// Any unwritten changes are written first, as [`flush`](Self::flush)
    /// writes them. The new files are written in a temporary directory
    /// beside the database, and they replace the database's files only once
    /// they are complete, with the same permissions. No other program should
    /// have the database open while it is compacted.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ReadOnly`] if the database was opened with
    /// [`open_read_only`](Self::open_read_only), [`Error::Game`] for the
    /// first game that isn't deleted whose record can't be read from the
    /// game file or, when names are merged, that refers to a name the name
    /// file doesn't have, and [`Error::Io`] if a file can't be written or
    /// replaced.
    /// After an error, the database's files are as they were after the
    /// unwritten changes were written, or before them if those couldn't be
    /// written, unless a file that was moved aside couldn't be put back; it
    /// is then left in the temporary directory. The `Database` can still be
    /// used, except that if the new game file can't be opened, reading or
    /// adding a game returns [`Error::FileNotOpen`]; open the database again.
    ///
    /// # Examples
    ///
    /// ```
    /// use scidtopgn::{parse_single_game, Database};
    ///
    /// # let dir = tempfile::tempdir()?;
    /// # let path = dir.path().join("games");
    /// # let mut db = Database::create(&path)?;
    /// # for pgn in ["[White \"Anand\"]\n\n1. e4 e5 *", "[White \"Carlsen\"]\n\n1. d4 d5 *"] {
    /// #     db.add_game(&parse_single_game(pgn)?)?;
    /// # }
    /// # db.close()?;
    /// let mut db = Database::open(&path)?;
    /// db.delete_game(0)?;
    /// db.compact_games()?;
    ///
    /// assert_eq!(db.num_games(), 1);
    /// assert_eq!(db.get_game(0)?.white, "Carlsen");
    /// # Ok::<(), scidtopgn::Error>(())
    /// ```
    pub fn compact_games(&mut self) -> Result<()> {
        self.check_writable()?;
        self.flush()?;

        let (mut namebase, new_ids) = match self.nfile.namebase().merged() {
            Some((namebase, new_ids)) => (namebase, Some(new_ids)),
            None => (self.nfile.namebase().clone(), None),
        };

        let mut new = NewFiles::new(&self.path)?;
        let mut index = empty_copy(&self.index);
        let mut gfile = GFile::from_empty_file(new.create("sg4")?);
        for (game_num, entry) in (0u32..).zip(self.index.entries()) {
            if entry.get_delete_flag() {
                continue;
            }
            let mut entry = entry.clone();
            if let Some(new_ids) = &new_ids {
                renumber_names(&mut entry, new_ids).map_err(|e| in_game(game_num, e))?;
            }
            let record = self
                .gfile
                .read_game_data(entry.get_offset(), entry.get_length())
                .map_err(|e| in_game(game_num, e))?;
            entry.set_offset(gfile.add_game_data(&record)?);
            index.add_entry(entry)?;
        }
        gfile.close()?;

        namebase.clear_frequencies();
        namebase.clear_converted();
        for entry in index.entries() {
            count_names(&mut namebase, entry, NameBase::increment_frequency);
        }
        write_names(&namebase, new.create("sn4")?)?;
        write_index(&index, new.create("si4")?)?;

        remove_tree_cache(&self.path)?;

        // Windows can't rename a file that is open.
        self.gfile.close()?;
        let replaced = new.replace();
        if replaced.is_ok() {
            self.index = index;
            self.index.mark_written();
            *self.nfile.namebase_mut() = namebase;
        }
        let reopened = GFile::open_file(&self.path, FileMode::Both).map(|gfile| self.gfile = gfile);
        replaced.and(reopened)?;
        self.saved_game_size = self.gfile.file_size();
        Ok(())
    }

    /// Removes the names that no game uses, as SCID's Compact names command
    /// does.
    ///
    /// The name frequencies are counted again from every game, without
    /// trusting those in the name file. Names that no game uses are dropped,
    /// and **the others get new IDs**, from 0 in the order of their old ones.
    /// The name IDs in every index entry change to match, and nothing else
    /// does. As in SCID, a name that only logically deleted games use is
    /// kept; call [`compact_games`](Self::compact_games) first to drop it.
    /// Names that read the same are merged, as [`flush`](Self::flush)
    /// merges them. If every name is used and none are merged, the IDs
    /// don't change: the name file is written again only if it held names
    /// that weren't UTF-8, which it then stores as UTF-8, and otherwise no
    /// file is changed.
    ///
    /// Any unwritten changes are written first, as [`flush`](Self::flush)
    /// writes them. The new name file and index file are written in a
    /// temporary directory beside the database, and they replace the
    /// database's files only once both are complete, with the same
    /// permissions. No other program should have the database open while
    /// its names are compacted.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ReadOnly`] if the database was opened with
    /// [`open_read_only`](Self::open_read_only), [`Error::Game`] with
    /// [`Error::NameNotFound`] for the first game that refers to a name the
    /// name file doesn't have, and [`Error::Io`] if a file can't be written
    /// or replaced. After an error, the database's files are as they were
    /// after the unwritten changes were written, or before them if those
    /// couldn't be written, unless a file that was moved aside couldn't be
    /// put back; it is then left in the temporary directory. The `Database`
    /// can still be used.
    ///
    /// # Examples
    ///
    /// ```
    /// use scidtopgn::{parse_single_game, Database, NameType};
    ///
    /// # let dir = tempfile::tempdir()?;
    /// # let path = dir.path().join("games");
    /// # let mut db = Database::create(&path)?;
    /// # db.add_game(&parse_single_game("[White \"Anand\"]\n\n1. e4 e5 *")?)?;
    /// # db.close()?;
    /// let mut db = Database::open(&path)?;
    /// db.update_game(0, &parse_single_game("[White \"Carlsen\"]\n\n1. d4 d5 *")?)?;
    /// let players = db.name_count(NameType::Player);
    ///
    /// // Anand is no longer used.
    /// db.compact_names()?;
    /// assert_eq!(db.name_count(NameType::Player), players - 1);
    /// assert_eq!(db.get_game(0)?.white, "Carlsen");
    /// # Ok::<(), scidtopgn::Error>(())
    /// ```
    pub fn compact_names(&mut self) -> Result<()> {
        self.check_writable()?;
        self.flush()?;

        let mut counted = self.nfile.namebase().clone();
        counted.clear_frequencies();
        for (game_num, entry) in (0u32..).zip(self.index.entries()) {
            for (name_type, id) in name_ids(entry) {
                if counted.get_name(name_type, id).is_none() {
                    return Err(in_game(game_num, Error::NameNotFound { name_type, id }));
                }
                counted.increment_frequency(name_type, id);
            }
        }
        let Some((namebase, new_ids)) = counted.without_unused() else {
            if counted.converted_names().is_empty() {
                return Ok(());
            }
            let mut new = NewFiles::new(&self.path)?;
            write_names(&counted, new.create("sn4")?)?;
            new.replace()?;
            counted.clear_converted();
            *self.nfile.namebase_mut() = counted;
            return Ok(());
        };
        let index = renumbered(&self.index, &new_ids)?;

        let mut new = NewFiles::new(&self.path)?;
        write_names(&namebase, new.create("sn4")?)?;
        write_index(&index, new.create("si4")?)?;
        new.replace()?;
        self.index = index;
        self.index.mark_written();
        *self.nfile.namebase_mut() = namebase;
        Ok(())
    }

    /// Returns an iterator that reads every game in order, including
    /// logically deleted games.
    ///
    /// Each item is the result of [`get_game`](Self::get_game). A game that
    /// can't be read gives an error, and the iterator goes on to the next
    /// game. To skip deleted games, see the example for
    /// [`get_entry`](Self::get_entry).
    ///
    /// # Examples
    ///
    /// ```
    /// use scidtopgn::Database;
    ///
    /// # let dir = tempfile::tempdir()?;
    /// # let path = dir.path().join("games");
    /// # let mut db = Database::create(&path)?;
    /// # for pgn in ["[White \"Anand\"]\n\n1. e4 e5 *", "[White \"Carlsen\"]\n\n1. d4 d5 *"] {
    /// #     db.add_game(&scidtopgn::parse_single_game(pgn)?)?;
    /// # }
    /// # db.close()?;
    /// let mut db = Database::open(&path)?;
    /// let white = db
    ///     .games()
    ///     .map(|game| Ok(game?.white))
    ///     .collect::<scidtopgn::Result<Vec<_>>>()?;
    /// assert_eq!(white, ["Anand", "Carlsen"]);
    /// # Ok::<(), scidtopgn::Error>(())
    /// ```
    pub fn games(&mut self) -> GameIterator<'_> {
        let total = self.num_games();
        GameIterator {
            db: self,
            current: 0,
            total,
        }
    }

    /// Returns the number of names of kind `name_type` in the name file.
    ///
    /// # Examples
    ///
    /// ```
    /// use scidtopgn::{parse_single_game, Database, NameType};
    ///
    /// # let dir = tempfile::tempdir()?;
    /// # let path = dir.path().join("games");
    /// let mut db = Database::create(&path)?;
    /// db.add_game(&parse_single_game("[White \"Anand\"]\n[Black \"Carlsen\"]\n\n1. e4 e5 *")?)?;
    /// assert_eq!(db.name_count(NameType::Player), 2);
    /// # Ok::<(), scidtopgn::Error>(())
    /// ```
    pub fn name_count(&self, name_type: NameType) -> u32 {
        self.nfile.namebase().count(name_type)
    }

    /// Returns the kind and text of each name that wasn't valid UTF-8 in
    /// the name file and was read as Windows-1252, in the order the file
    /// stores them. Older versions of SCID store the bytes of a Latin-1 or
    /// Windows-1252 PGN file unchanged, so their name files can hold such
    /// names.
    ///
    /// The list is empty once the database has written the names as UTF-8,
    /// as described in [`Database`].
    pub fn converted_names(&self) -> &[(NameType, String)] {
        self.nfile.namebase().converted_names()
    }

    /// Returns the index.
    #[cfg(test)]
    pub(crate) fn index(&self) -> &Index {
        &self.index
    }

    /// Returns the name file.
    #[cfg(test)]
    pub(crate) fn nfile(&self) -> &NFile {
        &self.nfile
    }

    /// Returns the game file.
    #[cfg(test)]
    pub(crate) fn gfile(&self) -> &GFile {
        &self.gfile
    }

    /// Returns the database's path, without an extension.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// What an index entry records about a game's moves, collected while its
/// record is encoded, as SCID's `Game::Encode` does. SCID uses these fields
/// to skip games that can't match a position or material search.
struct MoveSummary {
    /// The number of half-moves in the main line, including null moves.
    half_moves: u32,
    /// The pawns that leave their starting squares in the main line, or
    /// `None` for a game that starts from a FEN, whose list SCID leaves
    /// empty.
    home_pawns: Option<HomePawnList>,
    /// Whether the main line has a promotion.
    promotion: bool,
    /// Whether the main line has a promotion to a piece other than a queen.
    underpromotion: bool,
    /// The signature of the material at the end of the main line.
    final_material: u32,
    /// The number of variations, at any depth.
    variations: u32,
    /// The number of comments, including those before a line's first move.
    comments: u32,
    /// The number of NAGs.
    nags: u32,
}

impl MoveSummary {
    /// The summary of `game` before any move.
    fn new(game: &Game) -> Self {
        let (home_pawns, final_material) = match &game.start_position {
            Some(start) => (None, material_sig(start.board())),
            None => (Some(HomePawnList::new()), material_sig(&Board::default())),
        };
        MoveSummary {
            half_moves: 0,
            home_pawns,
            promotion: false,
            underpromotion: false,
            final_material,
            variations: 0,
            comments: 0,
            nags: 0,
        }
    }

    /// Adds the main line move `mv`, which led to `board`.
    fn add_main_line_move(&mut self, mv: GameMove, board: &Board) {
        self.half_moves += 1;
        if let GameMove::Move(m) = mv {
            if let Some(role) = m.promotion() {
                self.promotion = true;
                self.underpromotion |= role != Role::Queen;
            }
        }
        if let Some(home_pawns) = &mut self.home_pawns {
            home_pawns.update(board);
        }
    }

    /// The game flags byte of `game`'s record: 1 if it starts from a FEN,
    /// plus 2 if it has a promotion and 4 if it has an underpromotion.
    fn record_flags(&self, game: &Game) -> u8 {
        u8::from(game.start_position.is_some())
            | (u8::from(self.promotion) << 1)
            | (u8::from(self.underpromotion) << 2)
    }

    /// Sets the fields of `entry` that describe the moves.
    fn set_fields(&self, entry: &mut IndexEntry) {
        // SCID stores the count modulo 1,024.
        entry.set_num_half_moves((self.half_moves % 1024) as u16);
        if let Some(home_pawns) = &self.home_pawns {
            entry.set_home_pawn_data(home_pawns.data());
        }
        entry.set_flag(IDX_MASK_PROMO, self.promotion);
        entry.set_flag(IDX_MASK_UPROMO, self.underpromotion);
        entry.set_final_mat_sig(self.final_material);
        let count = |n: u32| u16::try_from(n).unwrap_or(u16::MAX);
        entry.set_variation_count(count(self.variations));
        entry.set_comment_count(count(self.comments));
        entry.set_nag_count(count(self.nags));
    }
}

/// Returns the kind and ID of each of the five names `entry` refers to.
fn name_ids(entry: &IndexEntry) -> [(NameType, u32); 5] {
    [
        (NameType::Player, entry.get_white()),
        (NameType::Player, entry.get_black()),
        (NameType::Event, entry.get_event()),
        (NameType::Site, entry.get_site()),
        (NameType::Round, entry.get_round()),
    ]
}

/// Changes the five name IDs of `entry` to their new IDs in `new_ids`.
///
/// # Errors
///
/// Returns [`Error::NameNotFound`] for the first ID that has no new ID.
fn renumber_names(entry: &mut IndexEntry, new_ids: &NewIds) -> Result<()> {
    let new_id = |name_type: NameType, id: u32| {
        new_ids[name_type.as_usize()]
            .get(id as usize)
            .copied()
            .flatten()
            .ok_or(Error::NameNotFound { name_type, id })
    };
    entry.set_white(new_id(NameType::Player, entry.get_white())?);
    entry.set_black(new_id(NameType::Player, entry.get_black())?);
    entry.set_event(new_id(NameType::Event, entry.get_event())?);
    entry.set_site(new_id(NameType::Site, entry.get_site())?);
    entry.set_round(new_id(NameType::Round, entry.get_round())?);
    Ok(())
}

/// Returns a copy of `index` whose entries refer to names by their new IDs
/// in `new_ids`.
///
/// # Errors
///
/// Returns [`Error::Game`] with [`Error::NameNotFound`] for the first game
/// that refers to an ID that has no new ID.
fn renumbered(index: &Index, new_ids: &NewIds) -> Result<Index> {
    let mut copy = empty_copy(index);
    for (game_num, entry) in (0u32..).zip(index.entries()) {
        let mut entry = entry.clone();
        renumber_names(&mut entry, new_ids).map_err(|e| in_game(game_num, e))?;
        copy.add_entry(entry)?;
    }
    Ok(copy)
}

/// Wraps `source` in an [`Error::Game`] for game `game_num`, counting from
/// 0.
fn in_game(game_num: u32, source: Error) -> Error {
    Error::Game {
        number: u64::from(game_num) + 1,
        source: Box::new(source),
    }
}

/// Calls `change` on the frequency of each of the five names `entry` refers
/// to.
fn count_names(nb: &mut NameBase, entry: &IndexEntry, change: fn(&mut NameBase, NameType, u32)) {
    for (name_type, id) in name_ids(entry) {
        change(nb, name_type, id);
    }
}

/// Returns an index with no entries and the header of `index`, apart from
/// the number of games.
fn empty_copy(index: &Index) -> Index {
    let mut copy = Index::new();
    *copy.header_mut() = index.header().clone();
    copy.header_mut().num_games = 0;
    copy
}

/// Returns an error if the database at `path` has a name file or index file
/// that [`Database::flush`] shouldn't replace.
fn check_replaceable(path: &Path) -> Result<()> {
    for ext in ["sn4", "si4"] {
        let file = path.with_extension(ext);
        // A missing file is reported when it would be replaced.
        let Ok(metadata) = fs::symlink_metadata(&file) else {
            continue;
        };
        let problem = if metadata.is_symlink() {
            "is a symbolic link, which writing the database would replace with a file"
        } else if metadata.permissions().readonly() {
            "is read-only"
        } else {
            continue;
        };
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("'{}' {problem}", file.display()),
        )
        .into());
    }
    Ok(())
}

/// Deletes SCID's tree cache file of the database at `path`, if it has one.
fn remove_tree_cache(path: &Path) -> Result<()> {
    match fs::remove_file(path.with_extension("stc")) {
        Err(e)
            if !matches!(
                e.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
            ) =>
        {
            Err(e.into())
        }
        _ => Ok(()),
    }
}

/// Writes `index` to `file` as an index file.
fn write_index<W: Write + Seek>(index: &Index, file: W) -> Result<()> {
    index.write_to(&mut BufWriter::new(file))
}

/// Writes `namebase` to `file` as a name file.
fn write_names<W: Write>(namebase: &NameBase, file: W) -> Result<()> {
    let mut writer = BufWriter::new(file);
    namebase.write_sn4(&mut writer)?;
    writer.flush()?;
    Ok(())
}

/// An iterator over the games of a [`Database`], made by
/// [`Database::games`].
pub struct GameIterator<'a> {
    db: &'a mut Database,
    current: u32,
    total: u32,
}

impl<'a> Iterator for GameIterator<'a> {
    type Item = Result<Game>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.total {
            return None;
        }

        let game_num = self.current;
        self.current += 1;

        Some(self.db.get_game(game_num))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_single_game;

    fn game(white: &str, round: &str) -> Game {
        parse_single_game(&format!(
            "[White \"{white}\"]\n[Round \"{round}\"]\n\n1. e4 *\n"
        ))
        .unwrap()
    }

    /// A new file that fails once `left` bytes have been written to it.
    struct LimitedFile {
        file: fs::File,
        left: usize,
    }

    impl Write for LimitedFile {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if self.left == 0 {
                return Err(io::Error::other("injected failure"));
            }
            let written = self.file.write(&buf[..buf.len().min(self.left)])?;
            self.left -= written;
            Ok(written)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.file.flush()
        }
    }

    impl Seek for LimitedFile {
        fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
            self.file.seek(pos)
        }
    }

    /// Flushes `db`, with writing the new file with extension `failing`
    /// failing part way, and checks the error.
    fn fail_flush(db: &mut Database, failing: &str) {
        let result = db.flush_with(|new, ext| {
            let file = new.create(ext)?;
            let left = if ext == failing { 16 } else { usize::MAX };
            Ok(LimitedFile { file, left })
        });
        assert!(
            matches!(&result, Err(Error::Io(e)) if e.to_string() == "injected failure"),
            "{result:?}"
        );
    }

    /// Copies the fixture `name` into `dir`.
    fn copy_of(dir: &Path, name: &str) -> PathBuf {
        let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/data");
        let path = dir.join(name);
        for ext in ["si4", "sn4", "sg4"] {
            fs::copy(
                data.join(name).with_extension(ext),
                path.with_extension(ext),
            )
            .unwrap();
        }
        path
    }

    fn all_pgn(db: &mut Database) -> Vec<String> {
        db.games().map(|game| game.unwrap().to_pgn()).collect()
    }

    fn file_bytes(path: &Path, ext: &str) -> Vec<u8> {
        fs::read(path.with_extension(ext)).unwrap()
    }

    /// The names of the files in `dir`, sorted.
    fn files_in(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    /// Fails to write the new file with extension `failing` for a game added
    /// to the fixture `name`, and checks that the database is as it was and
    /// that writing again succeeds.
    fn check_failed_write(name: &str, failing: &str) {
        let temp = tempfile::TempDir::new().unwrap();
        let path = copy_of(temp.path(), name);
        let files = files_in(temp.path());
        let si4 = file_bytes(&path, "si4");
        let sn4 = file_bytes(&path, "sn4");
        let sg4 = file_bytes(&path, "sg4");

        let mut db = Database::open(&path).unwrap();
        let games = all_pgn(&mut db);
        let converted = db.nfile.namebase().converted_names().len();
        assert!(converted > 0, "{name}");
        db.add_game(&game("A new player", "1")).unwrap();
        fail_flush(&mut db, failing);

        assert_eq!(file_bytes(&path, "si4"), si4, "{name} {failing}");
        assert_eq!(file_bytes(&path, "sn4"), sn4, "{name} {failing}");
        // The new game's record is written, after the others.
        let new_sg4 = file_bytes(&path, "sg4");
        assert!(new_sg4.len() > sg4.len(), "{name} {failing}");
        assert!(new_sg4.starts_with(&sg4), "{name} {failing}");
        assert_eq!(files_in(temp.path()), files, "{name} {failing}");
        let mut reopened = Database::open_read_only(&path).unwrap();
        assert_eq!(all_pgn(&mut reopened), games, "{name} {failing}");

        // The changes are kept for another try.
        assert!(db.modified, "{name} {failing}");
        assert!(db.index.is_dirty(), "{name} {failing}");
        assert_eq!(db.nfile.namebase().converted_names().len(), converted);
        db.flush().unwrap();
        assert!(!db.modified, "{name} {failing}");
        assert!(!db.index.is_dirty(), "{name} {failing}");
        assert_eq!(db.nfile.namebase().converted_names(), []);
        db.close().unwrap();

        assert_eq!(files_in(temp.path()), files, "{name} {failing}");
        let mut reopened = Database::open(&path).unwrap();
        let after = all_pgn(&mut reopened);
        assert_eq!(after[..games.len()], games, "{name} {failing}");
        assert_eq!(after.len(), games.len() + 1, "{name} {failing}");
        let last = reopened.num_games() - 1;
        assert_eq!(reopened.get_game(last).unwrap().white, "A new player");
    }

    #[test]
    fn test_a_failed_write_of_either_file_leaves_the_database_as_it_was() {
        for failing in ["sn4", "si4"] {
            // Names converted from Latin-1, none of them the same.
            check_failed_write("latin1", failing);
            // Names that are merged and get new IDs.
            check_failed_write("mixed", failing);
        }
    }

    #[test]
    fn test_discard_after_a_failed_write_restores_the_game_file() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = copy_of(temp.path(), "latin1");
        let before = ["si4", "sn4", "sg4"].map(|ext| file_bytes(&path, ext));

        let mut db = Database::open(&path).unwrap();
        db.add_game(&game("A new player", "1")).unwrap();
        fail_flush(&mut db, "si4");
        assert_ne!(file_bytes(&path, "sg4"), before[2]);
        db.discard().unwrap();

        assert_eq!(
            ["si4", "sn4", "sg4"].map(|ext| file_bytes(&path, ext)),
            before
        );
    }

    #[test]
    fn test_appending_after_a_failed_write_keeps_every_game_readable() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = copy_of(temp.path(), "latin1");

        let mut db = Database::open(&path).unwrap();
        let games = all_pgn(&mut db);
        db.add_game(&game("Lost", "1")).unwrap();
        fail_flush(&mut db, "si4");
        drop(db);

        // The record that failed stays in the game file, unused.
        let mut db = Database::open(&path).unwrap();
        db.add_game(&game("Kept", "1")).unwrap();
        db.close().unwrap();

        let mut db = Database::open(&path).unwrap();
        let after = all_pgn(&mut db);
        assert_eq!(after[..games.len()], games);
        assert_eq!(after.len(), games.len() + 1);
        assert_eq!(db.get_game(db.num_games() - 1).unwrap().white, "Kept");
    }

    #[test]
    fn test_a_failed_write_keeps_the_tree_cache_until_writing_succeeds() {
        for failing in ["sn4", "si4"] {
            let temp = tempfile::TempDir::new().unwrap();
            let path = copy_of(temp.path(), "five");
            let stc = path.with_extension("stc");
            fs::write(&stc, "cache").unwrap();

            let mut db = Database::open(&path).unwrap();
            db.update_game(0, &game("A new player", "1")).unwrap();
            fail_flush(&mut db, failing);
            assert!(stc.exists(), "{failing}");

            db.flush().unwrap();
            assert!(!stc.exists(), "{failing}");
        }
    }

    fn frequency(db: &Database, ntype: NameType, name: &str) -> u32 {
        let nb = db.nfile.namebase();
        nb.get_frequency(ntype, nb.find_name(ntype, name).unwrap())
    }

    #[test]
    fn test_update_game_counts_a_kept_name_whose_count_is_0() {
        let temp = tempfile::TempDir::new().unwrap();
        let mut db = Database::create(temp.path().join("db")).unwrap();
        db.add_game(&game("A", "1")).unwrap();
        let id = db.index.get_entry(0).unwrap().get_white();
        db.nfile
            .namebase_mut()
            .decrement_frequency(NameType::Player, id);
        assert_eq!(frequency(&db, NameType::Player, "A"), 0);

        db.update_game(0, &game("A", "1")).unwrap();
        assert_eq!(frequency(&db, NameType::Player, "A"), 1);
    }

    #[test]
    fn test_full_name_base_leaves_the_entry_and_frequencies_unchanged() {
        let temp = tempfile::TempDir::new().unwrap();
        let mut db = Database::create(temp.path().join("db")).unwrap();
        db.add_game(&game("A", "1")).unwrap();
        let nb = db.nfile.namebase_mut();
        for n in nb.count(NameType::Round)..NameType::Round.max_id() {
            nb.add_name(NameType::Round, &format!("r{n}")).unwrap();
        }
        let entry = db.index.get_entry(0).unwrap().clone();
        let full = |result: Result<()>| {
            matches!(
                result,
                Err(Error::NameBaseFull {
                    name_type: NameType::Round
                })
            )
        };

        assert!(full(db.update_game(0, &game("B", "new"))));
        assert!(full(db.add_game(&game("B", "new")).map(|_| ())));
        let mut source = Database::create(temp.path().join("source")).unwrap();
        source.add_game(&game("B", "new")).unwrap();
        assert!(full(db.copy_game(&mut source, 0).map(|_| ())));

        let mut before = Vec::new();
        entry.write(&mut before).unwrap();
        let mut after = Vec::new();
        db.index.get_entry(0).unwrap().write(&mut after).unwrap();
        assert_eq!(after, before);
        assert_eq!(db.num_games(), 1);
        assert_eq!(frequency(&db, NameType::Player, "A"), 1);
        assert_eq!(frequency(&db, NameType::Round, "1"), 1);
        // The new White was added before the round failed, and is unused.
        assert_eq!(frequency(&db, NameType::Player, "B"), 0);
    }
}
