use elfcode_core::{DayOutcome, DayReport, DayRequest, Part, PartOutcome, Parts, PuzzleId};
use thiserror::Error;

use crate::cli::{ResolveError, RunArgs};
use crate::data::{DataError, PuzzleData, Submitted};
use crate::harness::{Harness, HarnessError};
use crate::render;
use crate::site::Site;
use crate::submission::{Hint, Response};

#[derive(Debug, Error)]
pub enum RunError {
    #[error(transparent)]
    Puzzle(#[from] ResolveError),
    #[error(transparent)]
    Data(#[from] DataError),
    #[error(transparent)]
    Harness(#[from] HarnessError),
    #[error("the solutions finished without reporting on the day")]
    NoReport,
}

/// Runs one day against its puzzle input, downloading the input when it is
/// not on disk yet, optionally submits one answer, and returns the text to
/// print.
pub fn execute(
    args: &RunArgs,
    data: &PuzzleData<'_, impl Site>,
    harness: &impl Harness,
) -> Result<String, RunError> {
    let puzzle = args.puzzle.resolve(data.today())?;

    let parts = match args.submit.or(args.part) {
        Some(part) => Parts::Only(part),
        None => Parts::Both,
    };
    let day = DayRequest {
        puzzle,
        input: data.input(puzzle)?,
    };

    let reports = harness.run(parts, vec![day])?;
    let report = reports.first().ok_or(RunError::NoReport)?;

    let mut output = render::day_report(report);
    if let Some(part) = args.submit {
        output += &render::row("submit", &submit(data, report, part)?);
    }
    Ok(output)
}

fn submit(
    data: &PuzzleData<'_, impl Site>,
    report: &DayReport,
    part: Part,
) -> Result<String, DataError> {
    let Some(answer) = answer_of(report, part) else {
        return Ok(format!("nothing to send: part {part} has no answer"));
    };

    Ok(match data.submit(report.puzzle, part, answer)? {
        Submitted::Refused(refusal) => format!("not sent: {refusal}"),
        Submitted::Answered(response) => describe(data, report.puzzle, part, response),
    })
}

fn answer_of(report: &DayReport, part: Part) -> Option<&str> {
    let DayOutcome::Parsed { part1, part2, .. } = &report.outcome else {
        return None;
    };
    let outcome = match part {
        Part::One => part1,
        Part::Two => part2,
    };
    match outcome {
        Some(PartOutcome::Solved { answer, .. }) => Some(answer),
        _ => None,
    }
}

fn describe(
    data: &PuzzleData<'_, impl Site>,
    puzzle: PuzzleId,
    part: Part,
    response: Response,
) -> String {
    let verdict = match response {
        Response::Correct => "correct",
        Response::WrongLevel => "already solved",
        Response::Incorrect { hint, wait } => {
            let verdict = match hint {
                Some(Hint::TooHigh) => "wrong, too high",
                Some(Hint::TooLow) => "wrong, too low",
                None => "wrong",
            };
            return match wait {
                Some(wait) => format!(
                    "{verdict}; wait {} before the next answer",
                    render::wait(wait)
                ),
                None => verdict.to_owned(),
            };
        }
        Response::RateLimited { wait } => {
            return format!(
                "not judged, answered too recently; wait {}",
                render::wait(wait)
            );
        }
    };

    if part == Part::Two {
        return verdict.to_owned();
    }
    // Part 1 is solved, so the puzzle page has gained part 2.
    match data.refresh_puzzle_text(puzzle) {
        Ok(_) => format!("{verdict}; puzzle.md now includes part 2"),
        Err(error) => format!(
            "{verdict}; could not refresh puzzle.md: {:#}",
            anyhow::Error::new(error)
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::fs;
    use std::time::Duration;

    use elfcode_core::{Day, Year};
    use jiff::civil::date;

    use super::*;
    use crate::cli::PuzzleArgs;
    use crate::data::testing::{FakeSite, Scratch};

    /// Answers part 1 with the line count of the input and leaves part 2
    /// unimplemented, and records what it was asked.
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
                        part1: parts.includes(Part::One).then(|| PartOutcome::Solved {
                            answer: fs::read_to_string(&day.input)
                                .unwrap()
                                .lines()
                                .count()
                                .to_string(),
                            time: Duration::from_micros(2),
                        }),
                        part2: parts
                            .includes(Part::Two)
                            .then_some(PartOutcome::NotImplemented),
                    },
                })
                .collect();
            self.asked.borrow_mut().push((parts, days));
            Ok(reports)
        }
    }

    fn args(day: u8, year: Option<u16>) -> RunArgs {
        RunArgs {
            puzzle: PuzzleArgs {
                day: Day::new(day).unwrap(),
                year: year.map(|year| Year::new(year).unwrap()),
            },
            part: None,
            submit: None,
            debug: false,
        }
    }

    #[test]
    fn runs_the_day_on_its_input_and_renders_the_report() {
        let scratch = Scratch::logged_out();
        scratch.write("data/2024/03/input.txt", "a\nb\n");
        let harness = FakeHarness::default();

        let output = execute(&args(3, Some(2024)), &scratch.data(), &harness).unwrap();

        assert_eq!(
            output,
            "\
2024 · day 03
  parse      1.0µs
  part 1     2            2.0µs
  part 2     not implemented
"
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
            &args(3, Some(2024)),
            &scratch.data(),
            &FakeHarness::default(),
        )
        .unwrap();

        assert!(output.contains("part 1     3 "), "{output}");
        assert_eq!(scratch.site.downloads.get(), 1);
    }

    #[test]
    fn defaults_to_the_latest_event() {
        let scratch = Scratch::logged_in();

        let output = execute(
            &args(3, None),
            &scratch.data_on(date(2026, 11, 13)),
            &FakeHarness::default(),
        )
        .unwrap();

        assert!(output.starts_with("2025 · day 03\n"), "{output}");
    }

    #[test]
    fn asks_for_only_the_selected_part() {
        let scratch = Scratch::logged_in();
        let harness = FakeHarness::default();
        let args = RunArgs {
            part: Some(Part::Two),
            ..args(3, Some(2024))
        };

        execute(&args, &scratch.data(), &harness).unwrap();

        assert_eq!(harness.asked.borrow()[0].0, Parts::Only(Part::Two));
    }

    #[test]
    fn refuses_a_day_the_event_does_not_have_before_doing_anything() {
        let scratch = Scratch::logged_in();
        let harness = FakeHarness::default();

        let error = execute(&args(13, Some(2025)), &scratch.data(), &harness).unwrap_err();

        assert!(matches!(error, RunError::Puzzle(_)));
        assert_eq!(scratch.site.downloads.get(), 0);
        assert!(harness.asked.borrow().is_empty());
    }

    #[test]
    fn does_not_build_anything_when_the_input_cannot_be_had() {
        let scratch = Scratch::logged_out();
        let harness = FakeHarness::default();

        let error = execute(&args(4, Some(2024)), &scratch.data(), &harness).unwrap_err();

        assert!(
            matches!(error, RunError::Data(DataError::NotLoggedIn { .. })),
            "{error}"
        );
        assert!(harness.asked.borrow().is_empty());
    }

    fn submit(part: Part) -> RunArgs {
        RunArgs {
            submit: Some(part),
            ..args(3, Some(2026))
        }
    }

    fn last_line(output: &str) -> &str {
        output.lines().last().unwrap()
    }

    #[test]
    fn submits_only_the_part_it_runs() {
        let scratch = Scratch::logged_in();
        scratch.site.answer_page.set(FakeSite::TOO_HIGH);
        let harness = FakeHarness::default();

        execute(&submit(Part::One), &scratch.data(), &harness).unwrap();

        assert_eq!(harness.asked.borrow()[0].0, Parts::Only(Part::One));
        assert_eq!(
            *scratch.site.submitted.borrow(),
            [(Part::One, "3".to_owned())]
        );
    }

    #[test]
    fn a_correct_part_1_brings_in_part_2_of_the_puzzle_text() {
        let scratch = Scratch::logged_in();
        scratch.write("data/2026/03/puzzle.md", "part 1 only\n");
        scratch.site.answer_page.set(FakeSite::CORRECT);

        let output = execute(&submit(Part::One), &scratch.data(), &FakeHarness::default()).unwrap();

        assert_eq!(
            last_line(&output),
            "  submit     correct; puzzle.md now includes part 2"
        );
        assert_ne!(scratch.read("data/2026/03/puzzle.md"), "part 1 only\n");
    }

    #[test]
    fn shows_the_hint_and_the_wait_of_a_wrong_answer() {
        let scratch = Scratch::logged_in();
        scratch.site.answer_page.set(FakeSite::TOO_HIGH);

        let output = execute(&submit(Part::One), &scratch.data(), &FakeHarness::default()).unwrap();

        assert_eq!(
            last_line(&output),
            "  submit     wrong, too high; wait 1m before the next answer"
        );
    }

    #[test]
    fn says_why_an_answer_was_not_sent() {
        let scratch = Scratch::logged_in();
        scratch.site.answer_page.set(FakeSite::TOO_HIGH);
        execute(&submit(Part::One), &scratch.data(), &FakeHarness::default()).unwrap();

        let output = execute(&submit(Part::One), &scratch.data(), &FakeHarness::default()).unwrap();

        assert_eq!(
            last_line(&output),
            "  submit     not sent: 3 was already submitted for part 1 and was too high"
        );
        assert_eq!(scratch.site.submitted.borrow().len(), 1);
    }

    #[test]
    fn has_nothing_to_send_for_a_part_without_an_answer() {
        let scratch = Scratch::logged_in();

        let output = execute(&submit(Part::Two), &scratch.data(), &FakeHarness::default()).unwrap();

        assert_eq!(
            last_line(&output),
            "  submit     nothing to send: part 2 has no answer"
        );
        assert!(scratch.site.submitted.borrow().is_empty());
    }
}
