//! Runs the real binary against the example solutions crate, building it
//! with a nested cargo. `--debug` reuses the artifacts the test run already
//! built.

use std::process::{Command, Output};

const EXAMPLE_CRATE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/solutions");

fn cargo_elfcode(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-elfcode"))
        .arg("elfcode")
        .args(args)
        .current_dir(EXAMPLE_CRATE)
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn runs_a_day_of_a_real_crate() {
    let output = cargo_elfcode(&["run", "1", "--year", "2026", "--debug"]);

    let stdout = text(&output.stdout);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(stdout.contains("2026 · day 01\n  parse "), "{stdout}");
    assert!(stdout.contains("\n  part 1     42 "), "{stdout}");
    assert!(
        stdout.contains("\n  part 2     not implemented\n"),
        "{stdout}"
    );
    assert!(
        text(&output.stderr).contains("warning: debug build"),
        "{}",
        text(&output.stderr)
    );
}

#[test]
fn passes_through_what_the_solution_prints() {
    let output = cargo_elfcode(&["run", "1", "--year", "2026", "--part", "1", "--debug"]);

    let stdout = text(&output.stdout);
    assert!(
        stdout.starts_with("summing 3 numbers\n2026 · day 01\n"),
        "{stdout}"
    );
    assert!(!stdout.contains("part 2"), "{stdout}");
}

#[test]
fn explains_a_missing_input() {
    let output = cargo_elfcode(&["run", "12", "--year", "2025", "--debug"]);

    assert!(!output.status.success());
    assert!(
        text(&output.stderr).contains("error: no puzzle input at data/2025/12/input.txt"),
        "{}",
        text(&output.stderr)
    );
}
