use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub enum Part {
    One,
    Two,
}

impl Part {
    pub const ALL: [Self; 2] = [Self::One, Self::Two];

    #[must_use]
    pub const fn number(self) -> u8 {
        match self {
            Self::One => 1,
            Self::Two => 2,
        }
    }
}

impl TryFrom<u8> for Part {
    type Error = InvalidPart;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::One),
            2 => Ok(Self::Two),
            other => Err(InvalidPart(other)),
        }
    }
}

impl From<Part> for u8 {
    fn from(value: Part) -> Self {
        value.number()
    }
}

impl FromStr for Part {
    type Err = ParsePartError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::try_from(s.parse::<u8>()?)?)
    }
}

impl fmt::Display for Part {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.number(), f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("part {0} does not exist, a puzzle has parts 1 and 2")]
pub struct InvalidPart(u8);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParsePartError {
    #[error("part is not a number")]
    NotANumber(#[from] ParseIntError),
    #[error(transparent)]
    Invalid(#[from] InvalidPart),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_round_trips() {
        for part in Part::ALL {
            assert_eq!(Part::try_from(part.number()), Ok(part));
        }
    }

    #[test]
    fn rejects_other_numbers() {
        assert_eq!(Part::try_from(0), Err(InvalidPart(0)));
        assert_eq!(Part::try_from(3), Err(InvalidPart(3)));
    }

    #[test]
    fn parses_from_text() {
        assert_eq!("2".parse(), Ok(Part::Two));
        assert!(matches!(
            "two".parse::<Part>(),
            Err(ParsePartError::NotANumber(_))
        ));
    }
}
