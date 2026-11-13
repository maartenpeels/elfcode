use thiserror::Error;

use crate::cli::LoginArgs;
use crate::session::{SessionStore, StoreError};
use crate::site::{Site, SiteError};

#[derive(Debug, Error)]
pub enum LoginError {
    #[error(transparent)]
    Site(#[from] SiteError),
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// Stores the session once the site has accepted it, so a mistyped or
/// expired cookie never replaces a working one.
pub fn execute(
    args: &LoginArgs,
    site: &impl Site,
    store: &SessionStore,
) -> Result<String, LoginError> {
    site.check_session(&args.session)?;
    store.save(&args.session)?;

    Ok(format!("logged in; session saved to {}\n", store.path()))
}

#[cfg(test)]
mod tests {
    use camino::Utf8PathBuf;
    use elfcode_core::{Part, PuzzleId};
    use tempfile::TempDir;

    use super::*;
    use crate::session::Session;

    struct FakeSite {
        accepts: bool,
    }

    impl Site for FakeSite {
        fn check_session(&self, _session: &Session) -> Result<(), SiteError> {
            if self.accepts {
                Ok(())
            } else {
                Err(SiteError::SessionRejected)
            }
        }

        fn input(&self, _session: &Session, _puzzle: PuzzleId) -> Result<String, SiteError> {
            unreachable!("login downloads nothing")
        }

        fn puzzle_page(
            &self,
            _session: Option<&Session>,
            _puzzle: PuzzleId,
        ) -> Result<String, SiteError> {
            unreachable!("login downloads nothing")
        }

        fn submit(
            &self,
            _session: &Session,
            _puzzle: PuzzleId,
            _part: Part,
            _answer: &str,
        ) -> Result<String, SiteError> {
            unreachable!("login submits nothing")
        }
    }

    fn store() -> (TempDir, SessionStore) {
        let dir = tempfile::tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(dir.path().join("session")).unwrap();
        (dir, SessionStore::at(path))
    }

    fn args(token: &str) -> LoginArgs {
        LoginArgs {
            session: token.parse().unwrap(),
        }
    }

    #[test]
    fn saves_an_accepted_session_and_says_where() {
        let (_dir, store) = store();

        let output = execute(&args("abc123"), &FakeSite { accepts: true }, &store).unwrap();

        assert_eq!(
            output,
            format!("logged in; session saved to {}\n", store.path())
        );
        assert_eq!(store.load().unwrap(), Some("abc123".parse().unwrap()));
    }

    #[test]
    fn keeps_the_working_session_when_the_new_one_is_rejected() {
        let (_dir, store) = store();
        execute(&args("abc123"), &FakeSite { accepts: true }, &store).unwrap();

        let error = execute(&args("def456"), &FakeSite { accepts: false }, &store).unwrap_err();

        assert!(matches!(
            error,
            LoginError::Site(SiteError::SessionRejected)
        ));
        assert_eq!(store.load().unwrap(), Some("abc123".parse().unwrap()));
    }
}
