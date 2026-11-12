//! Values shared by the index, game, and PGN modules.

/// The result of a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameResult {
    /// White won: `1-0`.
    White,
    /// Black won: `0-1`.
    Black,
    /// A draw: `1/2-1/2`.
    Draw,
    /// No result, such as a game in progress: `*`.
    None,
}

impl GameResult {
    /// Returns the result as an index entry stores it: 0 for none, 1 for
    /// White, 2 for Black, and 3 for a draw.
    pub(crate) fn to_byte(self) -> u8 {
        match self {
            GameResult::None => 0,
            GameResult::White => 1,
            GameResult::Black => 2,
            GameResult::Draw => 3,
        }
    }
}

/// Formats the result as PGN writes it, such as `1-0`.
impl std::fmt::Display for GameResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            GameResult::White => "1-0",
            GameResult::Black => "0-1",
            GameResult::Draw => "1/2-1/2",
            GameResult::None => "*",
        };
        write!(f, "{}", s)
    }
}

/// The kind of a player's rating, as SCID's `ratingTypeNames` names it.
///
/// A PGN tag gives a rating of each kind as `White` or `Black` followed by
/// its [name](Self::name), such as `WhiteUSCF`. An index entry stores the
/// kind in the top 4 bits of each rating.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum RatingType {
    /// An Elo rating, such as FIDE's: `WhiteElo`.
    #[default]
    Elo,
    /// A rating of no named kind: `WhiteRating`.
    Rating,
    /// A rapid rating: `WhiteRapid`.
    Rapid,
    /// An ICCF correspondence rating: `WhiteICCF`.
    Iccf,
    /// A US Chess rating: `WhiteUSCF`.
    Uscf,
    /// A German DWZ rating: `WhiteDWZ`.
    Dwz,
    /// An English ECF grade: `WhiteECF`.
    Ecf,
}

impl RatingType {
    const ALL: [RatingType; 7] = [
        RatingType::Elo,
        RatingType::Rating,
        RatingType::Rapid,
        RatingType::Iccf,
        RatingType::Uscf,
        RatingType::Dwz,
        RatingType::Ecf,
    ];

    /// Reads the kind as an index entry stores it, from 0 for Elo to 6 for
    /// ECF. SCID names no kind above 6, so one is read as Elo.
    pub(crate) fn from_byte(b: u8) -> Self {
        Self::ALL
            .get(usize::from(b))
            .copied()
            .unwrap_or(RatingType::Elo)
    }

    /// Returns the kind as an index entry stores it, from 0 for Elo to 6
    /// for ECF.
    pub(crate) fn to_byte(self) -> u8 {
        self as u8
    }

    /// The kind's name, which follows `White` or `Black` in its PGN tag,
    /// such as `USCF`.
    pub fn name(self) -> &'static str {
        match self {
            RatingType::Elo => "Elo",
            RatingType::Rating => "Rating",
            RatingType::Rapid => "Rapid",
            RatingType::Iccf => "ICCF",
            RatingType::Uscf => "USCF",
            RatingType::Dwz => "DWZ",
            RatingType::Ecf => "ECF",
        }
    }

    /// The kind with this exact [name](Self::name), if any. Case matters, as
    /// in SCID, so `Uscf` names none.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.name() == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rating_type() {
        for (b, kind) in RatingType::ALL.into_iter().enumerate() {
            assert_eq!(RatingType::from_byte(b as u8), kind);
            assert_eq!(kind.to_byte(), b as u8);
            assert_eq!(RatingType::from_name(kind.name()), Some(kind));
        }
        assert_eq!(RatingType::from_byte(7), RatingType::Elo);
        assert_eq!(RatingType::from_byte(15), RatingType::Elo);
        assert_eq!(RatingType::from_name("Uscf"), None);
        assert_eq!(RatingType::from_name("FIDE"), None);
        assert_eq!(RatingType::default(), RatingType::Elo);
    }

    #[test]
    fn test_game_result() {
        assert_eq!(GameResult::White.to_string(), "1-0");
        assert_eq!(GameResult::Black.to_string(), "0-1");
        assert_eq!(GameResult::Draw.to_string(), "1/2-1/2");
        assert_eq!(GameResult::None.to_string(), "*");
    }

    #[test]
    fn test_game_result_bytes() {
        for (result, byte) in [
            (GameResult::None, 0),
            (GameResult::White, 1),
            (GameResult::Black, 2),
            (GameResult::Draw, 3),
        ] {
            assert_eq!(result.to_byte(), byte);
        }
    }
}
