//! Sending answers: what the site says back, and the local log that stops
//! answers which cannot be right from being sent at all.

mod history;
mod response;

pub use history::{History, HistoryError, Refusal};
pub use response::{Hint, Response, UnrecognizedResponse};
