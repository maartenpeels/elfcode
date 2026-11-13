//! Write an Advent of Code day as one [`Solution`] impl.
//!
//! ```
//! use elfcode::prelude::*;
//!
//! pub struct Day03;
//!
//! impl Solution for Day03 {
//!     type Input<'a> = Vec<&'a str>;
//!
//!     fn parse(raw: &str) -> Result<Self::Input<'_>> {
//!         Ok(raw.lines().collect())
//!     }
//!     fn part1(input: &Self::Input<'_>) -> Result<impl Answer> {
//!         Ok(input.len())
//!     }
//!     fn part2(_input: &Self::Input<'_>) -> Result<impl Answer> {
//!         Ok(Unsolved)
//!     }
//! }
//! ```

mod answer;
pub mod harness;
pub mod prelude;
mod registry;
mod serve;
mod solution;

pub use answer::{Answer, Unsolved};
pub use registry::Entry;
pub use serve::serve;
pub use solution::Solution;
