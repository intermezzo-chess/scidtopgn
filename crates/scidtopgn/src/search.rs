//! Header searches: [`HeaderSearch`] chooses the games
//! [`Database::search_header`](crate::Database::search_header) selects by
//! their names, date, result, ratings, ECO code, length, and flags, as SCID
//! vs. PC's `sc_search header` does.

use std::ops::RangeInclusive;

use shakmaty::Color;

use crate::common::GameResult;
use crate::date::{Date, YEAR_MAX};
use crate::error::{Error, Result};
use crate::index::{
    Eco, Index, IndexEntry, IDX_MASK_BLACK_OP, IDX_MASK_BLUNDER, IDX_MASK_BRILLIANCY,
    IDX_MASK_ENDGAME, IDX_MASK_KSIDE, IDX_MASK_MIDDLEGAME, IDX_MASK_NOVELTY, IDX_MASK_PAWN,
    IDX_MASK_QSIDE, IDX_MASK_TACTICS, IDX_MASK_UPROMO, IDX_MASK_USER, IDX_MASK_WHITE_OP, MAX_ELO,
};
use crate::namebase::{NameBase, NameType};

/// The longest game, in half-moves, that [`HeaderSearch::default`] matches,
/// as in SCID's search window.
pub const DEFAULT_MAX_LENGTH: u16 = 999;

/// Whether a game must have a flag for
/// [`Database::search_header`](crate::Database::search_header) to select it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FlagMatch {
    /// The game may have the flag or not.
    #[default]
    Either,
    /// The game has the flag.
    Yes,
    /// The game doesn't have the flag.
    No,
}

impl FlagMatch {
    fn matches(self, set: bool) -> bool {
        match self {
            FlagMatch::Either => true,
            FlagMatch::Yes => set,
            FlagMatch::No => !set,
        }
    }
}

/// The flags a game must have, or not, for
/// [`Database::search_header`](crate::Database::search_header) to select it.
/// Each is [`FlagMatch::Either`] by default.
///
/// Each field is an `sc_search header` option. The start, promotion,
/// comment, variation, and annotation flags come from the index, which
/// derives them from the main line's moves, or from the whole game for
/// comments, variations, and annotations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct HeaderFlags {
    /// The game starts from a position other than the standard one. SCID
    /// calls this option `fStdStart`, and its window "Non-standard start".
    pub non_standard_start: FlagMatch,
    /// The main line has a promotion.
    pub promotions: FlagMatch,
    /// The main line has a promotion to a piece other than a queen.
    pub underpromotions: FlagMatch,
    /// The game has a comment.
    pub comments: FlagMatch,
    /// The game has a variation.
    pub variations: FlagMatch,
    /// The game has a NAG, such as `!` or `+-`.
    pub annotations: FlagMatch,
    /// The game is logically deleted.
    pub deleted: FlagMatch,
    /// SCID's White opening flag.
    pub white_opening: FlagMatch,
    /// SCID's Black opening flag.
    pub black_opening: FlagMatch,
    /// SCID's middlegame flag.
    pub middlegame: FlagMatch,
    /// SCID's endgame flag.
    pub endgame: FlagMatch,
    /// SCID's novelty flag.
    pub novelty: FlagMatch,
    /// SCID's pawn structure flag.
    pub pawn_structure: FlagMatch,
    /// SCID's tactics flag.
    pub tactics: FlagMatch,
    /// SCID's kingside play flag.
    pub kingside: FlagMatch,
    /// SCID's queenside play flag.
    pub queenside: FlagMatch,
    /// SCID's brilliancy flag.
    pub brilliancy: FlagMatch,
    /// SCID's blunder flag.
    pub blunder: FlagMatch,
    /// SCID's user flag.
    pub user: FlagMatch,
    /// Custom flags 1 to 6, in that order.
    pub custom: [FlagMatch; 6],
}

impl HeaderFlags {
    /// Whether `entry` has the flags, as SCID's `matchGameFlags` checks.
    fn matches(&self, entry: &IndexEntry) -> bool {
        let masks = [
            (self.white_opening, IDX_MASK_WHITE_OP),
            (self.black_opening, IDX_MASK_BLACK_OP),
            (self.middlegame, IDX_MASK_MIDDLEGAME),
            (self.endgame, IDX_MASK_ENDGAME),
            (self.novelty, IDX_MASK_NOVELTY),
            (self.pawn_structure, IDX_MASK_PAWN),
            (self.tactics, IDX_MASK_TACTICS),
            (self.kingside, IDX_MASK_KSIDE),
            (self.queenside, IDX_MASK_QSIDE),
            (self.brilliancy, IDX_MASK_BRILLIANCY),
            (self.blunder, IDX_MASK_BLUNDER),
            (self.user, IDX_MASK_USER),
        ];
        self.non_standard_start.matches(entry.get_start_flag())
            && self
                .underpromotions
                .matches(entry.get_flag(IDX_MASK_UPROMO))
            && self.promotions.matches(entry.get_promotions_flag())
            && self.comments.matches(entry.get_comment_count() > 0)
            && self.variations.matches(entry.get_variation_count() > 0)
            && self.annotations.matches(entry.get_nag_count() > 0)
            && self.deleted.matches(entry.get_delete_flag())
            && masks
                .iter()
                .all(|&(flag, mask)| flag.matches(entry.get_flag(mask)))
            && (1..=self.custom.len()).all(|n| self.custom[n - 1].matches(entry.get_custom_flag(n)))
    }
}

/// What a game must have for
/// [`Database::search_header`](crate::Database::search_header) to select it.
/// A game must meet every criterion.
///
/// Each field is the `sc_search header` option of the same meaning, and the
/// defaults are those of SCID vs. PC's search window, which match every game
/// of at most [`DEFAULT_MAX_LENGTH`] half-moves.
///
/// # Names
///
/// The White, Black, event, site, and round names match as in SCID:
///
/// - Text such as `carlsen` matches a name that contains it, comparing
///   ASCII letters without case and skipping spaces in both the name and the
///   text. Text that ends with a space never matches, as in SCID.
/// - Text in double quotes, such as `"Carlsen*"`, matches the whole name as
///   a pattern, as Tcl's `string match` does, with case: `*` matches any
///   characters, `?` one character, `[ab]` or `[a-z]` one of the characters
///   given, and `\` makes the next character match only itself.
/// - Empty text, or `None`, matches every name.
///
/// Names are compared as they are read, as UTF-8, including names converted
/// from Windows-1252.
///
/// # Examples
///
/// ```
/// use scidtopgn::{Eco, HeaderSearch};
///
/// let search = HeaderSearch {
///     white: Some("Carlsen".into()),
///     eco: "B20".parse::<Eco>()?..="B99".parse()?,
///     with_no_eco: false,
///     ..HeaderSearch::default()
/// };
/// assert_eq!(search.length, 0..=999);
/// # Ok::<(), scidtopgn::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HeaderSearch {
    /// Text White's name must match.
    pub white: Option<String>,
    /// Text Black's name must match.
    pub black: Option<String>,
    /// Text the event's name must match.
    pub event: Option<String>,
    /// Text the site's name must match.
    pub site: Option<String>,
    /// Text the round's name must match.
    pub round: Option<String>,
    /// The dates the game may have. An unknown part of a date is 0, so
    /// `2023.??.??` comes before `2023.01.01`. By default, every date.
    pub dates: RangeInclusive<Date>,
    /// The results the game may have. By default, all four.
    pub results: Vec<GameResult>,
    /// White's rating. A game without one uses White's highest rating in
    /// any game of the database, deleted or not, or 0 if there is none, as
    /// SCID does. By default, 0 to [`MAX_ELO`].
    pub white_elo: RangeInclusive<u16>,
    /// Black's rating, which is taken as for
    /// [`white_elo`](Self::white_elo). By default, 0 to [`MAX_ELO`].
    pub black_elo: RangeInclusive<u16>,
    /// White's rating minus Black's, each taken as for
    /// [`white_elo`](Self::white_elo). A range whose start is above its end
    /// is swapped, as in SCID. By default, -[`MAX_ELO`] to [`MAX_ELO`].
    pub elo_diff: RangeInclusive<i32>,
    /// The ECO codes the game may have. The end includes its extensions,
    /// so `B07` means up to `B07z4`, and `B07a` up to `B07a4`. By default,
    /// [`Eco::A00`] to [`Eco::E99`], which is every code.
    pub eco: RangeInclusive<Eco>,
    /// Whether a game without an ECO code matches, whatever
    /// [`eco`](Self::eco) is. By default, `true`, as in SCID.
    pub with_no_eco: bool,
    /// The number of half-moves in the main line, which the index keeps
    /// modulo 1,024. By default, 0 to [`DEFAULT_MAX_LENGTH`].
    pub length: RangeInclusive<u16>,
    /// The side to move at the end of the game, or `None` for either. As in
    /// SCID, it is White after an even number of half-moves and Black after
    /// an odd number, even in a game that starts with Black to move.
    pub to_move: Option<Color>,
    /// The first and last game numbers, counting from 1, unlike the numbers
    /// [`Database::search_header`](crate::Database::search_header) returns.
    /// A negative number counts from the end: -1 is the last game. Numbers
    /// past either end are moved to it, and the two are swapped if the first
    /// is after the last, as in SCID. By default, `(1, -1)`, which is every
    /// game.
    pub game_numbers: (i64, i64),
    /// Whether a game also matches with White and Black swapped: their
    /// names, ratings, and wins, the side to move, and the sign of the
    /// rating difference.
    pub ignore_colors: bool,
    /// The flags the game must have, or not.
    pub flags: HeaderFlags,
}

impl Default for HeaderSearch {
    fn default() -> Self {
        let max_elo = i32::from(MAX_ELO);
        HeaderSearch {
            white: None,
            black: None,
            event: None,
            site: None,
            round: None,
            dates: Date::ZERO..=Date::new(YEAR_MAX, 12, 31).expect("the last date is valid"),
            results: vec![
                GameResult::White,
                GameResult::Draw,
                GameResult::Black,
                GameResult::None,
            ],
            white_elo: 0..=MAX_ELO,
            black_elo: 0..=MAX_ELO,
            elo_diff: -max_elo..=max_elo,
            eco: Eco::A00..=Eco::E99,
            with_no_eco: true,
            length: 0..=DEFAULT_MAX_LENGTH,
            to_move: None,
            game_numbers: (1, -1),
            ignore_colors: false,
            flags: HeaderFlags::default(),
        }
    }
}

/// Returns the games `search` selects from `index`, counting from 0, in
/// order, as `sc_search_header` does.
pub(crate) fn search(index: &Index, names: &NameBase, search: &HeaderSearch) -> Result<Vec<u32>> {
    let num_games = index.num_games();
    let (first, last) = game_range(search.game_numbers, num_games);

    let players = [&search.white, &search.black]
        .map(|text| NameMatches::new(names, NameType::Player, text.as_deref()));
    let event = NameMatches::new(names, NameType::Event, search.event.as_deref());
    let site = NameMatches::new(names, NameType::Site, search.site.as_deref());
    let round = NameMatches::new(names, NameType::Round, search.round.as_deref());

    let mut results = [false; 4];
    for result in &search.results {
        results[usize::from(result.to_byte())] = true;
    }
    let (diff_min, diff_max) = {
        let (a, b) = (*search.elo_diff.start(), *search.elo_diff.end());
        (a.min(b), a.max(b))
    };
    let to_move = match search.to_move {
        None => [true, true],
        Some(color) => [color == Color::White, color == Color::Black],
    };
    let sides = Sides {
        names: [players[0].as_ref(), players[1].as_ref()],
        results,
        elo: [u16_range(&search.white_elo), u16_range(&search.black_elo)],
        elo_diff: (diff_min, diff_max),
        to_move,
    };
    let criteria = Criteria {
        sides,
        swapped: sides.swapped(),
        highest_elo: highest_elo(index, names),
        dates: (search.dates.start().raw(), search.dates.end().raw()),
        eco: (
            search.eco.start().code(),
            search.eco.end().last_sub_code().code(),
        ),
        with_no_eco: search.with_no_eco,
        length: (*search.length.start(), *search.length.end()),
        event: event.as_ref(),
        site: site.as_ref(),
        round: round.as_ref(),
    };

    let mut games = Vec::new();
    for (game, entry) in (0..num_games).zip(index.entries()) {
        let number = u64::from(game) + 1;
        if number < first || number > last {
            continue;
        }
        // A game without a record never matches.
        if entry.get_length() == 0 || !search.flags.matches(entry) {
            continue;
        }
        if criteria.matches(entry, &criteria.sides)?
            || (search.ignore_colors && criteria.matches(entry, &criteria.swapped)?)
        {
            games.push(game);
        }
    }
    Ok(games)
}

/// The first and last game numbers to search, counting from 1, from
/// `numbers`, as `sc_search_header` takes them.
fn game_range((first, last): (i64, i64), num_games: u32) -> (u64, u64) {
    let num_games = i64::from(num_games);
    let [first, last] = [first, last].map(|n| {
        let n = if n < 0 { n + num_games + 1 } else { n };
        n.clamp(0, num_games) as u64
    });
    (first.min(last), first.max(last))
}

fn u16_range(range: &RangeInclusive<u16>) -> (i32, i32) {
    (i32::from(*range.start()), i32::from(*range.end()))
}

/// Each player's highest rating in any game of `index`, by player ID, as
/// SCID's `recalcFlagCounts` collects them.
fn highest_elo(index: &Index, names: &NameBase) -> Vec<u16> {
    let mut highest = vec![0u16; names.count(NameType::Player) as usize];
    for entry in index.entries() {
        for (id, elo) in [
            (entry.get_white(), entry.get_white_elo()),
            (entry.get_black(), entry.get_black_elo()),
        ] {
            if let Some(max) = highest.get_mut(id as usize) {
                *max = (*max).max(elo);
            }
        }
    }
    highest
}

/// Which names of one kind match a search's text, by ID.
struct NameMatches {
    name_type: NameType,
    /// Whether each name matches, or `None` for an ID with no name.
    matches: Vec<Option<bool>>,
}

impl NameMatches {
    /// Returns which names of kind `name_type` match `text`, or `None` if
    /// there is no text, so that every name matches.
    fn new(names: &NameBase, name_type: NameType, text: Option<&str>) -> Option<Self> {
        let text = text.filter(|text| !text.is_empty())?;
        let pattern = text
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .map(|pattern| pattern.chars().collect::<Vec<_>>());
        let matches = (0..names.count(name_type))
            .map(|id| {
                let name = names.get_name(name_type, id)?;
                Some(match &pattern {
                    Some(pattern) => string_match(&name.chars().collect::<Vec<_>>(), pattern),
                    None => alpha_contains(name.as_bytes(), text.as_bytes()),
                })
            })
            .collect();
        Some(NameMatches { name_type, matches })
    }

    /// Whether the name with ID `id` matches.
    fn get(&self, id: u32) -> Result<bool> {
        self.matches
            .get(id as usize)
            .copied()
            .flatten()
            .ok_or(Error::NameNotFound {
                name_type: self.name_type,
                id,
            })
    }
}

/// The criteria that change when White and Black are swapped, indexed by
/// side: White's first, then Black's.
#[derive(Clone, Copy)]
struct Sides<'a> {
    names: [Option<&'a NameMatches>; 2],
    results: [bool; 4],
    elo: [(i32, i32); 2],
    elo_diff: (i32, i32),
    to_move: [bool; 2],
}

impl Sides<'_> {
    /// The criteria with White and Black swapped, as `sc_search_header`
    /// swaps them to ignore colors.
    fn swapped(&self) -> Self {
        let mut results = self.results;
        results.swap(
            usize::from(GameResult::White.to_byte()),
            usize::from(GameResult::Black.to_byte()),
        );
        Sides {
            names: [self.names[1], self.names[0]],
            results,
            elo: [self.elo[1], self.elo[0]],
            elo_diff: (-self.elo_diff.1, -self.elo_diff.0),
            to_move: [self.to_move[1], self.to_move[0]],
        }
    }
}

/// A search's criteria, prepared for checking each game.
struct Criteria<'a> {
    sides: Sides<'a>,
    swapped: Sides<'a>,
    highest_elo: Vec<u16>,
    dates: (u32, u32),
    eco: (u16, u16),
    with_no_eco: bool,
    length: (u16, u16),
    event: Option<&'a NameMatches>,
    site: Option<&'a NameMatches>,
    round: Option<&'a NameMatches>,
}

impl Criteria<'_> {
    /// Whether `entry` meets the criteria with `sides`, in the order SCID's
    /// `matchGameHeader` checks them, so that a missing name is only an
    /// error where SCID would read it.
    fn matches(&self, entry: &IndexEntry, sides: &Sides) -> Result<bool> {
        if !sides.results[usize::from(entry.get_result().to_byte())] {
            return Ok(false);
        }

        let half_moves = entry.get_num_half_moves();
        if half_moves < self.length.0 || half_moves > self.length.1 {
            return Ok(false);
        }
        if !sides.to_move[usize::from(half_moves % 2)] {
            return Ok(false);
        }

        let date = entry.get_date();
        if date < self.dates.0 || date > self.dates.1 {
            return Ok(false);
        }

        let elo = |id: u32, elo: u16| -> i32 {
            match elo {
                0 => self.highest_elo.get(id as usize).copied().unwrap_or(0),
                elo => elo,
            }
            .into()
        };
        let white_elo = elo(entry.get_white(), entry.get_white_elo());
        let black_elo = elo(entry.get_black(), entry.get_black_elo());
        let in_range = |value: i32, (min, max): (i32, i32)| (min..=max).contains(&value);
        if !in_range(white_elo, sides.elo[0])
            || !in_range(black_elo, sides.elo[1])
            || !in_range(white_elo - black_elo, sides.elo_diff)
        {
            return Ok(false);
        }

        let eco = entry.get_eco_code();
        let eco_matches = match eco {
            0 => self.with_no_eco,
            eco => eco >= self.eco.0 && eco <= self.eco.1,
        };
        if !eco_matches {
            return Ok(false);
        }

        for (names, id) in [
            (self.event, entry.get_event()),
            (self.site, entry.get_site()),
            (self.round, entry.get_round()),
            (sides.names[0], entry.get_white()),
            (sides.names[1], entry.get_black()),
        ] {
            if let Some(names) = names {
                if !names.get(id)? {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}

/// Whether `name` contains `text`, comparing ASCII letters without case and
/// skipping spaces, as SCID's `strAlphaContains` does.
fn alpha_contains(name: &[u8], text: &[u8]) -> bool {
    (0..name.len()).any(|start| is_alpha_prefix(text, &name[start..]))
}

/// Whether `name` starts with `prefix`, comparing ASCII letters without case
/// and skipping spaces, as SCID's `strIsAlphaPrefix` does.
fn is_alpha_prefix(prefix: &[u8], name: &[u8]) -> bool {
    let (mut p, mut n) = (0, 0);
    while p < prefix.len() {
        while prefix.get(p) == Some(&b' ') {
            p += 1;
        }
        while name.get(n) == Some(&b' ') {
            n += 1;
        }
        let Some(&c) = name.get(n) else {
            return false;
        };
        // After spaces that end the prefix, SCID compares the prefix's
        // terminating NUL with the name's next byte, which never matches.
        match prefix.get(p) {
            Some(prefix_char) if prefix_char.eq_ignore_ascii_case(&c) => {}
            _ => return false,
        }
        p += 1;
        n += 1;
    }
    true
}

/// Whether `s` matches `pattern`, as Tcl's `Tcl_StringMatch` does: `*`
/// matches any characters, `?` one character, `[...]` one of the
/// characters or ranges in it, and `\` makes the next character literal.
fn string_match(mut s: &[char], mut pattern: &[char]) -> bool {
    loop {
        let Some(&p) = pattern.first() else {
            return s.is_empty();
        };
        if s.is_empty() && p != '*' {
            return false;
        }
        match p {
            '*' => {
                while pattern.first() == Some(&'*') {
                    pattern = &pattern[1..];
                }
                if pattern.is_empty() {
                    return true;
                }
                loop {
                    if string_match(s, pattern) {
                        return true;
                    }
                    let Some((_, rest)) = s.split_first() else {
                        return false;
                    };
                    s = rest;
                }
            }
            '?' => {
                pattern = &pattern[1..];
                s = &s[1..];
            }
            '[' => {
                pattern = &pattern[1..];
                let c = s[0];
                s = &s[1..];
                loop {
                    let Some((&start, rest)) = pattern.split_first().filter(|(&p, _)| p != ']')
                    else {
                        return false;
                    };
                    pattern = rest;
                    if pattern.first() == Some(&'-') {
                        let Some((&end, rest)) = pattern[1..].split_first() else {
                            return false;
                        };
                        pattern = rest;
                        if (start <= c && c <= end) || (end <= c && c <= start) {
                            break;
                        }
                    } else if start == c {
                        break;
                    }
                }
                // Skip the rest of the class. Without a closing `]`, Tcl
                // ends the pattern.
                pattern = match pattern.iter().position(|&p| p == ']') {
                    Some(close) => &pattern[close + 1..],
                    None => &[],
                };
            }
            _ => {
                if p == '\\' {
                    pattern = &pattern[1..];
                }
                match pattern.first() {
                    Some(&p) if p == s[0] => {
                        pattern = &pattern[1..];
                        s = &s[1..];
                    }
                    _ => return false,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches(s: &str, pattern: &str) -> bool {
        let chars = |s: &str| s.chars().collect::<Vec<_>>();
        string_match(&chars(s), &chars(pattern))
    }

    #[test]
    fn contains_text_ignoring_ascii_case_and_spaces() {
        let contains = |name: &str, text: &str| alpha_contains(name.as_bytes(), text.as_bytes());
        assert!(contains("Carlsen, Magnus", "carlsen"));
        assert!(contains("Carlsen, Magnus", "CARL SEN"));
        assert!(contains("Carlsen, Magnus", " sen,mag"));
        assert!(contains("van der Wiel, John", "derwiel"));
        assert!(contains("Müller", "müller"));
        assert!(!contains("MÜLLER", "müller"));
        // Text that ends with a space, or is only spaces, never matches.
        assert!(!contains("Carlsen, Magnus", "Carlsen "));
        assert!(!contains("Carlsen  Magnus", "Carlsen "));
        assert!(!contains("Carlsen", "  "));
        assert!(!contains("", "a"));
        assert!(!contains("Carl", "Carlsen"));
    }

    #[test]
    fn matches_patterns_as_tcl() {
        assert!(matches("Carlsen, Magnus", "Carlsen*"));
        assert!(matches("Carlsen", "*"));
        assert!(matches("", "*"));
        assert!(matches("", ""));
        assert!(!matches("Carlsen", ""));
        assert!(!matches("Carlsen", "carlsen"));
        assert!(matches("Anand, Viswanathan", "Anand, ?iswanathan"));
        assert!(matches("Müller", "M?ller"));
        assert!(matches("Carlsen", "*sen"));
        assert!(matches("Carlsen", "C**l*n"));
        assert!(!matches("Carlsen", "*x*"));
        assert!(matches("Carlsen", "[A-D]*"));
        assert!(matches("Carlsen", "[D-A]*"));
        assert!(matches("Smith", "[QRS]mith"));
        assert!(!matches("Smith", "[QR]mith"));
        assert!(matches("Smith [A]", "Smith \\[A\\]"));
        assert!(matches("Smith A", "Smith [A]"));
        assert!(matches("a*b", "a\\*b"));
        assert!(!matches("axb", "a\\*b"));
        // A pattern that ends inside a class or after `\` matches nothing
        // more.
        assert!(matches("C", "[A-C"));
        assert!(!matches("Carlsen", "[A-C*"));
        assert!(!matches("C", "[A-"));
        assert!(!matches("C", "[]"));
        assert!(!matches("a", "a\\"));
    }

    #[test]
    fn reads_game_numbers_as_scid() {
        assert_eq!(game_range((1, -1), 10), (1, 10));
        assert_eq!(game_range((-3, -1), 10), (8, 10));
        assert_eq!(game_range((7, 3), 10), (3, 7));
        assert_eq!(game_range((-100, 3), 10), (0, 3));
        assert_eq!(game_range((5, 100), 10), (5, 10));
        assert_eq!(game_range((0, 0), 10), (0, 0));
        assert_eq!(game_range((1, -1), 0), (0, 0));
    }

    #[test]
    fn extends_the_last_eco_code() {
        let last = |code: &str| code.parse::<Eco>().unwrap().last_sub_code().to_string();
        assert_eq!(last("B07"), "B07z4");
        assert_eq!(last("B07a"), "B07a4");
        assert_eq!(last("B07a2"), "B07a2");
        assert_eq!(last("E99"), "E99z4");
        assert_eq!(last("A00"), "A00z4");
    }

    /// An index and name base with one game for each entry `edit` makes,
    /// each between players 0 and 1, with a record.
    fn database(entries: &[fn(&mut IndexEntry)]) -> (Index, NameBase) {
        let mut names = NameBase::new();
        for (name_type, name) in [
            (NameType::Player, "White"),
            (NameType::Player, "Black"),
            (NameType::Event, "Event"),
            (NameType::Site, "Site"),
            (NameType::Round, "1"),
        ] {
            names.add_name(name_type, name).unwrap();
        }
        let mut index = Index::new();
        for edit in entries {
            let mut entry = IndexEntry::new();
            entry.set_white(0);
            entry.set_black(1);
            entry.set_length(10);
            edit(&mut entry);
            index.add_entry(entry).unwrap();
        }
        (index, names)
    }

    #[test]
    fn skips_games_without_a_record() {
        let (index, names) = database(&[|_| {}, |entry| entry.set_length(0), |_| {}]);
        let found = search(&index, &names, &HeaderSearch::default()).unwrap();
        assert_eq!(found, [0, 2]);
    }

    #[test]
    fn reports_a_name_the_name_file_lacks_where_scid_reads_it() {
        let (index, names) = database(&[|_| {}, |entry| entry.set_black(7)]);
        assert_eq!(
            search(&index, &names, &HeaderSearch::default()).unwrap(),
            [0, 1]
        );

        let by_black = HeaderSearch {
            black: Some("Black".into()),
            ..HeaderSearch::default()
        };
        assert!(matches!(
            search(&index, &names, &by_black),
            Err(Error::NameNotFound {
                name_type: NameType::Player,
                id: 7
            })
        ));

        // A game that fails an earlier criterion isn't read further.
        let by_black_and_result = HeaderSearch {
            results: vec![GameResult::White],
            ..by_black
        };
        assert_eq!(
            search(&index, &names, &by_black_and_result).unwrap(),
            Vec::<u32>::new()
        );
    }

    #[test]
    fn takes_an_unrated_players_highest_rating() {
        let (index, names) = database(&[
            |entry| entry.set_white_elo(2500),
            |entry| entry.set_black_elo(2600),
            |_| {},
        ]);
        let rated = HeaderSearch {
            white_elo: 2500..=2500,
            black_elo: 2600..=2600,
            ..HeaderSearch::default()
        };
        assert_eq!(search(&index, &names, &rated).unwrap(), [0, 1, 2]);
    }
}
