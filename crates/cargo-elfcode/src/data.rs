use std::{fs, io};

use camino::{Utf8Path, Utf8PathBuf};
use elfcode_core::{PuzzleId, layout};
use thiserror::Error;

use crate::gitignore;
use crate::project::Project;
use crate::session::{SessionStore, StoreError};
use crate::site::{Site, SiteError};

/// The puzzle files of a project, downloaded on first use and then served
/// from disk forever: the site asks never to fetch the same thing twice.
pub struct PuzzleData<'a, S> {
    project: &'a Project,
    site: &'a S,
    sessions: &'a SessionStore,
}

#[derive(Debug, Error)]
pub enum DataError {
    #[error(
        "{file} has not been downloaded yet and you are not logged in; run `cargo elfcode login <cookie>` first"
    )]
    NotLoggedIn { file: Utf8PathBuf },
    #[error(transparent)]
    Session(#[from] StoreError),
    #[error(transparent)]
    Site(#[from] SiteError),
    #[error("could not write {path}")]
    Write {
        path: Utf8PathBuf,
        source: io::Error,
    },
}

impl<'a, S: Site> PuzzleData<'a, S> {
    pub fn new(project: &'a Project, site: &'a S, sessions: &'a SessionStore) -> Self {
        Self {
            project,
            site,
            sessions,
        }
    }

    /// The absolute path of the puzzle input, downloading it when it is not
    /// on disk yet. A cached input needs neither network nor login.
    pub fn input(&self, puzzle: PuzzleId) -> Result<Utf8PathBuf, DataError> {
        let file = layout::input_file(puzzle);
        let path = self.project.path(&file);
        if path.is_file() {
            return Ok(path);
        }

        let Some(session) = self.sessions.load()? else {
            return Err(DataError::NotLoggedIn { file });
        };
        let input = self.site.input(&session, puzzle)?;

        self.save(&file, &input)
            .map_err(|source| DataError::Write { path: file, source })?;
        Ok(path)
    }

    fn save(&self, file: &Utf8Path, contents: &str) -> io::Result<()> {
        // Before the file exists, so it is never on disk without being
        // ignored.
        gitignore::ensure_downloads_ignored(self.project)?;

        let path = self.project.path(file);
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(path, contents)
    }
}

#[cfg(test)]
pub mod testing {
    use std::cell::Cell;

    use tempfile::TempDir;

    use super::*;
    use crate::session::Session;

    /// Serves the same input for every puzzle and counts the downloads.
    #[derive(Default)]
    pub struct FakeSite {
        pub downloads: Cell<usize>,
    }

    impl FakeSite {
        pub const INPUT: &str = "a\nb\nc\n";
    }

    impl Site for FakeSite {
        fn check_session(&self, _session: &Session) -> Result<(), SiteError> {
            Ok(())
        }

        fn input(&self, _session: &Session, _puzzle: PuzzleId) -> Result<String, SiteError> {
            self.downloads.set(self.downloads.get() + 1);
            Ok(Self::INPUT.to_owned())
        }
    }

    /// An empty project and a session store in a scratch directory.
    pub struct Scratch {
        _dir: TempDir,
        pub project: Project,
        pub sessions: SessionStore,
        pub site: FakeSite,
    }

    impl Scratch {
        pub fn logged_out() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let root = Utf8PathBuf::from_path_buf(dir.path().to_owned()).unwrap();
            fs::create_dir(root.join("project")).unwrap();

            Self {
                project: Project::at(root.join("project")),
                sessions: SessionStore::at(root.join("config/session")),
                site: FakeSite::default(),
                _dir: dir,
            }
        }

        pub fn logged_in() -> Self {
            let scratch = Self::logged_out();
            scratch.sessions.save(&"abc123".parse().unwrap()).unwrap();
            scratch
        }

        pub fn data(&self) -> PuzzleData<'_, FakeSite> {
            PuzzleData::new(&self.project, &self.site, &self.sessions)
        }

        pub fn write(&self, file: &str, contents: &str) {
            let path = self.project.path(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }

        pub fn read(&self, file: &str) -> String {
            fs::read_to_string(self.project.path(file)).unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use elfcode_core::{Day, Year};

    use super::testing::{FakeSite, Scratch};
    use super::*;

    fn puzzle() -> PuzzleId {
        PuzzleId::new(Year::new(2026).unwrap(), Day::new(3).unwrap()).unwrap()
    }

    const INPUT_FILE: &str = "data/2026/03/input.txt";

    #[test]
    fn downloads_a_missing_input_into_the_project() {
        let scratch = Scratch::logged_in();

        let path = scratch.data().input(puzzle()).unwrap();

        assert_eq!(path, scratch.project.path(INPUT_FILE));
        assert_eq!(scratch.read(INPUT_FILE), FakeSite::INPUT);
    }

    #[test]
    fn never_downloads_an_input_twice() {
        let scratch = Scratch::logged_in();

        scratch.data().input(puzzle()).unwrap();
        scratch.data().input(puzzle()).unwrap();

        assert_eq!(scratch.site.downloads.get(), 1);
    }

    #[test]
    fn serves_a_cached_input_without_login_or_network() {
        let scratch = Scratch::logged_out();
        scratch.write(INPUT_FILE, "saved by hand\n");

        scratch.data().input(puzzle()).unwrap();

        assert_eq!(scratch.site.downloads.get(), 0);
        assert_eq!(scratch.read(INPUT_FILE), "saved by hand\n");
    }

    #[test]
    fn asks_for_a_login_when_it_has_to_download() {
        let scratch = Scratch::logged_out();

        let error = scratch.data().input(puzzle()).unwrap_err();

        assert_eq!(
            error.to_string(),
            "data/2026/03/input.txt has not been downloaded yet and you are not logged in; \
             run `cargo elfcode login <cookie>` first"
        );
        assert_eq!(scratch.site.downloads.get(), 0);
    }

    #[test]
    fn makes_git_ignore_what_it_downloads() {
        let scratch = Scratch::logged_in();

        scratch.data().input(puzzle()).unwrap();

        assert!(scratch.read(".gitignore").contains("/data/**/input.txt\n"));
    }
}
