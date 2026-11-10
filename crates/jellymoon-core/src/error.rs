use std::time::Duration;
use std::{fmt, io};

/// Why there is no day or no page. Each prints as one line for the person. None holds text from a session log or from
/// the model's reading: a reading that is not page data is named by the place it went wrong, never quoted. The two that
/// hold what the harness itself printed ([`Error::Failed`], [`Error::Harness`]) hold it made one line, with control and
/// invisible characters removed, credential shapes replaced, and cut short (`redact::line`). An `io::Error` is the
/// system's own words, with no path in it.
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
    /// The model's text is not page data. Only where it stopped being so is kept: the parser's own message would quote
    /// the model.
    NotPageData { line: usize, column: usize },
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
            Error::NotPageData { line, column } => {
                write!(f, "the reading is not page data (line {line}, column {column} of the model's JSON)")
            }
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
