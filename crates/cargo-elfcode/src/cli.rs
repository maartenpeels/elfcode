use clap::{Args, Parser, Subcommand};
use elfcode_core::{Day, Part, Year};

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
    /// Run a day against its puzzle input
    Run(RunArgs),
}

#[derive(Debug, Args)]
pub struct LoginArgs {
    /// The value of the `session` cookie from adventofcode.com
    pub session: Session,
}

#[derive(Debug, Args)]
pub struct RunArgs {
    pub day: Day,

    /// Defaults to the latest event
    #[arg(long, short)]
    pub year: Option<Year>,

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

    use super::*;

    fn run_args(args: &[&str]) -> Result<RunArgs, clap::Error> {
        let command_line = ["cargo", "elfcode", "run"].iter().chain(args);
        Cargo::try_parse_from(command_line).map(|Cargo::Elfcode(cli)| match cli.command {
            Command::Run(args) => args,
            other @ Command::Login(_) => panic!("parsed as {other:?}"),
        })
    }

    #[test]
    fn definition_is_consistent() {
        Cargo::command().debug_assert();
    }

    #[test]
    fn day_is_the_only_required_argument() {
        let args = run_args(&["3"]).unwrap();

        assert_eq!(args.day, Day::new(3).unwrap());
        assert_eq!((args.year, args.part, args.debug), (None, None, false));
    }

    #[test]
    fn parses_year_and_part_into_domain_types() {
        let args = run_args(&["03", "--year", "2024", "--part", "2", "--debug"]).unwrap();

        assert_eq!(args.year, Some(Year::new(2024).unwrap()));
        assert_eq!(args.part, Some(Part::Two));
        assert!(args.debug);
    }

    #[test]
    fn rejects_a_day_that_does_not_exist_with_the_reason() {
        let error = run_args(&["26"]).unwrap_err().to_string();

        assert!(error.contains("day 26 is not between 1 and 25"), "{error}");
    }

    #[test]
    fn login_takes_the_session_cookie() {
        let token = "53616c7465645f5f0123456789abcdef";

        let Cargo::Elfcode(cli) =
            Cargo::try_parse_from(["cargo", "elfcode", "login", token]).unwrap();

        assert!(
            matches!(cli.command, Command::Login(args) if args.session.cookie() == format!("session={token}"))
        );
    }

    #[test]
    fn login_explains_what_a_session_cookie_looks_like() {
        let error = Cargo::try_parse_from(["cargo", "elfcode", "login", "my password"])
            .unwrap_err()
            .to_string();

        assert!(error.contains("hexadecimal"), "{error}");
    }

    #[test]
    fn requires_a_day() {
        assert!(run_args(&[]).is_err());
    }
}
