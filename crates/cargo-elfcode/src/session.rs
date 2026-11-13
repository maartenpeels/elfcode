use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::str::FromStr;
use std::{fmt, path::PathBuf};

use camino::{Utf8Path, Utf8PathBuf};
use etcetera::BaseStrategy;
use thiserror::Error;

const APP_DIR: &str = "elfcode";
const SESSION_FILE: &str = "session";

/// The value of the `session` cookie that adventofcode.com sets on login.
#[derive(Clone, PartialEq, Eq)]
pub struct Session(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error(
    "a session cookie is one long hexadecimal string; copy the value of the `session` cookie for adventofcode.com from your browser"
)]
pub struct InvalidSession;

impl Session {
    /// The `Cookie` header value that authenticates a request.
    pub fn cookie(&self) -> String {
        format!("session={}", self.0)
    }
}

impl FromStr for Session {
    type Err = InvalidSession;

    /// Accepts the bare value or the `session=` pair as browsers show it.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s.trim();
        let value = value.strip_prefix("session=").unwrap_or(value);

        if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            Ok(Self(value.to_owned()))
        } else {
            Err(InvalidSession)
        }
    }
}

/// Hides the value, so a session never ends up in a log or an error.
impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Session(<redacted>)")
    }
}

/// The file the session is kept in, outside any repository.
#[derive(Debug, Clone)]
pub struct SessionStore {
    path: Utf8PathBuf,
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("could not find your home directory")]
    NoHome(#[from] etcetera::HomeDirError),
    #[error("the config directory {} is not valid UTF-8", .0.display())]
    NonUtf8Path(PathBuf),
    #[error("could not write the session to {path}")]
    Write {
        path: Utf8PathBuf,
        source: io::Error,
    },
    #[error("could not read the session from {path}")]
    Read {
        path: Utf8PathBuf,
        source: io::Error,
    },
    #[error("the session stored in {path} is damaged; run `cargo elfcode login` again")]
    Damaged { path: Utf8PathBuf },
}

impl SessionStore {
    pub fn at(path: impl Into<Utf8PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The store in the user's config directory, such as
    /// `~/.config/elfcode/session`.
    pub fn for_user() -> Result<Self, StoreError> {
        let config = etcetera::choose_base_strategy()?.config_dir();
        let config = Utf8PathBuf::from_path_buf(config).map_err(StoreError::NonUtf8Path)?;

        Ok(Self::at(config.join(APP_DIR).join(SESSION_FILE)))
    }

    pub fn path(&self) -> &Utf8Path {
        &self.path
    }

    pub fn save(&self, session: &Session) -> Result<(), StoreError> {
        self.write(session).map_err(|source| StoreError::Write {
            path: self.path.clone(),
            source,
        })
    }

    fn write(&self, session: &Session) -> io::Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }

        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            // The session is a login credential: readable by the owner only.
            options.mode(0o600);
        }

        writeln!(options.open(&self.path)?, "{}", session.0)
    }

    /// The stored session, or `None` when the user has not logged in.
    pub fn load(&self) -> Result<Option<Session>, StoreError> {
        match fs::read_to_string(&self.path) {
            Ok(contents) => contents.parse().map(Some).map_err(|_| StoreError::Damaged {
                path: self.path.clone(),
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(StoreError::Read {
                path: self.path.clone(),
                source,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    const TOKEN: &str = "53616c7465645f5f0123456789abcdef";

    fn store() -> (TempDir, SessionStore) {
        let dir = tempfile::tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(dir.path().join("elfcode/session")).unwrap();
        (dir, SessionStore::at(path))
    }

    #[test]
    fn accepts_the_value_however_it_was_copied() {
        let expected = format!("session={TOKEN}");

        for copied in [
            TOKEN.to_owned(),
            format!("  {TOKEN}\n"),
            format!("session={TOKEN}"),
        ] {
            assert_eq!(copied.parse::<Session>().unwrap().cookie(), expected);
        }
    }

    #[test]
    fn rejects_what_cannot_be_a_session() {
        for wrong in ["", "   ", "session=", "my password", "53616c74-65645f5f"] {
            assert_eq!(wrong.parse::<Session>(), Err(InvalidSession), "{wrong:?}");
        }
    }

    #[test]
    fn debug_output_does_not_contain_the_value() {
        let session: Session = TOKEN.parse().unwrap();

        assert_eq!(format!("{session:?}"), "Session(<redacted>)");
    }

    #[test]
    fn loads_what_it_saved_creating_the_directory() {
        let (_dir, store) = store();
        let session: Session = TOKEN.parse().unwrap();

        store.save(&session).unwrap();

        assert_eq!(store.load().unwrap(), Some(session));
    }

    #[test]
    fn has_no_session_before_login() {
        let (_dir, store) = store();

        assert_eq!(store.load().unwrap(), None);
    }

    #[test]
    fn replaces_an_earlier_session() {
        let (_dir, store) = store();
        let newer: Session = "abcdef".parse().unwrap();

        store.save(&TOKEN.parse().unwrap()).unwrap();
        store.save(&newer).unwrap();

        assert_eq!(store.load().unwrap(), Some(newer));
    }

    #[test]
    fn reports_a_damaged_file_with_what_to_do() {
        let (_dir, store) = store();
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        fs::write(store.path(), "not a session").unwrap();

        let error = store.load().unwrap_err();

        assert!(matches!(error, StoreError::Damaged { .. }));
        assert!(error.to_string().contains("cargo elfcode login"));
    }

    #[cfg(unix)]
    #[test]
    fn only_the_owner_can_read_the_file() {
        use std::os::unix::fs::PermissionsExt;

        let (_dir, store) = store();
        store.save(&TOKEN.parse().unwrap()).unwrap();

        let mode = fs::metadata(store.path()).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
