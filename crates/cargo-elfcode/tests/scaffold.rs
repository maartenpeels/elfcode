//! Scaffolds a day in a brand-new solutions crate and runs it, proving the
//! templates compile and the examples template is valid.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

const WORKSPACE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

/// A fresh solutions crate depending on this workspace's crates, plus an
/// empty config directory so no real session is seen and nothing is
/// downloaded.
struct NewCrate {
    dir: TempDir,
    config: TempDir,
}

impl NewCrate {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let workspace = Path::new(WORKSPACE).canonicalize().unwrap();
        let manifest = format!(
            r#"[package]
name = "fresh-solutions"
version = "0.1.0"
edition = "2024"
publish = false

[dependencies]
elfcode = {{ path = '{elfcode}' }}

[build-dependencies]
elfcode-build = {{ path = '{build}' }}

[workspace]
"#,
            elfcode = workspace.join("crates/elfcode").display(),
            build = workspace.join("crates/elfcode-build").display(),
        );

        fs::write(dir.path().join("Cargo.toml"), manifest).unwrap();
        fs::write(
            dir.path().join("build.rs"),
            "fn main() {\n    elfcode_build::generate();\n}\n",
        )
        .unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/main.rs"), "elfcode::main!();\n").unwrap();

        Self {
            dir,
            config: tempfile::tempdir().unwrap(),
        }
    }

    fn write(&self, file: &str, contents: &str) {
        let path = self.dir.path().join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn read(&self, file: &str) -> String {
        fs::read_to_string(self.dir.path().join(file)).unwrap()
    }

    fn cargo_elfcode(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_cargo-elfcode"))
            .arg("elfcode")
            .args(args)
            .current_dir(self.dir.path())
            .env("XDG_CONFIG_HOME", self.config.path())
            .env("APPDATA", self.config.path())
            // Shared between runs, so dependencies are compiled only once.
            .env("CARGO_TARGET_DIR", Path::new(WORKSPACE).join("target/e2e"))
            .output()
            .unwrap()
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn a_scaffolded_day_builds_and_runs() {
    let project = NewCrate::new();
    project.write("data/2015/07/input.txt", "some\ninput\n");

    let scaffold = project.cargo_elfcode(&["scaffold", "7", "--year", "2015"]);

    assert!(scaffold.status.success(), "{}", text(&scaffold.stderr));
    assert_eq!(
        text(&scaffold.stdout),
        "\
2015 · day 07
  src/y2015/day07.rs          created
  data/2015/07/examples.toml  created
  data/2015/07/input.txt      already there
"
    );

    let run = project.cargo_elfcode(&["run", "7", "--year", "2015", "--debug"]);

    let stdout = text(&run.stdout);
    assert!(run.status.success(), "{}", text(&run.stderr));
    assert!(
        stdout.contains("  part 1     not implemented\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("  part 2     not implemented\n"),
        "{stdout}"
    );
}

#[test]
fn scaffolding_again_changes_nothing() {
    let project = NewCrate::new();
    project.write("src/y2015/day07.rs", "// my work\n");

    let output = project.cargo_elfcode(&["scaffold", "7", "--year", "2015"]);

    let stdout = text(&output.stdout);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(
        stdout.contains("  src/y2015/day07.rs          already there\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("  data/2015/07/input.txt      not downloaded: "),
        "{stdout}"
    );
    assert_eq!(project.read("src/y2015/day07.rs"), "// my work\n");
}
