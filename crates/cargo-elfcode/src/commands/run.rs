use elfcode_core::{DayRequest, Parts};
use jiff::civil::Date;
use thiserror::Error;

use crate::cli::{ResolveError, RunArgs};
use crate::data::{DataError, PuzzleData};
use crate::harness::{Harness, HarnessError};
use crate::render;
use crate::site::Site;

#[derive(Debug, Error)]
pub enum RunError {
    #[error(transparent)]
    Puzzle(#[from] ResolveError),
    #[error(transparent)]
    Input(#[from] DataError),
    #[error(transparent)]
    Harness(#[from] HarnessError),
    #[error("the solutions finished without reporting on the day")]
    NoReport,
}

/// Runs one day against its puzzle input, downloading the input when it is
/// not on disk yet, and returns the text to print.
pub fn execute(
    args: &RunArgs,
    data: &PuzzleData<'_, impl Site>,
    harness: &impl Harness,
    today: Date,
) -> Result<String, RunError> {
    let puzzle = args.puzzle.resolve(today)?;

    let parts = args.part.map_or(Parts::Both, Parts::Only);
    let day = DayRequest {
        puzzle,
        input: data.input(puzzle)?,
    };

    let reports = harness.run(parts, vec![day])?;
    let report = reports.first().ok_or(RunError::NoReport)?;

    Ok(render::day_report(report))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::fs;
    use std::time::Duration;

    use elfcode_core::{Day, DayOutcome, DayReport, Part, PartOutcome, Year};
    use jiff::civil::date;

    use super::*;
    use crate::cli::PuzzleArgs;
    use crate::data::testing::Scratch;

    /// Answers every day with its line count and records what it was asked.
    #[derive(Default)]
    struct FakeHarness {
        asked: RefCell<Vec<(Parts, Vec<DayRequest>)>>,
    }

    impl Harness for FakeHarness {
        fn run(&self, parts: Parts, days: Vec<DayRequest>) -> Result<Vec<DayReport>, HarnessError> {
            let reports = days
                .iter()
                .map(|day| DayReport {
                    puzzle: day.puzzle,
                    outcome: DayOutcome::Parsed {
                        parse_time: Duration::from_micros(1),
                        part1: Some(PartOutcome::Solved {
                            answer: fs::read_to_string(&day.input)
                                .unwrap()
                                .lines()
                                .count()
                                .to_string(),
                            time: Duration::from_micros(2),
                        }),
                        part2: None,
                    },
                })
                .collect();
            self.asked.borrow_mut().push((parts, days));
            Ok(reports)
        }
    }

    fn args(day: u8, year: Option<u16>, part: Option<Part>) -> RunArgs {
        RunArgs {
            puzzle: PuzzleArgs {
                day: Day::new(day).unwrap(),
                year: year.map(|year| Year::new(year).unwrap()),
            },
            part,
            debug: false,
        }
    }

    const IN_NOVEMBER_2026: Date = date(2026, 11, 13);

    #[test]
    fn runs_the_day_on_its_input_and_renders_the_report() {
        let scratch = Scratch::logged_out();
        scratch.write("data/2024/03/input.txt", "a\nb\n");
        let harness = FakeHarness::default();

        let output = execute(
            &args(3, Some(2024), None),
            &scratch.data(),
            &harness,
            IN_NOVEMBER_2026,
        )
        .unwrap();

        assert_eq!(
            output,
            "2024 · day 03\n  parse      1.0µs\n  part 1     2            2.0µs\n"
        );
        let asked = harness.asked.borrow();
        let (parts, days) = &asked[0];
        assert_eq!(*parts, Parts::Both);
        assert_eq!(
            days[0].input,
            scratch.project.path("data/2024/03/input.txt")
        );
    }

    #[test]
    fn downloads_the_input_when_it_is_missing() {
        let scratch = Scratch::logged_in();

        let output = execute(
            &args(3, Some(2024), None),
            &scratch.data(),
            &FakeHarness::default(),
            IN_NOVEMBER_2026,
        )
        .unwrap();

        assert!(output.contains("part 1     3 "), "{output}");
        assert_eq!(scratch.site.downloads.get(), 1);
    }

    #[test]
    fn defaults_to_the_latest_event() {
        let scratch = Scratch::logged_in();

        let output = execute(
            &args(3, None, None),
            &scratch.data(),
            &FakeHarness::default(),
            IN_NOVEMBER_2026,
        )
        .unwrap();

        assert!(output.starts_with("2025 · day 03\n"), "{output}");
    }

    #[test]
    fn asks_for_only_the_selected_part() {
        let scratch = Scratch::logged_in();
        let harness = FakeHarness::default();

        execute(
            &args(3, Some(2024), Some(Part::Two)),
            &scratch.data(),
            &harness,
            IN_NOVEMBER_2026,
        )
        .unwrap();

        assert_eq!(harness.asked.borrow()[0].0, Parts::Only(Part::Two));
    }

    #[test]
    fn refuses_a_day_the_event_does_not_have_before_doing_anything() {
        let scratch = Scratch::logged_in();
        let harness = FakeHarness::default();

        let error = execute(
            &args(13, Some(2025), None),
            &scratch.data(),
            &harness,
            IN_NOVEMBER_2026,
        )
        .unwrap_err();

        assert!(matches!(error, RunError::Puzzle(_)));
        assert_eq!(scratch.site.downloads.get(), 0);
        assert!(harness.asked.borrow().is_empty());
    }

    #[test]
    fn does_not_build_anything_when_the_input_cannot_be_had() {
        let scratch = Scratch::logged_out();
        let harness = FakeHarness::default();

        let error = execute(
            &args(4, Some(2024), None),
            &scratch.data(),
            &harness,
            IN_NOVEMBER_2026,
        )
        .unwrap_err();

        assert!(
            matches!(error, RunError::Input(DataError::NotLoggedIn { .. })),
            "{error}"
        );
        assert!(harness.asked.borrow().is_empty());
    }
}
