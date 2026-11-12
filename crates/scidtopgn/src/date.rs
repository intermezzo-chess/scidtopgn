//! [`Date`], SCID's packed date, and functions on packed dates.

use crate::error::{Error, Result};

const YEAR_SHIFT: u32 = 9;
const MONTH_SHIFT: u32 = 5;
/// The latest year a [`Date`] can hold.
pub const YEAR_MAX: u16 = 2047;

/// A date as SCID stores it: a year, month, and day packed into one number.
/// A part that is 0 is unknown, as `?` is in PGN.
///
/// Dates compare in calendar order, with unknown parts first.
///
/// # Examples
///
/// ```
/// use scidtopgn::Date;
///
/// let date = Date::from_string("2022.12.??");
/// assert_eq!((date.year(), date.month(), date.day()), (2022, 12, 0));
/// assert_eq!(date.to_string(), "2022.12.??");
/// assert_eq!(Date::new(2022, 12, 0)?, date);
/// # Ok::<(), scidtopgn::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Date(u32);

impl Date {
    /// The date with every part unknown, `????.??.??`.
    pub const ZERO: Date = Date(0);

    /// Makes a date. Use 0 for a part that is unknown.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Parse`] if `year` is after [`YEAR_MAX`], `month` is
    /// after 12, or `day` is after 31.
    pub fn new(year: u16, month: u8, day: u8) -> Result<Self> {
        if year > YEAR_MAX {
            return Err(Error::Parse {
                line: 0,
                message: format!("Year {} exceeds maximum {}", year, YEAR_MAX),
            });
        }
        if month > 12 {
            return Err(Error::Parse {
                line: 0,
                message: format!("Invalid month {}", month),
            });
        }
        if day > 31 {
            return Err(Error::Parse {
                line: 0,
                message: format!("Invalid day {}", day),
            });
        }
        let value = ((year as u32) << YEAR_SHIFT) | ((month as u32) << MONTH_SHIFT) | (day as u32);
        Ok(Date(value))
    }

    /// Makes a date from its packed form, as [`raw`](Self::raw) returns it.
    pub(crate) fn from_raw(value: u32) -> Self {
        Date(value)
    }

    /// Returns the packed form SCID stores.
    pub(crate) fn raw(self) -> u32 {
        self.0
    }

    /// Returns the year, or 0 if it is unknown.
    pub fn year(self) -> u16 {
        (self.0 >> YEAR_SHIFT) as u16
    }

    /// Returns the month, from 1 to 12, or 0 if it is unknown.
    pub fn month(self) -> u8 {
        ((self.0 >> MONTH_SHIFT) & 15) as u8
    }

    /// Returns the day of the month, or 0 if it is unknown.
    pub fn day(self) -> u8 {
        (self.0 & 31) as u8
    }

    /// Reads a PGN date such as `2022.12.19`. A part that is missing, isn't
    /// a number, or is out of range is unknown.
    pub fn from_string(s: &str) -> Self {
        let parts: Vec<&str> = s.split('.').collect();
        let year = parts
            .first()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(0);
        let month = parts.get(1).and_then(|p| p.parse::<u8>().ok()).unwrap_or(0);
        let day = parts.get(2).and_then(|p| p.parse::<u8>().ok()).unwrap_or(0);

        let year = if year > YEAR_MAX { 0 } else { year };
        let month = if month > 12 { 0 } else { month };
        let day = if day > 31 { 0 } else { day };

        let value = ((year as u32) << YEAR_SHIFT) | ((month as u32) << MONTH_SHIFT) | (day as u32);
        Date(value)
    }

    /// Returns whether any part of the date is known.
    pub fn is_valid(self) -> bool {
        self.0 != 0
    }
}

/// Formats the date as PGN writes it, with `?` for unknown parts, such as
/// `2022.12.??`.
impl std::fmt::Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let year = self.year();
        let month = self.month();
        let day = self.day();

        if year == 0 && month == 0 && day == 0 {
            write!(f, "????.??.??")
        } else if year == 0 {
            write!(f, "????.{:02}.{:02}", month, day)
        } else if month == 0 {
            write!(f, "{:04}.??.??", year)
        } else if day == 0 {
            write!(f, "{:04}.{:02}.??", year, month)
        } else {
            write!(f, "{:04}.{:02}.{:02}", year, month, day)
        }
    }
}

/// Packs a year, month, and day as SCID stores them, without checking their
/// ranges. [`Date::new`] checks them.
pub(crate) fn date_make(year: u16, month: u8, day: u8) -> u32 {
    ((year as u32) << YEAR_SHIFT) | ((month as u32) << MONTH_SHIFT) | (day as u32)
}

/// Returns the year of a packed date. See [`Date::year`].
pub(crate) fn date_get_year(date: u32) -> u16 {
    (date >> YEAR_SHIFT) as u16
}

/// Returns the month of a packed date. See [`Date::month`].
pub(crate) fn date_get_month(date: u32) -> u8 {
    ((date >> MONTH_SHIFT) & 15) as u8
}

/// Returns the day of a packed date. See [`Date::day`].
pub(crate) fn date_get_day(date: u32) -> u8 {
    (date & 31) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_new() {
        let date = Date::new(2022, 12, 19).unwrap();
        assert_eq!(date.year(), 2022);
        assert_eq!(date.month(), 12);
        assert_eq!(date.day(), 19);
    }

    #[test]
    fn test_date_encoding() {
        let date = Date::new(2022, 12, 19).unwrap();
        let expected = ((2022u32) << 9) | (12u32 << 5) | 19u32;
        assert_eq!(date.raw(), expected);
    }

    #[test]
    fn test_date_from_string() {
        let date = Date::from_string("2022.12.19");
        assert_eq!(date.year(), 2022);
        assert_eq!(date.month(), 12);
        assert_eq!(date.day(), 19);
    }

    #[test]
    fn test_date_to_string() {
        let date = Date::new(2022, 12, 19).unwrap();
        assert_eq!(date.to_string(), "2022.12.19");
    }

    #[test]
    fn test_date_unknown_parts() {
        let date = Date::from_string("2022.??.??");
        assert_eq!(date.year(), 2022);
        assert_eq!(date.month(), 0);
        assert_eq!(date.day(), 0);
        assert_eq!(date.to_string(), "2022.??.??");
    }

    #[test]
    fn test_date_zero() {
        let date = Date::ZERO;
        assert_eq!(date.to_string(), "????.??.??");
    }

    #[test]
    fn test_date_new_rejects_out_of_range_parts() {
        for (year, month, day, message) in [
            (2048, 1, 1, "Year 2048 exceeds maximum 2047"),
            (2022, 13, 1, "Invalid month 13"),
            (2022, 1, 32, "Invalid day 32"),
        ] {
            match Date::new(year, month, day) {
                Err(Error::Parse {
                    line: 0,
                    message: m,
                }) => assert_eq!(m, message),
                other => panic!("expected a parse error, got {other:?}"),
            }
        }
        assert!(Date::new(YEAR_MAX, 12, 31).is_ok());
    }

    #[test]
    fn test_date_parts() {
        let date = Date::new(2022, 12, 19).unwrap();
        assert!(date.is_valid());
        assert!(!Date::ZERO.is_valid());
        assert_eq!(date_get_year(date.raw()), 2022);
        assert_eq!(date_get_month(date.raw()), 12);
        assert_eq!(date_get_day(date.raw()), 19);
        assert_eq!(date_make(2022, 12, 19), date.raw());
    }

    #[test]
    fn test_date_strings_with_unknown_parts() {
        for (text, year, month, day) in [
            ("????.12.19", 0, 12, 19),
            ("2022.12.??", 2022, 12, 0),
            ("2022.??.??", 2022, 0, 0),
            ("????.??.??", 0, 0, 0),
        ] {
            let date = Date::from_string(text);
            assert_eq!(date.raw(), date_make(year, month, day), "{text}");
            assert_eq!(date.to_string(), text);
        }
        // Parts out of range read as unknown.
        assert_eq!(Date::from_string("2048.13.32"), Date::ZERO);
    }
}
