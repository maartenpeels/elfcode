use std::ffi::OsString;
use std::process::Command;
use std::{env, io};

use camino::{Utf8Path, Utf8PathBuf};
use thiserror::Error;

/// The solutions crate the command was run in.
#[derive(Debug, Clone)]
pub struct Project {
    root: Utf8PathBuf,
}

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("could not run cargo")]
    Cargo(#[source] io::Error),
    #[error("not inside a cargo project: {0}")]
    NotFound(String),
    #[error("the project path is not valid UTF-8")]
    NonUtf8Path,
}

impl Project {
    pub fn at(root: impl Into<Utf8PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Finds the crate that owns the current directory.
    pub fn locate() -> Result<Self, ProjectError> {
        let output = Command::new(cargo())
            .args(["locate-project", "--message-format", "plain"])
            .output()
            .map_err(ProjectError::Cargo)?;

        if !output.status.success() {
            let reason = String::from_utf8_lossy(&output.stderr);
            return Err(ProjectError::NotFound(
                reason.trim().trim_start_matches("error: ").to_owned(),
            ));
        }

        let manifest = String::from_utf8(output.stdout).map_err(|_| ProjectError::NonUtf8Path)?;
        let root = Utf8Path::new(manifest.trim())
            .parent()
            .expect("a manifest path ends in Cargo.toml");

        Ok(Self::at(root))
    }

    pub fn root(&self) -> &Utf8Path {
        &self.root
    }

    /// Resolves a path given relative to the crate root.
    pub fn path(&self, relative: impl AsRef<Utf8Path>) -> Utf8PathBuf {
        self.root.join(relative)
    }
}

/// The cargo that invoked this subcommand, so a `cargo +nightly elfcode`
/// builds the solutions with the same toolchain.
pub fn cargo() -> OsString {
    env::var_os("CARGO").unwrap_or_else(|| "cargo".into())
}
