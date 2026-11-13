use std::fmt::{self, Write as _};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};

use camino::{Utf8Path, Utf8PathBuf};
use elfcode_core::{PuzzleId, layout};
use jiff::civil::Date;
use thiserror::Error;

use crate::cli::{ResolveError, ScaffoldArgs};
use crate::data::PuzzleData;
use crate::project::Project;
use crate::site::Site;

const DAY_TEMPLATE: &str = include_str!("../../templates/day.rs.tmpl");
const EXAMPLES_TEMPLATE: &str = include_str!("../../templates/examples.toml.tmpl");

#[derive(Debug, Error)]
pub enum ScaffoldError {
    #[error(transparent)]
    Puzzle(#[from] ResolveError),
    #[error("could not create {path}")]
    Write {
        path: Utf8PathBuf,
        source: io::Error,
    },
}

/// What happened to one file.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Created,
    AlreadyThere,
    Downloaded,
    /// The input is optional here: `run` downloads it later. Scaffolding
    /// ahead of the unlock is the common case.
    NotDownloaded(String),
}

/// Creates the files for a day and downloads its input. Never overwrites a
/// file that is already there.
pub fn execute(
    args: &ScaffoldArgs,
    data: &PuzzleData<'_, impl Site>,
    today: Date,
) -> Result<String, ScaffoldError> {
    let puzzle = args.puzzle.resolve(today)?;
    let project = data.project();

    let source = layout::source_file(puzzle);
    let examples = layout::examples_file(puzzle);
    let input = layout::input_file(puzzle);

    let files = [
        (create(project, &source, &day_source(puzzle))?, source),
        (create(project, &examples, EXAMPLES_TEMPLATE)?, examples),
        (fetch_input(data, puzzle, &input), input),
    ];

    Ok(render(puzzle, &files))
}

fn day_source(puzzle: PuzzleId) -> String {
    DAY_TEMPLATE.replace("{{DAY}}", &format!("{:02}", puzzle.day()))
}

fn create(project: &Project, file: &Utf8Path, contents: &str) -> Result<Outcome, ScaffoldError> {
    let path = project.path(file);
    let written = path
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| OpenOptions::new().write(true).create_new(true).open(&path))
        .and_then(|mut new| new.write_all(contents.as_bytes()));

    match written {
        Ok(()) => Ok(Outcome::Created),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(Outcome::AlreadyThere),
        Err(source) => Err(ScaffoldError::Write {
            path: file.to_owned(),
            source,
        }),
    }
}

fn fetch_input(data: &PuzzleData<'_, impl Site>, puzzle: PuzzleId, file: &Utf8Path) -> Outcome {
    if data.project().path(file).is_file() {
        return Outcome::AlreadyThere;
    }

    match data.input(puzzle) {
        Ok(_) => Outcome::Downloaded,
        Err(error) => Outcome::NotDownloaded(format!("{:#}", anyhow::Error::new(error))),
    }
}

fn render(puzzle: PuzzleId, files: &[(Outcome, Utf8PathBuf)]) -> String {
    let width = files
        .iter()
        .map(|(_, path)| path.as_str().len())
        .max()
        .unwrap_or_default();

    let mut text = format!("{} · day {:02}\n", puzzle.year(), puzzle.day());
    for (outcome, path) in files {
        writeln!(text, "  {:<width$}  {outcome}", path.as_str())
            .expect("writing to a String cannot fail");
    }
    text
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created => f.write_str("created"),
            Self::AlreadyThere => f.write_str("already there"),
            Self::Downloaded => f.write_str("downloaded"),
            Self::NotDownloaded(reason) => write!(f, "not downloaded: {reason}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use elfcode_core::{Day, Year};
    use jiff::civil::date;

    use super::*;
    use crate::cli::PuzzleArgs;
    use crate::data::testing::{FakeSite, Scratch};

    const IN_DECEMBER_2026: Date = date(2026, 12, 25);

    fn args(day: u8, year: u16) -> ScaffoldArgs {
        ScaffoldArgs {
            puzzle: PuzzleArgs {
                day: Day::new(day).unwrap(),
                year: Some(Year::new(year).unwrap()),
            },
        }
    }

    #[test]
    fn creates_the_day_and_downloads_its_input() {
        let scratch = Scratch::logged_in();

        let output = execute(&args(3, 2026), &scratch.data(), IN_DECEMBER_2026).unwrap();

        assert_eq!(
            output,
            "\
2026 · day 03
  src/y2026/day03.rs          created
  data/2026/03/examples.toml  created
  data/2026/03/input.txt      downloaded
"
        );
        assert!(
            scratch
                .read("src/y2026/day03.rs")
                .contains("pub struct Day03;")
        );
        assert!(
            scratch
                .read("src/y2026/day03.rs")
                .contains("impl Solution for Day03 {")
        );
        assert_eq!(
            scratch.read("data/2026/03/examples.toml"),
            EXAMPLES_TEMPLATE
        );
        assert_eq!(scratch.read("data/2026/03/input.txt"), FakeSite::INPUT);
    }

    #[test]
    fn leaves_existing_files_alone() {
        let scratch = Scratch::logged_in();
        scratch.write("src/y2026/day03.rs", "// my solution\n");
        scratch.write("data/2026/03/examples.toml", "# my examples\n");
        scratch.write("data/2026/03/input.txt", "my input\n");

        let output = execute(&args(3, 2026), &scratch.data(), IN_DECEMBER_2026).unwrap();

        assert_eq!(output.matches("already there").count(), 3, "{output}");
        assert_eq!(scratch.read("src/y2026/day03.rs"), "// my solution\n");
        assert_eq!(
            scratch.read("data/2026/03/examples.toml"),
            "# my examples\n"
        );
        assert_eq!(scratch.site.downloads.get(), 0);
    }

    #[test]
    fn still_creates_the_files_when_the_input_cannot_be_downloaded() {
        let scratch = Scratch::logged_in();

        let output = execute(
            &args(3, 2026),
            &scratch.data_on(date(2026, 12, 2)),
            date(2026, 12, 2),
        )
        .unwrap();

        assert!(
            output.ends_with(
                "  data/2026/03/input.txt      not downloaded: 2026 day 3 unlocks on 2026-12-03 \
                 at midnight US Eastern; there is nothing to download yet\n"
            ),
            "{output}"
        );
        assert!(scratch.project.path("src/y2026/day03.rs").is_file());
        assert!(scratch.project.path("data/2026/03/examples.toml").is_file());
    }

    #[test]
    fn says_why_it_did_not_download_when_logged_out() {
        let scratch = Scratch::logged_out();

        let output = execute(&args(3, 2026), &scratch.data(), IN_DECEMBER_2026).unwrap();

        assert!(output.contains("not downloaded: data/2026/03/input.txt has not been downloaded yet and you are not logged in"), "{output}");
    }

    #[test]
    fn refuses_a_day_the_event_does_not_have() {
        let scratch = Scratch::logged_in();

        let error = execute(&args(13, 2025), &scratch.data(), IN_DECEMBER_2026).unwrap_err();

        assert!(matches!(error, ScaffoldError::Puzzle(_)));
        assert!(!scratch.project.path("src").exists());
    }

    #[test]
    fn names_the_day_type_after_the_padded_day() {
        let puzzle = PuzzleId::new(Year::new(2015).unwrap(), Day::new(7).unwrap()).unwrap();

        let source = day_source(puzzle);

        assert!(source.contains("pub struct Day07;"), "{source}");
        assert!(!source.contains("{{"), "{source}");
    }
}
