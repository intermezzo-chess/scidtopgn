use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::bytebuf::ByteBuffer;
use crate::common::{Piece, Square};
use crate::error::{Error, Result};
use crate::game::{Game, GAME_DECODE_ALL};
use crate::gfile::GFile;
use crate::index::{Index, IndexEntry};
use crate::mov::{
    encode_bishop, encode_king, encode_knight, encode_pawn, encode_queen, encode_rook,
};
use crate::namebase::NameType;
use crate::nfile::NFile;
use crate::position::Board;
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

        let mut flags: u8 = 0;
        if game.non_standard_start {
            flags |= 1;
        }
        buf.put_byte(flags)?;

        if game.non_standard_start {
            if let Some(ref board) = game.start_board {
                let fen = board.to_fen();
                buf.put_terminated_string(&fen)?;
            }
        }

        if let Some(start_idx) = game.first_move {
            self.encode_moves(&mut buf, game, start_idx)?;
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

    fn encode_moves(&self, buf: &mut ByteBuffer, game: &Game, start_idx: usize) -> Result<()> {
        let mut current_idx = Some(start_idx);
        let mut board = if game.non_standard_start {
            game.start_board.clone().unwrap_or_else(Board::std_start)
        } else {
            Board::std_start()
        };

        while let Some(idx) = current_idx {
            if idx >= game.moves.len() {
                break;
            }

            let node = &game.moves[idx];

            match node.marker {
                crate::game::Marker::EndMarker | crate::game::Marker::EndGame => {
                    break;
                }
                crate::game::Marker::StartMarker => {}
                crate::game::Marker::NoMarker => {}
            }

            let sm = &node.move_data;
            if sm.from != sm.to {
                let piece = board
                    .get_piece(sm.from)
                    .map(|(p, _)| p)
                    .unwrap_or(Piece::Empty);

                match piece {
                    Piece::King => encode_king(buf, sm)?,
                    Piece::Queen => encode_queen(buf, sm)?,
                    Piece::Rook => encode_rook(buf, sm)?,
                    Piece::Bishop => encode_bishop(buf, sm)?,
                    Piece::Knight => encode_knight(buf, sm)?,
                    Piece::Pawn => encode_pawn(buf, sm)?,
                    Piece::Empty => {}
                }

                let color = board.to_move;

                if sm.captured_square.is_valid() {
                    if let Some((_cap_piece, cap_color)) = board.get_piece(sm.captured_square) {
                        if cap_color != color {
                            board.remove_from_board(sm.captured_square);
                            board.piece_list.remove_piece(sm.captured_square, cap_color);
                        }
                    }
                }

                if let Some((_, _)) = board.get_piece(sm.from) {
                    board.remove_from_board(sm.from);

                    if sm.promote != Piece::Empty {
                        board.add_to_board(sm.promote, color, sm.to);
                    } else {
                        board.add_to_board(piece, color, sm.to);
                    }

                    board.piece_list.move_piece(sm.from, sm.to, color);

                    if piece == Piece::King {
                        let diff = sm.to.to_index() as i32 - sm.from.to_index() as i32;
                        if diff == 2 {
                            let rook_from = Square::make(7, sm.from.rank());
                            let rook_to = Square::make(5, sm.from.rank());
                            board.remove_from_board(rook_from);
                            board.add_to_board(Piece::Rook, color, rook_to);
                            board.piece_list.move_piece(rook_from, rook_to, color);
                        } else if diff == -2 {
                            let rook_from = Square::make(0, sm.from.rank());
                            let rook_to = Square::make(3, sm.from.rank());
                            board.remove_from_board(rook_from);
                            board.add_to_board(Piece::Rook, color, rook_to);
                            board.piece_list.move_piece(rook_from, rook_to, color);
                        }
                    }
                }

                board.to_move = board.to_move.flip();
            }

            for nag in &node.nags {
                buf.put_byte(crate::mov::ENCODE_NAG)?;
                buf.put_byte(*nag)?;
            }

            if node.comment.is_some() {
                buf.put_byte(crate::mov::ENCODE_COMMENT)?;
            }

            if node.var_child.is_some() {
                buf.put_byte(crate::mov::ENCODE_START_MARKER)?;
                self.encode_moves(buf, game, node.var_child.unwrap())?;
                buf.put_byte(crate::mov::ENCODE_END_MARKER)?;
            }

            current_idx = node.next;
        }

        Ok(())
    }

    fn encode_comments(&self, buf: &mut ByteBuffer, game: &Game) -> Result<()> {
        if let Some(start_idx) = game.first_move {
            self.encode_comments_recursive(buf, game, start_idx)?;
        }
        Ok(())
    }

    fn encode_comments_recursive(
        &self,
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

            if let Some(var_child) = node.var_child {
                self.encode_comments_recursive(buf, game, var_child)?;
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
