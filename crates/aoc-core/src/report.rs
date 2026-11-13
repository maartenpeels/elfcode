use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::PuzzleId;

/// What running one day produced, as sent from the harness to the CLI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayReport {
    pub puzzle: PuzzleId,
    pub outcome: DayOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DayOutcome {
    /// A part is `None` when it was not requested.
    Parsed {
        parse_time: Duration,
        part1: Option<PartOutcome>,
        part2: Option<PartOutcome>,
    },
    ParseFailed {
        failure: Failure,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PartOutcome {
    Solved { answer: String, time: Duration },
    NotImplemented,
    Failed { failure: Failure },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Failure {
    pub kind: FailureKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    /// The solution returned `Err`.
    Error,
    Panic,
}
