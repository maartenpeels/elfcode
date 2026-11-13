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
//!
//! A solutions crate puts each day in `src/y{YYYY}/day{DD}.rs`, calls
//! `elfcode_build::generate()` from its build script, and has [`main!`] as
//! its whole `src/main.rs`.

mod answer;
pub mod harness;
pub mod prelude;
mod registry;
mod serve;
mod solution;
pub mod testing;

pub use answer::{Answer, Unsolved};
pub use registry::Entry;
pub use serve::serve;
pub use solution::Solution;

/// Declares the day modules that `elfcode-build` found and a `main` that
/// serves them.
///
/// The modules land at the crate root, so a day is reachable as
/// `crate::y2026::day03`.
#[macro_export]
macro_rules! main {
    () => {
        include!(concat!(env!("OUT_DIR"), "/elfcode.rs"));

        fn main() -> ::std::process::ExitCode {
            $crate::serve(REGISTRY)
        }
    };
}
