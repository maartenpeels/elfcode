use clap::{Args, Parser, Subcommand};
use elfcode_core::{Day, DayNotInEvent, InvalidYear, Part, PuzzleId, Year};
use jiff::civil::Date;
use thiserror::Error;

use crate::event;
use crate::session::Session;

/// Cargo runs a subcommand as `cargo-elfcode elfcode <args>`, so the real
/// interface sits one level down.
#[derive(Debug, Parser)]
#[command(name = "cargo", bin_name = "cargo")]
pub enum Cargo {
    Elfcode(Elfcode),
}

/// Run, test and submit Advent of Code solutions
#[derive(Debug, Args)]
#[command(version)]
pub struct Elfcode {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Verify a session cookie and store it for later commands
    Login(LoginArgs),
    /// Create the files for a new day and download its input
    Scaffold(ScaffoldArgs),
    /// Run a day against its puzzle input
    Run(RunArgs),
}

#[derive(Debug, Args)]
pub struct LoginArgs {
    /// The value of the `session` cookie from adventofcode.com
    pub session: Session,
}

/// The day a command is about.
#[derive(Debug, Clone, Copy, Args)]
pub struct PuzzleArgs {
    pub day: Day,

    /// Defaults to the latest event
    #[arg(long, short)]
    pub year: Option<Year>,
}

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("could not tell the latest event from the system clock; pass --year")]
    Clock(#[source] InvalidYear),
    #[error(transparent)]
    NotInEvent(#[from] DayNotInEvent),
}

impl PuzzleArgs {
    /// The puzzle meant, taking the latest event as of `today` when no year
    /// was given.
    pub fn resolve(self, today: Date) -> Result<PuzzleId, ResolveError> {
        let year = match self.year {
            Some(year) => year,
            None => event::latest(today).map_err(ResolveError::Clock)?,
        };
        Ok(PuzzleId::new(year, self.day)?)
    }
}

#[derive(Debug, Args)]
pub struct ScaffoldArgs {
    #[command(flatten)]
    pub puzzle: PuzzleArgs,
}

#[derive(Debug, Args)]
pub struct RunArgs {
    #[command(flatten)]
    pub puzzle: PuzzleArgs,

    /// Run only this part
    #[arg(long, short)]
    pub part: Option<Part>,

    /// Build without optimizations; timings become unreliable
    #[arg(long)]
    pub debug: bool,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;
    use jiff::civil::date;

    use super::*;

    fn parse(args: &[&str]) -> Result<Command, clap::Error> {
        let command_line = ["cargo", "elfcode"].iter().chain(args);
        Cargo::try_parse_from(command_line).map(|Cargo::Elfcode(cli)| cli.command)
    }

    fn run_args(args: &[&str]) -> Result<RunArgs, clap::Error> {
        let command_line: Vec<_> = ["run"].iter().chain(args).copied().collect();
        parse(&command_line).map(|command| match command {
            Command::Run(args) => args,
            other => panic!("parsed as {other:?}"),
        })
    }

    #[test]
    fn definition_is_consistent() {
        Cargo::command().debug_assert();
    }

    #[test]
    fn day_is_the_only_required_argument() {
        let args = run_args(&["3"]).unwrap();

        assert_eq!(args.puzzle.day, Day::new(3).unwrap());
        assert_eq!(
            (args.puzzle.year, args.part, args.debug),
            (None, None, false)
        );
    }

    #[test]
    fn parses_year_and_part_into_domain_types() {
        let args = run_args(&["03", "--year", "2024", "--part", "2", "--debug"]).unwrap();

        assert_eq!(args.puzzle.year, Some(Year::new(2024).unwrap()));
        assert_eq!(args.part, Some(Part::Two));
        assert!(args.debug);
    }

    #[test]
    fn rejects_a_day_that_does_not_exist_with_the_reason() {
        let error = run_args(&["26"]).unwrap_err().to_string();

        assert!(error.contains("day 26 is not between 1 and 25"), "{error}");
    }

    #[test]
    fn requires_a_day() {
        assert!(run_args(&[]).is_err());
        assert!(parse(&["scaffold"]).is_err());
    }

    #[test]
    fn scaffold_takes_a_day_and_a_year() {
        let command = parse(&["scaffold", "7", "-y", "2015"]).unwrap();

        let Command::Scaffold(args) = command else {
            panic!("parsed as {command:?}");
        };
        assert_eq!(args.puzzle.day, Day::new(7).unwrap());
        assert_eq!(args.puzzle.year, Some(Year::new(2015).unwrap()));
    }

    #[test]
    fn login_takes_the_session_cookie() {
        let token = "53616c7465645f5f0123456789abcdef";

        let command = parse(&["login", token]).unwrap();

        assert!(
            matches!(command, Command::Login(args) if args.session.cookie() == format!("session={token}"))
        );
    }

    #[test]
    fn login_explains_what_a_session_cookie_looks_like() {
        let error = parse(&["login", "my password"]).unwrap_err().to_string();

        assert!(error.contains("hexadecimal"), "{error}");
    }

    fn puzzle_args(day: u8, year: Option<u16>) -> PuzzleArgs {
        PuzzleArgs {
            day: Day::new(day).unwrap(),
            year: year.map(|year| Year::new(year).unwrap()),
        }
    }

    #[test]
    fn resolves_a_missing_year_to_the_latest_event() {
        let puzzle = puzzle_args(3, None).resolve(date(2026, 11, 13)).unwrap();

        assert_eq!(puzzle.year(), Year::new(2025).unwrap());
    }

    #[test]
    fn resolving_checks_the_day_against_the_event() {
        let error = puzzle_args(13, Some(2025))
            .resolve(date(2026, 11, 13))
            .unwrap_err();

        assert!(matches!(error, ResolveError::NotInEvent(_)));
    }
}
