use crate::common::GameResult;
use crate::error::{Error, Result};
use crate::{date_get_day, date_get_month, date_get_year, date_make, Date};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;

pub const INDEX_MAGIC: &[u8; 8] = b"Scid.si\0";
pub const INDEX_SUFFIX: &str = ".si4";
pub const INDEX_HEADER_SIZE: usize = 182;
pub const INDEX_ENTRY_SIZE: usize = 47;
pub const MAX_GAMES: u32 = 16_777_214;
pub const SCID_DESC_LENGTH: usize = 107;
pub const CUSTOM_FLAG_DESC_LENGTH: usize = 8;
pub const CUSTOM_FLAG_MAX: usize = 6;
pub const MAX_ELO: u16 = 4000;
pub const HPSIG_SIZE: usize = 9;

pub const SCID_VERSION: u16 = 400;

pub const IDX_FLAG_START: u16 = 0;
pub const IDX_FLAG_PROMO: u16 = 1;
pub const IDX_FLAG_UPROMO: u16 = 2;
pub const IDX_FLAG_DELETE: u16 = 3;
pub const IDX_FLAG_WHITE_OP: u16 = 4;
pub const IDX_FLAG_BLACK_OP: u16 = 5;
pub const IDX_FLAG_MIDDLEGAME: u16 = 6;
pub const IDX_FLAG_ENDGAME: u16 = 7;
pub const IDX_FLAG_NOVELTY: u16 = 8;
pub const IDX_FLAG_PAWN: u16 = 9;
pub const IDX_FLAG_TACTICS: u16 = 10;
pub const IDX_FLAG_KSIDE: u16 = 11;
pub const IDX_FLAG_QSIDE: u16 = 12;
pub const IDX_FLAG_BRILLIANCY: u16 = 13;
pub const IDX_FLAG_BLUNDER: u16 = 14;
pub const IDX_FLAG_USER: u16 = 15;

pub const IDX_MASK_START: u16 = 1 << IDX_FLAG_START;
pub const IDX_MASK_PROMO: u16 = 1 << IDX_FLAG_PROMO;
pub const IDX_MASK_UPROMO: u16 = 1 << IDX_FLAG_UPROMO;
pub const IDX_MASK_DELETE: u16 = 1 << IDX_FLAG_DELETE;
pub const IDX_MASK_WHITE_OP: u16 = 1 << IDX_FLAG_WHITE_OP;
pub const IDX_MASK_BLACK_OP: u16 = 1 << IDX_FLAG_BLACK_OP;
pub const IDX_MASK_MIDDLEGAME: u16 = 1 << IDX_FLAG_MIDDLEGAME;
pub const IDX_MASK_ENDGAME: u16 = 1 << IDX_FLAG_ENDGAME;
pub const IDX_MASK_NOVELTY: u16 = 1 << IDX_FLAG_NOVELTY;
pub const IDX_MASK_PAWN: u16 = 1 << IDX_FLAG_PAWN;
pub const IDX_MASK_TACTICS: u16 = 1 << IDX_FLAG_TACTICS;
pub const IDX_MASK_KSIDE: u16 = 1 << IDX_FLAG_KSIDE;
pub const IDX_MASK_QSIDE: u16 = 1 << IDX_FLAG_QSIDE;
pub const IDX_MASK_BRILLIANCY: u16 = 1 << IDX_FLAG_BRILLIANCY;
pub const IDX_MASK_BLUNDER: u16 = 1 << IDX_FLAG_BLUNDER;
pub const IDX_MASK_USER: u16 = 1 << IDX_FLAG_USER;

#[derive(Debug, Clone)]
pub struct IndexHeader {
    pub magic: [u8; 8],
    pub version: u16,
    pub base_type: u32,
    pub num_games: u32,
    pub auto_load: u32,
    pub description: [u8; SCID_DESC_LENGTH + 1],
    pub custom_flag_desc: [[u8; CUSTOM_FLAG_DESC_LENGTH + 1]; CUSTOM_FLAG_MAX],
}

impl Default for IndexHeader {
    fn default() -> Self {
        let mut magic = [0u8; 8];
        magic.copy_from_slice(INDEX_MAGIC);

        let description = [0u8; SCID_DESC_LENGTH + 1];
        let custom_flag_desc = [[0u8; CUSTOM_FLAG_DESC_LENGTH + 1]; CUSTOM_FLAG_MAX];

        Self {
            magic,
            version: SCID_VERSION,
            base_type: 0,
            num_games: 0,
            auto_load: 2,
            description,
            custom_flag_desc,
        }
    }
}

impl IndexHeader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let mut header = Self::default();

        reader.read_exact(&mut header.magic)?;
        if &header.magic != INDEX_MAGIC {
            return Err(Error::Database("Invalid magic number".to_string()));
        }

        header.version = read_u16_be(reader)?;
        header.base_type = read_u32_be(reader)?;
        header.num_games = read_u24_be(reader)?;
        header.auto_load = read_u24_be(reader)?;

        reader.read_exact(&mut header.description)?;

        for i in 0..CUSTOM_FLAG_MAX {
            reader.read_exact(&mut header.custom_flag_desc[i])?;
        }

        Ok(header)
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.magic)?;
        write_u16_be(writer, self.version)?;
        write_u32_be(writer, self.base_type)?;
        write_u24_be(writer, self.num_games)?;
        write_u24_be(writer, self.auto_load)?;
        writer.write_all(&self.description)?;

        for i in 0..CUSTOM_FLAG_MAX {
            writer.write_all(&self.custom_flag_desc[i])?;
        }

        Ok(())
    }

    pub fn description_str(&self) -> &str {
        let end = self
            .description
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(SCID_DESC_LENGTH);
        std::str::from_utf8(&self.description[..end]).unwrap_or("")
    }

    pub fn set_description(&mut self, desc: &str) {
        let bytes = desc.as_bytes();
        let len = bytes.len().min(SCID_DESC_LENGTH);
        self.description[..len].copy_from_slice(&bytes[..len]);
        for i in len..=SCID_DESC_LENGTH {
            self.description[i] = 0;
        }
    }
}

#[derive(Debug, Clone)]
pub struct IndexEntry {
    offset: u32,
    length_low: u16,
    length_high: u8,
    flags: u16,
    white_black_high: u8,
    white_id_low: u16,
    black_id_low: u16,
    event_site_rnd_high: u8,
    event_id_low: u16,
    site_id_low: u16,
    round_id_low: u16,
    var_counts: u16,
    eco_code: u16,
    dates: u32,
    white_elo: u16,
    black_elo: u16,
    final_mat_sig: u32,
    num_half_moves: u8,
    home_pawn_data: [u8; HPSIG_SIZE],
}

impl Default for IndexEntry {
    fn default() -> Self {
        Self {
            offset: 0,
            length_low: 0,
            length_high: 0,
            flags: 0,
            white_black_high: 0,
            white_id_low: 0,
            black_id_low: 0,
            event_site_rnd_high: 0,
            event_id_low: 0,
            site_id_low: 0,
            round_id_low: 0,
            var_counts: 0,
            eco_code: 0,
            dates: 0,
            white_elo: 0,
            black_elo: 0,
            final_mat_sig: 0,
            num_half_moves: 0,
            home_pawn_data: [0; HPSIG_SIZE],
        }
    }
}

impl IndexEntry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.offset = 0;
        self.length_low = 0;
        self.length_high = 0;
        self.flags = 0;
        self.white_black_high = 0;
        self.white_id_low = 0;
        self.black_id_low = 0;
        self.event_site_rnd_high = 0;
        self.event_id_low = 0;
        self.site_id_low = 0;
        self.round_id_low = 0;
        self.var_counts = 0;
        self.eco_code = 0;
        self.dates = 0;
        self.white_elo = 0;
        self.black_elo = 0;
        self.final_mat_sig = 0;
        self.num_half_moves = 0;
        self.home_pawn_data = [0; HPSIG_SIZE];
    }

    pub fn get_length(&self) -> u32 {
        (self.length_low as u32) | (((self.length_high & 0x80) as u32) << 9)
    }

    pub fn set_length(&mut self, length: u32) {
        debug_assert!(length < 131072);
        self.length_low = (length & 0xFFFF) as u16;
        self.length_high = (self.length_high & 0x7F) | (((length >> 16) << 7) as u8);
    }

    pub fn get_offset(&self) -> u32 {
        self.offset
    }

    pub fn set_offset(&mut self, offset: u32) {
        self.offset = offset;
    }

    pub fn get_white(&self) -> u32 {
        let id = (self.white_black_high >> 4) as u32;
        (id << 16) | (self.white_id_low as u32)
    }

    pub fn set_white(&mut self, id: u32) {
        self.white_id_low = (id & 0xFFFF) as u16;
        self.white_black_high = (self.white_black_high & 0x0F) | (((id >> 16) << 4) as u8);
    }

    pub fn get_black(&self) -> u32 {
        let id = (self.white_black_high & 0x0F) as u32;
        (id << 16) | (self.black_id_low as u32)
    }

    pub fn set_black(&mut self, id: u32) {
        self.black_id_low = (id & 0xFFFF) as u16;
        self.white_black_high = (self.white_black_high & 0xF0) | ((id >> 16) as u8);
    }

    pub fn get_event(&self) -> u32 {
        let id = (self.event_site_rnd_high >> 5) as u32;
        (id << 16) | (self.event_id_low as u32)
    }

    pub fn set_event(&mut self, id: u32) {
        self.event_id_low = (id & 0xFFFF) as u16;
        self.event_site_rnd_high = (self.event_site_rnd_high & 0x1F) | (((id >> 16) << 5) as u8);
    }

    pub fn get_site(&self) -> u32 {
        let id = ((self.event_site_rnd_high >> 2) & 0x07) as u32;
        (id << 16) | (self.site_id_low as u32)
    }

    pub fn set_site(&mut self, id: u32) {
        self.site_id_low = (id & 0xFFFF) as u16;
        self.event_site_rnd_high = (self.event_site_rnd_high & 0xE3) | (((id >> 16) << 2) as u8);
    }

    pub fn get_round(&self) -> u32 {
        let id = (self.event_site_rnd_high & 0x03) as u32;
        (id << 16) | (self.round_id_low as u32)
    }

    pub fn set_round(&mut self, id: u32) {
        self.round_id_low = (id & 0xFFFF) as u16;
        self.event_site_rnd_high = (self.event_site_rnd_high & 0xFC) | ((id >> 16) as u8);
    }

    pub fn get_date(&self) -> u32 {
        self.dates & 0x000FFFFF
    }

    pub fn set_date(&mut self, date: u32) {
        self.dates = (self.dates & 0xFFF00000) | (date & 0x000FFFFF);
    }

    pub fn get_date_obj(&self) -> Date {
        Date::from_raw(self.get_date())
    }

    pub fn get_year(&self) -> u16 {
        date_get_year(self.get_date())
    }

    pub fn get_month(&self) -> u8 {
        date_get_month(self.get_date())
    }

    pub fn get_day(&self) -> u8 {
        date_get_day(self.get_date())
    }

    pub fn get_event_date(&self) -> u32 {
        let game_year = date_get_year(self.get_date()) as i32;
        let edate = (self.dates >> 20) & 0xFFF;
        let month = date_get_month(edate);
        let day = date_get_day(edate);
        let year_offset = (date_get_year(edate) as i32) & 7;

        if year_offset == 0 {
            return 0;
        }

        let year = game_year + year_offset - 4;
        if year < 0 {
            return 0;
        }
        date_make(year as u16, month, day)
    }

    pub fn set_event_date(&mut self, edate: u32) {
        let mut coded_date = (date_get_month(edate) as u32) << 5;
        coded_date |= date_get_day(edate) as u32;

        let eyear = date_get_year(edate) as i32;
        let dyear = date_get_year(self.get_date()) as i32;
        let mut eyear = eyear;

        if eyear < (dyear - 3) || eyear > (dyear + 3) {
            eyear = 0;
        }

        if eyear == 0 {
            coded_date = 0;
        } else {
            coded_date |= (((eyear + 4 - dyear) as u32) & 7) << 9;
        }

        self.dates = (self.dates & 0x000FFFFF) | (coded_date << 20);
    }

    pub fn get_result(&self) -> GameResult {
        match (self.var_counts >> 12) & 3 {
            1 => GameResult::White,
            2 => GameResult::Black,
            3 => GameResult::Draw,
            _ => GameResult::None,
        }
    }

    pub fn set_result(&mut self, result: GameResult) {
        let res = result.to_byte() as u16;
        self.var_counts = (self.var_counts & 0x0FFF) | (res << 12);
    }

    pub fn get_white_elo(&self) -> u16 {
        self.white_elo & 0x0FFF
    }

    pub fn set_white_elo(&mut self, elo: u16) {
        let elo = elo.min(MAX_ELO);
        self.white_elo = (self.white_elo & 0xF000) | (elo & 0x0FFF);
    }

    pub fn get_black_elo(&self) -> u16 {
        self.black_elo & 0x0FFF
    }

    pub fn set_black_elo(&mut self, elo: u16) {
        let elo = elo.min(MAX_ELO);
        self.black_elo = (self.black_elo & 0xF000) | (elo & 0x0FFF);
    }

    pub fn get_white_rating_type(&self) -> u8 {
        (self.white_elo >> 12) as u8
    }

    pub fn set_white_rating_type(&mut self, rating_type: u8) {
        self.white_elo = (self.white_elo & 0x0FFF) | ((rating_type as u16) << 12);
    }

    pub fn get_black_rating_type(&self) -> u8 {
        (self.black_elo >> 12) as u8
    }

    pub fn set_black_rating_type(&mut self, rating_type: u8) {
        self.black_elo = (self.black_elo & 0x0FFF) | ((rating_type as u16) << 12);
    }

    pub fn get_eco_code(&self) -> u16 {
        self.eco_code
    }

    pub fn set_eco_code(&mut self, eco: u16) {
        self.eco_code = eco;
    }

    pub fn get_eco_string(&self) -> Option<String> {
        eco_to_string(self.eco_code)
    }

    pub fn get_num_half_moves(&self) -> u16 {
        self.num_half_moves as u16
    }

    pub fn set_num_half_moves(&mut self, moves: u16) {
        self.num_half_moves = (moves & 0xFF) as u8;
    }

    pub fn get_flag(&self, mask: u16) -> bool {
        (self.flags & mask) != 0
    }

    pub fn set_flag(&mut self, mask: u16, value: bool) {
        if value {
            self.flags |= mask;
        } else {
            self.flags &= !mask;
        }
    }

    pub fn get_start_flag(&self) -> bool {
        self.get_flag(IDX_MASK_START)
    }

    pub fn set_start_flag(&mut self, value: bool) {
        self.set_flag(IDX_MASK_START, value);
    }

    pub fn get_promotions_flag(&self) -> bool {
        self.get_flag(IDX_MASK_PROMO)
    }

    pub fn get_delete_flag(&self) -> bool {
        self.get_flag(IDX_MASK_DELETE)
    }

    pub fn set_delete_flag(&mut self, value: bool) {
        self.set_flag(IDX_MASK_DELETE, value);
    }

    fn decode_count(x: u16) -> u16 {
        const COUNT_CODES: [u16; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 15, 20, 30, 40, 50];
        COUNT_CODES[(x & 15) as usize]
    }

    fn encode_count(x: u16) -> u16 {
        if x <= 10 {
            return x;
        }
        if x <= 12 {
            return 10;
        }
        if x <= 17 {
            return 11;
        }
        if x <= 24 {
            return 12;
        }
        if x <= 34 {
            return 13;
        }
        if x <= 44 {
            return 14;
        }
        15
    }

    pub fn get_variation_count(&self) -> u16 {
        Self::decode_count(self.var_counts & 15)
    }

    pub fn set_variation_count(&mut self, count: u16) {
        self.var_counts = (self.var_counts & 0xFFF0) | Self::encode_count(count);
    }

    pub fn get_comment_count(&self) -> u16 {
        Self::decode_count((self.var_counts >> 4) & 15)
    }

    pub fn set_comment_count(&mut self, count: u16) {
        self.var_counts = (self.var_counts & 0xFF0F) | (Self::encode_count(count) << 4);
    }

    pub fn get_nag_count(&self) -> u16 {
        Self::decode_count((self.var_counts >> 8) & 15)
    }

    pub fn set_nag_count(&mut self, count: u16) {
        self.var_counts = (self.var_counts & 0xF0FF) | (Self::encode_count(count) << 8);
    }

    pub fn get_final_mat_sig(&self) -> u32 {
        self.final_mat_sig & 0x00FFFFFF
    }

    pub fn set_final_mat_sig(&mut self, sig: u32) {
        self.final_mat_sig = (self.final_mat_sig & 0xFF000000) | (sig & 0x00FFFFFF);
    }

    pub fn get_stored_line_code(&self) -> u8 {
        (self.final_mat_sig >> 24) as u8
    }

    pub fn set_stored_line_code(&mut self, code: u8) {
        self.final_mat_sig = (self.final_mat_sig & 0x00FFFFFF) | ((code as u32) << 24);
    }

    pub fn get_home_pawn_data(&self) -> &[u8; HPSIG_SIZE] {
        &self.home_pawn_data
    }

    pub fn set_home_pawn_data(&mut self, data: &[u8; HPSIG_SIZE]) {
        self.home_pawn_data.copy_from_slice(data);
    }

    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let offset = read_u32_be(reader)?;
        let length_low = read_u16_be(reader)?;
        let length_high = read_u8(reader)?;
        let flags = read_u16_be(reader)?;

        let white_black_high = read_u8(reader)?;
        let white_id_low = read_u16_be(reader)?;
        let black_id_low = read_u16_be(reader)?;

        let event_site_rnd_high = read_u8(reader)?;
        let event_id_low = read_u16_be(reader)?;
        let site_id_low = read_u16_be(reader)?;
        let round_id_low = read_u16_be(reader)?;

        let var_counts = read_u16_be(reader)?;
        let eco_code = read_u16_be(reader)?;

        let dates = read_u32_be(reader)?;

        let white_elo = read_u16_be(reader)?;
        let black_elo = read_u16_be(reader)?;

        let final_mat_sig = read_u32_be(reader)?;
        let num_half_moves = read_u8(reader)?;

        let mut home_pawn_data = [0u8; HPSIG_SIZE];
        reader.read_exact(&mut home_pawn_data)?;

        let pb0 = home_pawn_data[0];
        home_pawn_data[0] = pb0 & 63;
        let num_half_moves_high = ((pb0 as u16) >> 6) << 8;
        let num_half_moves = (num_half_moves as u16) | num_half_moves_high;

        let mut entry = Self {
            offset,
            length_low,
            length_high,
            flags,
            white_black_high,
            white_id_low,
            black_id_low,
            event_site_rnd_high,
            event_id_low,
            site_id_low,
            round_id_low,
            var_counts,
            eco_code,
            dates,
            white_elo,
            black_elo,
            final_mat_sig,
            num_half_moves: num_half_moves as u8,
            home_pawn_data,
        };

        if entry.get_white_elo() > MAX_ELO {
            entry.set_white_elo(MAX_ELO);
        }
        if entry.get_black_elo() > MAX_ELO {
            entry.set_black_elo(MAX_ELO);
        }

        Ok(entry)
    }

    pub fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        write_u32_be(writer, self.offset)?;
        write_u16_be(writer, self.length_low)?;
        write_u8(writer, self.length_high)?;
        write_u16_be(writer, self.flags)?;

        write_u8(writer, self.white_black_high)?;
        write_u16_be(writer, self.white_id_low)?;
        write_u16_be(writer, self.black_id_low)?;

        write_u8(writer, self.event_site_rnd_high)?;
        write_u16_be(writer, self.event_id_low)?;
        write_u16_be(writer, self.site_id_low)?;
        write_u16_be(writer, self.round_id_low)?;

        write_u16_be(writer, self.var_counts)?;
        write_u16_be(writer, self.eco_code)?;
        write_u32_be(writer, self.dates)?;

        write_u16_be(writer, self.white_elo)?;
        write_u16_be(writer, self.black_elo)?;

        write_u32_be(writer, self.final_mat_sig)?;
        write_u8(writer, self.num_half_moves)?;

        let pb0 = (self.home_pawn_data[0] & 63) | (((self.num_half_moves as u16) >> 8) << 6) as u8;
        write_u8(writer, pb0)?;
        writer.write_all(&self.home_pawn_data[1..])?;

        Ok(())
    }
}

pub struct Index {
    header: IndexHeader,
    entries: Vec<IndexEntry>,
    dirty: bool,
}

impl Default for Index {
    fn default() -> Self {
        Self::new()
    }
}

impl Index {
    pub fn new() -> Self {
        Self {
            header: IndexHeader::new(),
            entries: Vec::new(),
            dirty: false,
        }
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        let header = IndexHeader::read(&mut reader)?;

        let mut entries = Vec::with_capacity(header.num_games as usize);
        for _ in 0..header.num_games {
            let entry = IndexEntry::read(&mut reader)?;
            entries.push(entry);
        }

        Ok(Self {
            header,
            entries,
            dirty: false,
        })
    }

    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        let mut index = Self::new();
        index.header.write(&mut writer)?;
        writer.flush()?;
        index.dirty = false;

        Ok(index)
    }

    pub fn num_games(&self) -> u32 {
        self.header.num_games
    }

    pub fn version(&self) -> u16 {
        self.header.version
    }

    pub fn description(&self) -> &str {
        self.header.description_str()
    }

    pub fn set_description(&mut self, desc: &str) {
        self.header.set_description(desc);
        self.dirty = true;
    }

    pub fn get_entry(&self, game_num: u32) -> Option<&IndexEntry> {
        self.entries.get(game_num as usize)
    }

    pub fn get_entry_mut(&mut self, game_num: u32) -> Option<&mut IndexEntry> {
        self.entries.get_mut(game_num as usize)
    }

    pub fn add_entry(&mut self, entry: IndexEntry) -> Result<u32> {
        if self.header.num_games >= MAX_GAMES {
            return Err(Error::Database("Index full".to_string()));
        }

        let game_num = self.header.num_games;
        self.entries.push(entry);
        self.header.num_games += 1;
        self.dirty = true;

        Ok(game_num)
    }

    pub fn write_entry<W: Write + Seek>(&mut self, writer: &mut W, game_num: u32) -> Result<()> {
        if game_num >= self.header.num_games {
            return Err(Error::Database("Invalid game number".to_string()));
        }

        let pos = INDEX_HEADER_SIZE as u64 + (game_num as u64) * (INDEX_ENTRY_SIZE as u64);
        writer.seek(SeekFrom::Start(pos))?;

        self.entries[game_num as usize].write(writer)?;

        Ok(())
    }

    pub fn flush<W: Write + Seek>(&mut self, writer: &mut W) -> Result<()> {
        writer.seek(SeekFrom::Start(0))?;
        self.header.write(writer)?;

        for entry in &self.entries {
            entry.write(writer)?;
        }

        writer.flush()?;
        self.dirty = false;

        Ok(())
    }

    pub fn header(&self) -> &IndexHeader {
        &self.header
    }

    pub fn header_mut(&mut self) -> &mut IndexHeader {
        self.dirty = true;
        &mut self.header
    }

    pub fn entries(&self) -> &[IndexEntry] {
        &self.entries
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
}

fn read_u8<R: Read>(reader: &mut R) -> Result<u8> {
    let mut buf = [0u8; 1];
    reader.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn write_u8<W: Write>(writer: &mut W, value: u8) -> Result<()> {
    writer.write_all(&[value])?;
    Ok(())
}

fn read_u16_be<R: Read>(reader: &mut R) -> Result<u16> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(u16::from_be_bytes(buf))
}

fn write_u16_be<W: Write>(writer: &mut W, value: u16) -> Result<()> {
    writer.write_all(&value.to_be_bytes())?;
    Ok(())
}

fn read_u24_be<R: Read>(reader: &mut R) -> Result<u32> {
    let mut buf = [0u8; 3];
    reader.read_exact(&mut buf)?;
    Ok((buf[0] as u32) << 16 | (buf[1] as u32) << 8 | (buf[2] as u32))
}

fn write_u24_be<W: Write>(writer: &mut W, value: u32) -> Result<()> {
    let buf = [
        ((value >> 16) & 0xFF) as u8,
        ((value >> 8) & 0xFF) as u8,
        (value & 0xFF) as u8,
    ];
    writer.write_all(&buf)?;
    Ok(())
}

fn read_u32_be<R: Read>(reader: &mut R) -> Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

fn write_u32_be<W: Write>(writer: &mut W, value: u32) -> Result<()> {
    writer.write_all(&value.to_be_bytes())?;
    Ok(())
}

pub fn eco_to_string(eco_code: u16) -> Option<String> {
    if eco_code == 0 {
        return None;
    }

    let eco_code = eco_code - 1;
    let basic_code = eco_code / 131;

    let letter = (b'A' + (basic_code / 100) as u8) as char;
    let digit1 = ((basic_code / 10) % 10) as u8 + b'0';
    let digit2 = (basic_code % 10) as u8 + b'0';

    let mut result = String::new();
    result.push(letter);
    result.push(digit1 as char);
    result.push(digit2 as char);

    let mut sub_code = eco_code % 131;
    if sub_code > 0 {
        sub_code -= 1;
        let sub_letter = (b'a' + (sub_code / 5) as u8) as char;
        result.push(sub_letter);
        let sub_digit = sub_code % 5;
        if sub_digit > 0 {
            result.push((b'0' + sub_digit as u8) as char);
        }
    }

    Some(result)
}

pub fn eco_from_string(s: &str) -> u16 {
    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() {
        return 0;
    }

    let mut eco: u16;

    let first = chars[0];
    if ('A'..='E').contains(&first) {
        eco = (first as u16 - 'A' as u16) * 13100;
    } else if ('a'..='e').contains(&first) {
        eco = (first as u16 - 'a' as u16) * 13100;
    } else {
        return 0;
    }

    if chars.len() < 2 {
        return eco + 1;
    }

    let second = chars[1];
    if !second.is_ascii_digit() {
        return 0;
    }
    eco += (second as u16 - '0' as u16) * 1310;

    if chars.len() < 3 {
        return eco + 1;
    }

    let third = chars[2];
    if !third.is_ascii_digit() {
        return 0;
    }
    eco += (third as u16 - '0' as u16) * 131;

    if chars.len() >= 4 {
        let fourth = chars[3];
        if fourth.is_ascii_lowercase() {
            eco += 1;
            eco += (fourth as u16 - 'a' as u16) * 5;

            if chars.len() >= 5 {
                let fifth = chars[4];
                if ('1'..='4').contains(&fifth) {
                    eco += fifth as u16 - '0' as u16;
                }
            }
        }
    }

    eco + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_header_size() {
        assert_eq!(INDEX_HEADER_SIZE, 182);
    }

    #[test]
    fn test_entry_size() {
        let entry = IndexEntry::new();
        let mut cursor = Cursor::new(Vec::new());
        entry.write(&mut cursor).unwrap();
        assert_eq!(cursor.position(), INDEX_ENTRY_SIZE as u64);
    }

    #[test]
    fn test_entry_length_encoding() {
        let mut entry = IndexEntry::new();

        entry.set_length(1000);
        assert_eq!(entry.get_length(), 1000);

        entry.set_length(70000);
        assert_eq!(entry.get_length(), 70000);

        entry.set_length(131071);
        assert_eq!(entry.get_length(), 131071);
    }

    #[test]
    fn test_name_id_encoding() {
        let mut entry = IndexEntry::new();

        entry.set_white(0x12345);
        assert_eq!(entry.get_white(), 0x12345);

        entry.set_black(0xABCDE);
        assert_eq!(entry.get_black(), 0xABCDE);

        entry.set_event(0x1A2B3);
        assert_eq!(entry.get_event(), 0x1A2B3);

        entry.set_site(0x2B3C4);
        assert_eq!(entry.get_site(), 0x2B3C4);

        entry.set_round(0x3C4D);
        assert_eq!(entry.get_round(), 0x3C4D);
    }

    #[test]
    fn test_date_encoding() {
        let mut entry = IndexEntry::new();

        entry.set_date(date_make(2022, 12, 19));
        assert_eq!(entry.get_year(), 2022);
        assert_eq!(entry.get_month(), 12);
        assert_eq!(entry.get_day(), 19);
    }

    #[test]
    fn test_event_date_encoding() {
        let mut entry = IndexEntry::new();

        entry.set_date(date_make(2022, 6, 15));
        entry.set_event_date(date_make(2022, 8, 10));

        let edate = entry.get_event_date();
        assert_eq!(date_get_year(edate), 2022);
        assert_eq!(date_get_month(edate), 8);
        assert_eq!(date_get_day(edate), 10);
    }

    #[test]
    fn test_result_encoding() {
        let mut entry = IndexEntry::new();

        entry.set_result(GameResult::White);
        assert_eq!(entry.get_result(), GameResult::White);

        entry.set_result(GameResult::Black);
        assert_eq!(entry.get_result(), GameResult::Black);

        entry.set_result(GameResult::Draw);
        assert_eq!(entry.get_result(), GameResult::Draw);

        entry.set_result(GameResult::None);
        assert_eq!(entry.get_result(), GameResult::None);
    }

    #[test]
    fn test_elo_encoding() {
        let mut entry = IndexEntry::new();

        entry.set_white_elo(2850);
        entry.set_white_rating_type(1);
        assert_eq!(entry.get_white_elo(), 2850);
        assert_eq!(entry.get_white_rating_type(), 1);

        entry.set_black_elo(2700);
        entry.set_black_rating_type(2);
        assert_eq!(entry.get_black_elo(), 2700);
        assert_eq!(entry.get_black_rating_type(), 2);
    }

    #[test]
    fn test_var_counts_encoding() {
        let mut entry = IndexEntry::new();

        entry.set_variation_count(5);
        entry.set_comment_count(8);
        entry.set_nag_count(20);
        entry.set_result(GameResult::Draw);

        assert_eq!(entry.get_variation_count(), 5);
        assert_eq!(entry.get_comment_count(), 8);
        assert_eq!(entry.get_nag_count(), 20);
        assert_eq!(entry.get_result(), GameResult::Draw);
    }

    #[test]
    fn test_var_counts_approx_encoding() {
        let mut entry = IndexEntry::new();

        entry.set_comment_count(12);
        assert_eq!(entry.get_comment_count(), 10);

        entry.set_comment_count(18);
        assert_eq!(entry.get_comment_count(), 20);

        entry.set_comment_count(50);
        assert_eq!(entry.get_comment_count(), 50);

        entry.set_comment_count(100);
        assert_eq!(entry.get_comment_count(), 50);
    }

    #[test]
    fn test_entry_roundtrip() {
        let mut entry = IndexEntry::new();
        entry.set_offset(12345);
        entry.set_length(67890);
        entry.set_white(0x12345);
        entry.set_black(0xABCDE);
        entry.set_event(0x1A2B3);
        entry.set_site(0x2B3C4);
        entry.set_round(0x3C4D);
        entry.set_date(date_make(2022, 12, 19));
        entry.set_event_date(date_make(2022, 8, 10));
        entry.set_result(GameResult::White);
        entry.set_white_elo(2850);
        entry.set_black_elo(2700);
        entry.set_eco_code(0x1234);
        entry.set_flag(IDX_MASK_START, true);
        entry.set_flag(IDX_MASK_PROMO, true);

        let mut buffer = Vec::new();
        {
            let mut cursor = Cursor::new(&mut buffer);
            entry.write(&mut cursor).unwrap();
        }

        let mut cursor = Cursor::new(&buffer);
        let entry2 = IndexEntry::read(&mut cursor).unwrap();

        assert_eq!(entry2.get_offset(), 12345);
        assert_eq!(entry2.get_length(), 67890);
        assert_eq!(entry2.get_white(), 0x12345);
        assert_eq!(entry2.get_black(), 0xABCDE);
        assert_eq!(entry2.get_event(), 0x1A2B3);
        assert_eq!(entry2.get_site(), 0x2B3C4);
        assert_eq!(entry2.get_round(), 0x3C4D);
        assert_eq!(entry2.get_year(), 2022);
        assert_eq!(entry2.get_month(), 12);
        assert_eq!(entry2.get_day(), 19);
        assert_eq!(entry2.get_result(), GameResult::White);
        assert_eq!(entry2.get_white_elo(), 2850);
        assert_eq!(entry2.get_black_elo(), 2700);
        assert_eq!(entry2.get_eco_code(), 0x1234);
        assert!(entry2.get_start_flag());
        assert!(entry2.get_promotions_flag());
    }
}
