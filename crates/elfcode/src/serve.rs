use std::fs;
use std::io::{self, BufRead, IsTerminal, Write};
use std::process::ExitCode;

use camino::Utf8PathBuf;
use elfcode_core::{DecodeError, PuzzleId, Request};
use thiserror::Error;

use crate::Entry;

const NOT_DRIVEN: &str = "\
This binary is driven by `cargo elfcode`, which sends it a request on stdin.
Use `cargo elfcode run <day>` instead of running it directly.";

/// The whole `main` of a solutions crate: answers one request from
/// `cargo elfcode` on stdin with one report per day on stdout.
#[must_use]
pub fn serve(registry: &[Entry]) -> ExitCode {
    let stdin = io::stdin();
    if stdin.is_terminal() {
        eprintln!("{NOT_DRIVEN}");
        return ExitCode::from(2);
    }

    match respond(registry, stdin.lock(), io::stdout().lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {:#}", anyhow::Error::from(error));
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug, Error)]
enum ServeError {
    #[error("could not read the request")]
    ReadRequest(#[source] io::Error),
    #[error("could not understand the request")]
    DecodeRequest(#[from] DecodeError),
    #[error("{} day {} has no solution in this crate", .0.year(), .0.day())]
    NotRegistered(PuzzleId),
    #[error("could not read input file {path}")]
    ReadInput {
        path: Utf8PathBuf,
        source: io::Error,
    },
    #[error("could not write the report")]
    WriteReport(#[source] io::Error),
}

fn respond(
    registry: &[Entry],
    mut requests: impl BufRead,
    mut reports: impl Write,
) -> Result<(), ServeError> {
    let mut line = String::new();
    requests
        .read_line(&mut line)
        .map_err(ServeError::ReadRequest)?;
    let request = Request::from_line(&line)?;

    for day in request.days {
        let entry = registry
            .iter()
            .find(|entry| entry.puzzle() == day.puzzle)
            .ok_or(ServeError::NotRegistered(day.puzzle))?;
        let raw = fs::read_to_string(&day.input).map_err(|source| ServeError::ReadInput {
            path: day.input,
            source,
        })?;

        let report = entry.run(&raw, request.parts);

        // Flushed per day so the CLI can show progress while later days run.
        writeln!(reports, "{}", report.to_line())
            .and_then(|()| reports.flush())
            .map_err(ServeError::WriteReport)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use elfcode_core::{Day, DayOutcome, DayReport, DayRequest, PartOutcome, Parts, Year};

    use super::*;
    use crate::prelude::*;

    struct LineCount;

    impl Solution for LineCount {
        type Input<'a> = usize;

        fn parse(raw: &str) -> Result<Self::Input<'_>> {
            Ok(raw.lines().count())
        }
        fn part1(input: &Self::Input<'_>) -> Result<impl Answer> {
            Ok(*input)
        }
        fn part2(_input: &Self::Input<'_>) -> Result<impl Answer> {
            Ok(Unsolved)
        }
    }

    static REGISTRY: &[Entry] = &[
        Entry::of::<LineCount>(2025, 12),
        Entry::of::<LineCount>(2026, 3),
    ];

    fn puzzle(year: u16, day: u8) -> PuzzleId {
        PuzzleId::new(Year::new(year).unwrap(), Day::new(day).unwrap()).unwrap()
    }

    fn input_file(contents: &str) -> (tempfile::TempDir, Utf8PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(dir.path().join("input.txt")).unwrap();
        fs::write(&path, contents).unwrap();
        (dir, path)
    }

    fn request_line(days: &[(PuzzleId, &Utf8PathBuf)]) -> String {
        Request {
            parts: Parts::Both,
            days: days
                .iter()
                .map(|&(puzzle, input)| DayRequest {
                    puzzle,
                    input: input.clone(),
                })
                .collect(),
        }
        .to_line()
    }

    fn respond_to(line: &str) -> Result<Vec<DayReport>, ServeError> {
        let mut output = Vec::new();
        respond(REGISTRY, line.as_bytes(), &mut output)?;

        Ok(String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| DayReport::from_line(line).unwrap())
            .collect())
    }

    #[test]
    fn answers_with_one_report_per_day_in_request_order() {
        let (_dir, input) = input_file("a\nb\n");
        let line = request_line(&[(puzzle(2026, 3), &input), (puzzle(2025, 12), &input)]);

        let reports = respond_to(&line).unwrap();

        let puzzles: Vec<_> = reports.iter().map(|report| report.puzzle).collect();
        assert_eq!(puzzles, [puzzle(2026, 3), puzzle(2025, 12)]);
        assert!(matches!(
            &reports[0].outcome,
            DayOutcome::Parsed {
                part1: Some(PartOutcome::Solved { answer, .. }),
                part2: Some(PartOutcome::NotImplemented),
                ..
            } if answer == "2"
        ));
    }

    #[test]
    fn refuses_a_day_that_has_no_solution() {
        let (_dir, input) = input_file("");
        let line = request_line(&[(puzzle(2026, 4), &input)]);

        let error = respond_to(&line).unwrap_err();

        assert_eq!(
            error.to_string(),
            "2026 day 4 has no solution in this crate"
        );
    }

    #[test]
    fn names_the_input_file_it_could_not_read() {
        let (dir, _) = input_file("");
        let missing = Utf8PathBuf::from_path_buf(dir.path().join("missing.txt")).unwrap();
        let line = request_line(&[(puzzle(2026, 3), &missing)]);

        let error = respond_to(&line).unwrap_err();

        assert!(matches!(error, ServeError::ReadInput { path, .. } if path == missing));
    }

    #[test]
    fn refuses_a_request_it_cannot_decode() {
        let error = respond_to("run day 3 please").unwrap_err();

        assert!(matches!(error, ServeError::DecodeRequest(_)));
    }
}
