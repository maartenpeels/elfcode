//! Where a solutions crate keeps its puzzle data, relative to the crate
//! root: `data/{YYYY}/{DD}/`.
//!
//! The directory mixes two kinds of file. The puzzle text and input are
//! downloaded, can be fetched again, and must not be published. The
//! examples are written by hand and belong in version control.

use camino::Utf8PathBuf;

use crate::PuzzleId;

pub const DATA_DIR: &str = "data";
pub const INPUT_FILE: &str = "input.txt";
pub const PUZZLE_FILE: &str = "puzzle.md";
pub const EXAMPLES_FILE: &str = "examples.toml";

#[must_use]
pub fn puzzle_dir(puzzle: PuzzleId) -> Utf8PathBuf {
    Utf8PathBuf::from(DATA_DIR)
        .join(puzzle.year().to_string())
        .join(format!("{:02}", puzzle.day()))
}

#[must_use]
pub fn input_file(puzzle: PuzzleId) -> Utf8PathBuf {
    puzzle_dir(puzzle).join(INPUT_FILE)
}

#[must_use]
pub fn puzzle_file(puzzle: PuzzleId) -> Utf8PathBuf {
    puzzle_dir(puzzle).join(PUZZLE_FILE)
}

#[must_use]
pub fn examples_file(puzzle: PuzzleId) -> Utf8PathBuf {
    puzzle_dir(puzzle).join(EXAMPLES_FILE)
}

/// The `.gitignore` patterns for a solutions crate.
///
/// They name the downloaded files rather than the whole data directory, so
/// the examples stay tracked.
#[must_use]
pub fn gitignore_patterns() -> [String; 2] {
    [INPUT_FILE, PUZZLE_FILE].map(|file| format!("/{DATA_DIR}/**/{file}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Day, Year};

    #[test]
    fn pads_the_day_but_not_the_year() {
        let puzzle = PuzzleId::new(Year::new(2026).unwrap(), Day::new(3).unwrap()).unwrap();

        assert_eq!(examples_file(puzzle), "data/2026/03/examples.toml");
    }

    #[test]
    fn ignores_the_downloaded_files_but_not_the_examples() {
        assert_eq!(
            gitignore_patterns(),
            ["/data/**/input.txt", "/data/**/puzzle.md"]
        );
    }
}
