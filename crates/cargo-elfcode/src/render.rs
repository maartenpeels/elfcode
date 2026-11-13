//! Turns reports into the text the user sees. Pure: no printing here.

use std::time::Duration;

use elfcode_core::{DayOutcome, DayReport, Failure, FailureKind, Part, PartOutcome};

const LABEL_WIDTH: usize = 9;
const MIN_ANSWER_WIDTH: usize = 11;
const GAP: &str = "  ";

/// The result of running one day, ending in a newline.
pub fn day_report(report: &DayReport) -> String {
    let title = format!("{} · day {:02}", report.puzzle.year(), report.puzzle.day());

    let rows = match &report.outcome {
        DayOutcome::ParseFailed { failure } => vec![row("parse", &describe(failure))],
        DayOutcome::Parsed {
            parse_time,
            part1,
            part2,
        } => {
            let parts: Vec<_> = [(Part::One, part1), (Part::Two, part2)]
                .into_iter()
                .filter_map(|(part, outcome)| Some((part, outcome.as_ref()?)))
                .collect();
            let answer_width = parts
                .iter()
                .filter_map(|(_, outcome)| match outcome {
                    PartOutcome::Solved { answer, .. } => Some(answer.chars().count()),
                    _ => None,
                })
                .fold(MIN_ANSWER_WIDTH, usize::max);

            let mut rows = vec![row("parse", &duration(*parse_time))];
            rows.extend(parts.into_iter().map(|(part, outcome)| {
                row(
                    &format!("part {part}"),
                    &part_outcome(outcome, answer_width),
                )
            }));
            rows
        }
    };

    format!("{title}\n{}", rows.concat())
}

fn row(label: &str, value: &str) -> String {
    format!("{GAP}{label:<LABEL_WIDTH$}{GAP}{value}\n")
}

fn part_outcome(outcome: &PartOutcome, answer_width: usize) -> String {
    match outcome {
        PartOutcome::Solved { answer, time } => {
            format!("{answer:<answer_width$}{GAP}{}", duration(*time))
        }
        PartOutcome::NotImplemented => "not implemented".to_owned(),
        PartOutcome::Failed { failure } => describe(failure),
    }
}

fn describe(failure: &Failure) -> String {
    let verb = match failure.kind {
        FailureKind::Error => "failed",
        FailureKind::Panic => "panicked",
    };
    format!("{verb}: {}", failure.message)
}

fn duration(duration: Duration) -> String {
    format!("{duration:.1?}")
}

#[cfg(test)]
mod tests {
    use elfcode_core::{Day, PuzzleId, Year};

    use super::*;

    fn report(outcome: DayOutcome) -> DayReport {
        DayReport {
            puzzle: PuzzleId::new(Year::new(2026).unwrap(), Day::new(3).unwrap()).unwrap(),
            outcome,
        }
    }

    fn solved(answer: &str, nanos: u64) -> PartOutcome {
        PartOutcome::Solved {
            answer: answer.to_owned(),
            time: Duration::from_nanos(nanos),
        }
    }

    fn failure(kind: FailureKind, message: &str) -> Failure {
        Failure {
            kind,
            message: message.to_owned(),
        }
    }

    #[test]
    fn shows_parse_time_and_each_part() {
        let rendered = day_report(&report(DayOutcome::Parsed {
            parse_time: Duration::from_nanos(112_400),
            part1: Some(solved("6", 18_200)),
            part2: Some(PartOutcome::NotImplemented),
        }));

        assert_eq!(
            rendered,
            "\
2026 · day 03
  parse      112.4µs
  part 1     6            18.2µs
  part 2     not implemented
"
        );
    }

    #[test]
    fn aligns_times_after_the_longest_answer() {
        let rendered = day_report(&report(DayOutcome::Parsed {
            parse_time: Duration::from_millis(3),
            part1: Some(solved("7", 1_500_000)),
            part2: Some(solved("ZKJFBCEHAPLR-and-more", 2_000_000_000)),
        }));

        assert_eq!(
            rendered,
            "\
2026 · day 03
  parse      3.0ms
  part 1     7                      1.5ms
  part 2     ZKJFBCEHAPLR-and-more  2.0s
"
        );
    }

    #[test]
    fn leaves_out_a_part_that_was_not_run() {
        let rendered = day_report(&report(DayOutcome::Parsed {
            parse_time: Duration::from_millis(3),
            part1: None,
            part2: Some(solved("31", 1_000)),
        }));

        assert_eq!(
            rendered,
            "\
2026 · day 03
  parse      3.0ms
  part 2     31           1.0µs
"
        );
    }

    #[test]
    fn tells_an_error_from_a_panic() {
        let rendered = day_report(&report(DayOutcome::Parsed {
            parse_time: Duration::from_millis(3),
            part1: Some(PartOutcome::Failed {
                failure: failure(FailureKind::Error, "no path to the exit"),
            }),
            part2: Some(PartOutcome::Failed {
                failure: failure(FailureKind::Panic, "index out of bounds"),
            }),
        }));

        assert_eq!(
            rendered,
            "\
2026 · day 03
  parse      3.0ms
  part 1     failed: no path to the exit
  part 2     panicked: index out of bounds
"
        );
    }

    #[test]
    fn shows_only_the_parse_failure_when_parsing_fails() {
        let rendered = day_report(&report(DayOutcome::ParseFailed {
            failure: failure(FailureKind::Error, "line 4: unexpected `x`"),
        }));

        assert_eq!(
            rendered,
            "\
2026 · day 03
  parse      failed: line 4: unexpected `x`
"
        );
    }
}
