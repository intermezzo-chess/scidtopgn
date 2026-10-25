//! [`Database`], which reads and writes a SCID database's three files
//! together.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::bytebuf::ByteBuffer;
use crate::error::{Error, Result};
use crate::game::{Game, Marker, GAME_DECODE_ALL};
use crate::gfile::GFile;
use crate::index::{Index, IndexEntry};
use crate::mov::encode_move;
use crate::namebase::NameType;
use crate::nfile::NFile;
use crate::position::{fen, ScidPosition};
use crate::Date;

/// A SCID database: an index file (`.si4`), a name file (`.sn4`), and a game
/// file (`.sg4`) that share a path without the extension.
///
/// Games are numbered from 0 in the order they were added. A game marked as
/// logically deleted keeps its record and its number, and the methods that
/// read games include it. See [Logically deleted
/// games](#logically-deleted-games).
///
/// Changes are written by [`flush`](Self::flush) and
/// [`close`](Self::close). Dropping a `Database` with unwritten changes
/// loses them.
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
/// SCID deletes a game by setting its delete flag. The game's record stays
/// in the database, and its number doesn't change, until the database is
/// compacted. [`num_games`](Self::num_games), [`games`](Self::games), and
/// [`get_game`](Self::get_game) include deleted games, which is useful for
/// inspecting or recovering them. The `scidtopgn` command skips them when
/// it exports, as SCID does. To do the same, check each game's
/// [`IndexEntry::get_delete_flag`] before reading it, as the example for
/// [`get_entry`](Self::get_entry) shows.
pub struct Database {
    path: PathBuf,
    index: Index,
    gfile: GFile,
    nfile: NFile,
    modified: bool,
}

impl Database {
    /// Opens the database at `path`, given without an extension, for
    /// reading and adding games.
    ///
    /// # Errors
    ///
    /// Returns [`Error::File`] naming the file that couldn't be opened or is
    /// damaged.
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
        let path = path.as_ref().to_path_buf();

        let si4_path = path.with_extension("si4");
        let index = Index::open(&si4_path)?;

        let gfile = GFile::open(&path)?;

        let sn4_path = path.with_extension("sn4");
        let nfile = NFile::open(&sn4_path)?;

        Ok(Self {
            path,
            index,
            gfile,
            nfile,
            modified: false,
        })
    }

    /// Creates an empty database at `path`, given without an extension,
    /// replacing the files of any database already there.
    ///
    /// The index and game files are created at once, and the name file when
    /// the changes are first written.
    ///
    /// # Errors
    ///
    /// Returns [`Error::File`] naming the file that couldn't be created.
    ///
    /// # Examples
    ///
    /// ```
    /// use scidtopgn::{parse_single_game, Database};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let path = dir.path().join("games");
    ///
    /// let mut db = Database::create(&path)?;
    /// db.add_game(&parse_single_game("1. e4 e5 *")?)?;
    /// db.close()?;
    ///
    /// assert!(path.with_extension("si4").exists());
    /// assert!(path.with_extension("sn4").exists());
    /// assert!(path.with_extension("sg4").exists());
    /// # Ok::<(), scidtopgn::Error>(())
    /// ```
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        let si4_path = path.with_extension("si4");
        let index = Index::create(&si4_path)?;

        let gfile = GFile::create(&path)?;

        let nfile = NFile::create(&path)?;

        Ok(Self {
            path,
            index,
            gfile,
            nfile,
            modified: false,
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
    /// use scidtopgn::{Database, Index};
    ///
    /// # let dir = tempfile::tempdir()?;
    /// # let path = dir.path().join("games");
    /// # let mut db = Database::create(&path)?;
    /// # for pgn in ["[White \"Anand\"]\n\n1. e4 e5 *", "[White \"Carlsen\"]\n\n1. d4 d5 *"] {
    /// #     db.add_game(&scidtopgn::parse_single_game(pgn)?)?;
    /// # }
    /// # db.close()?;
    /// # let si4 = path.with_extension("si4");
    /// # let mut index = Index::open(&si4)?;
    /// # index.get_entry_mut(0).unwrap().set_delete_flag(true);
    /// # index.flush(&mut std::fs::File::create(&si4)?)?;
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

        game.date = entry.get_date_obj();

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
    /// # Errors
    ///
    /// Returns [`Error::GameTooLarge`] if the game's record is longer than
    /// 131,071 bytes, the most the index can record,
    /// [`Error::IndexFull`] or [`Error::NameBaseFull`] if the database can't
    /// hold another game or name, [`Error::InvalidMove`] or
    /// [`Error::InvalidFen`] if the game's moves or starting position can't
    /// be stored, and [`Error::Io`] if the game file can't be written.
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
        let encoded_data = self.encode_game(game)?;
        let buf = ByteBuffer::from_slice(&encoded_data);
        let offset = self.gfile.add_game(&buf)?;

        let mut entry = IndexEntry::new();
        entry.set_offset(offset);
        entry.set_length(encoded_data.len() as u32);
        entry.set_start_flag(game.start_position.is_some());

        let nb = self.nfile.namebase_mut();

        let white_id = nb.add_name(NameType::Player, &game.white)?;
        nb.increment_frequency(NameType::Player, white_id);
        entry.set_white(white_id);

        let black_id = nb.add_name(NameType::Player, &game.black)?;
        nb.increment_frequency(NameType::Player, black_id);
        entry.set_black(black_id);

        let event_id = nb.add_name(NameType::Event, &game.event)?;
        nb.increment_frequency(NameType::Event, event_id);
        entry.set_event(event_id);

        let site_id = nb.add_name(NameType::Site, &game.site)?;
        nb.increment_frequency(NameType::Site, site_id);
        entry.set_site(site_id);

        let round_id = nb.add_name(NameType::Round, &game.round)?;
        nb.increment_frequency(NameType::Round, round_id);
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

        if let Some(ref eco) = game.eco {
            let eco_code = crate::index::eco_from_string(eco);
            if eco_code > 0 {
                entry.set_eco_code(eco_code);
            }
        }

        let game_num = self.index.add_entry(entry)?;

        self.modified = true;

        Ok(game_num)
    }

    fn encode_game(&self, game: &Game) -> Result<Vec<u8>> {
        let mut buf = ByteBuffer::new();

        self.encode_tags(&mut buf, game)?;

        buf.put_byte(0)?;

        match game.start_position {
            Some(ref start) => {
                buf.put_byte(1)?;
                buf.put_terminated_string(&fen(start))?;
            }
            None => buf.put_byte(0)?,
        }

        if let Some(start_idx) = game.first_move {
            Self::encode_moves(&mut buf, game, start_idx, game.initial_position()?)?;
        }

        buf.put_byte(crate::mov::ENCODE_END_GAME)?;

        self.encode_comments(&mut buf, game)?;

        Ok(buf.as_slice().to_vec())
    }

    fn encode_tags(&self, buf: &mut ByteBuffer, game: &Game) -> Result<()> {
        if let Some(event_date) = game.event_date {
            buf.put_byte(255)?;
            let raw = event_date.raw();
            buf.put_byte(((raw >> 16) & 0xFF) as u8)?;
            buf.put_byte(((raw >> 8) & 0xFF) as u8)?;
            buf.put_byte((raw & 0xFF) as u8)?;
        }

        for tag in &game.tags {
            let name_bytes = tag.name.as_bytes();
            if name_bytes.len() <= crate::game::MAX_TAG_LEN as usize {
                buf.put_byte(name_bytes.len() as u8)?;
                buf.put_fixed_string(&tag.name, name_bytes.len())?;
                buf.put_byte(tag.value.len() as u8)?;
                buf.put_fixed_string(&tag.value, tag.value.len())?;
            }
        }

        Ok(())
    }

    /// Encodes the line that starts at `start_idx`, played from `position`,
    /// with its variations.
    fn encode_moves(
        buf: &mut ByteBuffer,
        game: &Game,
        start_idx: usize,
        mut position: ScidPosition,
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
            }

            for nag in &node.nags {
                buf.put_byte(crate::mov::ENCODE_NAG)?;
                buf.put_byte(*nag)?;
            }

            if node.comment.is_some() {
                buf.put_byte(crate::mov::ENCODE_COMMENT)?;
            }

            for var_start in game.variations(idx) {
                buf.put_byte(crate::mov::ENCODE_START_MARKER)?;
                Self::encode_moves(buf, game, var_start, before.clone())?;
                buf.put_byte(crate::mov::ENCODE_END_MARKER)?;
            }

            current_idx = node.next;
        }

        Ok(())
    }

    fn encode_comments(&self, buf: &mut ByteBuffer, game: &Game) -> Result<()> {
        if let Some(start_idx) = game.first_move {
            Self::encode_comments_recursive(buf, game, start_idx)?;
        }
        Ok(())
    }

    fn encode_comments_recursive(
        buf: &mut ByteBuffer,
        game: &Game,
        start_idx: usize,
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
                buf.put_terminated_string(comment)?;
            }

            for var_start in game.variations(idx) {
                Self::encode_comments_recursive(buf, game, var_start)?;
            }

            current_idx = node.next;
        }

        Ok(())
    }

    /// Writes the games added since the database was opened, created, or
    /// last flushed: the game file's pending data, then the name file, then
    /// the index file. Does nothing if no game was added.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if a file can't be written.
    pub fn flush(&mut self) -> Result<()> {
        if !self.modified {
            return Ok(());
        }

        self.gfile.flush()?;

        self.nfile.close()?;

        let si4_path = self.path.with_extension("si4");
        let file = File::create(&si4_path)?;
        let mut writer = BufWriter::new(file);
        self.index.flush(&mut writer)?;
        writer.flush()?;

        self.modified = false;

        Ok(())
    }

    /// Writes any changes, as [`flush`](Self::flush) does, and closes the
    /// database.
    ///
    /// # Errors
    ///
    /// The same as [`flush`](Self::flush).
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

    /// Returns the index.
    pub fn index(&self) -> &Index {
        &self.index
    }

    /// Returns the name file.
    pub fn nfile(&self) -> &NFile {
        &self.nfile
    }

    /// Returns the game file.
    pub fn gfile(&self) -> &GFile {
        &self.gfile
    }

    /// Returns the database's path, without an extension.
    pub fn path(&self) -> &Path {
        &self.path
    }
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
