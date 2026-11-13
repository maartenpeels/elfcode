use crate::Answer;

/// One puzzle day.
///
/// `parse` runs once and both parts borrow its result, so work the parts
/// share belongs there.
pub trait Solution {
    /// The parsed puzzle input. It may borrow from the raw text.
    type Input<'a>;

    /// # Errors
    ///
    /// Fails when the raw input does not have the expected shape.
    fn parse(raw: &str) -> anyhow::Result<Self::Input<'_>>;

    /// # Errors
    ///
    /// Fails when the input has no answer.
    fn part1(input: &Self::Input<'_>) -> anyhow::Result<impl Answer>;

    /// # Errors
    ///
    /// Fails when the input has no answer.
    fn part2(input: &Self::Input<'_>) -> anyhow::Result<impl Answer>;
}
