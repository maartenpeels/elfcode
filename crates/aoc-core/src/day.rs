use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

use thiserror::Error;

/// A puzzle day, 1 through 25.
///
/// Whether the day exists in a particular event is checked by
/// [`PuzzleId`](crate::PuzzleId), because that depends on the year.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Day(u8);

impl Day {
    pub const FIRST: Self = Self(1);
    pub const LAST: Self = Self(25);

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for Day {
    type Error = InvalidDay;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if (Self::FIRST.0..=Self::LAST.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(InvalidDay(value))
        }
    }
}

impl FromStr for Day {
    type Err = ParseDayError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::try_from(s.parse::<u8>()?)?)
    }
}

/// Forwards to the number, so `{:02}` gives the zero-padded form used in
/// file names.
impl fmt::Display for Day {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("day {0} is not between {first} and {last}", first = Day::FIRST, last = Day::LAST)]
pub struct InvalidDay(u8);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseDayError {
    #[error("day is not a number")]
    NotANumber(#[from] ParseIntError),
    #[error(transparent)]
    Invalid(#[from] InvalidDay),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_full_range() {
        assert_eq!(Day::try_from(1), Ok(Day::FIRST));
        assert_eq!(Day::try_from(25), Ok(Day::LAST));
    }

    #[test]
    fn rejects_days_outside_the_range() {
        assert_eq!(Day::try_from(0), Err(InvalidDay(0)));
        assert_eq!(Day::try_from(26), Err(InvalidDay(26)));
    }

    #[test]
    fn parses_with_or_without_leading_zero() {
        assert_eq!("3".parse(), Ok(Day(3)));
        assert_eq!("03".parse(), Ok(Day(3)));
        assert!(matches!(
            "three".parse::<Day>(),
            Err(ParseDayError::NotANumber(_))
        ));
    }

    #[test]
    fn display_honours_padding() {
        assert_eq!(format!("{}", Day(3)), "3");
        assert_eq!(format!("{:02}", Day(3)), "03");
    }
}
