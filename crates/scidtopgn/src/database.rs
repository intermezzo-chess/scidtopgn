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

pub struct Database {
    path: PathBuf,
    index: Index,
    gfile: GFile,
    nfile: NFile,
    modified: bool,
}

impl Database {
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

    pub fn num_games(&self) -> u32 {
        self.index.num_games()
    }

    pub fn get_entry(&self, game_num: u32) -> Option<&IndexEntry> {
        self.index.get_entry(game_num)
    }

    pub fn get_game(&mut self, game_num: u32) -> Result<Game> {
        let entry = self
            .index
            .get_entry(game_num)
            .ok_or_else(|| Error::Database(format!("Invalid game number: {}", game_num)))?;

        let offset = entry.get_offset();
        let length = entry.get_length() as u16;

        let mut buf = ByteBuffer::new();
        self.gfile.read_game(&mut buf, offset, length as u32)?;
        buf.back_to_start();

        let mut game = Game::new();
        game.decode(&mut buf, GAME_DECODE_ALL)?;

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

    pub fn add_game(&mut self, game: &Game) -> Result<u32> {
        let encoded_data = self.encode_game(game)?;

        let buf = ByteBuffer::from_slice(&encoded_data);
        let offset = self.gfile.add_game(&buf)?;

        let mut entry = IndexEntry::new();
        entry.set_offset(offset);
        entry.set_length(encoded_data.len() as u32);

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

            for var_start in Self::variations(game, idx) {
                buf.put_byte(crate::mov::ENCODE_START_MARKER)?;
                Self::encode_moves(buf, game, var_start, before.clone())?;
                buf.put_byte(crate::mov::ENCODE_END_MARKER)?;
            }

            current_idx = node.next;
        }

        Ok(())
    }

    /// The start markers of the move at `idx`'s variations, in order. A
    /// start marker's `var_child` links to the next variation of the same
    /// move.
    fn variations(game: &Game, idx: usize) -> impl Iterator<Item = usize> + '_ {
        let node = &game.moves[idx];
        let first = if node.marker == Marker::StartMarker {
            None
        } else {
            node.var_child
        };
        let valid = move |idx: &usize| *idx < game.moves.len();
        std::iter::successors(first.filter(valid), move |&start| {
            game.moves[start].var_child.filter(valid)
        })
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

            for var_start in Self::variations(game, idx) {
                Self::encode_comments_recursive(buf, game, var_start)?;
            }

            current_idx = node.next;
        }

        Ok(())
    }

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

    pub fn close(mut self) -> Result<()> {
        self.flush()
    }

    pub fn games(&mut self) -> GameIterator<'_> {
        let total = self.num_games();
        GameIterator {
            db: self,
            current: 0,
            total,
        }
    }

    pub fn index(&self) -> &Index {
        &self.index
    }

    pub fn nfile(&self) -> &NFile {
        &self.nfile
    }

    pub fn gfile(&self) -> &GFile {
        &self.gfile
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

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
