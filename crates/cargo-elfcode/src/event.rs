use elfcode_core::{InvalidYear, PuzzleId, Year};
use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::{self, TimeZone};

/// Advent of Code runs on EST all December, so a fixed offset is exact and
/// needs no time zone database.
const EST_OFFSET_HOURS: i8 = -5;

fn eastern() -> TimeZone {
    TimeZone::fixed(tz::offset(EST_OFFSET_HOURS))
}

/// The date of `moment` where puzzles unlock.
pub fn eastern_date(moment: Timestamp) -> Date {
    moment.to_zoned(eastern()).date()
}

/// Midday US Eastern on `date`, safely inside that day.
#[cfg(test)]
pub fn noon_on(date: Date) -> Timestamp {
    date.at(12, 0, 0, 0)
        .to_zoned(eastern())
        .expect("a fixed offset has no gaps")
        .timestamp()
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

/// The date, US Eastern, on which a puzzle unlocks at midnight.
pub fn unlock_date(puzzle: PuzzleId) -> Date {
    let year = i16::try_from(puzzle.year().get()).expect("event years fit in an i16");
    let day = i8::try_from(puzzle.day().get()).expect("days fit in an i8");
    Date::new(year, 12, day).expect("every puzzle day is a date in December")
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;

    #[test]
    fn a_puzzle_unlocks_on_its_day_of_december() {
        let puzzle =
            PuzzleId::new(Year::new(2026).unwrap(), elfcode_core::Day::new(3).unwrap()).unwrap();

        assert_eq!(unlock_date(puzzle), date(2026, 12, 3));
    }

    #[test]
    fn the_date_changes_at_midnight_us_eastern() {
        let utc = |hour| {
            jiff::civil::date(2026, 12, 3)
                .at(hour, 0, 0, 0)
                .to_zoned(TimeZone::UTC)
                .unwrap()
                .timestamp()
        };

        assert_eq!(eastern_date(utc(4)), date(2026, 12, 2));
        assert_eq!(eastern_date(utc(5)), date(2026, 12, 3));
    }

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
