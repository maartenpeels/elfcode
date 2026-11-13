//! Domain types shared by every crate in the framework.
//!
//! Each type validates on construction, so a value that exists is a value
//! that is valid.

mod day;
mod part;
mod puzzle;
mod year;

pub use day::{Day, InvalidDay, ParseDayError};
pub use part::{InvalidPart, ParsePartError, Part};
pub use puzzle::{DayNotInEvent, PuzzleId};
pub use year::{InvalidYear, ParseYearError, Year};
