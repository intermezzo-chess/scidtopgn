use std::ops::RangeInclusive;
use std::str::FromStr;

use clap::{Args, ValueEnum};
use scidtopgn::index::MAX_ELO;
use scidtopgn::shakmaty::Color;
use scidtopgn::{Date, Eco, FlagMatch, GameResult, HeaderSearch, YEAR_MAX};

/// The options that search games by their header, shared by the default
/// invocation and `export`. Each is an option of SCID's header search.
#[derive(Args, Clone, Debug, Default)]
#[command(next_help_heading = "Search (a game must match every option given)")]
pub struct SearchArgs {
    /// Only games whose White contains this text, ignoring case and spaces.
    /// In double quotes, such as '"Carlsen*"', the whole name must match it
    /// as a pattern, with case: * matches any characters, ? one character,
    /// [a-z] one of a set, and \ makes the next character literal
    #[arg(long, value_name = "TEXT")]
    pub white: Option<String>,

    /// Only games whose Black matches this text, as for --white
    #[arg(long, value_name = "TEXT")]
    pub black: Option<String>,

    /// Only games whose event matches this text, as for --white
    #[arg(long, value_name = "TEXT")]
    pub event: Option<String>,

    /// Only games whose site matches this text, as for --white
    #[arg(long, value_name = "TEXT")]
    pub site: Option<String>,

    /// Only games whose round matches this text, as for --white
    #[arg(long, value_name = "TEXT")]
    pub round: Option<String>,

    /// Only games dated in this range, such as 2020..2023.06, or ..1999 for
    /// games up to 1999. Dates are YYYY, YYYY.MM, or YYYY.MM.DD. A shorter
    /// end date means the end of that year or month. Unknown parts of a
    /// game's date come first, so 2023.??.?? is before 2023.01.01 and after
    /// 2023
    #[arg(long, value_name = "FROM..TO", value_parser = parse_dates)]
    pub date: Option<RangeInclusive<Date>>,

    /// Only games with one of these results, separated by commas: 1-0,
    /// 1/2-1/2, 0-1, and * for none
    #[arg(long, value_name = "LIST", value_parser = parse_results)]
    pub result: Option<Results>,

    /// Only games in which White is rated in this range, such as 2500.. or
    /// 2000..2200. An unrated player counts with their highest rating in the
    /// database, or 0
    #[arg(long, value_name = "MIN..MAX", value_parser = parse_elo)]
    pub white_elo: Option<RangeInclusive<u16>>,

    /// Only games in which Black is rated in this range, as for --white-elo
    #[arg(long, value_name = "MIN..MAX", value_parser = parse_elo)]
    pub black_elo: Option<RangeInclusive<u16>>,

    /// Only games in which White's rating minus Black's is in this range,
    /// such as -100..100 or 200..
    #[arg(long, value_name = "MIN..MAX", value_parser = parse_elo_diff, allow_hyphen_values = true)]
    pub elo_diff: Option<RangeInclusive<i32>>,

    /// Only games with an ECO code in this range, such as B20..B99 or C42.
    /// The end includes its extensions: B07 means up to B07z4. Games with no
    /// ECO code also match, unless --eco-missing exclude is given
    #[arg(long, value_name = "FROM..TO", value_parser = parse_eco)]
    pub eco: Option<RangeInclusive<Eco>>,

    /// Whether games with no ECO code match
    #[arg(long, value_name = "WHICH", value_enum)]
    pub eco_missing: Option<EcoMissing>,

    /// Only games with this many half-moves in the main line, such as 40..
    /// or ..20. Unless given, games longer than 999 half-moves don't match
    /// any search, as in SCID
    #[arg(long, value_name = "MIN..MAX", value_parser = parse_length)]
    pub length: Option<RangeInclusive<u16>>,

    /// Only games that end with this side to move, counting from White's
    /// first move, as SCID does
    #[arg(long, value_name = "SIDE", value_enum)]
    pub to_move: Option<Side>,

    /// Also match games with White and Black swapped: their names, ratings,
    /// and wins, the side to move, and the sign of the rating difference
    #[arg(long)]
    pub ignore_colors: bool,

    /// Only games with, or without, a flag, such as comments=yes or
    /// deleted=no. Can be given more than once. Flags: non-standard-start,
    /// promotions, underpromotions, comments, variations, annotations,
    /// deleted, white-opening, black-opening, middlegame, endgame, novelty,
    /// pawn-structure, tactics, kingside, queenside, brilliancy, blunder,
    /// user, and custom1 to custom6
    #[arg(long, value_name = "FLAG=yes|no", value_parser = parse_flag)]
    pub flag: Vec<(Flag, FlagMatch)>,
}

impl SearchArgs {
    /// Returns the search the options give, or `None` if none is given.
    pub fn to_search(&self) -> anyhow::Result<Option<HeaderSearch>> {
        let SearchArgs {
            white,
            black,
            event,
            site,
            round,
            date,
            result,
            white_elo,
            black_elo,
            elo_diff,
            eco,
            eco_missing,
            length,
            to_move,
            ignore_colors,
            flag,
        } = self.clone();
        let given = white.is_some()
            || black.is_some()
            || event.is_some()
            || site.is_some()
            || round.is_some()
            || date.is_some()
            || result.is_some()
            || white_elo.is_some()
            || black_elo.is_some()
            || elo_diff.is_some()
            || eco.is_some()
            || eco_missing.is_some()
            || length.is_some()
            || to_move.is_some()
            || ignore_colors
            || !flag.is_empty();
        if !given {
            return Ok(None);
        }

        let default = HeaderSearch::default();
        let mut search = HeaderSearch {
            white,
            black,
            event,
            site,
            round,
            dates: date.unwrap_or(default.dates),
            results: result.map_or(default.results, |Results(results)| results),
            white_elo: white_elo.unwrap_or(default.white_elo),
            black_elo: black_elo.unwrap_or(default.black_elo),
            elo_diff: elo_diff.unwrap_or(default.elo_diff),
            eco: eco.unwrap_or(default.eco),
            with_no_eco: eco_missing.is_none_or(|missing| missing == EcoMissing::Include),
            length: length.unwrap_or(default.length),
            to_move: to_move.map(Color::from),
            game_numbers: default.game_numbers,
            ignore_colors,
            flags: default.flags,
        };
        let mut seen = Vec::new();
        for (name, value) in flag {
            if seen.contains(&name) {
                anyhow::bail!("--flag {} is given more than once", name.name());
            }
            seen.push(name);
            *name.field(&mut search.flags) = value;
        }
        Ok(Some(search))
    }
}

/// The results `--result` lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Results(Vec<GameResult>);

/// Whether `--eco` also matches games with no ECO code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum EcoMissing {
    /// Games with no ECO code match, as in SCID
    Include,
    /// Games with no ECO code don't match
    Exclude,
}

/// The side to move at the end of a game.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Side {
    White,
    Black,
}

impl From<Side> for Color {
    fn from(side: Side) -> Self {
        match side {
            Side::White => Color::White,
            Side::Black => Color::Black,
        }
    }
}

/// A flag `--flag` names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flag {
    NonStandardStart,
    Promotions,
    Underpromotions,
    Comments,
    Variations,
    Annotations,
    Deleted,
    WhiteOpening,
    BlackOpening,
    Middlegame,
    Endgame,
    Novelty,
    PawnStructure,
    Tactics,
    Kingside,
    Queenside,
    Brilliancy,
    Blunder,
    User,
    Custom(usize),
}

impl Flag {
    const NAMED: [(&'static str, Flag); 19] = [
        ("non-standard-start", Flag::NonStandardStart),
        ("promotions", Flag::Promotions),
        ("underpromotions", Flag::Underpromotions),
        ("comments", Flag::Comments),
        ("variations", Flag::Variations),
        ("annotations", Flag::Annotations),
        ("deleted", Flag::Deleted),
        ("white-opening", Flag::WhiteOpening),
        ("black-opening", Flag::BlackOpening),
        ("middlegame", Flag::Middlegame),
        ("endgame", Flag::Endgame),
        ("novelty", Flag::Novelty),
        ("pawn-structure", Flag::PawnStructure),
        ("tactics", Flag::Tactics),
        ("kingside", Flag::Kingside),
        ("queenside", Flag::Queenside),
        ("brilliancy", Flag::Brilliancy),
        ("blunder", Flag::Blunder),
        ("user", Flag::User),
    ];

    fn name(self) -> String {
        match self {
            Flag::Custom(n) => format!("custom{n}"),
            flag => Flag::NAMED
                .iter()
                .find(|(_, named)| *named == flag)
                .map(|(name, _)| (*name).to_owned())
                .expect("every other flag is named"),
        }
    }

    fn field(self, flags: &mut scidtopgn::HeaderFlags) -> &mut FlagMatch {
        match self {
            Flag::NonStandardStart => &mut flags.non_standard_start,
            Flag::Promotions => &mut flags.promotions,
            Flag::Underpromotions => &mut flags.underpromotions,
            Flag::Comments => &mut flags.comments,
            Flag::Variations => &mut flags.variations,
            Flag::Annotations => &mut flags.annotations,
            Flag::Deleted => &mut flags.deleted,
            Flag::WhiteOpening => &mut flags.white_opening,
            Flag::BlackOpening => &mut flags.black_opening,
            Flag::Middlegame => &mut flags.middlegame,
            Flag::Endgame => &mut flags.endgame,
            Flag::Novelty => &mut flags.novelty,
            Flag::PawnStructure => &mut flags.pawn_structure,
            Flag::Tactics => &mut flags.tactics,
            Flag::Kingside => &mut flags.kingside,
            Flag::Queenside => &mut flags.queenside,
            Flag::Brilliancy => &mut flags.brilliancy,
            Flag::Blunder => &mut flags.blunder,
            Flag::User => &mut flags.user,
            Flag::Custom(n) => &mut flags.custom[n - 1],
        }
    }
}

impl FromStr for Flag {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((_, flag)) = Flag::NAMED.iter().find(|(name, _)| *name == s) {
            return Ok(*flag);
        }
        match s.strip_prefix("custom").and_then(|n| n.parse().ok()) {
            Some(n @ 1..=6) => Ok(Flag::Custom(n)),
            _ => Err(format!("{s:?} isn't a flag")),
        }
    }
}

fn parse_flag(s: &str) -> Result<(Flag, FlagMatch), String> {
    let (name, value) = s
        .split_once('=')
        .ok_or_else(|| format!("{s:?} isn't FLAG=yes or FLAG=no"))?;
    let value = match value {
        "yes" => FlagMatch::Yes,
        "no" => FlagMatch::No,
        _ => return Err(format!("{value:?} isn't yes or no")),
    };
    Ok((name.parse()?, value))
}

/// Splits `s` into the ends of a range, `FROM..TO`, where either may be
/// left out, or a single value, which is both ends.
fn range_ends<'a>(s: &'a str) -> Result<(Option<&'a str>, Option<&'a str>), String> {
    let (from, to) = s.split_once("..").unwrap_or((s, s));
    if from.is_empty() && to.is_empty() {
        return Err(format!("{s:?} gives neither end of the range"));
    }
    let nonempty = |end: &'a str| (!end.is_empty()).then_some(end);
    Ok((nonempty(from), nonempty(to)))
}

/// Parses a range of `T`, filling a missing end from `default`.
fn parse_range<T: Copy>(
    s: &str,
    default: RangeInclusive<T>,
    parse: impl Fn(&str) -> Result<T, String>,
) -> Result<RangeInclusive<T>, String> {
    let (from, to) = range_ends(s)?;
    let from = from.map_or(Ok(*default.start()), &parse)?;
    let to = to.map_or(Ok(*default.end()), &parse)?;
    Ok(from..=to)
}

fn number<T: FromStr>(s: &str) -> Result<T, String> {
    let digits = s.strip_prefix('-').unwrap_or(s);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("{s:?} isn't a number"));
    }
    s.parse().map_err(|_| format!("{s:?} is out of range"))
}

fn parse_elo(s: &str) -> Result<RangeInclusive<u16>, String> {
    parse_range(s, HeaderSearch::default().white_elo, |end| {
        let elo: u16 = number(end)?;
        if elo > MAX_ELO {
            return Err(format!("{elo} is above the highest rating, {MAX_ELO}"));
        }
        Ok(elo)
    })
}

fn parse_elo_diff(s: &str) -> Result<RangeInclusive<i32>, String> {
    parse_range(s, HeaderSearch::default().elo_diff, number)
}

fn parse_length(s: &str) -> Result<RangeInclusive<u16>, String> {
    parse_range(s, HeaderSearch::default().length, number)
}

fn parse_eco(s: &str) -> Result<RangeInclusive<Eco>, String> {
    parse_range(s, HeaderSearch::default().eco, |end| {
        end.parse()
            .map_err(|_| format!("{end:?} isn't an ECO code such as B20 or B20a1"))
    })
}

/// Parses `YYYY`, `YYYY.MM`, or `YYYY.MM.DD`. The parts left out are 0, or,
/// with `end`, the last month and day.
fn parse_date(s: &str, end: bool) -> Result<Date, String> {
    let invalid = || format!("{s:?} isn't a date such as 2023, 2023.06, or 2023.06.15");
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() > 3
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(invalid());
    }
    let year: u16 = parts[0].parse().map_err(|_| invalid())?;
    let part = |i: usize, last: u8| match parts.get(i) {
        Some(part) => part.parse::<u8>().map_err(|_| invalid()),
        None => Ok(if end { last } else { 0 }),
    };
    let (month, day) = (part(1, 12)?, part(2, 31)?);
    if year > YEAR_MAX || month > 12 || day > 31 {
        return Err(invalid());
    }
    Date::new(year, month, day).map_err(|_| invalid())
}

fn parse_dates(s: &str) -> Result<RangeInclusive<Date>, String> {
    let default = HeaderSearch::default().dates;
    let (from, to) = range_ends(s)?;
    let from = from.map_or(Ok(*default.start()), |from| parse_date(from, false))?;
    let to = to.map_or(Ok(*default.end()), |to| parse_date(to, true))?;
    Ok(from..=to)
}

fn parse_results(s: &str) -> Result<Results, String> {
    s.split(',')
        .map(|result| match result {
            "1-0" => Ok(GameResult::White),
            "1/2-1/2" => Ok(GameResult::Draw),
            "0-1" => Ok(GameResult::Black),
            "*" => Ok(GameResult::None),
            _ => Err(format!("{result:?} isn't 1-0, 1/2-1/2, 0-1, or *")),
        })
        .collect::<Result<_, _>>()
        .map(Results)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: u16, month: u8, day: u8) -> Date {
        Date::new(year, month, day).unwrap()
    }

    fn eco(code: &str) -> Eco {
        code.parse().unwrap()
    }

    #[test]
    fn parses_ranges_with_either_end_left_out() {
        assert_eq!(parse_elo("2000..2200"), Ok(2000..=2200));
        assert_eq!(parse_elo("2500.."), Ok(2500..=MAX_ELO));
        assert_eq!(parse_elo("..1800"), Ok(0..=1800));
        assert_eq!(parse_elo("2400"), Ok(2400..=2400));
        assert_eq!(parse_elo_diff("-100..100"), Ok(-100..=100));
        assert_eq!(parse_elo_diff("..-50"), Ok(-4000..=-50));
        assert_eq!(parse_length("40.."), Ok(40..=999));
        assert_eq!(parse_length("1000..1023"), Ok(1000..=1023));
        assert_eq!(parse_eco("B20..B99"), Ok(eco("B20")..=eco("B99")));
        assert_eq!(parse_eco("C42"), Ok(eco("C42")..=eco("C42")));
        assert_eq!(parse_eco("..A09"), Ok(Eco::A00..=eco("A09")));
        assert_eq!(parse_eco("b07a1.."), Ok(eco("B07a1")..=Eco::E99));
    }

    #[test]
    fn rejects_bad_ranges() {
        for (input, message) in [
            ("..", "\"..\" gives neither end of the range"),
            ("", "\"\" gives neither end of the range"),
            ("x..1", "\"x\" isn't a number"),
            ("+5", "\"+5\" isn't a number"),
            ("1..2..3", "\"2..3\" isn't a number"),
            ("4001", "4001 is above the highest rating, 4000"),
            ("-1", "\"-1\" is out of range"),
            ("70000", "\"70000\" is out of range"),
        ] {
            assert_eq!(parse_elo(input), Err(message.to_owned()), "{input:?}");
        }
        assert!(parse_eco("B2..B99").is_err());
        assert!(parse_eco("F00").is_err());
        assert!(parse_eco("B20aa").is_err());
    }

    #[test]
    fn parses_dates_with_a_shorter_end_at_its_end() {
        assert_eq!(
            parse_dates("2020..2023"),
            Ok(date(2020, 0, 0)..=date(2023, 12, 31))
        );
        assert_eq!(
            parse_dates("2023.06..2023.06"),
            Ok(date(2023, 6, 0)..=date(2023, 6, 31))
        );
        assert_eq!(
            parse_dates("2023.06.15"),
            Ok(date(2023, 6, 15)..=date(2023, 6, 15))
        );
        assert_eq!(
            parse_dates("2020.05.01.."),
            Ok(date(2020, 5, 1)..=date(YEAR_MAX, 12, 31))
        );
        assert_eq!(parse_dates("..1999"), Ok(Date::ZERO..=date(1999, 12, 31)));
        for input in [
            "2023.6.x",
            "2023/06/15",
            "2048",
            "2023.13",
            "2023.01.32",
            "2023..",
        ] {
            let parsed = parse_dates(input);
            assert_eq!(parsed.is_err(), input != "2023..", "{input:?}");
        }
    }

    #[test]
    fn parses_results_and_flags() {
        assert_eq!(
            parse_results("1-0,*"),
            Ok(Results(vec![GameResult::White, GameResult::None]))
        );
        assert_eq!(
            parse_results("1/2-1/2,0-1"),
            Ok(Results(vec![GameResult::Draw, GameResult::Black]))
        );
        assert!(parse_results("1-0,").is_err());
        assert!(parse_results("=").is_err());

        assert_eq!(
            parse_flag("comments=yes"),
            Ok((Flag::Comments, FlagMatch::Yes))
        );
        assert_eq!(
            parse_flag("custom6=no"),
            Ok((Flag::Custom(6), FlagMatch::No))
        );
        for input in [
            "comments",
            "comments=maybe",
            "custom7=yes",
            "custom0=yes",
            "x=yes",
        ] {
            assert!(parse_flag(input).is_err(), "{input:?}");
        }
        for (name, flag) in Flag::NAMED {
            assert_eq!(flag.name(), name);
            assert_eq!(name.parse(), Ok(flag));
        }
    }

    #[test]
    fn builds_a_search_only_from_options_given() {
        assert_eq!(SearchArgs::default().to_search().unwrap(), None);

        let args = SearchArgs {
            white: Some("Carlsen".into()),
            eco: Some(eco("B20")..=eco("B99")),
            eco_missing: Some(EcoMissing::Exclude),
            to_move: Some(Side::Black),
            flag: vec![
                (Flag::Deleted, FlagMatch::No),
                (Flag::Custom(2), FlagMatch::Yes),
            ],
            ..SearchArgs::default()
        };
        let search = args.to_search().unwrap().unwrap();
        let mut expected = HeaderSearch {
            white: Some("Carlsen".into()),
            eco: eco("B20")..=eco("B99"),
            with_no_eco: false,
            to_move: Some(Color::Black),
            ..HeaderSearch::default()
        };
        expected.flags.deleted = FlagMatch::No;
        expected.flags.custom[1] = FlagMatch::Yes;
        assert_eq!(search, expected);

        let ignore_colors = SearchArgs {
            ignore_colors: true,
            ..SearchArgs::default()
        };
        assert!(ignore_colors.to_search().unwrap().is_some());

        let twice = SearchArgs {
            flag: vec![
                (Flag::Comments, FlagMatch::Yes),
                (Flag::Comments, FlagMatch::No),
            ],
            ..SearchArgs::default()
        };
        assert_eq!(
            twice.to_search().unwrap_err().to_string(),
            "--flag comments is given more than once"
        );
    }
}
