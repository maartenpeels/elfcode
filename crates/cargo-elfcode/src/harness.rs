use std::io::{self, Write};
use std::process::{Command, ExitStatus, Stdio};
use std::{fs, path::PathBuf};

use camino::Utf8PathBuf;
use elfcode_core::{DayReport, DayRequest, DecodeError, Parts, Request};
use tempfile::NamedTempFile;
use thiserror::Error;

use crate::project::{self, Project};

/// Something that can run days of the solutions crate.
///
/// A trait so commands can be tested without building a real crate.
pub trait Harness {
    /// Runs `days` in order and returns one report per day.
    fn run(&self, parts: Parts, days: Vec<DayRequest>) -> Result<Vec<DayReport>, HarnessError>;
}

#[derive(Debug, Error)]
pub enum HarnessError {
    #[error("could not create a file for the reports")]
    CreateReportFile(#[source] io::Error),
    #[error("the temporary directory {} is not valid UTF-8", .0.display())]
    NonUtf8TempPath(PathBuf),
    #[error("could not start cargo")]
    Spawn(#[source] io::Error),
    #[error("could not send the request to the solutions")]
    SendRequest(#[source] io::Error),
    #[error("could not wait for the solutions to finish")]
    Wait(#[source] io::Error),
    #[error("the solutions did not build or run ({0}); see the output above")]
    Failed(ExitStatus),
    #[error("could not read the reports")]
    ReadReports(#[source] io::Error),
    #[error(
        "could not understand a report; `cargo-elfcode` and the `elfcode` dependency may be different versions"
    )]
    DecodeReport(#[from] DecodeError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Release,
    Debug,
}

/// Builds the solutions crate with cargo and runs it as a child process.
pub struct CargoHarness<'a> {
    project: &'a Project,
    profile: Profile,
}

impl<'a> CargoHarness<'a> {
    pub fn new(project: &'a Project, profile: Profile) -> Self {
        Self { project, profile }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(project::cargo());
        command.args(["run", "--quiet"]);
        if self.profile == Profile::Release {
            command.arg("--release");
        }
        command.current_dir(self.project.root());
        command
    }
}

impl Harness for CargoHarness<'_> {
    fn run(&self, parts: Parts, days: Vec<DayRequest>) -> Result<Vec<DayReport>, HarnessError> {
        let report_file = NamedTempFile::new().map_err(HarnessError::CreateReportFile)?;
        let request = Request {
            report_file: Utf8PathBuf::from_path_buf(report_file.path().to_owned())
                .map_err(HarnessError::NonUtf8TempPath)?,
            parts,
            days,
        };

        // Stdout and stderr are inherited: build errors and anything the
        // solutions print go straight to the terminal.
        let mut child = self
            .command()
            .stdin(Stdio::piped())
            .spawn()
            .map_err(HarnessError::Spawn)?;

        let mut stdin = child.stdin.take().expect("stdin was requested as a pipe");
        writeln!(stdin, "{}", request.to_line()).map_err(HarnessError::SendRequest)?;
        drop(stdin);

        let status = child.wait().map_err(HarnessError::Wait)?;
        if !status.success() {
            return Err(HarnessError::Failed(status));
        }

        fs::read_to_string(report_file.path())
            .map_err(HarnessError::ReadReports)?
            .lines()
            .map(|line| Ok(DayReport::from_line(line)?))
            .collect()
    }
}
