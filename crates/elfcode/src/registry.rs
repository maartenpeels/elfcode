use elfcode_core::{Day, DayOutcome, DayReport, Parts, PuzzleId, Year};

use crate::{Solution, harness};

/// One registered day: a puzzle and the solution that runs it.
///
/// The solution's input type differs per day, so it is erased here behind a
/// function pointer rather than a trait object.
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    puzzle: PuzzleId,
    run: fn(&str, Parts) -> DayOutcome,
}

impl Entry {
    /// Registers `S` as the solution for a day.
    ///
    /// Takes plain numbers because it is written by generated code into a
    /// `static`, where an invalid date then fails the build.
    ///
    /// # Panics
    ///
    /// When `year` and `day` do not name an existing puzzle.
    #[must_use]
    pub const fn of<S: Solution>(year: u16, day: u8) -> Self {
        let Ok(year) = Year::new(year) else {
            panic!("year is before the first event");
        };
        let Ok(day) = Day::new(day) else {
            panic!("day is not between 1 and 25");
        };
        let Ok(puzzle) = PuzzleId::new(year, day) else {
            panic!("the event of this year does not have this day");
        };

        Self {
            puzzle,
            run: harness::run::<S>,
        }
    }

    #[must_use]
    pub const fn puzzle(&self) -> PuzzleId {
        self.puzzle
    }

    #[must_use]
    pub fn run(&self, raw: &str, parts: Parts) -> DayReport {
        DayReport {
            puzzle: self.puzzle,
            outcome: (self.run)(raw, parts),
        }
    }
}
