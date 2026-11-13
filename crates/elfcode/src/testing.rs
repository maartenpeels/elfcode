//! Support for the example tests that `elfcode-build` generates.

pub use elfcode_core::Part;
use elfcode_core::{DayOutcome, Failure, FailureKind, PartOutcome, Parts};

use crate::{Solution, harness};

/// Runs one part of `S` against an example input.
///
/// # Panics
///
/// When the part does not produce `expected`, with a message saying what it
/// did instead.
pub fn assert_example<S: Solution>(part: Part, input: &str, expected: &str) {
    if let Err(mismatch) = check::<S>(part, input, expected) {
        panic!("{mismatch}");
    }
}

fn check<S: Solution>(part: Part, input: &str, expected: &str) -> Result<(), String> {
    let outcome = match harness::run::<S>(input, Parts::Only(part)) {
        DayOutcome::Parsed { part1, part2, .. } => match part {
            Part::One => part1,
            Part::Two => part2,
        },
        DayOutcome::ParseFailed { failure } => {
            return Err(format!("parsing the example {}", describe(&failure)));
        }
    };

    match outcome.expect("the harness runs the part it was asked for") {
        PartOutcome::Solved { answer, .. } if answer == expected => Ok(()),
        PartOutcome::Solved { answer, .. } => Err(format!(
            "wrong answer for part {part}\n  expected: {expected}\n       got: {answer}"
        )),
        PartOutcome::NotImplemented => Err(format!("part {part} is not implemented")),
        PartOutcome::Failed { failure } => Err(format!("part {part} {}", describe(&failure))),
    }
}

fn describe(failure: &Failure) -> String {
    let verb = match failure.kind {
        FailureKind::Error => "failed",
        FailureKind::Panic => "panicked",
    };
    format!("{verb}: {}", failure.message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    struct Doubler;

    impl Solution for Doubler {
        type Input<'a> = u64;

        fn parse(raw: &str) -> Result<Self::Input<'_>> {
            raw.trim().parse().context("input is not a number")
        }
        fn part1(input: &Self::Input<'_>) -> Result<impl Answer> {
            Ok(input * 2)
        }
        fn part2(_input: &Self::Input<'_>) -> Result<impl Answer> {
            Ok(Unsolved)
        }
    }

    #[test]
    fn accepts_the_expected_answer() {
        assert_eq!(check::<Doubler>(Part::One, "21\n", "42"), Ok(()));
    }

    #[test]
    fn shows_expected_and_got_for_a_wrong_answer() {
        assert_eq!(
            check::<Doubler>(Part::One, "21\n", "43").unwrap_err(),
            "wrong answer for part 1\n  expected: 43\n       got: 42"
        );
    }

    #[test]
    fn says_when_the_part_is_not_implemented() {
        assert_eq!(
            check::<Doubler>(Part::Two, "21\n", "42").unwrap_err(),
            "part 2 is not implemented"
        );
    }

    #[test]
    fn says_when_the_example_does_not_parse() {
        let mismatch = check::<Doubler>(Part::One, "many\n", "42").unwrap_err();

        assert!(
            mismatch.starts_with("parsing the example failed: input is not a number"),
            "{mismatch}"
        );
    }

    #[test]
    #[should_panic(expected = "wrong answer for part 1")]
    fn panics_on_a_mismatch() {
        assert_example::<Doubler>(Part::One, "21\n", "43");
    }
}
