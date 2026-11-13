use std::io;

use camino::{Utf8Path, Utf8PathBuf};
use elfcode_core::{Day, DayNotInEvent, PuzzleId, Year};
use thiserror::Error;

const SRC: &str = "src";

/// A day file that follows the convention.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DayFile {
    pub puzzle: PuzzleId,
    /// Relative to the crate root, as it should read in messages.
    pub path: Utf8PathBuf,
}

#[derive(Debug, Error)]
pub(crate) enum DiscoverError {
    #[error("could not read {path}")]
    Read {
        path: Utf8PathBuf,
        source: io::Error,
    },
    #[error("{path}: {problem}")]
    Convention { path: Utf8PathBuf, problem: Problem },
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum Problem {
    #[error("a year directory is named `y` plus an event year from 2015 on, like `y2026`")]
    YearDirectory,
    #[error(
        "a Rust file in a year directory is named `dayDD.rs` with a two-digit day from 01 to 25, like `day03.rs`"
    )]
    DayFile,
    #[error(transparent)]
    DayNotInEvent(#[from] DayNotInEvent),
}

/// Finds every `src/y{YYYY}/day{DD}.rs` under `crate_root`, ordered by year
/// and day.
///
/// Inside a year directory only `.rs` files are held to the convention.
/// Subdirectories are left alone because a day may have submodules
/// (`day03/grid.rs`), and other files because of the likes of `.DS_Store`.
pub(crate) fn discover(crate_root: &Utf8Path) -> Result<Vec<DayFile>, DiscoverError> {
    let mut days = Vec::new();

    for entry in list(crate_root, Utf8Path::new(SRC))? {
        if !entry.is_dir {
            continue;
        }
        let Some(year) = year_of(&entry.name) else {
            continue;
        };
        let year = year.map_err(|problem| DiscoverError::Convention {
            path: entry.path.clone(),
            problem,
        })?;

        for file in list(crate_root, &entry.path)? {
            if file.is_dir || file.path.extension() != Some("rs") {
                continue;
            }
            let puzzle =
                puzzle_of(year, &file.name).map_err(|problem| DiscoverError::Convention {
                    path: file.path.clone(),
                    problem,
                })?;
            days.push(DayFile {
                puzzle,
                path: file.path,
            });
        }
    }

    days.sort_by_key(|day| day.puzzle);
    Ok(days)
}

struct Listed {
    name: String,
    path: Utf8PathBuf,
    is_dir: bool,
}

fn list(crate_root: &Utf8Path, dir: &Utf8Path) -> Result<Vec<Listed>, DiscoverError> {
    let unreadable = |source| DiscoverError::Read {
        path: dir.to_owned(),
        source,
    };

    crate_root
        .join(dir)
        .read_dir_utf8()
        .map_err(unreadable)?
        .map(|entry| {
            let entry = entry.map_err(unreadable)?;
            Ok(Listed {
                name: entry.file_name().to_owned(),
                path: dir.join(entry.file_name()),
                is_dir: entry.file_type().map_err(unreadable)?.is_dir(),
            })
        })
        .collect()
}

/// `None` when the directory does not claim to be a year directory at all,
/// so `src/yaml/` and friends are not mistaken for one.
fn year_of(dir_name: &str) -> Option<Result<Year, Problem>> {
    let digits = dir_name.strip_prefix('y')?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    Some(
        digits
            .parse::<Year>()
            .ok()
            .filter(|year| format!("y{year}") == dir_name)
            .ok_or(Problem::YearDirectory),
    )
}

fn puzzle_of(year: Year, file_name: &str) -> Result<PuzzleId, Problem> {
    let day = file_name
        .strip_prefix("day")
        .and_then(|rest| rest.strip_suffix(".rs"))
        .and_then(|digits| digits.parse::<Day>().ok())
        .filter(|day| format!("day{day:02}.rs") == file_name)
        .ok_or(Problem::DayFile)?;

    Ok(PuzzleId::new(year, day)?)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    /// A scratch crate root containing the given files, all empty.
    fn crate_with(files: &[&str]) -> (TempDir, Utf8PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_owned()).unwrap();
        fs::create_dir(root.join(SRC)).unwrap();
        for file in files {
            let path = root.join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "").unwrap();
        }
        (dir, root)
    }

    fn found(files: &[&str]) -> Vec<String> {
        let (_dir, root) = crate_with(files);
        discover(&root)
            .unwrap()
            .into_iter()
            .map(|day| day.path.into_string())
            .collect()
    }

    fn violation(files: &[&str]) -> (String, Problem) {
        let (_dir, root) = crate_with(files);
        match discover(&root).unwrap_err() {
            DiscoverError::Convention { path, problem } => (path.into_string(), problem),
            other @ DiscoverError::Read { .. } => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn finds_days_ordered_by_year_then_day() {
        let days = found(&[
            "src/y2026/day10.rs",
            "src/y2026/day02.rs",
            "src/y2015/day25.rs",
        ]);

        assert_eq!(
            days,
            [
                "src/y2015/day25.rs",
                "src/y2026/day02.rs",
                "src/y2026/day10.rs"
            ]
        );
    }

    #[test]
    fn finds_nothing_in_a_crate_without_days() {
        assert_eq!(found(&["src/main.rs"]), Vec::<String>::new());
    }

    #[test]
    fn leaves_everything_outside_the_convention_alone() {
        let days = found(&[
            "src/main.rs",
            "src/grid.rs",
            "src/yaml/parser.rs",
            "src/y2026/day03.rs",
            "src/y2026/day03/grid.rs",
            "src/y2026/.DS_Store",
            "src/y2026/notes.md",
        ]);

        assert_eq!(days, ["src/y2026/day03.rs"]);
    }

    #[test]
    fn rejects_a_year_directory_that_is_not_an_event_year() {
        for dir in ["y26", "y1999", "y02026"] {
            let (path, problem) = violation(&[&format!("src/{dir}/day01.rs")]);

            assert_eq!(path, format!("src/{dir}"));
            assert_eq!(problem, Problem::YearDirectory);
        }
    }

    #[test]
    fn rejects_a_rust_file_that_is_not_a_day() {
        for file in ["day3.rs", "day003.rs", "day00.rs", "day26.rs", "mod.rs"] {
            let (path, problem) = violation(&[&format!("src/y2024/{file}")]);

            assert_eq!(path, format!("src/y2024/{file}"));
            assert_eq!(problem, Problem::DayFile);
        }
    }

    #[test]
    fn rejects_a_day_the_event_does_not_have() {
        let (path, problem) = violation(&["src/y2025/day13.rs"]);

        assert_eq!(path, "src/y2025/day13.rs");
        assert_eq!(
            problem.to_string(),
            "day 13 is not part of 2025, which has 12 days"
        );
    }

    #[test]
    fn names_the_directory_it_could_not_read() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_owned()).unwrap();

        let error = discover(&root).unwrap_err();

        assert_eq!(error.to_string(), "could not read src");
    }
}
