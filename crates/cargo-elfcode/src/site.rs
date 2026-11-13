//! The only module that talks to adventofcode.com.

use std::time::Duration;

use thiserror::Error;
use ureq::Agent;

use crate::session::Session;

const BASE_URL: &str = "https://adventofcode.com";
/// Advent of Code asks automated clients to say who to contact; the
/// repository is where that conversation would happen.
const USER_AGENT: &str = concat!(
    "cargo-elfcode/",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CARGO_PKG_REPOSITORY"),
    ")"
);
const TIMEOUT: Duration = Duration::from_secs(30);

/// Any input needs a login and every account has this one, so fetching it
/// tells whether a session is accepted.
const SESSION_PROBE: &str = "/2015/day/1/input";

/// What commands need from the site. A trait so they can be tested without
/// the network.
pub trait Site {
    /// Checks that the site accepts `session`.
    fn check_session(&self, session: &Session) -> Result<(), SiteError>;
}

#[derive(Debug, Error)]
pub enum SiteError {
    #[error(
        "adventofcode.com did not accept the session cookie: it is wrong or has expired; \
         log in on the site, copy the `session` cookie, and run `cargo elfcode login <cookie>`"
    )]
    SessionRejected,
    #[error("adventofcode.com answered {url} with an unexpected status {status}")]
    UnexpectedStatus { url: String, status: u16 },
    #[error("could not reach adventofcode.com")]
    Transport(#[from] ureq::Error),
}

pub struct HttpSite {
    agent: Agent,
    base_url: String,
}

impl HttpSite {
    pub fn new() -> Self {
        Self::at(BASE_URL)
    }

    fn at(base_url: impl Into<String>) -> Self {
        let agent = Agent::config_builder()
            .user_agent(USER_AGENT)
            .timeout_global(Some(TIMEOUT))
            // Statuses are part of the protocol here, not transport errors.
            .http_status_as_error(false)
            .build()
            .into();

        Self {
            agent,
            base_url: base_url.into(),
        }
    }

    fn get(&self, path: &str, session: &Session) -> Result<u16, SiteError> {
        let url = format!("{}{path}", self.base_url);
        let response = self
            .agent
            .get(&url)
            .header("Cookie", session.cookie())
            .call()?;

        match response.status().as_u16() {
            // Observed on the live site: 400 "Please log in" without a
            // usable cookie, 500 for a session it does not know.
            400 | 500 => Err(SiteError::SessionRejected),
            status @ 200..=299 => Ok(status),
            status => Err(SiteError::UnexpectedStatus { url, status }),
        }
    }
}

impl Site for HttpSite {
    fn check_session(&self, session: &Session) -> Result<(), SiteError> {
        self.get(SESSION_PROBE, session).map(drop)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::thread::{self, JoinHandle};

    use super::*;

    const TOKEN: &str = "53616c7465645f5f0123456789abcdef";

    /// Answers one request with `status` and hands back the request head.
    fn serve_once(status: &'static str) -> (HttpSite, JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let site = HttpSite::at(format!("http://{}", listener.local_addr().unwrap()));

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let head: String = BufReader::new(&stream)
                .lines()
                .map(Result::unwrap)
                .take_while(|line| !line.is_empty())
                .map(|line| line + "\n")
                .collect();
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            head
        });

        (site, server)
    }

    fn check(status: &'static str) -> (Result<(), SiteError>, String) {
        let (site, server) = serve_once(status);
        let result = site.check_session(&TOKEN.parse().unwrap());
        (result, server.join().unwrap().to_lowercase())
    }

    #[test]
    fn accepts_a_session_the_site_serves_an_input_for() {
        let (result, request) = check("200 OK");

        assert!(result.is_ok(), "{result:?}");
        assert!(request.starts_with("get /2015/day/1/input "), "{request}");
        assert!(
            request.contains(&format!("cookie: session={TOKEN}\n")),
            "{request}"
        );
    }

    #[test]
    fn identifies_itself() {
        let (_, request) = check("200 OK");

        assert!(USER_AGENT.contains("(https://github.com/"), "{USER_AGENT}");
        assert!(
            request.contains(&format!("user-agent: {}\n", USER_AGENT.to_lowercase())),
            "{request}"
        );
    }

    #[test]
    fn reports_a_rejected_session_as_such() {
        for status in ["400 Bad Request", "500 Internal Server Error"] {
            let (result, _) = check(status);

            assert!(
                matches!(result, Err(SiteError::SessionRejected)),
                "{status}: {result:?}"
            );
        }
    }

    #[test]
    fn does_not_blame_the_session_for_other_statuses() {
        let (result, _) = check("503 Service Unavailable");

        assert!(
            matches!(result, Err(SiteError::UnexpectedStatus { status: 503, .. })),
            "{result:?}"
        );
    }

    #[test]
    fn reports_an_unreachable_site() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let site = HttpSite::at(format!("http://{}", listener.local_addr().unwrap()));
        drop(listener);

        let result = site.check_session(&TOKEN.parse().unwrap());

        assert!(matches!(result, Err(SiteError::Transport(_))), "{result:?}");
    }
}
