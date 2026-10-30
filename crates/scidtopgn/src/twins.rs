//! Twin games: [`TwinOptions`] chooses which games
//! [`Database::delete_twins`](crate::Database::delete_twins) treats as twins,
//! and which of them it deletes, as SCID vs. PC's `sc_base duplicates` does.

use std::cmp::Reverse;

use shakmaty::{ByColor, Color, Piece, Position, Role};

use crate::bytebuf::ByteBuffer;
use crate::error::{Error, Result};
use crate::game::Game;
use crate::gfile::GFile;
use crate::index::{Index, IndexEntry, HPSIG_SIZE};
use crate::mov::{
    ENCODE_COMMENT, ENCODE_END_GAME, ENCODE_END_MARKER, ENCODE_NAG, ENCODE_START_MARKER,
    KING_SQDIFF, KNIGHT_SQDIFF, PAWN_PROMO, PAWN_SQDIFF,
};
use crate::namebase::{NameBase, NameType};
use crate::position::ScidPosition;

/// Games shorter than this many half-moves are skipped with
/// [`TwinOptions::skip_short`].
const SHORT_GAME: u16 = 10;
/// Games with at most this many half-moves can be placeholders.
const PLACEHOLDER: u16 = 2;
/// The most half-moves [`TwinOptions::moves`] compares.
const MAX_SAME_MOVES: usize = 120;

/// Which game of a pair of twins
/// [`Database::delete_twins`](crate::Database::delete_twins) deletes, as
/// SCID's `-delete` option chooses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DeleteTwin {
    /// The game with fewer half-moves, or the lower-numbered game if they
    /// have as many.
    #[default]
    Shorter,
    /// The lower-numbered game.
    Older,
    /// The higher-numbered game.
    Newer,
}

/// What makes two games twins, and which of them
/// [`Database::delete_twins`](crate::Database::delete_twins) deletes.
///
/// Each field is the `sc_base duplicates` option of the same name, and the
/// defaults are those of SCID vs. PC's Delete Twin Games window. The
/// command's own defaults differ in three fields:
///
/// | Field | Default | Command's default |
/// | --- | --- | --- |
/// | [`players`](Self::players) | `true` | `false` |
/// | [`event`](Self::event) | `false` | `true` |
/// | [`skip_short`](Self::skip_short) | `true` | `false` |
///
/// The window also clears every game's delete flag before searching, unless
/// its Undelete option is turned off. There is no such option here, because
/// it would undelete games that were deleted on purpose. To do the same,
/// call [`Database::undelete_game`](crate::Database::undelete_game) on
/// each game first.
///
/// # Examples
///
/// Compare only the first 4 letters of the players' names, as the
/// `sc_base duplicates` command does by default:
///
/// ```
/// use scidtopgn::TwinOptions;
///
/// let options = TwinOptions {
///     players: false,
///     ..TwinOptions::default()
/// };
/// assert!(options.moves);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TwinOptions {
    /// Twins have the same players. If `false`, only the first 4 bytes of
    /// each player's name must match, as SCID compares them through a hash.
    pub players: bool,
    /// Twins have the same players with the same colors. If `false`, the
    /// players may have swapped colors.
    pub colors: bool,
    /// Twins have the same event.
    pub event: bool,
    /// Twins have the same site.
    pub site: bool,
    /// Twins have the same round.
    pub round: bool,
    /// Twins have the same year. An unknown year matches only an unknown
    /// year.
    pub year: bool,
    /// Twins have the same month.
    pub month: bool,
    /// Twins have the same day.
    pub day: bool,
    /// Twins have the same result.
    pub result: bool,
    /// Twins have the same ECO code without SCID's extension, such as `B90`
    /// for `B90a`. Games without an ECO code match each other.
    pub eco: bool,
    /// Twins have the same main-line moves for the length of the shorter
    /// game, up to 120 half-moves.
    pub moves: bool,
    /// Games shorter than 10 half-moves are left out: they are neither
    /// deleted nor cause another game to be deleted.
    pub skip_short: bool,
    /// A game with comments is never deleted. If it is the twin that would
    /// be, neither twin is deleted.
    pub comments: bool,
    /// A game with variations is never deleted. If it is the twin that would
    /// be, neither twin is deleted.
    pub variations: bool,
    /// When a game is deleted and its twin has no rating for White or for
    /// Black, the deleted game's rating, and its kind, are copied to the
    /// twin. White's rating is copied to White's, even if the players have
    /// swapped colors.
    pub copy_ratings: bool,
    /// Which twin is deleted.
    pub delete: DeleteTwin,
}

impl Default for TwinOptions {
    fn default() -> Self {
        TwinOptions {
            players: true,
            colors: true,
            event: false,
            site: true,
            round: true,
            year: true,
            month: true,
            day: false,
            result: false,
            eco: false,
            moves: true,
            skip_short: true,
            comments: true,
            variations: true,
            copy_ratings: false,
            delete: DeleteTwin::Shorter,
        }
    }
}

/// A game that can have a twin, and what the search has changed about it.
#[derive(Debug)]
pub(crate) struct Candidate {
    /// The game's number, counting from 0.
    pub(crate) game: u32,
    /// White and Black as they are compared: name IDs, or hashes of the
    /// names' first bytes.
    players: [u32; 2],
    /// Whether the game is to be deleted.
    pub(crate) deleted: bool,
    /// White's and Black's ratings, each with its kind, including any copied
    /// from a deleted twin.
    pub(crate) ratings: [(u16, u8); 2],
    /// Whether a rating was copied to the game.
    pub(crate) ratings_copied: bool,
}

impl Candidate {
    /// Twins have the same sum of players, whichever color each has.
    fn players_sum(&self) -> u32 {
        self.players[0].wrapping_add(self.players[1])
    }
}

/// Finds the twin games in `index` as SCID's `sc_base duplicates` does, and
/// returns the games to delete or copy ratings to. Nothing is changed.
///
/// SCID compares the games in each of its hash buckets in a fixed order, and
/// a game deleted earlier no longer matches, so the order decides which
/// games of three or more twins are kept. Each bucket lists its games from
/// the highest number down, and each game is compared with those after it.
/// Only games with the same sum of players can be twins, so comparing the
/// games of each sum in that order gives the same result.
///
/// # Errors
///
/// Returns [`Error::Game`] for a game whose record can't be read from the
/// game file.
pub(crate) fn find(
    index: &Index,
    names: &NameBase,
    gfile: &mut GFile,
    options: &TwinOptions,
) -> Result<Vec<Candidate>> {
    let entries = index.entries();
    let mut candidates: Vec<Candidate> = (0u32..)
        .zip(entries)
        .filter(|(_, entry)| {
            !entry.get_delete_flag()
                && !(options.skip_short && entry.get_num_half_moves() < SHORT_GAME)
        })
        .map(|(game, entry)| Candidate {
            game,
            players: players(entry, names, options.players),
            deleted: false,
            ratings: [
                (entry.get_white_elo(), entry.get_white_rating_type()),
                (entry.get_black_elo(), entry.get_black_rating_type()),
            ],
            ratings_copied: false,
        })
        .collect();
    candidates.sort_by_key(|c| (c.players_sum(), Reverse(c.game)));

    let entry = |c: &Candidate| &entries[c.game as usize];
    for group in candidates.chunk_by_mut(|a, b| a.players_sum() == b.players_sum()) {
        for first in 0..group.len() {
            let (before, after) = group.split_at_mut(first + 1);
            let head = &mut before[first];
            for compare in after {
                if are_twins(head, compare, entries, names, gfile, options)? {
                    delete_one(head, compare, entry(head), entry(compare), options);
                }
            }
        }
    }

    candidates.retain(|c| c.deleted || c.ratings_copied);
    Ok(candidates)
}

/// White and Black of `entry` as they are compared: their name IDs if
/// `exact`, and otherwise SCID's hash of their names' first bytes.
fn players(entry: &IndexEntry, names: &NameBase, exact: bool) -> [u32; 2] {
    let ids = [entry.get_white(), entry.get_black()];
    if exact {
        return ids;
    }
    ids.map(|id| hash_name(name_bytes(names, NameType::Player, id)))
}

/// The bytes the name file stores for a name, or `?` for a name it doesn't
/// have, as the game reads it.
fn name_bytes(names: &NameBase, ntype: NameType, id: u32) -> &[u8] {
    names.stored_bytes(ntype, id).unwrap_or(b"?")
}

/// SCID's `hashName`: the first 4 bytes of `name`, each a signed `char` as
/// on x86 and Apple arm64, added after shifting the hash 7 bits left.
fn hash_name(name: &[u8]) -> u32 {
    name.iter()
        .take_while(|&&byte| byte != 0)
        .take(4)
        .fold(0, |hash: u32, &byte| {
            (hash << 7).wrapping_add(byte as i8 as u32)
        })
}

/// Whether `head` and `compare` are twins, as SCID's `checkDuplicate` decides.
/// `head` is the higher-numbered game.
fn are_twins(
    head: &Candidate,
    compare: &Candidate,
    entries: &[IndexEntry],
    names: &NameBase,
    gfile: &mut GFile,
    options: &TwinOptions,
) -> Result<bool> {
    if head.deleted || compare.deleted {
        return Ok(false);
    }
    let [head_white, head_black] = head.players;
    let [white, black] = compare.players;
    let same_colors = head_white == white && head_black == black;
    let swapped = head_white == black && head_black == white;
    if !(same_colors || !options.colors && swapped) {
        return Ok(false);
    }

    let (a, b) = (
        &entries[head.game as usize],
        &entries[compare.game as usize],
    );
    let criteria = [
        (options.event, a.get_event() == b.get_event()),
        (options.site, a.get_site() == b.get_site()),
        (options.round, a.get_round() == b.get_round()),
        (options.year, a.get_year() == b.get_year()),
        (options.month, a.get_month() == b.get_month()),
        (options.day, a.get_day() == b.get_day()),
        (options.result, a.get_result() == b.get_result()),
        (options.eco, basic_eco(a) == basic_eco(b)),
    ];
    if criteria.iter().any(|&(selected, same)| selected && !same) {
        return Ok(false);
    }

    // SCID takes a game of at most 2 half-moves to hold the place of the
    // full game, if the higher-numbered game names its White and event.
    if a.get_num_half_moves() <= PLACEHOLDER || b.get_num_half_moves() <= PLACEHOLDER {
        let named = |ntype, id| name_bytes(names, ntype, id) != b"?";
        if named(NameType::Player, a.get_white())
            && named(NameType::Event, a.get_event())
            && a.get_year() == b.get_year()
            && a.get_site() == b.get_site()
            && a.get_round() == b.get_round()
        {
            return Ok(true);
        }
    }

    if !options.moves {
        return Ok(true);
    }
    if !home_pawns_prefix(a.get_home_pawn_data(), b.get_home_pawn_data()) {
        return Ok(false);
    }
    same_moves(gfile, (head.game, a), (compare.game, b))
}

/// The ECO code without SCID's extension, or `None` if there is none.
fn basic_eco(entry: &IndexEntry) -> Option<u16> {
    match entry.get_eco_code() {
        0 => None,
        code => Some((code - 1) / 131),
    }
}

/// SCID's `hpSig_Prefix`: whether the shorter of two home pawn lists is the
/// start of the other. Each list is a count, then one change per nibble.
fn home_pawns_prefix(a: &[u8; HPSIG_SIZE], b: &[u8; HPSIG_SIZE]) -> bool {
    let nibble = |list: &[u8; HPSIG_SIZE], i: usize| {
        let byte = list[1 + i / 2];
        if i.is_multiple_of(2) {
            byte >> 4
        } else {
            byte & 15
        }
    };
    // The data holds 16 changes; a larger count is damaged.
    let count = usize::from(a[0].min(b[0])).min(2 * (HPSIG_SIZE - 1));
    (0..count).all(|i| nibble(a, i) == nibble(b, i))
}

/// Whether games `a` and `b` have the same main-line moves for the length of
/// the shorter, up to 120 half-moves, as SCID's `gamesHaveSameMoves`
/// decides. The shorter game is the one with fewer half-moves in the index,
/// or `a` if they have as many.
///
/// The moves of the shorter game are read until its end, or until a move
/// that can't be read. The longer game must have the same moves, compared
/// by their squares and promotion. A record whose start can't be read has
/// no twin.
///
/// # Errors
///
/// Returns [`Error::Game`] if either game's record can't be read from the
/// game file.
fn same_moves(gfile: &mut GFile, a: (u32, &IndexEntry), b: (u32, &IndexEntry)) -> Result<bool> {
    let (shorter, longer) = if b.1.get_num_half_moves() < a.1.get_num_half_moves() {
        (b, a)
    } else {
        (a, b)
    };
    let shorter = read_record(gfile, shorter)?;
    let longer = read_record(gfile, longer)?;
    let (Some(mut shorter), Some(mut longer)) = (MainLine::new(shorter), MainLine::new(longer))
    else {
        return Ok(false);
    };
    let moves: Vec<_> = std::iter::from_fn(|| shorter.next_move())
        .take(MAX_SAME_MOVES)
        .collect();
    Ok(moves.into_iter().all(|m| longer.next_move() == Some(m)))
}

/// Reads the record of game `game`, whose index entry is `entry`.
fn read_record(gfile: &mut GFile, (game, entry): (u32, &IndexEntry)) -> Result<ByteBuffer> {
    let mut buf = ByteBuffer::new();
    gfile
        .read_game(&mut buf, entry.get_offset(), entry.get_length())
        .map_err(|source| Error::Game {
            number: u64::from(game) + 1,
            source: Box::new(source),
        })?;
    Ok(buf)
}

/// A move as SCID compares moves: its from-square, its to-square (the
/// king's for castling, and the king's own square for a null move), and its
/// promotion. Squares count from a1 to h8, as in SCID and shakmaty.
type MoveSquares = (usize, usize, Option<Role>);

/// Reads the main line of a game record one move at a time, as SCID's
/// `Game::DecodeNextMove` does.
struct MainLine {
    buf: ByteBuffer,
    board: Board,
}

impl MainLine {
    /// Reads the start of `buf`, a game record, or returns `None` if it
    /// can't be read.
    fn new(mut buf: ByteBuffer) -> Option<Self> {
        let mut game = Game::new();
        game.decode_start(&mut buf).ok()?;
        let position = game.initial_position().ok()?;
        Some(MainLine {
            buf,
            board: Board::new(&position),
        })
    }

    /// Returns the next move of the main line, skipping NAGs, comments, and
    /// variations, or `None` at the end of the game or at a move that can't
    /// be read.
    fn next_move(&mut self) -> Option<MoveSquares> {
        loop {
            match self.buf.get_byte().ok()? {
                ENCODE_NAG => {
                    self.buf.get_byte().ok()?;
                }
                ENCODE_COMMENT => {}
                ENCODE_START_MARKER => self.skip_variation()?,
                ENCODE_END_MARKER | ENCODE_END_GAME => return None,
                byte => {
                    let squares = self.board.decode(&mut self.buf, byte)?;
                    self.board.play(squares)?;
                    return Some(squares);
                }
            }
        }
    }

    /// Skips a variation, with the variations inside it, after its start
    /// marker. Returns `None` if the game ends first.
    fn skip_variation(&mut self) -> Option<()> {
        let mut depth = 1;
        while depth > 0 {
            match self.buf.get_byte().ok()? {
                ENCODE_NAG => {
                    self.buf.get_byte().ok()?;
                }
                ENCODE_START_MARKER => depth += 1,
                ENCODE_END_MARKER => depth -= 1,
                ENCODE_END_GAME => return None,
                _ => {}
            }
        }
        Some(())
    }
}

/// The pieces and SCID's numbering of them, as SCID's `decodeMove` and
/// `Position::DoSimpleMove` read and change them while the twin search
/// compares moves. Like SCID, it doesn't check that a move is legal, so a
/// game is compared past an illegal move SCID can decode.
///
/// Where SCID would read a square off the board, or the number of a piece
/// that isn't there, its result is undefined, and the move ends the game
/// instead.
struct Board {
    pieces: [Option<Piece>; 64],
    /// Each side's pieces' squares, by number.
    squares: ByColor<[usize; 16]>,
    /// The number of pieces each side has.
    counts: ByColor<usize>,
    /// The number of the piece on each square.
    numbers: [usize; 64],
    turn: Color,
}

impl Board {
    fn new(position: &ScidPosition) -> Self {
        let chess = position.chess();
        let list = position.pieces();
        let mut board = Board {
            pieces: [None; 64],
            squares: ByColor::default(),
            counts: ByColor::default(),
            numbers: [0; 64],
            turn: chess.turn(),
        };
        for (square, piece) in chess.board() {
            board.pieces[square.to_usize()] = Some(piece);
        }
        for color in Color::ALL {
            let count = list.count(color);
            for number in 0..count {
                if let Some(square) = list.square(color, number) {
                    board.squares.get_mut(color)[usize::from(number)] = square.to_usize();
                    board.numbers[square.to_usize()] = usize::from(number);
                }
            }
            *board.counts.get_mut(color) = usize::from(count);
        }
        board
    }

    /// Decodes the move whose first byte, `byte`, was just read from `buf`,
    /// as SCID's `decodeMove` does.
    fn decode(&self, buf: &mut ByteBuffer, byte: u8) -> Option<MoveSquares> {
        let number = usize::from(byte >> 4);
        if number >= *self.counts.get(self.turn) {
            return None;
        }
        let from = self.squares.get(self.turn)[number];
        let piece = self.pieces[from]?;
        let value = byte & 15;
        let index = usize::from(value);
        let (from_i, value_i) = (from as i32, i32::from(value));
        let (file, rank) = (from_i % 8, from_i / 8);
        let mut promotion = None;
        let to = match piece.role {
            Role::Pawn => {
                promotion = PAWN_PROMO[index];
                match self.turn {
                    Color::White => from_i + PAWN_SQDIFF[index],
                    Color::Black => from_i - PAWN_SQDIFF[index],
                }
            }
            Role::Knight => from_i + KNIGHT_SQDIFF.get(index).filter(|_| index > 0)?,
            Role::Rook if value >= 8 => file + 8 * (value_i - 8),
            Role::Rook => value_i + 8 * rank,
            Role::Bishop => {
                let file_diff = (value_i & 7) - file;
                if value >= 8 {
                    from_i - 7 * file_diff
                } else {
                    from_i + 9 * file_diff
                }
            }
            // A king move of 0 is a null move.
            Role::King => from_i + KING_SQDIFF.get(index)?,
            Role::Queen if value >= 8 => file + 8 * (value_i - 8),
            Role::Queen if value_i != file => value_i + 8 * rank,
            Role::Queen => match buf.get_byte().ok()? {
                diagonal @ 64..=127 => i32::from(diagonal) - 64,
                _ => return None,
            },
        };
        let to = usize::try_from(to).ok().filter(|&to| to < 64)?;
        Some((from, to, promotion))
    }

    /// Plays `(from, to, promotion)`, a move [`decode`](Self::decode)
    /// returned, as SCID's `Position::DoSimpleMove` does.
    fn play(&mut self, (from, to, promotion): MoveSquares) -> Option<()> {
        let turn = self.turn;
        let enemy = !turn;
        let piece = self.pieces[from]?;
        let number = self.numbers[from];
        self.turn = enemy;
        if from == to && piece.role == Role::King {
            return Some(());
        }

        // SCID takes a pawn that changes file without capturing to capture
        // en passant, whatever is on the square behind.
        let mut captured = to;
        if piece.role == Role::Pawn && self.pieces[to].is_none() && from % 8 != to % 8 {
            captured = match turn {
                Color::White => to.checked_sub(8)?,
                Color::Black => Some(to + 8).filter(|&sq| sq < 64)?,
            };
            self.pieces[captured]?;
        }
        if self.pieces[captured].is_some() {
            // The enemy's last piece takes the captured piece's number, even
            // if the captured piece is the mover's own.
            let captured_number = self.numbers[captured];
            let count = self.counts.get_mut(enemy);
            *count = count.checked_sub(1)?;
            let last = self.squares.get(enemy)[*count];
            self.numbers[last] = captured_number;
            self.squares.get_mut(enemy)[captured_number] = last;
            self.pieces[captured] = None;
        }

        let moved = match promotion {
            Some(role) => Piece { color: turn, role },
            None => piece,
        };
        self.squares.get_mut(turn)[number] = to;
        self.numbers[to] = number;
        self.pieces[from] = None;
        self.pieces[to] = Some(moved);

        // SCID castles any king move from the e-file to the c- or g-file,
        // putting a rook beside it.
        if piece.role == Role::King && from % 8 == 4 && matches!(to % 8, 2 | 6) {
            let (rook_from, rook_to) = if to % 8 == 2 {
                (to - 2, to + 1)
            } else {
                (to + 1, to - 1)
            };
            self.pieces[rook_from]?;
            let rook_number = self.numbers[rook_from];
            self.numbers[rook_to] = rook_number;
            self.squares.get_mut(turn)[rook_number] = rook_to;
            self.pieces[rook_from] = None;
            self.pieces[rook_to] = Some(Piece {
                color: turn,
                role: Role::Rook,
            });
        }
        Some(())
    }
}

/// Marks one of the twins `head` and `compare` deleted, as
/// `sc_base duplicates` chooses, and copies ratings to the other if
/// [`TwinOptions::copy_ratings`] is set. `head` is the higher-numbered game.
fn delete_one(
    head: &mut Candidate,
    compare: &mut Candidate,
    head_entry: &IndexEntry,
    compare_entry: &IndexEntry,
    options: &TwinOptions,
) {
    let delete_head = match options.delete {
        DeleteTwin::Shorter => head_entry.get_num_half_moves() < compare_entry.get_num_half_moves(),
        DeleteTwin::Older => head.game < compare.game,
        DeleteTwin::Newer => head.game > compare.game,
    };
    let (deleted, deleted_entry, kept) = if delete_head {
        (head, head_entry, compare)
    } else {
        (compare, compare_entry, head)
    };
    let protected = options.comments && deleted_entry.get_comment_count() > 0
        || options.variations && deleted_entry.get_variation_count() > 0;
    if protected {
        return;
    }

    deleted.deleted = true;
    if options.copy_ratings {
        for color in 0..2 {
            if kept.ratings[color].0 == 0 && deleted.ratings[color].0 != 0 {
                kept.ratings[color] = deleted.ratings[color];
                kept.ratings_copied = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_name_uses_first_four_signed_bytes() {
        assert_ne!(hash_name(b"Bob"), hash_name(b"Bobby"));
        assert_eq!(hash_name(b"Bobby"), hash_name(b"Bobb, Al"));
        // As unsigned bytes, 0x63 0xC3 and 0x64 0x43 add to the same hash.
        assert_ne!(hash_name("AbcÀ".as_bytes()), hash_name(b"AbdC"));
        assert_eq!(hash_name(b"\xC3"), 0xFFFF_FFC3);
    }

    #[test]
    fn test_home_pawns_prefix_compares_the_shorter_list() {
        let list = |count: u8, changes: &[u8]| {
            let mut data = [0; HPSIG_SIZE];
            data[0] = count;
            data[1..=changes.len()].copy_from_slice(changes);
            data
        };
        let three = list(3, &[0x12, 0x30]);
        assert!(home_pawns_prefix(&three, &list(2, &[0x12])));
        assert!(home_pawns_prefix(&list(1, &[0x10]), &three));
        assert!(home_pawns_prefix(&list(0, &[]), &three));
        assert!(!home_pawns_prefix(&list(2, &[0x13]), &three));
        assert!(!home_pawns_prefix(&list(3, &[0x12, 0x40]), &three));
        // A count past the data compares all of it.
        assert!(home_pawns_prefix(&[255; HPSIG_SIZE], &[255; HPSIG_SIZE]));
    }
}
