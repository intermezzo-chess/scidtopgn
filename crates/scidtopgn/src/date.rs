use crate::error::{Error, Result};

const YEAR_SHIFT: u32 = 9;
const MONTH_SHIFT: u32 = 5;
pub const YEAR_MAX: u16 = 2047;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Date(u32);

impl Date {
    pub const ZERO: Date = Date(0);

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

    pub fn from_raw(value: u32) -> Self {
        Date(value)
    }

    pub fn raw(self) -> u32 {
        self.0
    }

    pub fn year(self) -> u16 {
        (self.0 >> YEAR_SHIFT) as u16
    }

    pub fn month(self) -> u8 {
        ((self.0 >> MONTH_SHIFT) & 15) as u8
    }

    pub fn day(self) -> u8 {
        (self.0 & 31) as u8
    }

    pub fn month_day(self) -> u16 {
        (self.0 & 511) as u16
    }

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

    pub fn is_valid(self) -> bool {
        self.0 != 0
    }

    pub fn add_months(self, num_months: i32) -> Self {
        let mut year = self.year() as i32;
        let mut month = self.month() as i32;
        let day = self.day();

        let mut remaining = num_months;
        while remaining < 0 {
            if month <= 1 {
                year -= 1;
                month = 12;
            } else {
                month -= 1;
            }
            remaining += 1;
        }
        while remaining > 0 {
            month += 1;
            if month > 12 {
                year += 1;
                month = 1;
            }
            remaining -= 1;
        }

        let year = year.max(0) as u16;
        let month = month.max(0) as u8;

        Date::new(year, month, day).unwrap_or(Date::ZERO)
    }
}

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

pub fn date_make(year: u16, month: u8, day: u8) -> u32 {
    ((year as u32) << YEAR_SHIFT) | ((month as u32) << MONTH_SHIFT) | (day as u32)
}

pub fn date_get_year(date: u32) -> u16 {
    (date >> YEAR_SHIFT) as u16
}

pub fn date_get_month(date: u32) -> u8 {
    ((date >> MONTH_SHIFT) & 15) as u8
}

pub fn date_get_day(date: u32) -> u8 {
    (date & 31) as u8
}

pub fn date_decode_to_string(date: u32) -> String {
    Date::from_raw(date).to_string()
}

pub fn date_encode_from_string(s: &str) -> u32 {
    Date::from_string(s).raw()
}

pub fn date_valid_string(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 {
        return false;
    }

    let max_values = [YEAR_MAX as u32, 12, 31];

    for (i, part) in parts.iter().enumerate() {
        let max_val = max_values[i];
        let mut seen_question = false;
        let mut seen_digit = false;
        let mut seen_other = false;

        for ch in part.chars() {
            if ch.is_ascii_digit() {
                seen_digit = true;
            } else if ch == '?' {
                seen_question = true;
            } else {
                seen_other = true;
            }
        }

        if seen_other {
            return false;
        }
        if seen_question && seen_digit {
            return false;
        }
        if seen_digit {
            if let Ok(value) = part.parse::<u32>() {
                if value > max_val {
                    return false;
                }
            }
        }
    }

    true
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
    fn test_date_add_months() {
        let date = Date::new(2022, 6, 15).unwrap();
        let new_date = date.add_months(3);
        assert_eq!(new_date.month(), 9);

        let new_date = date.add_months(8);
        assert_eq!(new_date.year(), 2023);
        assert_eq!(new_date.month(), 2);
    }

    #[test]
    fn test_date_valid_string() {
        assert!(date_valid_string("2022.12.19"));
        assert!(date_valid_string("2022.??.??"));
        assert!(date_valid_string("????.12.??"));
        assert!(!date_valid_string("2022.13.01"));
        assert!(!date_valid_string("2022.12.32"));
        assert!(!date_valid_string("2022.12")); // Missing day part
    }
}
