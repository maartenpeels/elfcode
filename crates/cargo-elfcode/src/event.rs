use elfcode_core::{InvalidYear, Year};
use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::{self, TimeZone};

/// Advent of Code runs on EST all December, so a fixed offset is exact and
/// needs no time zone database.
const EST_OFFSET_HOURS: i8 = -5;

/// Today's date where puzzles unlock.
pub fn today() -> Date {
    Timestamp::now()
        .to_zoned(TimeZone::fixed(tz::offset(EST_OFFSET_HOURS)))
        .date()
}

/// The event to assume when no year is given: this year's once it has
/// started on December 1, last year's until then.
///
/// # Errors
///
/// Fails when `today` is before the first event, which means the system
/// clock is wrong.
pub fn latest(today: Date) -> Result<Year, InvalidYear> {
    let year = if today.month() == 12 {
        today.year()
    } else {
        today.year() - 1
    };

    Year::new(u16::try_from(year).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;

    #[test]
    fn december_belongs_to_its_own_year() {
        assert_eq!(latest(date(2026, 12, 1)), Year::new(2026));
        assert_eq!(latest(date(2026, 12, 31)), Year::new(2026));
    }

    #[test]
    fn the_rest_of_the_year_belongs_to_the_previous_event() {
        assert_eq!(latest(date(2026, 11, 30)), Year::new(2025));
        assert_eq!(latest(date(2027, 1, 1)), Year::new(2026));
    }

    #[test]
    fn a_clock_before_the_first_event_is_an_error() {
        assert!(latest(date(2015, 11, 30)).is_err());
        assert!(latest(date(-44, 3, 15)).is_err());
    }
}
