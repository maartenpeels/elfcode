use elfcode::prelude::*;

pub struct Day01;

impl Solution for Day01 {
    type Input<'a> = Vec<u64>;

    fn parse(raw: &str) -> Result<Self::Input<'_>> {
        raw.lines()
            .map(|line| line.parse().with_context(|| format!("`{line}` is not a number")))
            .collect()
    }

    fn part1(input: &Self::Input<'_>) -> Result<impl Answer> {
        println!("summing {} numbers", input.len());
        Ok(input.iter().sum::<u64>())
    }

    fn part2(_input: &Self::Input<'_>) -> Result<impl Answer> {
        Ok(Unsolved)
    }
}
