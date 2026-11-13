use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::{Parts, PuzzleId};

/// What the CLI asks the harness to run. Days are run in the given order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub parts: Parts,
    pub days: Vec<DayRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayRequest {
    pub puzzle: PuzzleId,
    /// The harness reads this file itself, so it needs no knowledge of
    /// where puzzle data is kept.
    pub input: Utf8PathBuf,
}
