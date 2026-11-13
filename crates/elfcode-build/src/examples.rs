use std::{fs, io};

use camino::{Utf8Path, Utf8PathBuf};
use elfcode_core::{Part, PuzzleId, layout};
use serde::Deserialize;
use thiserror::Error;

/// One hand-written example: an input and the answers it should give.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Example {
    pub input: String,
    /// Never empty: an example that tests nothing is rejected on load.
    pub answers: Vec<(Part, String)>,
}

#[derive(Debug, Error)]
pub(crate) enum ExamplesError {
    #[error("could not read {path}")]
    Read {
        path: Utf8PathBuf,
        source: io::Error,
    },
    #[error("{path}:{line}: {message}")]
    Syntax {
        path: Utf8PathBuf,
        line: usize,
        message: String,
    },
    #[error("{path}: example {number} has no expected answer; give it `part1`, `part2` or both")]
    NoAnswer { path: Utf8PathBuf, number: usize },
}

/// Loads the examples of a day. A day without an examples file has none.
pub(crate) fn load(crate_root: &Utf8Path, puzzle: PuzzleId) -> Result<Vec<Example>, ExamplesError> {
    let path = layout::examples_file(puzzle);

    match fs::read_to_string(crate_root.join(&path)) {
        Ok(text) => parse(&path, &text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(source) => Err(ExamplesError::Read { path, source }),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default, rename = "example")]
    examples: Vec<Entry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    input: String,
    part1: Option<Expected>,
    part2: Option<Expected>,
}

/// Answers are compared in their displayed form, so `11` and `"11"` mean
/// the same. A number beyond TOML's 64-bit integers has to be quoted.
#[derive(Deserialize)]
#[serde(untagged, expecting = "an answer is text or a whole number")]
enum Expected {
    Text(String),
    Number(i64),
}

impl From<Expected> for String {
    fn from(expected: Expected) -> Self {
        match expected {
            Expected::Text(text) => text,
            Expected::Number(number) => number.to_string(),
        }
    }
}

fn parse(path: &Utf8Path, text: &str) -> Result<Vec<Example>, ExamplesError> {
    let file: File = toml::from_str(text).map_err(|error| ExamplesError::Syntax {
        path: path.to_owned(),
        line: error
            .span()
            .map_or(1, |span| 1 + text[..span.start].matches('\n').count()),
        message: error.message().to_owned(),
    })?;

    file.examples
        .into_iter()
        .zip(1..)
        .map(|(entry, number)| {
            let answers: Vec<_> = [(Part::One, entry.part1), (Part::Two, entry.part2)]
                .into_iter()
                .filter_map(|(part, answer)| Some((part, answer?.into())))
                .collect();

            if answers.is_empty() {
                return Err(ExamplesError::NoAnswer {
                    path: path.to_owned(),
                    number,
                });
            }
            Ok(Example {
                input: entry.input,
                answers,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATH: &str = "data/2026/03/examples.toml";

    fn parsed(text: &str) -> Vec<Example> {
        parse(Utf8Path::new(PATH), text).unwrap()
    }

    fn rejected(text: &str) -> String {
        parse(Utf8Path::new(PATH), text).unwrap_err().to_string()
    }

    #[test]
    fn reads_the_answers_each_example_gives() {
        let examples = parsed(
            r#"
[[example]]
input = '''
1 2
3 4
'''
part1 = "11"
part2 = "31"

[[example]]
input = "5"
part2 = "7"
"#,
        );

        assert_eq!(
            examples,
            [
                Example {
                    input: "1 2\n3 4\n".to_owned(),
                    answers: vec![(Part::One, "11".to_owned()), (Part::Two, "31".to_owned())],
                },
                Example {
                    input: "5".to_owned(),
                    answers: vec![(Part::Two, "7".to_owned())],
                },
            ]
        );
    }

    #[test]
    fn keeps_leading_whitespace_in_a_literal_input() {
        let examples = parsed("[[example]]\ninput = '''\n  #\n ###\n'''\npart1 = \"4\"\n");

        assert_eq!(examples[0].input, "  #\n ###\n");
    }

    #[test]
    fn a_file_with_only_comments_has_no_examples() {
        assert_eq!(parsed("# [[example]]\n# input = '''\n"), []);
    }

    #[test]
    fn rejects_an_example_that_tests_nothing() {
        let message =
            rejected("[[example]]\ninput = \"1\"\npart1 = \"1\"\n\n[[example]]\ninput = \"2\"\n");

        assert_eq!(
            message,
            "data/2026/03/examples.toml: example 2 has no expected answer; give it `part1`, `part2` or both"
        );
    }

    #[test]
    fn points_at_the_line_of_a_mistake() {
        let message = rejected("[[example]]\ninput = \"1\"\npart_1 = \"1\"\n");

        assert!(
            message.starts_with("data/2026/03/examples.toml:3: unknown field `part_1`"),
            "{message}"
        );
    }

    #[test]
    fn a_bare_number_is_the_same_answer_as_its_text() {
        let examples = parsed("[[example]]\ninput = \"1\"\npart1 = 11\npart2 = \"-3\"\n");
        let bare = parsed("[[example]]\ninput = \"1\"\npart1 = \"11\"\npart2 = -3\n");

        assert_eq!(examples, bare);
        assert_eq!(
            examples[0].answers,
            [(Part::One, "11".to_owned()), (Part::Two, "-3".to_owned())]
        );
    }

    #[test]
    fn rejects_an_answer_that_is_neither_text_nor_a_whole_number() {
        let message = rejected("[[example]]\ninput = \"1\"\npart1 = 1.5\n");

        assert_eq!(
            message,
            "data/2026/03/examples.toml:3: an answer is text or a whole number"
        );
    }

    #[test]
    fn a_day_without_an_examples_file_has_no_examples() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        let puzzle = PuzzleId::new(
            elfcode_core::Year::new(2026).unwrap(),
            elfcode_core::Day::new(3).unwrap(),
        )
        .unwrap();

        assert_eq!(load(root, puzzle).unwrap(), []);
    }
}
