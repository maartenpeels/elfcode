use elfcode::prelude::*;

pub struct Day12;

impl Solution for Day12 {
    type Input<'a> = Vec<&'a str>;

    fn parse(raw: &str) -> Result<Self::Input<'_>> {
        Ok(raw.lines().collect())
    }

    fn part1(input: &Self::Input<'_>) -> Result<impl Answer> {
        Ok(input.len())
    }

    fn part2(input: &Self::Input<'_>) -> Result<impl Answer> {
        input
            .iter()
            .copied()
            .max_by_key(|line| line.len())
            .context("input has no lines")
    }
}
