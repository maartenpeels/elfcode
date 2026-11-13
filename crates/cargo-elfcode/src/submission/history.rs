use std::{fmt, fs, io};

use camino::Utf8PathBuf;
use elfcode_core::{Part, PuzzleId, layout};
use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{Hint, Response};
use crate::project::Project;
use crate::render;

/// Every answer sent for one day, oldest first.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct History {
    #[serde(default, rename = "submission")]
    submissions: Vec<Submission>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Submission {
    part: Part,
    answer: String,
    at: Timestamp,
    verdict: Verdict,
    /// When the site accepts the next answer for this day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    retry_after: Option<Timestamp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Verdict {
    Correct,
    Incorrect,
    TooHigh,
    TooLow,
    /// Not judged at all.
    RateLimited,
    /// Solved before, elsewhere or without a record here.
    AlreadySolved,
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Correct => "correct",
            Self::Incorrect => "wrong",
            Self::TooHigh => "too high",
            Self::TooLow => "too low",
            Self::RateLimited => "not judged",
            Self::AlreadySolved => "already solved",
        })
    }
}

/// Why an answer is not sent.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Refusal {
    #[error(
        "part 1 is not solved yet as far as this project knows; submit it first \
         (if you solved it elsewhere, submitting it again records that)"
    )]
    PartOneUnsolved,
    #[error("part {part} is already solved{}", .answer.as_ref().map(|answer| format!(", the answer was {answer}")).unwrap_or_default())]
    AlreadySolved { part: Part, answer: Option<String> },
    #[error("{answer} was already submitted for part {part} and was {verdict}")]
    AlreadySubmitted {
        part: Part,
        answer: String,
        verdict: String,
    },
    #[error("{answer} cannot be right: {bound} was already {verdict}")]
    OutOfBounds {
        answer: String,
        bound: String,
        verdict: String,
    },
    #[error("the site asks to wait {} more before the next answer", render::wait(*.left))]
    Cooldown { left: SignedDuration },
}

impl History {
    /// Checks an answer against everything sent before, so that nothing is
    /// sent which cannot be right or which the site would refuse.
    pub fn check(&self, part: Part, answer: &str, now: Timestamp) -> Result<(), Refusal> {
        if part == Part::Two && !self.solved(Part::One) {
            return Err(Refusal::PartOneUnsolved);
        }
        if self.solved(part) {
            return Err(Refusal::AlreadySolved {
                part,
                answer: self
                    .of(part)
                    .find(|sent| sent.verdict == Verdict::Correct)
                    .map(|sent| sent.answer.clone()),
            });
        }
        if let Some(sent) = self
            .of(part)
            .find(|sent| sent.answer == answer && sent.verdict != Verdict::RateLimited)
        {
            return Err(Refusal::AlreadySubmitted {
                part,
                answer: answer.to_owned(),
                verdict: sent.verdict.to_string(),
            });
        }
        self.check_bounds(part, answer)?;

        match self.retry_after() {
            Some(retry_after) if now < retry_after => Err(Refusal::Cooldown {
                left: now.duration_until(retry_after),
            }),
            _ => Ok(()),
        }
    }

    fn check_bounds(&self, part: Part, answer: &str) -> Result<(), Refusal> {
        let Ok(value) = answer.parse::<i128>() else {
            return Ok(());
        };

        let crossed = self.of(part).find(|sent| {
            let Ok(sent_value) = sent.answer.parse::<i128>() else {
                return false;
            };
            match sent.verdict {
                Verdict::TooHigh => value >= sent_value,
                Verdict::TooLow => value <= sent_value,
                _ => false,
            }
        });

        match crossed {
            Some(sent) => Err(Refusal::OutOfBounds {
                answer: answer.to_owned(),
                bound: sent.answer.clone(),
                verdict: sent.verdict.to_string(),
            }),
            None => Ok(()),
        }
    }

    /// Records what the site said to `answer`.
    pub fn record(&mut self, part: Part, answer: &str, at: Timestamp, response: Response) {
        let (verdict, wait) = match response {
            Response::Correct => (Verdict::Correct, None),
            Response::Incorrect { hint, wait } => {
                let verdict = match hint {
                    Some(Hint::TooHigh) => Verdict::TooHigh,
                    Some(Hint::TooLow) => Verdict::TooLow,
                    None => Verdict::Incorrect,
                };
                (verdict, wait)
            }
            Response::RateLimited { wait } => (Verdict::RateLimited, Some(wait)),
            Response::WrongLevel => (Verdict::AlreadySolved, None),
        };

        self.submissions.push(Submission {
            part,
            answer: answer.to_owned(),
            at,
            verdict,
            retry_after: wait.map(|wait| at + wait),
        });
    }

    pub fn solved(&self, part: Part) -> bool {
        self.of(part)
            .any(|sent| matches!(sent.verdict, Verdict::Correct | Verdict::AlreadySolved))
    }

    fn of(&self, part: Part) -> impl Iterator<Item = &Submission> {
        self.submissions
            .iter()
            .filter(move |sent| sent.part == part)
    }

    /// The site's wait applies to the whole day, whichever part was sent.
    fn retry_after(&self) -> Option<Timestamp> {
        self.submissions
            .iter()
            .filter_map(|sent| sent.retry_after)
            .max()
    }
}

#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("could not read {path}")]
    Read {
        path: Utf8PathBuf,
        source: io::Error,
    },
    #[error("{path} is damaged: {message}")]
    Damaged { path: Utf8PathBuf, message: String },
    #[error("could not write {path}")]
    Write {
        path: Utf8PathBuf,
        source: io::Error,
    },
}

impl History {
    /// The history of a day; empty when nothing was submitted yet.
    pub fn load(project: &Project, puzzle: PuzzleId) -> Result<Self, HistoryError> {
        let path = layout::submissions_file(puzzle);

        match fs::read_to_string(project.path(&path)) {
            Ok(text) => toml::from_str(&text).map_err(|error| HistoryError::Damaged {
                path,
                message: error.message().to_owned(),
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(HistoryError::Read { path, source }),
        }
    }

    pub fn save(&self, project: &Project, puzzle: PuzzleId) -> Result<(), HistoryError> {
        let path = layout::submissions_file(puzzle);
        let text = toml::to_string(self).expect("a history holds only strings, numbers and times");

        let full = project.path(&path);
        full.parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| fs::write(&full, text))
            .map_err(|source| HistoryError::Write { path, source })
    }
}

#[cfg(test)]
mod tests {
    use elfcode_core::{Day, Year};

    use super::*;

    fn at(seconds: i64) -> Timestamp {
        Timestamp::from_second(1_800_000_000 + seconds).unwrap()
    }

    fn wrong(hint: Option<Hint>) -> Response {
        Response::Incorrect {
            hint,
            wait: Some(SignedDuration::from_mins(1)),
        }
    }

    /// A history built from `(part, answer, response)` sent ten minutes
    /// apart, so no wait is still running afterwards.
    fn history(sent: &[(Part, &str, Response)]) -> History {
        let mut history = History::default();
        for (n, &(part, answer, response)) in (0..).zip(sent) {
            history.record(part, answer, at(n * 600), response);
        }
        history
    }

    fn later() -> Timestamp {
        at(1_000_000)
    }

    #[test]
    fn accepts_anything_for_part_1_of_a_fresh_day() {
        assert_eq!(History::default().check(Part::One, "42", later()), Ok(()));
    }

    #[test]
    fn refuses_part_2_before_part_1_is_known_solved() {
        assert_eq!(
            History::default().check(Part::Two, "42", later()),
            Err(Refusal::PartOneUnsolved)
        );
    }

    #[test]
    fn allows_part_2_once_part_1_is_correct_or_known_solved() {
        for response in [Response::Correct, Response::WrongLevel] {
            let history = history(&[(Part::One, "7", response)]);

            assert_eq!(history.check(Part::Two, "42", later()), Ok(()));
        }
    }

    #[test]
    fn refuses_a_solved_part_and_names_its_answer() {
        let history = history(&[(Part::One, "7", Response::Correct)]);

        let refusal = history.check(Part::One, "8", later()).unwrap_err();

        assert_eq!(
            refusal.to_string(),
            "part 1 is already solved, the answer was 7"
        );
    }

    #[test]
    fn refuses_an_answer_sent_before() {
        let history = history(&[(Part::One, "abc", wrong(None))]);

        let refusal = history.check(Part::One, "abc", later()).unwrap_err();

        assert_eq!(
            refusal.to_string(),
            "abc was already submitted for part 1 and was wrong"
        );
    }

    #[test]
    fn sends_again_an_answer_that_was_never_judged() {
        let history = history(&[(
            Part::One,
            "42",
            Response::RateLimited {
                wait: SignedDuration::from_secs(30),
            },
        )]);

        assert_eq!(history.check(Part::One, "42", later()), Ok(()));
    }

    #[test]
    fn refuses_answers_outside_the_known_bounds() {
        let history = history(&[
            (Part::One, "100", wrong(Some(Hint::TooHigh))),
            (Part::One, "10", wrong(Some(Hint::TooLow))),
        ]);

        assert_eq!(
            history
                .check(Part::One, "150", later())
                .unwrap_err()
                .to_string(),
            "150 cannot be right: 100 was already too high"
        );
        assert_eq!(
            history
                .check(Part::One, "-3", later())
                .unwrap_err()
                .to_string(),
            "-3 cannot be right: 10 was already too low"
        );
        assert_eq!(history.check(Part::One, "55", later()), Ok(()));
    }

    #[test]
    fn keeps_the_bounds_of_each_part_apart() {
        let history = history(&[
            (Part::One, "7", Response::Correct),
            (Part::Two, "100", wrong(Some(Hint::TooHigh))),
        ]);

        assert!(history.check(Part::Two, "150", later()).is_err());
        assert_eq!(
            history.check(Part::One, "150", later()),
            Err(Refusal::AlreadySolved {
                part: Part::One,
                answer: Some("7".to_owned())
            })
        );
    }

    #[test]
    fn does_not_compare_answers_that_are_not_numbers() {
        let history = history(&[(Part::One, "100", wrong(Some(Hint::TooHigh)))]);

        assert_eq!(history.check(Part::One, "ABC", later()), Ok(()));
    }

    #[test]
    fn refuses_while_the_site_asks_to_wait() {
        let mut history = History::default();
        history.record(Part::One, "1", at(0), wrong(None));

        assert_eq!(
            history
                .check(Part::One, "2", at(15))
                .unwrap_err()
                .to_string(),
            "the site asks to wait 45s more before the next answer"
        );
        assert_eq!(history.check(Part::One, "2", at(60)), Ok(()));
    }

    #[test]
    fn a_rate_limit_also_starts_a_wait() {
        let mut history = History::default();
        history.record(
            Part::One,
            "1",
            at(0),
            Response::RateLimited {
                wait: SignedDuration::from_secs(150),
            },
        );

        assert_eq!(
            history.check(Part::One, "1", at(29)),
            Err(Refusal::Cooldown {
                left: SignedDuration::from_secs(121)
            })
        );
    }

    #[test]
    fn counts_a_part_as_solved_after_a_correct_or_wrong_level_response() {
        assert!(!History::default().solved(Part::One));
        assert!(history(&[(Part::One, "7", Response::Correct)]).solved(Part::One));
        assert!(history(&[(Part::One, "7", Response::WrongLevel)]).solved(Part::One));
        assert!(!history(&[(Part::One, "7", wrong(None))]).solved(Part::One));
    }

    fn project() -> (tempfile::TempDir, Project) {
        let dir = tempfile::tempdir().unwrap();
        let project = Project::at(Utf8PathBuf::from_path_buf(dir.path().to_owned()).unwrap());
        (dir, project)
    }

    fn puzzle() -> PuzzleId {
        PuzzleId::new(Year::new(2026).unwrap(), Day::new(3).unwrap()).unwrap()
    }

    #[test]
    fn a_day_without_a_log_has_an_empty_history() {
        let (_dir, project) = project();

        assert_eq!(
            History::load(&project, puzzle()).unwrap(),
            History::default()
        );
    }

    #[test]
    fn saves_a_readable_log_and_loads_it_back() {
        let (_dir, project) = project();
        let mut history = History::default();
        history.record(Part::One, "10", at(0), wrong(Some(Hint::TooLow)));
        history.record(Part::One, "11", at(600), Response::Correct);

        history.save(&project, puzzle()).unwrap();

        let text = fs::read_to_string(project.path("data/2026/03/submissions.toml")).unwrap();
        assert_eq!(
            text,
            r#"[[submission]]
part = 1
answer = "10"
at = "2027-01-15T08:00:00Z"
verdict = "too_low"
retry_after = "2027-01-15T08:01:00Z"

[[submission]]
part = 1
answer = "11"
at = "2027-01-15T08:10:00Z"
verdict = "correct"
"#
        );
        assert_eq!(History::load(&project, puzzle()).unwrap(), history);
    }

    #[test]
    fn names_a_damaged_log() {
        let (_dir, project) = project();
        fs::create_dir_all(project.path("data/2026/03")).unwrap();
        fs::write(
            project.path("data/2026/03/submissions.toml"),
            "[[submission]]\npart = 3\n",
        )
        .unwrap();

        let error = History::load(&project, puzzle()).unwrap_err();

        assert!(
            error
                .to_string()
                .starts_with("data/2026/03/submissions.toml is damaged: "),
            "{error}"
        );
    }
}
