use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// An Advent of Code event year.
///
/// Only the lower bound is checked here. Whether a year has started yet
/// depends on the clock, which this crate does not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct Year(u16);

impl Year {
    pub const FIRST: Self = Self(2015);

    const FIRST_SHORT_EVENT: u16 = 2025;

    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }

    /// The number of puzzle days in this event: 25 until 2024, 12 since.
    #[must_use]
    pub const fn day_count(self) -> u8 {
        if self.0 >= Self::FIRST_SHORT_EVENT {
            12
        } else {
            25
        }
    }
}

impl TryFrom<u16> for Year {
    type Error = InvalidYear;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if value >= Self::FIRST.0 {
            Ok(Self(value))
        } else {
            Err(InvalidYear(value))
        }
    }
}

impl From<Year> for u16 {
    fn from(value: Year) -> Self {
        value.get()
    }
}

impl FromStr for Year {
    type Err = ParseYearError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::try_from(s.parse::<u16>()?)?)
    }
}

impl fmt::Display for Year {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("year {0} is before the first event in {first}", first = Year::FIRST)]
pub struct InvalidYear(u16);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseYearError {
    #[error("year is not a number")]
    NotANumber(#[from] ParseIntError),
    #[error(transparent)]
    Invalid(#[from] InvalidYear),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_first_event() {
        assert_eq!(Year::try_from(2015), Ok(Year::FIRST));
    }

    #[test]
    fn rejects_years_before_the_first_event() {
        assert_eq!(Year::try_from(2014), Err(InvalidYear(2014)));
    }

    #[test]
    fn events_are_shorter_since_2025() {
        assert_eq!(Year(2024).day_count(), 25);
        assert_eq!(Year(2025).day_count(), 12);
    }

    #[test]
    fn parses_from_text() {
        assert_eq!("2026".parse(), Ok(Year(2026)));
        assert!(matches!(
            "soon".parse::<Year>(),
            Err(ParseYearError::NotANumber(_))
        ));
        assert!(matches!(
            "1999".parse::<Year>(),
            Err(ParseYearError::Invalid(_))
        ));
    }
}
