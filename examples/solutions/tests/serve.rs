//! Drives the built example binary the way `cargo elfcode` will.

use std::fs;
use std::io::Write;
use std::process::{Command, Output, Stdio};

use camino::Utf8PathBuf;
use elfcode_core::{
    Day, DayOutcome, DayReport, DayRequest, PartOutcome, Parts, PuzzleId, Request, Year,
};
use tempfile::TempDir;

fn puzzle(year: u16, day: u8) -> PuzzleId {
    PuzzleId::new(Year::new(year).unwrap(), Day::new(day).unwrap()).unwrap()
}

struct Scratch(TempDir);

impl Scratch {
    fn new() -> Self {
        Self(tempfile::tempdir().unwrap())
    }

    fn path(&self, name: &str) -> Utf8PathBuf {
        Utf8PathBuf::from_path_buf(self.0.path().join(name)).unwrap()
    }

    fn input(&self, name: &str, contents: &str) -> Utf8PathBuf {
        let path = self.path(name);
        fs::write(&path, contents).unwrap();
        path
    }

    fn serve(&self, days: Vec<DayRequest>) -> Output {
        let request = Request {
            report_file: self.path("reports.jsonl"),
            parts: Parts::Both,
            days,
        };

        let mut harness = Command::new(env!("CARGO_BIN_EXE_example-solutions"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        writeln!(harness.stdin.take().unwrap(), "{}", request.to_line()).unwrap();
        harness.wait_with_output().unwrap()
    }

    fn reports(&self) -> Vec<DayReport> {
        fs::read_to_string(self.path("reports.jsonl"))
            .unwrap()
            .lines()
            .map(|line| DayReport::from_line(line).unwrap())
            .collect()
    }
}

fn answers(report: &DayReport) -> (Option<&str>, Option<&str>) {
    let DayOutcome::Parsed { part1, part2, .. } = &report.outcome else {
        panic!("parse failed: {report:?}");
    };
    (answer(part1.as_ref()), answer(part2.as_ref()))
}

fn answer(part: Option<&PartOutcome>) -> Option<&str> {
    match part {
        Some(PartOutcome::Solved { answer, .. }) => Some(answer),
        _ => None,
    }
}

#[test]
fn runs_discovered_days_from_every_year() {
    let scratch = Scratch::new();
    let days = vec![
        DayRequest {
            puzzle: puzzle(2026, 1),
            input: scratch.input("numbers.txt", "1\n2\n39\n"),
        },
        DayRequest {
            puzzle: puzzle(2025, 12),
            input: scratch.input("lines.txt", "ab\nabcd\nabc\n"),
        },
    ];

    let output = scratch.serve(days);

    assert!(output.status.success(), "{output:?}");
    let reports = scratch.reports();
    assert_eq!(answers(&reports[0]), (Some("42"), None));
    assert_eq!(answers(&reports[1]), (Some("3"), Some("abcd")));
}

#[test]
fn leaves_stdout_to_the_solutions() {
    let scratch = Scratch::new();
    let days = vec![DayRequest {
        puzzle: puzzle(2026, 1),
        input: scratch.input("numbers.txt", "1\n2\n"),
    }];

    let output = scratch.serve(days);

    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "summing 2 numbers\n"
    );
}

#[test]
fn fails_for_a_day_without_a_file() {
    let scratch = Scratch::new();
    let days = vec![DayRequest {
        puzzle: puzzle(2026, 2),
        input: scratch.input("numbers.txt", ""),
    }];

    let output = scratch.serve(days);

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "error: 2026 day 2 has no solution in this crate\n"
    );
}
