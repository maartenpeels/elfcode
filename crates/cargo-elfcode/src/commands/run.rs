use camino::Utf8PathBuf;
use elfcode_core::{DayNotInEvent, DayRequest, InvalidYear, Parts, PuzzleId, layout};
use jiff::civil::Date;
use thiserror::Error;

use crate::cli::RunArgs;
use crate::event;
use crate::harness::{Harness, HarnessError};
use crate::project::Project;
use crate::render;

#[derive(Debug, Error)]
pub enum RunError {
    #[error("could not tell the latest event from the system clock; pass --year")]
    Clock(#[source] InvalidYear),
    #[error(transparent)]
    Puzzle(#[from] DayNotInEvent),
    #[error("no puzzle input at {0}; downloading it is not built yet, so save it there by hand")]
    MissingInput(Utf8PathBuf),
    #[error(transparent)]
    Harness(#[from] HarnessError),
    #[error("the solutions finished without reporting on the day")]
    NoReport,
}

/// Runs one day against its puzzle input and returns the text to print.
pub fn execute(
    args: &RunArgs,
    project: &Project,
    harness: &impl Harness,
    today: Date,
) -> Result<String, RunError> {
    let year = match args.year {
        Some(year) => year,
        None => event::latest(today).map_err(RunError::Clock)?,
    };
    let puzzle = PuzzleId::new(year, args.day)?;

    let input = layout::input_file(puzzle);
    if !project.path(&input).is_file() {
        return Err(RunError::MissingInput(input));
    }

    let parts = args.part.map_or(Parts::Both, Parts::Only);
    let day = DayRequest {
        puzzle,
        input: project.path(input),
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
    use tempfile::TempDir;

    use super::*;

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

    fn project_with_input(relative: &str) -> (TempDir, Project) {
        let dir = tempfile::tempdir().unwrap();
        let project = Project::at(Utf8PathBuf::from_path_buf(dir.path().to_owned()).unwrap());
        let input = project.path(relative);
        fs::create_dir_all(input.parent().unwrap()).unwrap();
        fs::write(input, "a\nb\nc\n").unwrap();
        (dir, project)
    }

    fn args(day: u8, year: Option<u16>, part: Option<Part>) -> RunArgs {
        RunArgs {
            day: Day::new(day).unwrap(),
            year: year.map(|year| Year::new(year).unwrap()),
            part,
            debug: false,
        }
    }

    const IN_NOVEMBER_2026: Date = date(2026, 11, 13);

    #[test]
    fn runs_the_day_on_its_input_and_renders_the_report() {
        let (_dir, project) = project_with_input("data/2024/03/input.txt");
        let harness = FakeHarness::default();

        let output = execute(
            &args(3, Some(2024), None),
            &project,
            &harness,
            IN_NOVEMBER_2026,
        )
        .unwrap();

        assert_eq!(
            output,
            "2024 · day 03\n  parse      1.0µs\n  part 1     3            2.0µs\n"
        );
        let asked = harness.asked.borrow();
        let (parts, days) = &asked[0];
        assert_eq!(*parts, Parts::Both);
        assert_eq!(days[0].input, project.path("data/2024/03/input.txt"));
    }

    #[test]
    fn defaults_to_the_latest_event() {
        let (_dir, project) = project_with_input("data/2025/03/input.txt");

        let output = execute(
            &args(3, None, None),
            &project,
            &FakeHarness::default(),
            IN_NOVEMBER_2026,
        )
        .unwrap();

        assert!(output.starts_with("2025 · day 03\n"), "{output}");
    }

    #[test]
    fn asks_for_only_the_selected_part() {
        let (_dir, project) = project_with_input("data/2024/03/input.txt");
        let harness = FakeHarness::default();

        execute(
            &args(3, Some(2024), Some(Part::Two)),
            &project,
            &harness,
            IN_NOVEMBER_2026,
        )
        .unwrap();

        assert_eq!(harness.asked.borrow()[0].0, Parts::Only(Part::Two));
    }

    #[test]
    fn refuses_a_day_the_event_does_not_have_before_running_anything() {
        let (_dir, project) = project_with_input("data/2025/13/input.txt");
        let harness = FakeHarness::default();

        let error = execute(
            &args(13, Some(2025), None),
            &project,
            &harness,
            IN_NOVEMBER_2026,
        )
        .unwrap_err();

        assert!(matches!(error, RunError::Puzzle(_)));
        assert!(harness.asked.borrow().is_empty());
    }

    #[test]
    fn names_the_input_file_that_is_missing() {
        let (_dir, project) = project_with_input("data/2024/03/input.txt");

        let error = execute(
            &args(4, Some(2024), None),
            &project,
            &FakeHarness::default(),
            IN_NOVEMBER_2026,
        )
        .unwrap_err();

        assert!(
            matches!(&error, RunError::MissingInput(path) if path == "data/2024/04/input.txt"),
            "{error}"
        );
    }
}
