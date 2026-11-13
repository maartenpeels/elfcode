use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, IsTerminal, Write};
use std::process::ExitCode;

use camino::Utf8PathBuf;
use elfcode_core::{DecodeError, PuzzleId, Request};
use thiserror::Error;

use crate::registry::Entry;

const NOT_DRIVEN: &str = "\
This binary is driven by `cargo elfcode`, which sends it a request on stdin.
Use `cargo elfcode run <day>` instead of running it directly.";

/// The whole `main` of a solutions crate: reads one request from
/// `cargo elfcode` on stdin and appends one report per day to the file the
/// request names.
///
/// Stdout and stderr are left to the solutions, so their debug output never
/// mixes with the reports.
#[must_use]
pub fn serve(registry: &[Entry]) -> ExitCode {
    let stdin = io::stdin();
    if stdin.is_terminal() {
        eprintln!("{NOT_DRIVEN}");
        return ExitCode::from(2);
    }

    match respond(registry, stdin.lock()) {
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
    #[error("could not open report file {path}")]
    OpenReportFile {
        path: Utf8PathBuf,
        source: io::Error,
    },
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

fn respond(registry: &[Entry], mut requests: impl BufRead) -> Result<(), ServeError> {
    let mut line = String::new();
    requests
        .read_line(&mut line)
        .map_err(ServeError::ReadRequest)?;
    let request = Request::from_line(&line)?;

    let mut reports = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&request.report_file)
        .map_err(|source| ServeError::OpenReportFile {
            path: request.report_file,
            source,
        })?;

    for day in request.days {
        let entry = registry
            .iter()
            .find(|entry| entry.puzzle() == day.puzzle)
            .ok_or(ServeError::NotRegistered(day.puzzle))?;
        let raw = fs::read_to_string(&day.input).map_err(|source| ServeError::ReadInput {
            path: day.input,
            source,
        })?;

        let mut line = entry.run(&raw, request.parts).to_line();
        line.push('\n');

        // One unbuffered write per day, so a reader following the file sees
        // whole lines as soon as each day finishes.
        reports
            .write_all(line.as_bytes())
            .map_err(ServeError::WriteReport)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use elfcode_core::{Day, DayOutcome, DayReport, DayRequest, PartOutcome, Parts, Year};
    use tempfile::TempDir;

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

    /// A scratch directory holding a two-line `input.txt`.
    struct Workspace(TempDir);

    impl Workspace {
        fn new() -> Self {
            let workspace = Self(tempfile::tempdir().unwrap());
            fs::write(workspace.path("input.txt"), "a\nb\n").unwrap();
            workspace
        }

        fn path(&self, name: &str) -> Utf8PathBuf {
            Utf8PathBuf::from_path_buf(self.0.path().join(name)).unwrap()
        }

        fn request(&self, days: &[(PuzzleId, &str)]) -> String {
            Request {
                report_file: self.path("reports.jsonl"),
                parts: Parts::Both,
                days: days
                    .iter()
                    .map(|&(puzzle, input)| DayRequest {
                        puzzle,
                        input: self.path(input),
                    })
                    .collect(),
            }
            .to_line()
        }

        fn reports(&self) -> Vec<DayReport> {
            fs::read_to_string(self.path("reports.jsonl"))
                .unwrap()
                .lines()
                .map(|line| DayReport::from_line(line).unwrap())
                .collect()
        }
    }

    #[test]
    fn appends_one_report_per_day_in_request_order() {
        let workspace = Workspace::new();
        let request = workspace.request(&[
            (puzzle(2026, 3), "input.txt"),
            (puzzle(2025, 12), "input.txt"),
        ]);

        respond(REGISTRY, request.as_bytes()).unwrap();

        let reports = workspace.reports();
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
    fn keeps_the_reports_written_before_a_failing_day() {
        let workspace = Workspace::new();
        let request = workspace.request(&[
            (puzzle(2026, 3), "input.txt"),
            (puzzle(2026, 4), "input.txt"),
        ]);

        let error = respond(REGISTRY, request.as_bytes()).unwrap_err();

        assert_eq!(
            error.to_string(),
            "2026 day 4 has no solution in this crate"
        );
        assert_eq!(workspace.reports().len(), 1);
    }

    #[test]
    fn names_the_input_file_it_could_not_read() {
        let workspace = Workspace::new();
        let request = workspace.request(&[(puzzle(2026, 3), "missing.txt")]);

        let error = respond(REGISTRY, request.as_bytes()).unwrap_err();

        assert!(matches!(
            error,
            ServeError::ReadInput { path, .. } if path == workspace.path("missing.txt")
        ));
    }

    #[test]
    fn names_the_report_file_it_could_not_open() {
        let workspace = Workspace::new();
        let request = Request {
            report_file: workspace.path("no-such-dir/reports.jsonl"),
            parts: Parts::Both,
            days: Vec::new(),
        }
        .to_line();

        let error = respond(REGISTRY, request.as_bytes()).unwrap_err();

        assert!(matches!(error, ServeError::OpenReportFile { .. }));
    }

    #[test]
    fn refuses_a_request_it_cannot_decode() {
        let error = respond(REGISTRY, "run day 3 please".as_bytes()).unwrap_err();

        assert!(matches!(error, ServeError::DecodeRequest(_)));
    }
}
