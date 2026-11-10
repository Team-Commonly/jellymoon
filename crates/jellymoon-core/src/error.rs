use std::time::Duration;
use std::{fmt, io};

/// Why there is no day or no page. Each prints as one sentence for the person; none holds text from a session log, and
/// what a harness printed is cut short and has credential shapes replaced before it is kept here.
#[derive(Debug)]
pub enum Error {
    /// The date is not a calendar day written YYYY-MM-DD.
    Date,
    /// The session logs could not be read.
    Logs(io::Error),
    /// The model's name is not a name: it could be read as a flag by the harness.
    Model,
    /// No `claude` program is on `PATH`.
    NoHarness,
    /// The reading call could not be started, or its folder could not be made.
    Call(io::Error),
    /// The reading call ran past its time limit and was stopped.
    TooLong(Duration),
    /// The harness exited with a failure; `said` is the start of what it printed on stderr.
    Failed { status: String, said: String },
    /// The harness printed something other than one result object.
    NotAResult,
    /// The harness's result says the call was an error.
    Harness(String),
    /// The model's text is not page data.
    NotPageData(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Date => write!(f, "--date must be a day as YYYY-MM-DD"),
            Error::Logs(e) => write!(f, "reading the session logs: {e}"),
            Error::Model => write!(f, "--model must be a model's name: letters, digits, dot, dash and underscore"),
            Error::NoHarness => write!(f, "finding Claude Code: no `claude` program on PATH"),
            Error::Call(e) => write!(f, "the reading call could not run: {e}"),
            Error::TooLong(limit) => write!(f, "the reading call took longer than {} seconds", limit.as_secs_f32()),
            Error::Failed { status, said } => {
                write!(f, "the reading call failed: {status}: {said}")
            }
            Error::NotAResult => write!(f, "the reading call printed something other than its result"),
            Error::Harness(said) => write!(f, "the harness reported an error: {said}"),
            Error::NotPageData(why) => write!(f, "the reading is not page data: {why}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Logs(e) | Error::Call(e) => Some(e),
            _ => None,
        }
    }
}
