//! The line protocol between the harness and the CLI: one JSON object per
//! line, each carrying the protocol version it was written with.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::DayReport;

/// Bump on any change to the report types that an older reader would
/// misread. The two sides are released separately and can drift.
pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Serialize)]
struct Envelope<'a> {
    protocol: u32,
    #[serde(flatten)]
    report: &'a DayReport,
}

#[derive(Deserialize)]
struct Header {
    protocol: u32,
}

impl DayReport {
    /// Encodes the report as a single line, without a trailing newline.
    ///
    /// # Panics
    ///
    /// Never in practice: a report contains nothing JSON cannot represent.
    #[must_use]
    pub fn to_line(&self) -> String {
        let envelope = Envelope {
            protocol: PROTOCOL_VERSION,
            report: self,
        };
        serde_json::to_string(&envelope)
            .expect("a report holds only strings, numbers and string-keyed maps")
    }

    /// # Errors
    ///
    /// Fails when the line was written with another protocol version, or is
    /// not a valid report.
    pub fn from_line(line: &str) -> Result<Self, DecodeError> {
        let Header { protocol } = serde_json::from_str(line)?;
        if protocol != PROTOCOL_VERSION {
            return Err(DecodeError::UnsupportedProtocol { found: protocol });
        }
        Ok(serde_json::from_str(line)?)
    }
}

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("report uses protocol {found}, this version understands {PROTOCOL_VERSION}")]
    UnsupportedProtocol { found: u32 },
    #[error("report is malformed")]
    Malformed(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::{Day, DayOutcome, Failure, FailureKind, PartOutcome, PuzzleId, Year};

    fn puzzle(year: u16, day: u8) -> PuzzleId {
        PuzzleId::new(Year::try_from(year).unwrap(), Day::try_from(day).unwrap()).unwrap()
    }

    fn solved_part1_only() -> DayReport {
        DayReport {
            puzzle: puzzle(2026, 3),
            outcome: DayOutcome::Parsed {
                parse_time: Duration::from_nanos(112_400),
                part1: Some(PartOutcome::Solved {
                    answer: "6".to_owned(),
                    time: Duration::from_nanos(18_200),
                }),
                part2: Some(PartOutcome::NotImplemented),
            },
        }
    }

    #[test]
    fn encodes_to_a_stable_line() {
        assert_eq!(
            solved_part1_only().to_line(),
            concat!(
                r#"{"protocol":1,"puzzle":{"year":2026,"day":3},"#,
                r#""outcome":{"status":"parsed","parse_time":{"secs":0,"nanos":112400},"#,
                r#""part1":{"status":"solved","answer":"6","time":{"secs":0,"nanos":18200}},"#,
                r#""part2":{"status":"not_implemented"}}}"#,
            )
        );
    }

    #[test]
    fn round_trips_every_outcome() {
        let failure = |kind| Failure {
            kind,
            message: "line 4: unexpected `x`".to_owned(),
        };
        let reports = [
            solved_part1_only(),
            DayReport {
                puzzle: puzzle(2015, 25),
                outcome: DayOutcome::ParseFailed {
                    failure: failure(FailureKind::Error),
                },
            },
            DayReport {
                puzzle: puzzle(2025, 12),
                outcome: DayOutcome::Parsed {
                    parse_time: Duration::from_secs(2),
                    part1: None,
                    part2: Some(PartOutcome::Failed {
                        failure: failure(FailureKind::Panic),
                    }),
                },
            },
        ];

        for report in reports {
            assert_eq!(DayReport::from_line(&report.to_line()).unwrap(), report);
        }
    }

    #[test]
    fn rejects_another_protocol_version_before_reading_the_body() {
        let error = DayReport::from_line(r#"{"protocol":2,"shape":"unknown"}"#).unwrap_err();

        assert!(matches!(
            error,
            DecodeError::UnsupportedProtocol { found: 2 }
        ));
    }

    #[test]
    fn rejects_a_day_the_event_does_not_have() {
        let line = solved_part1_only()
            .to_line()
            .replace(r#""year":2026,"day":3"#, r#""year":2026,"day":13"#);

        assert!(matches!(
            DayReport::from_line(&line),
            Err(DecodeError::Malformed(_))
        ));
    }

    #[test]
    fn rejects_text_that_is_not_a_report() {
        assert!(matches!(
            DayReport::from_line("warning: unused variable"),
            Err(DecodeError::Malformed(_))
        ));
    }
}
