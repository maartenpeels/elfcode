//! Runs a [`Solution`] and describes what happened as a [`DayReport`].
//!
//! Nothing here reads files, prints, or exits: a failing or panicking
//! solution is a value in the report, not a failure of the run.

use std::any::Any;
use std::panic::{self, AssertUnwindSafe};
use std::time::{Duration, Instant};

use elfcode_core::{
    DayOutcome, DayReport, Failure, FailureKind, Part, PartOutcome, Parts, PuzzleId,
};

use crate::{Answer, Solution};

/// Parses `raw` once, then runs the selected parts against the result.
#[must_use]
pub fn run<S: Solution>(puzzle: PuzzleId, raw: &str, parts: Parts) -> DayReport {
    let outcome = match attempt(|| S::parse(raw)) {
        Ok((input, parse_time)) => DayOutcome::Parsed {
            parse_time,
            part1: parts
                .includes(Part::One)
                .then(|| run_part(|| S::part1(&input))),
            part2: parts
                .includes(Part::Two)
                .then(|| run_part(|| S::part2(&input))),
        },
        Err(failure) => DayOutcome::ParseFailed { failure },
    };

    DayReport { puzzle, outcome }
}

fn run_part<A: Answer>(solve: impl FnOnce() -> anyhow::Result<A>) -> PartOutcome {
    match attempt(solve) {
        Ok((answer, _)) if answer.is_unsolved() => PartOutcome::NotImplemented,
        Ok((answer, time)) => PartOutcome::Solved {
            answer: answer.to_string(),
            time,
        },
        Err(failure) if is_todo(&failure) => PartOutcome::NotImplemented,
        Err(failure) => PartOutcome::Failed { failure },
    }
}

/// Times `step`, turning both an `Err` and a panic into a [`Failure`].
fn attempt<T>(step: impl FnOnce() -> anyhow::Result<T>) -> Result<(T, Duration), Failure> {
    // Sound to assert: after a panic nothing the closure touched is used again.
    let caught = panic::catch_unwind(AssertUnwindSafe(|| {
        let start = Instant::now();
        let result = step();
        (result, start.elapsed())
    }));

    match caught {
        Ok((Ok(value), time)) => Ok((value, time)),
        Ok((Err(error), _)) => Err(Failure {
            kind: FailureKind::Error,
            // `{:#}` joins the whole context chain on one line.
            message: format!("{error:#}"),
        }),
        Err(payload) => Err(Failure {
            kind: FailureKind::Panic,
            message: panic_message(payload.as_ref()).to_owned(),
        }),
    }
}

fn panic_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("panicked with a payload that is not a string")
}

fn is_todo(failure: &Failure) -> bool {
    // The message `todo!()` panics with; the standard library offers no
    // other way to tell it apart from any other panic.
    const TODO: &str = "not yet implemented";

    failure.kind == FailureKind::Panic
        && failure
            .message
            .strip_prefix(TODO)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(": "))
}

#[cfg(test)]
mod tests {
    use anyhow::Context;
    use elfcode_core::{Day, Year};

    use super::*;
    use crate::prelude::*;

    fn puzzle() -> PuzzleId {
        PuzzleId::new(Year::try_from(2026).unwrap(), Day::try_from(3).unwrap()).unwrap()
    }

    fn parts_of(report: DayReport) -> (Option<PartOutcome>, Option<PartOutcome>) {
        match report.outcome {
            DayOutcome::Parsed { part1, part2, .. } => (part1, part2),
            DayOutcome::ParseFailed { failure } => panic!("parse failed: {failure:?}"),
        }
    }

    fn answer_of(outcome: Option<PartOutcome>) -> String {
        match outcome {
            Some(PartOutcome::Solved { answer, .. }) => answer,
            other => panic!("not solved: {other:?}"),
        }
    }

    fn failure_of(outcome: Option<PartOutcome>) -> Failure {
        match outcome {
            Some(PartOutcome::Failed { failure }) => failure,
            other => panic!("did not fail: {other:?}"),
        }
    }

    struct Borrowing;

    impl Solution for Borrowing {
        type Input<'a> = Vec<&'a str>;

        fn parse(raw: &str) -> Result<Self::Input<'_>> {
            Ok(raw.lines().collect())
        }
        fn part1(input: &Self::Input<'_>) -> Result<impl Answer> {
            Ok(input.len())
        }
        fn part2(input: &Self::Input<'_>) -> Result<impl Answer> {
            input.last().copied().context("input is empty")
        }
    }

    struct Unfinished;

    impl Solution for Unfinished {
        type Input<'a> = u64;

        fn parse(raw: &str) -> Result<Self::Input<'_>> {
            raw.trim().parse().context("input is not a number")
        }
        fn part1(input: &Self::Input<'_>) -> Result<impl Answer> {
            Ok(input.checked_sub(1).expect("input is at least 1"))
        }
        #[allow(unreachable_code)]
        fn part2(_input: &Self::Input<'_>) -> Result<impl Answer> {
            todo!("needs a faster approach") as Result<u64>
        }
    }

    struct Scaffolded;

    impl Solution for Scaffolded {
        type Input<'a> = &'a str;

        fn parse(raw: &str) -> Result<Self::Input<'_>> {
            Ok(raw)
        }
        fn part1(_input: &Self::Input<'_>) -> Result<impl Answer> {
            Ok(Unsolved)
        }
        fn part2(_input: &Self::Input<'_>) -> Result<impl Answer> {
            Ok(&Unsolved)
        }
    }

    #[test]
    fn reports_the_unsolved_placeholder_as_not_implemented() {
        let (part1, part2) = parts_of(run::<Scaffolded>(puzzle(), "", Parts::Both));

        assert_eq!(part1, Some(PartOutcome::NotImplemented));
        assert_eq!(part2, Some(PartOutcome::NotImplemented));
    }

    #[test]
    fn solves_both_parts_from_one_borrowed_parse() {
        let (part1, part2) = parts_of(run::<Borrowing>(puzzle(), "a\nb\nc", Parts::Both));

        assert_eq!(answer_of(part1), "3");
        assert_eq!(answer_of(part2), "c");
    }

    #[test]
    fn skips_the_part_that_was_not_selected() {
        let (part1, part2) = parts_of(run::<Borrowing>(puzzle(), "a", Parts::Only(Part::Two)));

        assert_eq!(part1, None);
        assert_eq!(answer_of(part2), "a");
    }

    #[test]
    fn reports_todo_as_not_implemented_and_keeps_the_other_part() {
        let (part1, part2) = parts_of(run::<Unfinished>(puzzle(), "7", Parts::Both));

        assert_eq!(answer_of(part1), "6");
        assert_eq!(part2, Some(PartOutcome::NotImplemented));
    }

    #[test]
    fn reports_a_returned_error_with_its_context() {
        let (_, part2) = parts_of(run::<Borrowing>(puzzle(), "", Parts::Both));

        assert_eq!(
            failure_of(part2),
            Failure {
                kind: FailureKind::Error,
                message: "input is empty".to_owned(),
            }
        );
    }

    #[test]
    fn reports_a_panic_that_is_not_todo_as_a_failure() {
        let (part1, _) = parts_of(run::<Unfinished>(puzzle(), "0", Parts::Both));

        assert_eq!(
            failure_of(part1),
            Failure {
                kind: FailureKind::Panic,
                message: "input is at least 1".to_owned(),
            }
        );
    }

    #[test]
    fn reports_a_parse_error_without_running_any_part() {
        let report = run::<Unfinished>(puzzle(), "seven", Parts::Both);

        let DayOutcome::ParseFailed { failure } = report.outcome else {
            panic!("parse succeeded: {report:?}");
        };
        assert_eq!(failure.kind, FailureKind::Error);
        assert!(
            failure.message.starts_with("input is not a number: "),
            "{}",
            failure.message
        );
    }

    #[test]
    fn only_the_todo_message_counts_as_not_implemented() {
        let panic = |message: &str| Failure {
            kind: FailureKind::Panic,
            message: message.to_owned(),
        };

        assert!(is_todo(&panic("not yet implemented")));
        assert!(is_todo(&panic("not yet implemented: part 2")));
        assert!(!is_todo(&panic("not yet implemented properly")));
        assert!(!is_todo(&Failure {
            kind: FailureKind::Error,
            message: "not yet implemented".to_owned(),
        }));
    }
}
