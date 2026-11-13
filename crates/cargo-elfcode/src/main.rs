//! `cargo elfcode`: drives a solutions crate written with `elfcode`.
//!
//! This binary never links the solutions. It builds and starts them as a
//! separate process, so everything that does not need them keeps working
//! while a day does not compile.

mod cli;
mod commands;
mod data;
mod event;
mod gitignore;
mod harness;
mod project;
mod render;
mod session;
mod site;

use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cargo, Command};
use crate::data::PuzzleData;
use crate::harness::{CargoHarness, Profile};
use crate::project::Project;
use crate::session::SessionStore;
use crate::site::HttpSite;

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
        Command::Login(args) => {
            let store = SessionStore::for_user()?;
            let output = commands::login::execute(&args, &HttpSite::new(), &store)?;
            print!("{output}");
        }
        Command::Run(args) => {
            let project = Project::locate()?;
            let profile = if args.debug {
                eprintln!("warning: debug build, timings are unreliable");
                Profile::Debug
            } else {
                Profile::Release
            };
            let harness = CargoHarness::new(&project, profile);
            let site = HttpSite::new();
            let sessions = SessionStore::for_user()?;
            let data = PuzzleData::new(&project, &site, &sessions);

            let output = commands::run::execute(&args, &data, &harness, event::today())?;
            print!("{output}");
        }
    }

    Ok(())
}
