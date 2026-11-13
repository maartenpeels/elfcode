use std::{fs, io};

use camino::{Utf8Path, Utf8PathBuf};
use elfcode_core::{Part, PuzzleId, layout};
use jiff::Timestamp;
use jiff::civil::Date;
use thiserror::Error;

use crate::event;
use crate::gitignore;
use crate::project::Project;
use crate::puzzle_page::{self, NoDescription};
use crate::session::{Session, SessionStore, StoreError};
use crate::site::{self, Site, SiteError};
use crate::submission::{History, HistoryError, Refusal, Response, UnrecognizedResponse};

/// A project's puzzles as far as they come from the site: files downloaded
/// on first use and then served from disk forever, as the site asks, and
/// answers sent only after the local history allows it.
pub struct PuzzleData<'a, S> {
    project: &'a Project,
    site: &'a S,
    sessions: &'a SessionStore,
    now: Timestamp,
}

#[derive(Debug, Error)]
pub enum DataError {
    #[error(
        "{} day {} unlocks on {date} at midnight US Eastern; there is nothing to download yet",
        .puzzle.year(),
        .puzzle.day()
    )]
    NotUnlocked { puzzle: PuzzleId, date: Date },
    #[error(
        "{file} has not been downloaded yet and you are not logged in; run `cargo elfcode login <cookie>` first"
    )]
    NotLoggedIn { file: Utf8PathBuf },
    #[error("submitting needs a login; run `cargo elfcode login <cookie>` first")]
    NotLoggedInToSubmit,
    #[error(transparent)]
    Session(#[from] StoreError),
    #[error(transparent)]
    Site(#[from] SiteError),
    #[error(transparent)]
    NoDescription(#[from] NoDescription),
    #[error(transparent)]
    UnrecognizedResponse(#[from] UnrecognizedResponse),
    #[error(transparent)]
    History(#[from] HistoryError),
    #[error("could not write {path}")]
    Write {
        path: Utf8PathBuf,
        source: io::Error,
    },
}

/// What became of an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Submitted {
    /// Not sent: the history shows it cannot be right, or must wait.
    Refused(Refusal),
    Answered(Response),
}

impl<'a, S: Site> PuzzleData<'a, S> {
    pub fn new(
        project: &'a Project,
        site: &'a S,
        sessions: &'a SessionStore,
        now: Timestamp,
    ) -> Self {
        Self {
            project,
            site,
            sessions,
            now,
        }
    }

    pub fn project(&self) -> &Project {
        self.project
    }

    /// Today where puzzles unlock.
    pub fn today(&self) -> Date {
        event::eastern_date(self.now)
    }

    /// The absolute path of the puzzle input, downloading it when it is not
    /// on disk yet. A cached input needs neither network nor login.
    pub fn input(&self, puzzle: PuzzleId) -> Result<Utf8PathBuf, DataError> {
        let file = layout::input_file(puzzle);
        if self.project.path(&file).is_file() {
            return Ok(self.project.path(file));
        }

        self.download(puzzle, &file, |session| {
            let session = session.ok_or_else(|| DataError::NotLoggedIn { file: file.clone() })?;
            Ok(self.site.input(&session, puzzle)?)
        })
    }

    /// The absolute path of the puzzle text as markdown, downloading it when
    /// it is not on disk yet. Without a login the site shows part 1 only.
    pub fn puzzle_text(&self, puzzle: PuzzleId) -> Result<Utf8PathBuf, DataError> {
        let file = layout::puzzle_file(puzzle);
        if self.project.path(&file).is_file() {
            return Ok(self.project.path(file));
        }

        self.refresh_puzzle_text(puzzle)
    }

    /// Downloads the puzzle text again, replacing the file only once the new
    /// text is in hand. The one time refetching is right: after part 1 is
    /// solved the page gains part 2.
    pub fn refresh_puzzle_text(&self, puzzle: PuzzleId) -> Result<Utf8PathBuf, DataError> {
        self.download(puzzle, &layout::puzzle_file(puzzle), |session| {
            let page = self.site.puzzle_page(session.as_ref(), puzzle)?;
            Ok(puzzle_page::to_markdown(&page, &site::puzzle_url(puzzle))?)
        })
    }

    /// Sends an answer unless the history of the day rules it out, and
    /// records what the site says.
    pub fn submit(
        &self,
        puzzle: PuzzleId,
        part: Part,
        answer: &str,
    ) -> Result<Submitted, DataError> {
        let mut history = History::load(self.project, puzzle)?;
        if let Err(refusal) = history.check(part, answer, self.now) {
            return Ok(Submitted::Refused(refusal));
        }

        let session = self
            .sessions
            .load()?
            .ok_or(DataError::NotLoggedInToSubmit)?;
        let page = self.site.submit(&session, puzzle, part, answer)?;
        let response = Response::parse(&page)?;

        history.record(part, answer, self.now, response);
        history.save(self.project, puzzle)?;
        Ok(Submitted::Answered(response))
    }

    fn download(
        &self,
        puzzle: PuzzleId,
        file: &Utf8Path,
        download: impl FnOnce(Option<Session>) -> Result<String, DataError>,
    ) -> Result<Utf8PathBuf, DataError> {
        let unlocks = event::unlock_date(puzzle);
        if self.today() < unlocks {
            return Err(DataError::NotUnlocked {
                puzzle,
                date: unlocks,
            });
        }

        let contents = download(self.sessions.load()?)?;
        self.save(file, &contents)
            .map_err(|source| DataError::Write {
                path: file.to_owned(),
                source,
            })?;
        Ok(self.project.path(file))
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
    use std::cell::{Cell, RefCell};

    use jiff::civil::date;
    use tempfile::TempDir;

    use super::*;

    /// Serves the same input and puzzle page for every puzzle, answers
    /// submissions with the page set in `answer_page`, and counts requests.
    #[derive(Default)]
    pub struct FakeSite {
        pub downloads: Cell<usize>,
        pub page_requests: Cell<usize>,
        pub page_sessions: Cell<usize>,
        pub answer_page: Cell<&'static str>,
        pub submitted: RefCell<Vec<(Part, String)>>,
    }

    impl FakeSite {
        pub const INPUT: &str = "a\nb\nc\n";
        pub const PAGE: &str =
            r#"<article class="day-desc"><h2>--- Day 3: Test ---</h2><p>Count.</p></article>"#;
        pub const CORRECT: &str = "<main><article><p>That's the right answer!</p></article></main>";
        pub const TOO_HIGH: &str = "<main><article><p>That's not the right answer; \
            your answer is too high. Please wait one minute before trying again.</p></article></main>";
    }

    impl Site for FakeSite {
        fn check_session(&self, _session: &Session) -> Result<(), SiteError> {
            Ok(())
        }

        fn input(&self, _session: &Session, _puzzle: PuzzleId) -> Result<String, SiteError> {
            self.downloads.set(self.downloads.get() + 1);
            Ok(Self::INPUT.to_owned())
        }

        fn puzzle_page(
            &self,
            session: Option<&Session>,
            _puzzle: PuzzleId,
        ) -> Result<String, SiteError> {
            self.page_requests.set(self.page_requests.get() + 1);
            if session.is_some() {
                self.page_sessions.set(self.page_sessions.get() + 1);
            }
            Ok(Self::PAGE.to_owned())
        }

        fn submit(
            &self,
            _session: &Session,
            _puzzle: PuzzleId,
            part: Part,
            answer: &str,
        ) -> Result<String, SiteError> {
            self.submitted.borrow_mut().push((part, answer.to_owned()));
            Ok(self.answer_page.get().to_owned())
        }
    }

    /// An empty project and a session store in a scratch directory, on a
    /// day when every puzzle the tests use has unlocked.
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
            self.data_on(date(2026, 12, 25))
        }

        pub fn data_on(&self, today: Date) -> PuzzleData<'_, FakeSite> {
            self.data_at(event::noon_on(today))
        }

        pub fn data_at(&self, now: Timestamp) -> PuzzleData<'_, FakeSite> {
            PuzzleData::new(&self.project, &self.site, &self.sessions, now)
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
    fn requests_nothing_before_the_puzzle_unlocks() {
        let scratch = Scratch::logged_in();

        let error = scratch
            .data_on(jiff::civil::date(2026, 12, 2))
            .input(puzzle())
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "2026 day 3 unlocks on 2026-12-03 at midnight US Eastern; there is nothing to download yet"
        );
        assert_eq!(scratch.site.downloads.get(), 0);
    }

    #[test]
    fn downloads_on_the_day_the_puzzle_unlocks() {
        let scratch = Scratch::logged_in();

        scratch
            .data_on(jiff::civil::date(2026, 12, 3))
            .input(puzzle())
            .unwrap();

        assert_eq!(scratch.site.downloads.get(), 1);
    }

    #[test]
    fn converts_the_puzzle_page_to_markdown() {
        let scratch = Scratch::logged_in();

        let path = scratch.data().puzzle_text(puzzle()).unwrap();

        assert_eq!(path, scratch.project.path("data/2026/03/puzzle.md"));
        assert!(
            scratch
                .read("data/2026/03/puzzle.md")
                .ends_with("\n\n## --- Day 3: Test ---\n\nCount.\n")
        );
        assert_eq!(scratch.site.page_sessions.get(), 1);
    }

    #[test]
    fn fetches_the_puzzle_text_without_a_login() {
        let scratch = Scratch::logged_out();

        scratch.data().puzzle_text(puzzle()).unwrap();

        assert_eq!(scratch.site.page_requests.get(), 1);
        assert_eq!(scratch.site.page_sessions.get(), 0);
    }

    #[test]
    fn never_fetches_the_puzzle_text_twice() {
        let scratch = Scratch::logged_in();

        scratch.data().puzzle_text(puzzle()).unwrap();
        scratch.data().puzzle_text(puzzle()).unwrap();

        assert_eq!(scratch.site.page_requests.get(), 1);
    }

    #[test]
    fn refreshing_replaces_the_puzzle_text() {
        let scratch = Scratch::logged_in();
        scratch.write("data/2026/03/puzzle.md", "part 1 only\n");

        scratch.data().refresh_puzzle_text(puzzle()).unwrap();

        assert!(
            scratch
                .read("data/2026/03/puzzle.md")
                .contains("## --- Day 3: Test ---")
        );
    }

    #[test]
    fn records_what_the_site_says_to_an_answer() {
        let scratch = Scratch::logged_in();
        scratch.site.answer_page.set(FakeSite::TOO_HIGH);

        let submitted = scratch.data().submit(puzzle(), Part::One, "100").unwrap();

        assert!(matches!(
            submitted,
            Submitted::Answered(Response::Incorrect { .. })
        ));
        assert!(
            scratch
                .read("data/2026/03/submissions.toml")
                .contains("answer = \"100\"\n")
        );
    }

    #[test]
    fn sends_nothing_the_history_rules_out() {
        let scratch = Scratch::logged_in();
        scratch.site.answer_page.set(FakeSite::TOO_HIGH);
        scratch.data().submit(puzzle(), Part::One, "100").unwrap();

        let an_hour_later =
            event::noon_on(jiff::civil::date(2026, 12, 25)) + jiff::SignedDuration::from_hours(1);
        let submitted = scratch
            .data_at(an_hour_later)
            .submit(puzzle(), Part::One, "120")
            .unwrap();

        assert!(matches!(
            submitted,
            Submitted::Refused(Refusal::OutOfBounds { .. })
        ));
        assert_eq!(scratch.site.submitted.borrow().len(), 1);
    }

    #[test]
    fn submitting_needs_a_login() {
        let scratch = Scratch::logged_out();

        let error = scratch.data().submit(puzzle(), Part::One, "7").unwrap_err();

        assert!(matches!(error, DataError::NotLoggedInToSubmit));
        assert!(scratch.site.submitted.borrow().is_empty());
    }

    #[test]
    fn makes_git_ignore_what_it_downloads() {
        let scratch = Scratch::logged_in();

        scratch.data().input(puzzle()).unwrap();

        assert!(scratch.read(".gitignore").contains("/data/**/input.txt\n"));
    }
}
