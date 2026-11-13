//! The only module that talks to adventofcode.com.

use std::time::Duration;

use elfcode_core::{Part, PuzzleId};
use thiserror::Error;
use ureq::http::Response;
use ureq::{Agent, Body};

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

    /// Downloads the puzzle input of the account `session` belongs to.
    fn input(&self, session: &Session, puzzle: PuzzleId) -> Result<String, SiteError>;

    /// Downloads the puzzle page as HTML. With a session it includes part 2
    /// once part 1 is solved.
    fn puzzle_page(&self, session: Option<&Session>, puzzle: PuzzleId)
    -> Result<String, SiteError>;

    /// Sends an answer and returns the response page as HTML.
    fn submit(
        &self,
        session: &Session,
        puzzle: PuzzleId,
        part: Part,
        answer: &str,
    ) -> Result<String, SiteError>;
}

/// Where a puzzle's page is on the public site.
pub fn puzzle_url(puzzle: PuzzleId) -> String {
    format!("{BASE_URL}{}", puzzle_path(puzzle))
}

fn puzzle_path(puzzle: PuzzleId) -> String {
    format!("/{}/day/{}", puzzle.year(), puzzle.day())
}

#[derive(Debug, Error)]
pub enum SiteError {
    #[error(
        "adventofcode.com did not accept the session cookie: it is wrong or has expired; \
         log in on the site, copy the `session` cookie, and run `cargo elfcode login <cookie>`"
    )]
    SessionRejected,
    #[error("adventofcode.com has nothing at {url}; a new puzzle unlocks at midnight US Eastern")]
    NotFound { url: String },
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

    fn get(&self, path: &str, session: Option<&Session>) -> Result<String, SiteError> {
        let url = format!("{}{path}", self.base_url);
        let mut request = self.agent.get(&url);
        if let Some(session) = session {
            request = request.header("Cookie", session.cookie());
        }
        let response = request.call()?;
        Self::body(url, response)
    }

    fn post_form(
        &self,
        path: &str,
        session: &Session,
        form: &[(&str, &str)],
    ) -> Result<String, SiteError> {
        let url = format!("{}{path}", self.base_url);
        let response = self
            .agent
            .post(&url)
            .header("Cookie", session.cookie())
            .send_form(form.iter().copied())?;
        Self::body(url, response)
    }

    fn body(url: String, mut response: Response<Body>) -> Result<String, SiteError> {
        match response.status().as_u16() {
            // Observed on the live site: 400 "Please log in" without a
            // usable cookie, 500 for a session it does not know.
            400 | 500 => Err(SiteError::SessionRejected),
            404 => Err(SiteError::NotFound { url }),
            200..=299 => Ok(response.body_mut().read_to_string()?),
            status => Err(SiteError::UnexpectedStatus { url, status }),
        }
    }
}

impl Site for HttpSite {
    fn check_session(&self, session: &Session) -> Result<(), SiteError> {
        self.get(SESSION_PROBE, Some(session)).map(drop)
    }

    fn input(&self, session: &Session, puzzle: PuzzleId) -> Result<String, SiteError> {
        self.get(&format!("{}/input", puzzle_path(puzzle)), Some(session))
    }

    fn puzzle_page(
        &self,
        session: Option<&Session>,
        puzzle: PuzzleId,
    ) -> Result<String, SiteError> {
        self.get(&puzzle_path(puzzle), session)
    }

    fn submit(
        &self,
        session: &Session,
        puzzle: PuzzleId,
        part: Part,
        answer: &str,
    ) -> Result<String, SiteError> {
        let level = part.to_string();
        let form = [("level", level.as_str()), ("answer", answer)];
        self.post_form(&format!("{}/answer", puzzle_path(puzzle)), session, &form)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::thread::{self, JoinHandle};

    use super::*;

    const TOKEN: &str = "53616c7465645f5f0123456789abcdef";

    /// Answers one request with `status` and `body`, and hands back the
    /// request head followed by the request body.
    fn serve_once(status: &'static str, body: &'static str) -> (HttpSite, JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let site = HttpSite::at(format!("http://{}", listener.local_addr().unwrap()));

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(&stream);
            let head: String = reader
                .by_ref()
                .lines()
                .map(Result::unwrap)
                .take_while(|line| !line.is_empty())
                .map(|line| line + "\n")
                .collect();
            let length = head
                .lines()
                .find_map(|line| {
                    line.to_lowercase()
                        .strip_prefix("content-length: ")?
                        .parse()
                        .ok()
                })
                .unwrap_or(0);
            let mut request_body = vec![0; length];
            reader.read_exact(&mut request_body).unwrap();
            let head = head + &String::from_utf8(request_body).unwrap();
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            head.to_lowercase()
        });

        (site, server)
    }

    fn session() -> Session {
        TOKEN.parse().unwrap()
    }

    fn check(status: &'static str) -> (Result<(), SiteError>, String) {
        let (site, server) = serve_once(status, "");
        let result = site.check_session(&session());
        (result, server.join().unwrap())
    }

    fn day_3_of_2026() -> PuzzleId {
        use elfcode_core::{Day, Year};
        PuzzleId::new(Year::new(2026).unwrap(), Day::new(3).unwrap()).unwrap()
    }

    #[test]
    fn downloads_an_input_exactly_as_served() {
        let (site, server) = serve_once("200 OK", "  1 2\n3 4\n");

        let input = site.input(&session(), day_3_of_2026()).unwrap();

        assert_eq!(input, "  1 2\n3 4\n");
        let request = server.join().unwrap();
        assert!(request.starts_with("get /2026/day/3/input "), "{request}");
        assert!(
            request.contains(&format!("cookie: session={TOKEN}\n")),
            "{request}"
        );
    }

    #[test]
    fn downloads_a_puzzle_page_with_or_without_a_session() {
        for session in [Some(session()), None] {
            let (site, server) = serve_once("200 OK", "<html></html>");

            let page = site.puzzle_page(session.as_ref(), day_3_of_2026()).unwrap();

            assert_eq!(page, "<html></html>");
            let request = server.join().unwrap();
            assert!(request.starts_with("get /2026/day/3 "), "{request}");
            assert_eq!(request.contains("cookie:"), session.is_some(), "{request}");
        }
    }

    #[test]
    fn submits_an_answer_as_a_form() {
        let (site, server) = serve_once("200 OK", "<main><article>ok</article></main>");

        let page = site
            .submit(&session(), day_3_of_2026(), Part::Two, "a b&c")
            .unwrap();

        assert_eq!(page, "<main><article>ok</article></main>");
        let request = server.join().unwrap();
        assert!(request.starts_with("post /2026/day/3/answer "), "{request}");
        assert!(
            request.contains("content-type: application/x-www-form-urlencoded\n"),
            "{request}"
        );
        assert!(request.ends_with("\nlevel=2&answer=a+b%26c"), "{request}");
    }

    #[test]
    fn links_to_the_public_puzzle_page() {
        assert_eq!(
            puzzle_url(day_3_of_2026()),
            "https://adventofcode.com/2026/day/3"
        );
    }

    #[test]
    fn says_when_a_puzzle_is_not_there_yet() {
        let (site, _server) = serve_once("404 Not Found", "");

        let error = site.input(&session(), day_3_of_2026()).unwrap_err();

        assert!(matches!(error, SiteError::NotFound { .. }), "{error:?}");
        assert!(error.to_string().contains("unlocks at midnight"), "{error}");
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

        let result = site.check_session(&session());

        assert!(matches!(result, Err(SiteError::Transport(_))), "{result:?}");
    }
}
