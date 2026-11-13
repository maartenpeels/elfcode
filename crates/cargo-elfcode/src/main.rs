//! `cargo elfcode`: drives a solutions crate written with `elfcode`.
//!
//! This binary never links the solutions. It builds and starts them as a
//! separate process, so everything that does not need them keeps working
//! while a day does not compile.

mod cli;
mod commands;
mod event;
mod harness;
mod project;
mod render;

use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cargo, Command};
use crate::harness::{CargoHarness, Profile};
use crate::project::Project;

fn main() -> ExitCode {
    let Cargo::Elfcode(cli) = Cargo::parse();

    match dispatch(cli.command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn dispatch(command: Command) -> anyhow::Result<()> {
    match command {
        Command::Run(args) => {
            let project = Project::locate()?;
            let profile = if args.debug {
                eprintln!("warning: debug build, timings are unreliable");
                Profile::Debug
            } else {
                Profile::Release
            };
            let harness = CargoHarness::new(&project, profile);

            let output = commands::run::execute(&args, &project, &harness, event::today())?;
            print!("{output}");
        }
    }

    Ok(())
}
