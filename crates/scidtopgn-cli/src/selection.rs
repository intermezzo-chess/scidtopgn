use std::ops::RangeInclusive;
use std::str::FromStr;

/// The games chosen with `--games`, numbered from 1, as sorted ranges that
/// don't overlap or touch. There is always at least one range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameSelection(Vec<RangeInclusive<u32>>);

impl GameSelection {
    /// Returns the highest game number selected.
    pub fn last(&self) -> u32 {
        self.0.last().map_or(0, |range| *range.end())
    }

    /// Returns the number of games selected.
    pub fn count(&self) -> u32 {
        self.0
            .iter()
            .map(|range| range.end() - range.start() + 1)
            .sum()
    }

    /// Returns whether the game with index `index`, counting from 0, is
    /// selected.
    pub fn contains(&self, index: u32) -> bool {
        let number = u64::from(index) + 1;
        self.0
            .iter()
            .any(|range| (u64::from(*range.start())..=u64::from(*range.end())).contains(&number))
    }

    /// Returns the selected games' indexes, counting from 0, in order.
    pub fn indexes(&self) -> impl Iterator<Item = u32> + '_ {
        self.0
            .iter()
            .flat_map(|range| range.start() - 1..=range.end() - 1)
    }
}

impl FromStr for GameSelection {
    type Err = String;

    /// Parses a list such as `1-5,8,12-14`. Ranges include both ends, and
    /// may overlap or come in any order.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut ranges = Vec::new();
        for item in s.split(',') {
            let range = match item.split_once('-') {
                Some((first, last)) => game_number(first)?..=game_number(last)?,
                None => game_number(item)?..=game_number(item)?,
            };
            if range.is_empty() {
                return Err(format!("the range {item} ends before it starts"));
            }
            ranges.push(range);
        }

        ranges.sort_by_key(|range| *range.start());
        let mut merged: Vec<RangeInclusive<u32>> = Vec::new();
        for range in ranges {
            match merged.last_mut() {
                Some(prev) if *range.start() <= prev.end().saturating_add(1) => {
                    if range.end() > prev.end() {
                        *prev = *prev.start()..=*range.end();
                    }
                }
                _ => merged.push(range),
            }
        }
        Ok(Self(merged))
    }
}

fn game_number(s: &str) -> Result<u32, String> {
    let not_a_number = || format!("{s:?} isn't a game number");
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(not_a_number());
    }
    match s.parse() {
        Ok(0) => Err("games are numbered from 1".to_string()),
        Ok(n) => Ok(n),
        Err(_) => Err(not_a_number()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn indexes(s: &str) -> Vec<u32> {
        s.parse::<GameSelection>().unwrap().indexes().collect()
    }

    #[test]
    fn parses_numbers_and_ranges() {
        assert_eq!(indexes("3"), [2]);
        assert_eq!(indexes("10-12"), [9, 10, 11]);
        assert_eq!(indexes("1-2,5,7-8"), [0, 1, 4, 6, 7]);
        assert_eq!(indexes("4-4"), [3]);
    }

    #[test]
    fn selects_each_game_once_in_order() {
        let selection: GameSelection = "8,1-3,2-5,6,3".parse().unwrap();
        assert_eq!(selection, GameSelection(vec![1..=6, 8..=8]));
        assert_eq!(selection.count(), 7);
        assert_eq!(selection.last(), 8);
        assert!(selection.contains(0) && selection.contains(5) && selection.contains(7));
        assert!(!selection.contains(6) && !selection.contains(8));
        assert_eq!(indexes("5,4"), [3, 4]);
    }

    #[test]
    fn handles_the_largest_game_number() {
        let selection: GameSelection = "4294967295,1".parse().unwrap();
        assert_eq!(selection.count(), 2);
        assert_eq!(selection.last(), u32::MAX);
    }

    #[test]
    fn rejects_bad_lists() {
        for (input, message) in [
            ("", "\"\" isn't a game number"),
            ("1,,2", "\"\" isn't a game number"),
            ("1,", "\"\" isn't a game number"),
            ("x", "\"x\" isn't a game number"),
            ("+5", "\"+5\" isn't a game number"),
            (" 5", "\" 5\" isn't a game number"),
            ("5-", "\"\" isn't a game number"),
            ("-5", "\"\" isn't a game number"),
            ("1-2-3", "\"2-3\" isn't a game number"),
            ("4294967296", "\"4294967296\" isn't a game number"),
            ("0", "games are numbered from 1"),
            ("0-3", "games are numbered from 1"),
            ("5-3", "the range 5-3 ends before it starts"),
        ] {
            assert_eq!(
                input.parse::<GameSelection>(),
                Err(message.to_string()),
                "{input:?}"
            );
        }
    }
}
