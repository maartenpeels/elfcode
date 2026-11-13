use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{Day, Year};

/// A day that exists in its event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "UncheckedPuzzleId")]
pub struct PuzzleId {
    year: Year,
    day: Day,
}

impl PuzzleId {
    /// # Errors
    ///
    /// Fails when the event of `year` has fewer days than `day`.
    pub const fn new(year: Year, day: Day) -> Result<Self, DayNotInEvent> {
        if day.get() <= year.day_count() {
            Ok(Self { year, day })
        } else {
            Err(DayNotInEvent { year, day })
        }
    }

    #[must_use]
    pub const fn year(self) -> Year {
        self.year
    }

    #[must_use]
    pub const fn day(self) -> Day {
        self.day
    }
}

#[derive(Deserialize)]
struct UncheckedPuzzleId {
    year: Year,
    day: Day,
}

impl TryFrom<UncheckedPuzzleId> for PuzzleId {
    type Error = DayNotInEvent;

    fn try_from(unchecked: UncheckedPuzzleId) -> Result<Self, Self::Error> {
        Self::new(unchecked.year, unchecked.day)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("day {day} is not part of {year}, which has {count} days", count = year.day_count())]
pub struct DayNotInEvent {
    year: Year,
    day: Day,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn year(value: u16) -> Year {
        Year::try_from(value).unwrap()
    }

    fn day(value: u8) -> Day {
        Day::try_from(value).unwrap()
    }

    #[test]
    fn accepts_the_last_day_of_each_event_length() {
        assert!(PuzzleId::new(year(2024), day(25)).is_ok());
        assert!(PuzzleId::new(year(2025), day(12)).is_ok());
    }

    #[test]
    fn rejects_days_past_the_end_of_a_short_event() {
        let error = PuzzleId::new(year(2025), day(13)).unwrap_err();

        assert_eq!(
            error.to_string(),
            "day 13 is not part of 2025, which has 12 days"
        );
    }

    #[test]
    fn orders_by_year_then_day() {
        let earlier = PuzzleId::new(year(2024), day(25)).unwrap();
        let later = PuzzleId::new(year(2025), day(1)).unwrap();

        assert!(earlier < later);
    }
}
