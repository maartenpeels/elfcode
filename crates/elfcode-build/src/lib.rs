//! Build script support for a solutions crate.
//!
//! ```no_run
//! // build.rs
//! fn main() {
//!     elfcode_build::generate();
//! }
//! ```
//!
//! Every `src/y{YYYY}/day{DD}.rs` becomes a module holding a `Day{DD}`
//! solution, and `elfcode::main!()` picks up the generated registry.

#![allow(
    clippy::needless_doctest_main,
    reason = "the example shows a whole build.rs"
)]

mod discover;
mod render;

use std::error::Error;
use std::{env, fs, io, iter, process};

use camino::{Utf8Path, Utf8PathBuf};
use thiserror::Error;

use crate::discover::{DiscoverError, discover};
use crate::render::render;

/// Must match the file name `elfcode::main!` includes.
const GENERATED_FILE: &str = "elfcode.rs";

/// Generates the day modules and registry for the crate being built.
///
/// A file that breaks the naming convention fails the build with a message
/// naming that file.
pub fn generate() {
    if let Err(error) = try_generate() {
        println!("cargo::error={}", chain(&error));
        process::exit(1);
    }
}

#[derive(Debug, Error)]
enum GenerateError {
    #[error("{0} is not set; `generate` must be called from a build script")]
    NotABuildScript(&'static str),
    #[error(transparent)]
    Discover(#[from] DiscoverError),
    #[error("could not write {path}")]
    Write {
        path: Utf8PathBuf,
        source: io::Error,
    },
}

fn try_generate() -> Result<(), GenerateError> {
    let crate_root = env_path("CARGO_MANIFEST_DIR")?;
    let generated = env_path("OUT_DIR")?.join(GENERATED_FILE);

    println!("cargo::rerun-if-changed=src");

    let days = discover(&crate_root)?;
    write(&generated, &render(&crate_root, &days))
}

fn env_path(name: &'static str) -> Result<Utf8PathBuf, GenerateError> {
    env::var(name)
        .map(Utf8PathBuf::from)
        .map_err(|_| GenerateError::NotABuildScript(name))
}

fn write(path: &Utf8Path, contents: &str) -> Result<(), GenerateError> {
    fs::write(path, contents).map_err(|source| GenerateError::Write {
        path: path.to_owned(),
        source,
    })
}

/// `cargo::error` takes a single line, so the causes are joined onto it.
fn chain(error: &dyn Error) -> String {
    iter::successors(Some(error), |&error| error.source())
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(": ")
}
