use std::sync::LazyLock;

use jiff::SignedDuration;
use scraper::{Html, Selector};
use thiserror::Error;

static MESSAGE: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("main article").expect("the selector is valid"));

/// What the site says to a submitted answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Response {
    Correct,
    Incorrect {
        hint: Option<Hint>,
        /// How long the site asks to wait before the next answer.
        wait: Option<SignedDuration>,
    },
    /// The answer was not judged: an earlier one was too recent.
    RateLimited {
        wait: SignedDuration,
    },
    /// The part is already solved, or is not the one the site expects next.
    WrongLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hint {
    TooHigh,
    TooLow,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("adventofcode.com answered in a way this version does not understand: \"{0}\"")]
pub struct UnrecognizedResponse(pub String);

impl Response {
    /// Reads the response page of an answer submission.
    pub fn parse(html: &str) -> Result<Self, UnrecognizedResponse> {
        let document = Html::parse_document(html);
        let text: String = document
            .select(&MESSAGE)
            .flat_map(|article| article.text())
            .collect();
        let message = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let unrecognized = || UnrecognizedResponse(message.clone());

        if message.contains("That's the right answer") {
            Ok(Self::Correct)
        } else if message.contains("That's not the right answer") {
            let hint = if message.contains("your answer is too high") {
                Some(Hint::TooHigh)
            } else if message.contains("your answer is too low") {
                Some(Hint::TooLow)
            } else {
                None
            };
            Ok(Self::Incorrect {
                hint,
                wait: wait_after_wrong_answer(&message),
            })
        } else if message.contains("You gave an answer too recently") {
            let wait = time_left(&message).ok_or_else(unrecognized)?;
            Ok(Self::RateLimited { wait })
        } else if message.contains("You don't seem to be solving the right level") {
            Ok(Self::WrongLevel)
        } else {
            Err(unrecognized())
        }
    }
}

/// Reads "Please wait one minute before trying again" and the like, which
/// starts a sentence or follows a comma.
fn wait_after_wrong_answer(message: &str) -> Option<SignedDuration> {
    let message = message.to_lowercase();
    let (_, rest) = message.split_once("please wait ")?;
    let mut words = rest.split(' ');
    let amount = match words.next()? {
        "one" => 1,
        number => number.parse().ok()?,
    };

    match words.next()?.trim_end_matches(['.', ',']) {
        "second" | "seconds" => Some(SignedDuration::from_secs(amount)),
        "minute" | "minutes" => Some(SignedDuration::from_mins(amount)),
        "hour" | "hours" => Some(SignedDuration::from_hours(amount)),
        _ => None,
    }
}

/// Reads "You have 1m 30s left to wait".
fn time_left(message: &str) -> Option<SignedDuration> {
    let (_, rest) = message.split_once("You have ")?;
    let (amount, _) = rest.split_once(" left to wait")?;

    amount
        .split(' ')
        .try_fold(SignedDuration::ZERO, |total, part| {
            let unit = part.chars().last()?;
            let value: i64 = part[..part.len() - 1].parse().ok()?;
            let part = match unit {
                'h' => SignedDuration::from_hours(value),
                'm' => SignedDuration::from_mins(value),
                's' => SignedDuration::from_secs(value),
                _ => return None,
            };
            total.checked_add(part)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wraps a message the way the site does.
    fn page(message: &str) -> String {
        format!(
            "<!DOCTYPE html><html><body><header><h1>Advent of Code</h1></header>\n\
             <main>\n<article><p>{message}</p></article>\n</main></body></html>"
        )
    }

    #[test]
    fn reads_the_already_solved_response_captured_from_the_site() {
        let html = page(
            "You don't seem to be solving the right level.  Did you already complete it? \
             <a href=\"/2025/day/1\">[Return to Day 1]</a>",
        );

        assert_eq!(Response::parse(&html), Ok(Response::WrongLevel));
    }

    #[test]
    fn reads_a_correct_answer() {
        let html = page(
            "That's the right answer!  You are <span class=\"day-success\">one gold star</span> \
             closer to finishing the decorations. <a href=\"/2025/day/3#part2\">[Continue to Part Two]</a>",
        );

        assert_eq!(Response::parse(&html), Ok(Response::Correct));
    }

    #[test]
    fn reads_a_wrong_answer_with_its_hint_and_wait() {
        let html = |hint: &str| {
            page(&format!(
                "That's not the right answer{hint}.  If you're stuck, make sure you're using the \
                 full input data; there are also some general tips on the \
                 <a href=\"/2025/about\">about page</a>.  Please wait one minute before trying \
                 again. <a href=\"/2025/day/3\">[Return to Day 3]</a>"
            ))
        };
        let wrong = |hint| {
            Ok(Response::Incorrect {
                hint,
                wait: Some(SignedDuration::from_mins(1)),
            })
        };

        assert_eq!(
            Response::parse(&html("; your answer is too high")),
            wrong(Some(Hint::TooHigh))
        );
        assert_eq!(
            Response::parse(&html("; your answer is too low")),
            wrong(Some(Hint::TooLow))
        );
        assert_eq!(Response::parse(&html("")), wrong(None));
    }

    #[test]
    fn reads_a_longer_wait_after_repeated_wrong_answers() {
        let html = page(
            "That's not the right answer.  Because you have guessed incorrectly 6 times on this \
             puzzle, please wait 5 minutes before trying again.",
        );

        assert_eq!(
            Response::parse(&html),
            Ok(Response::Incorrect {
                hint: None,
                wait: Some(SignedDuration::from_mins(5)),
            })
        );
    }

    #[test]
    fn reads_how_long_a_rate_limit_lasts() {
        let html = |left: &str| {
            page(&format!(
                "You gave an answer too recently; you have to wait after submitting an answer \
                 before trying again.  You have {left} left to wait. \
                 <a href=\"/2025/day/3\">[Return to Day 3]</a>"
            ))
        };

        assert_eq!(
            Response::parse(&html("39s")),
            Ok(Response::RateLimited {
                wait: SignedDuration::from_secs(39)
            })
        );
        assert_eq!(
            Response::parse(&html("4m 2s")),
            Ok(Response::RateLimited {
                wait: SignedDuration::from_secs(242)
            })
        );
    }

    #[test]
    fn shows_the_message_it_does_not_understand() {
        let html = page("The elves are  on\nstrike.");

        assert_eq!(
            Response::parse(&html),
            Err(UnrecognizedResponse("The elves are on strike.".to_owned()))
        );
    }
}
