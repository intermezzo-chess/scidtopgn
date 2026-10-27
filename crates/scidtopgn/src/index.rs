//! The `.si4` index file: a header and an [`IndexEntry`] for each game.
//!
//! An entry gives the position and length of the game's record in the game
//! file, the IDs of its player, event, site, and round names in the name
//! file, and its date, result, ratings, ECO code, and flags.

use crate::common::GameResult;
use crate::error::{DatabaseFile, Error, Result};
use crate::{date_get_day, date_get_month, date_get_year, date_make, Date};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;

/// The first eight bytes of a `.si4` file.
pub const INDEX_MAGIC: &[u8; 8] = b"Scid.si\0";
/// The extension of an index file.
pub const INDEX_SUFFIX: &str = ".si4";
/// The size of the index file's header, in bytes.
pub const INDEX_HEADER_SIZE: usize = 182;
/// The size of an index entry, in bytes.
pub const INDEX_ENTRY_SIZE: usize = 47;
/// The most games a SCID database can hold.
pub const MAX_GAMES: u32 = 16_777_214;
/// The longest database description the header holds, in bytes.
pub const SCID_DESC_LENGTH: usize = 107;
/// The longest description of a custom flag the header holds, in bytes.
pub const CUSTOM_FLAG_DESC_LENGTH: usize = 8;
/// The number of custom flags the header describes.
pub const CUSTOM_FLAG_MAX: usize = 6;
/// The highest rating an index entry stores. Higher ratings are stored as
/// this.
pub const MAX_ELO: u16 = 4000;
/// The size of an index entry's home pawn data, in bytes.
pub const HPSIG_SIZE: usize = 9;
/// The highest half-move count an index entry stores. Higher counts are
/// stored modulo 1,024.
pub const MAX_HALF_MOVES: u16 = 1023;

/// The SCID format version this crate writes: 400, for SCID 4.
pub const SCID_VERSION: u16 = 400;

/// The bit of the flag [`IDX_MASK_START`] tests.
pub const IDX_FLAG_START: u16 = 0;
/// The bit of the flag [`IDX_MASK_PROMO`] tests.
pub const IDX_FLAG_PROMO: u16 = 1;
/// The bit of the flag [`IDX_MASK_UPROMO`] tests.
pub const IDX_FLAG_UPROMO: u16 = 2;
/// The bit of the flag [`IDX_MASK_DELETE`] tests.
pub const IDX_FLAG_DELETE: u16 = 3;
/// The bit of the flag [`IDX_MASK_WHITE_OP`] tests.
pub const IDX_FLAG_WHITE_OP: u16 = 4;
/// The bit of the flag [`IDX_MASK_BLACK_OP`] tests.
pub const IDX_FLAG_BLACK_OP: u16 = 5;
/// The bit of the flag [`IDX_MASK_MIDDLEGAME`] tests.
pub const IDX_FLAG_MIDDLEGAME: u16 = 6;
/// The bit of the flag [`IDX_MASK_ENDGAME`] tests.
pub const IDX_FLAG_ENDGAME: u16 = 7;
/// The bit of the flag [`IDX_MASK_NOVELTY`] tests.
pub const IDX_FLAG_NOVELTY: u16 = 8;
/// The bit of the flag [`IDX_MASK_PAWN`] tests.
pub const IDX_FLAG_PAWN: u16 = 9;
/// The bit of the flag [`IDX_MASK_TACTICS`] tests.
pub const IDX_FLAG_TACTICS: u16 = 10;
/// The bit of the flag [`IDX_MASK_KSIDE`] tests.
pub const IDX_FLAG_KSIDE: u16 = 11;
/// The bit of the flag [`IDX_MASK_QSIDE`] tests.
pub const IDX_FLAG_QSIDE: u16 = 12;
/// The bit of the flag [`IDX_MASK_BRILLIANCY`] tests.
pub const IDX_FLAG_BRILLIANCY: u16 = 13;
/// The bit of the flag [`IDX_MASK_BLUNDER`] tests.
pub const IDX_FLAG_BLUNDER: u16 = 14;
/// The bit of the flag [`IDX_MASK_USER`] tests.
pub const IDX_FLAG_USER: u16 = 15;

/// The game starts from a position other than the standard one.
pub const IDX_MASK_START: u16 = 1 << IDX_FLAG_START;
/// The game has a promotion.
pub const IDX_MASK_PROMO: u16 = 1 << IDX_FLAG_PROMO;
/// The game has an underpromotion.
pub const IDX_MASK_UPROMO: u16 = 1 << IDX_FLAG_UPROMO;
/// The game is logically deleted: SCID hides it, but its record stays in
/// the database until the database is compacted.
pub const IDX_MASK_DELETE: u16 = 1 << IDX_FLAG_DELETE;
/// User flag: a White opening.
pub const IDX_MASK_WHITE_OP: u16 = 1 << IDX_FLAG_WHITE_OP;
/// User flag: a Black opening.
pub const IDX_MASK_BLACK_OP: u16 = 1 << IDX_FLAG_BLACK_OP;
/// User flag: a middlegame.
pub const IDX_MASK_MIDDLEGAME: u16 = 1 << IDX_FLAG_MIDDLEGAME;
/// User flag: an endgame.
pub const IDX_MASK_ENDGAME: u16 = 1 << IDX_FLAG_ENDGAME;
/// User flag: a novelty.
pub const IDX_MASK_NOVELTY: u16 = 1 << IDX_FLAG_NOVELTY;
/// User flag: pawn structure.
pub const IDX_MASK_PAWN: u16 = 1 << IDX_FLAG_PAWN;
/// User flag: tactics.
pub const IDX_MASK_TACTICS: u16 = 1 << IDX_FLAG_TACTICS;
/// User flag: kingside play.
pub const IDX_MASK_KSIDE: u16 = 1 << IDX_FLAG_KSIDE;
/// User flag: queenside play.
pub const IDX_MASK_QSIDE: u16 = 1 << IDX_FLAG_QSIDE;
/// User flag: a brilliancy or good play.
pub const IDX_MASK_BRILLIANCY: u16 = 1 << IDX_FLAG_BRILLIANCY;
/// User flag: a blunder or bad play.
pub const IDX_MASK_BLUNDER: u16 = 1 << IDX_FLAG_BLUNDER;
/// User flag: user-defined.
pub const IDX_MASK_USER: u16 = 1 << IDX_FLAG_USER;

/// The header of a `.si4` index file.
#[derive(Debug, Clone)]
pub struct IndexHeader {
    /// [`INDEX_MAGIC`].
    pub magic: [u8; 8],
    /// The SCID format version, such as [`SCID_VERSION`].
    pub version: u16,
    /// The kind of database, such as tournament or theory, as SCID numbers
    /// them.
    pub base_type: u32,
    /// The number of games, counting logically deleted ones.
    pub num_games: u32,
    /// The game SCID opens with the database: 0 or 2 for the first game,
    /// 1 for none, 3 for the second, and so on.
    pub auto_load: u32,
    /// The database's description, padded with NULs.
    pub description: [u8; SCID_DESC_LENGTH + 1],
    /// The descriptions of the custom flags, padded with NULs.
    pub custom_flag_desc: [[u8; CUSTOM_FLAG_DESC_LENGTH + 1]; CUSTOM_FLAG_MAX],
}

/// The header of an empty database.
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
    /// The header of an empty database.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads a header.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidMagic`] if the data doesn't start with
    /// [`INDEX_MAGIC`], and [`Error::Io`] if it can't be read or ends early.
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        let mut header = Self::default();

        reader.read_exact(&mut header.magic)?;
        if &header.magic != INDEX_MAGIC {
            return Err(Error::InvalidMagic);
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

    /// Writes the header.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if it can't be written.
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

    /// Returns the description, or `""` if it isn't valid UTF-8.
    pub fn description_str(&self) -> &str {
        let end = self
            .description
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(SCID_DESC_LENGTH);
        std::str::from_utf8(&self.description[..end]).unwrap_or("")
    }

    /// Sets the description, cutting it to [`SCID_DESC_LENGTH`] bytes.
    pub fn set_description(&mut self, desc: &str) {
        let bytes = desc.as_bytes();
        let len = bytes.len().min(SCID_DESC_LENGTH);
        self.description[..len].copy_from_slice(&bytes[..len]);
        for i in len..=SCID_DESC_LENGTH {
            self.description[i] = 0;
        }
    }
}

/// A game's entry in the index: where its record is in the game file, the IDs
/// of its names, and its date, result, ratings, ECO code, and flags.
///
/// The fields are packed as SCID stores them, so they are read and set
/// through methods.
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
    num_half_moves: u16,
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
    /// An entry with every field 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets every field to 0.
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

    /// Returns the length of the game's record, in bytes.
    pub fn get_length(&self) -> u32 {
        (self.length_low as u32) | (((self.length_high & 0x80) as u32) << 9)
    }

    /// Sets the length of the game's record. It must be less than 131,072.
    pub fn set_length(&mut self, length: u32) {
        debug_assert!(length < 131072);
        self.length_low = (length & 0xFFFF) as u16;
        self.length_high = (self.length_high & 0x7F) | (((length >> 16) << 7) as u8);
    }

    /// Returns where the game's record starts in the game file.
    pub fn get_offset(&self) -> u32 {
        self.offset
    }

    /// Sets where the game's record starts in the game file.
    pub fn set_offset(&mut self, offset: u32) {
        self.offset = offset;
    }

    /// Returns the ID of the White player's name.
    pub fn get_white(&self) -> u32 {
        let id = (self.white_black_high >> 4) as u32;
        (id << 16) | (self.white_id_low as u32)
    }

    /// Sets the ID of the White player's name.
    pub fn set_white(&mut self, id: u32) {
        self.white_id_low = (id & 0xFFFF) as u16;
        self.white_black_high = (self.white_black_high & 0x0F) | (((id >> 16) << 4) as u8);
    }

    /// Returns the ID of the Black player's name.
    pub fn get_black(&self) -> u32 {
        let id = (self.white_black_high & 0x0F) as u32;
        (id << 16) | (self.black_id_low as u32)
    }

    /// Sets the ID of the Black player's name.
    pub fn set_black(&mut self, id: u32) {
        self.black_id_low = (id & 0xFFFF) as u16;
        self.white_black_high = (self.white_black_high & 0xF0) | ((id >> 16) as u8);
    }

    /// Returns the ID of the event's name.
    pub fn get_event(&self) -> u32 {
        let id = (self.event_site_rnd_high >> 5) as u32;
        (id << 16) | (self.event_id_low as u32)
    }

    /// Sets the ID of the event's name.
    pub fn set_event(&mut self, id: u32) {
        self.event_id_low = (id & 0xFFFF) as u16;
        self.event_site_rnd_high = (self.event_site_rnd_high & 0x1F) | (((id >> 16) << 5) as u8);
    }

    /// Returns the ID of the site's name.
    pub fn get_site(&self) -> u32 {
        let id = ((self.event_site_rnd_high >> 2) & 0x07) as u32;
        (id << 16) | (self.site_id_low as u32)
    }

    /// Sets the ID of the site's name.
    pub fn set_site(&mut self, id: u32) {
        self.site_id_low = (id & 0xFFFF) as u16;
        self.event_site_rnd_high = (self.event_site_rnd_high & 0xE3) | (((id >> 16) << 2) as u8);
    }

    /// Returns the ID of the round's name.
    pub fn get_round(&self) -> u32 {
        let id = (self.event_site_rnd_high & 0x03) as u32;
        (id << 16) | (self.round_id_low as u32)
    }

    /// Sets the ID of the round's name.
    pub fn set_round(&mut self, id: u32) {
        self.round_id_low = (id & 0xFFFF) as u16;
        self.event_site_rnd_high = (self.event_site_rnd_high & 0xFC) | ((id >> 16) as u8);
    }

    /// Returns the game's date, packed. See [`Date`].
    pub fn get_date(&self) -> u32 {
        self.dates & 0x000FFFFF
    }

    /// Sets the game's date, packed.
    pub fn set_date(&mut self, date: u32) {
        self.dates = (self.dates & 0xFFF00000) | (date & 0x000FFFFF);
    }

    /// Returns the game's date.
    pub fn get_date_obj(&self) -> Date {
        Date::from_raw(self.get_date())
    }

    /// Returns the year of the game's date, or 0 if it is unknown.
    pub fn get_year(&self) -> u16 {
        date_get_year(self.get_date())
    }

    /// Returns the month of the game's date, or 0 if it is unknown.
    pub fn get_month(&self) -> u8 {
        date_get_month(self.get_date())
    }

    /// Returns the day of the game's date, or 0 if it is unknown.
    pub fn get_day(&self) -> u8 {
        date_get_day(self.get_date())
    }

    /// Returns the event date, packed, or 0 if there is none.
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

    /// Sets the event date, packed. The entry stores its year relative to the
    /// game's, so set the game's date first. An event date more than three
    /// years from the game's date is stored as none.
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

    /// Returns the game's result.
    pub fn get_result(&self) -> GameResult {
        match (self.var_counts >> 12) & 3 {
            1 => GameResult::White,
            2 => GameResult::Black,
            3 => GameResult::Draw,
            _ => GameResult::None,
        }
    }

    /// Sets the game's result.
    pub fn set_result(&mut self, result: GameResult) {
        let res = result.to_byte() as u16;
        self.var_counts = (self.var_counts & 0x0FFF) | (res << 12);
    }

    /// Returns White's rating, or 0 if there is none.
    pub fn get_white_elo(&self) -> u16 {
        self.white_elo & 0x0FFF
    }

    /// Sets White's rating, up to [`MAX_ELO`].
    pub fn set_white_elo(&mut self, elo: u16) {
        let elo = elo.min(MAX_ELO);
        self.white_elo = (self.white_elo & 0xF000) | (elo & 0x0FFF);
    }

    /// Returns Black's rating, or 0 if there is none.
    pub fn get_black_elo(&self) -> u16 {
        self.black_elo & 0x0FFF
    }

    /// Sets Black's rating, up to [`MAX_ELO`].
    pub fn set_black_elo(&mut self, elo: u16) {
        let elo = elo.min(MAX_ELO);
        self.black_elo = (self.black_elo & 0xF000) | (elo & 0x0FFF);
    }

    /// Returns the kind of White's rating, such as Elo, as SCID numbers them.
    pub fn get_white_rating_type(&self) -> u8 {
        (self.white_elo >> 12) as u8
    }

    /// Sets the kind of White's rating.
    pub fn set_white_rating_type(&mut self, rating_type: u8) {
        self.white_elo = (self.white_elo & 0x0FFF) | ((rating_type as u16) << 12);
    }

    /// Returns the kind of Black's rating, such as Elo, as SCID numbers them.
    pub fn get_black_rating_type(&self) -> u8 {
        (self.black_elo >> 12) as u8
    }

    /// Sets the kind of Black's rating.
    pub fn set_black_rating_type(&mut self, rating_type: u8) {
        self.black_elo = (self.black_elo & 0x0FFF) | ((rating_type as u16) << 12);
    }

    /// Returns the ECO code in SCID's numbering, or 0 if there is none. See
    /// [`eco_to_string`].
    pub fn get_eco_code(&self) -> u16 {
        self.eco_code
    }

    /// Sets the ECO code in SCID's numbering. See [`eco_from_string`].
    pub fn set_eco_code(&mut self, eco: u16) {
        self.eco_code = eco;
    }

    /// Returns the ECO code, such as `B36f`, or `None` if there is none.
    pub fn get_eco_string(&self) -> Option<String> {
        eco_to_string(self.eco_code)
    }

    /// Returns the number of half-moves in the main line, modulo 1,024.
    pub fn get_num_half_moves(&self) -> u16 {
        self.num_half_moves
    }

    /// Sets the number of half-moves in the main line. SCID stores 10 bits,
    /// so a count above [`MAX_HALF_MOVES`] is stored modulo 1,024.
    pub fn set_num_half_moves(&mut self, moves: u16) {
        self.num_half_moves = moves & MAX_HALF_MOVES;
    }

    /// Returns whether any flag in `mask` is set, such as
    /// [`IDX_MASK_DELETE`].
    pub fn get_flag(&self, mask: u16) -> bool {
        (self.flags & mask) != 0
    }

    /// Sets or clears the flags in `mask`.
    pub fn set_flag(&mut self, mask: u16, value: bool) {
        if value {
            self.flags |= mask;
        } else {
            self.flags &= !mask;
        }
    }

    /// Returns whether the game starts from a position other than the standard
    /// one.
    pub fn get_start_flag(&self) -> bool {
        self.get_flag(IDX_MASK_START)
    }

    /// Sets whether the game starts from a position other than the standard
    /// one.
    pub fn set_start_flag(&mut self, value: bool) {
        self.set_flag(IDX_MASK_START, value);
    }

    /// Returns whether the game has a promotion.
    pub fn get_promotions_flag(&self) -> bool {
        self.get_flag(IDX_MASK_PROMO)
    }

    /// Returns whether the game is logically deleted.
    ///
    /// A deleted game's record and game number stay in the database, so
    /// [`Database::get_game`](crate::Database::get_game) still reads it. SCID,
    /// and the `scidtopgn` command, skip it when exporting.
    pub fn get_delete_flag(&self) -> bool {
        self.get_flag(IDX_MASK_DELETE)
    }

    /// Marks the game as logically deleted, or not. Its record stays in the
    /// database.
    pub fn set_delete_flag(&mut self, value: bool) {
        self.set_flag(IDX_MASK_DELETE, value);
    }

    /// Replaces this entry's user-settable flags with those of `other`, as
    /// SCID does when it replaces a game: the delete flag, the flags from
    /// [`IDX_MASK_WHITE_OP`] to [`IDX_MASK_USER`], and the six custom flags.
    pub(crate) fn copy_user_flags(&mut self, other: &IndexEntry) {
        const USER_FLAGS: u16 = !(IDX_MASK_START | IDX_MASK_PROMO | IDX_MASK_UPROMO);
        // The custom flags are the low six bits of `length_high`.
        const CUSTOM_FLAGS: u8 = 0x3F;
        self.flags = (self.flags & !USER_FLAGS) | (other.flags & USER_FLAGS);
        self.length_high = (self.length_high & !CUSTOM_FLAGS) | (other.length_high & CUSTOM_FLAGS);
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

    /// Returns about how many variations the game has. Counts above 10 are
    /// rounded to 15, 20, 30, 40, or 50.
    pub fn get_variation_count(&self) -> u16 {
        Self::decode_count(self.var_counts & 15)
    }

    /// Sets about how many variations the game has, rounded as
    /// [`get_variation_count`](Self::get_variation_count) returns it.
    pub fn set_variation_count(&mut self, count: u16) {
        self.var_counts = (self.var_counts & 0xFFF0) | Self::encode_count(count);
    }

    /// Returns about how many comments the game has, rounded like
    /// [`get_variation_count`](Self::get_variation_count).
    pub fn get_comment_count(&self) -> u16 {
        Self::decode_count((self.var_counts >> 4) & 15)
    }

    /// Sets about how many comments the game has.
    pub fn set_comment_count(&mut self, count: u16) {
        self.var_counts = (self.var_counts & 0xFF0F) | (Self::encode_count(count) << 4);
    }

    /// Returns about how many NAGs the game has, rounded like
    /// [`get_variation_count`](Self::get_variation_count).
    pub fn get_nag_count(&self) -> u16 {
        Self::decode_count((self.var_counts >> 8) & 15)
    }

    /// Sets about how many NAGs the game has.
    pub fn set_nag_count(&mut self, count: u16) {
        self.var_counts = (self.var_counts & 0xF0FF) | (Self::encode_count(count) << 8);
    }

    /// Returns SCID's signature of the material at the end of the game.
    pub fn get_final_mat_sig(&self) -> u32 {
        self.final_mat_sig & 0x00FFFFFF
    }

    /// Sets SCID's signature of the material at the end of the game.
    pub fn set_final_mat_sig(&mut self, sig: u32) {
        self.final_mat_sig = (self.final_mat_sig & 0xFF000000) | (sig & 0x00FFFFFF);
    }

    /// Returns the number of the stored opening line the game follows, as
    /// SCID numbers them, or 0.
    pub fn get_stored_line_code(&self) -> u8 {
        (self.final_mat_sig >> 24) as u8
    }

    /// Sets the number of the stored opening line the game follows.
    pub fn set_stored_line_code(&mut self, code: u8) {
        self.final_mat_sig = (self.final_mat_sig & 0x00FFFFFF) | ((code as u32) << 24);
    }

    /// Returns SCID's record of when pawns leave their starting squares,
    /// which it uses to search for positions.
    pub fn get_home_pawn_data(&self) -> &[u8; HPSIG_SIZE] {
        &self.home_pawn_data
    }

    /// Sets SCID's record of when pawns leave their starting squares.
    pub fn set_home_pawn_data(&mut self, data: &[u8; HPSIG_SIZE]) {
        self.home_pawn_data.copy_from_slice(data);
    }

    /// Reads an entry. Ratings above [`MAX_ELO`] become [`MAX_ELO`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if it can't be read or ends early.
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
            num_half_moves,
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

    /// Writes the entry.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if it can't be written.
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
        // The low 8 bits of the half-move count have their own byte, and the
        // top 2 bits are the top of the first home pawn byte.
        write_u8(writer, (self.num_half_moves & 0xFF) as u8)?;

        let pb0 = (self.home_pawn_data[0] & 63) | ((self.num_half_moves >> 8) << 6) as u8;
        write_u8(writer, pb0)?;
        writer.write_all(&self.home_pawn_data[1..])?;

        Ok(())
    }
}

/// A `.si4` index file, read into memory: its header and one entry per game.
///
/// A game's number is its position in the index, counting from 0.
/// [`Database`](crate::Database) uses the index to find each game's record
/// and names.
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
    /// An empty index.
    pub fn new() -> Self {
        Self {
            header: IndexHeader::new(),
            entries: Vec::new(),
            dirty: false,
        }
    }

    /// Reads the index file at `path`, including its extension.
    ///
    /// # Errors
    ///
    /// Returns [`Error::File`] for [`DatabaseFile::Index`] if the file can't be
    /// read, doesn't start with [`INDEX_MAGIC`], or is shorter than its header
    /// says.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::read_file(path.as_ref()).map_err(|e| e.in_file(DatabaseFile::Index))
    }

    fn read_file(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let file_size = file.metadata()?.len();
        let mut reader = BufReader::new(file);

        let header = IndexHeader::read(&mut reader)?;
        // Check before allocating for the entries the header claims.
        let entries_size = u64::from(header.num_games) * INDEX_ENTRY_SIZE as u64;
        if file_size < INDEX_HEADER_SIZE as u64 + entries_size {
            return Err(Error::Truncated);
        }

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

    /// Creates an empty index file at `path`, including its extension,
    /// replacing any file there.
    ///
    /// # Errors
    ///
    /// Returns [`Error::File`] for [`DatabaseFile::Index`] if the file can't be
    /// written.
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::create_file(path.as_ref()).map_err(|e| e.in_file(DatabaseFile::Index))
    }

    fn create_file(path: &Path) -> Result<Self> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        let mut index = Self::new();
        index.header.write(&mut writer)?;
        writer.flush()?;
        index.dirty = false;

        Ok(index)
    }

    /// Returns the number of entries, counting logically deleted games.
    pub fn num_games(&self) -> u32 {
        self.header.num_games
    }

    /// Returns the SCID format version from the header.
    pub fn version(&self) -> u16 {
        self.header.version
    }

    /// Returns the database's description.
    pub fn description(&self) -> &str {
        self.header.description_str()
    }

    /// Sets the database's description, cutting it to [`SCID_DESC_LENGTH`]
    /// bytes.
    pub fn set_description(&mut self, desc: &str) {
        self.header.set_description(desc);
        self.dirty = true;
    }

    /// Returns the entry of game `game_num`, counting from 0, or `None` if there
    /// is no such game. Logically deleted games have entries.
    pub fn get_entry(&self, game_num: u32) -> Option<&IndexEntry> {
        self.entries.get(game_num as usize)
    }

    /// Returns the entry of game `game_num` for changing. Write the index with
    /// [`flush`](Self::flush) or [`write_entry`](Self::write_entry) to keep the
    /// change.
    pub fn get_entry_mut(&mut self, game_num: u32) -> Option<&mut IndexEntry> {
        self.entries.get_mut(game_num as usize)
    }

    /// Adds an entry and returns its game number, counting from 0.
    ///
    /// # Errors
    ///
    /// Returns [`Error::IndexFull`] if the index already has [`MAX_GAMES`]
    /// entries.
    pub fn add_entry(&mut self, entry: IndexEntry) -> Result<u32> {
        if self.header.num_games >= MAX_GAMES {
            return Err(Error::IndexFull);
        }

        let game_num = self.header.num_games;
        self.entries.push(entry);
        self.header.num_games += 1;
        self.dirty = true;

        Ok(game_num)
    }

    /// Writes the entry of game `game_num`, counting from 0, at its place in
    /// the index file `writer`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::GameNotFound`] if there is no such game, and
    /// [`Error::Io`] if it can't be written.
    pub fn write_entry<W: Write + Seek>(&mut self, writer: &mut W, game_num: u32) -> Result<()> {
        if game_num >= self.header.num_games {
            return Err(Error::GameNotFound {
                number: u64::from(game_num) + 1,
            });
        }

        let pos = INDEX_HEADER_SIZE as u64 + (game_num as u64) * (INDEX_ENTRY_SIZE as u64);
        writer.seek(SeekFrom::Start(pos))?;

        self.entries[game_num as usize].write(writer)?;

        Ok(())
    }

    /// Writes the whole index file to `writer`, from its start.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if it can't be written.
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

    /// Returns the header.
    pub fn header(&self) -> &IndexHeader {
        &self.header
    }

    /// Returns the header for changing.
    pub fn header_mut(&mut self) -> &mut IndexHeader {
        self.dirty = true;
        &mut self.header
    }

    /// Returns every entry, in game order, counting logically deleted games.
    pub fn entries(&self) -> &[IndexEntry] {
        &self.entries
    }

    /// Returns whether the index has changed since it was read or last written.
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

/// Formats an ECO code in SCID's numbering, such as `B36f`, or returns
/// `None` for 0, which means no code.
///
/// SCID extends each ECO code, such as `B36`, with a letter from `a` to `z`
/// and an optional digit from 1 to 4.
///
/// # Examples
///
/// ```
/// use scidtopgn::index::{eco_from_string, eco_to_string};
///
/// let code = eco_from_string("B36f");
/// assert_eq!(eco_to_string(code).as_deref(), Some("B36f"));
/// assert_eq!(eco_to_string(0), None);
/// ```
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

/// Reads an ECO code, such as `B36` or `B36f`, into SCID's numbering.
/// Returns 0 if `s` doesn't start with a letter from A to E, or has
/// something other than a digit where the code's digits belong.
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
    fn test_copy_user_flags() {
        let mut old = IndexEntry::new();
        old.set_flag(IDX_MASK_DELETE | IDX_MASK_NOVELTY | IDX_MASK_USER, true);
        old.set_flag(IDX_MASK_START | IDX_MASK_PROMO, true);
        // Custom flags 1 and 6, and a long record.
        old.length_high = 0x21;
        old.set_length(70_000);

        let mut new = IndexEntry::new();
        new.set_flag(IDX_MASK_UPROMO | IDX_MASK_TACTICS, true);
        new.length_high = 0x02;
        new.set_length(1_000);
        new.copy_user_flags(&old);

        // The user flags come from the old entry, and the others stay.
        assert_eq!(
            new.flags,
            IDX_MASK_DELETE | IDX_MASK_NOVELTY | IDX_MASK_USER | IDX_MASK_UPROMO
        );
        assert_eq!(new.length_high, 0x21);
        assert_eq!(new.get_length(), 1_000);
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
