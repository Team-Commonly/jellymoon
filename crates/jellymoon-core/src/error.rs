use std::time::Duration;
use std::{fmt, io};

/// Why there is no day or no page, or why an uninstall stopped. Each prints as one line for the person. None holds text from a session log or from
/// the model's reading: a reading that is not page data is named by the place it went wrong, never quoted. The two that
/// hold what the harness itself printed ([`Error::Failed`], [`Error::Harness`]) hold it made one line, with control and
/// invisible characters removed, credential shapes replaced, and cut short (`redact::line`). An `io::Error` is the
/// system's own words, with no path in it.
#[derive(Debug)]
pub enum Error {
    /// The date is not a calendar day written YYYY-MM-DD.
    Date,
    /// There is no session logs folder: Claude Code has not been used yet, or keeps its files elsewhere. It holds the
    /// folder looked for, made one line.
    NoLogs(String),
    /// The session logs could not be read.
    Logs(io::Error),
    /// The list of folders the person switched off is there and could not be read whole. It holds why.
    Skip(String),
    /// The model's name is not a name: it could be read as a flag by the harness.
    Model,
    /// No `claude` program is on `PATH` or in a well-known place. It holds the places looked in, as a person names them.
    NoHarness(String),
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
    /// `JELLYMOON_HOME`, or the home a caller gave, is not a full path: which folder it named would depend on where
    /// the program was started.
    Home,
    /// The app's or the binary's place is not a full path: where it led would depend on the folder the command ran in.
    NotAbsolute,
    /// A piece of the install, or the delivery hook, could not be removed.
    Uninstall(io::Error),
    /// A ruling lacks the statement that stands or the one it replaces.
    Ruling,
    /// The ruling, or the record of the installed hook, could not be kept or read.
    Store(io::Error),
    /// The harness settings file could not be read or written.
    Settings(io::Error),
    /// The harness settings are not a JSON object, or their hooks are not shaped as Claude Code shapes them.
    NotSettings,
    /// The hook is installed in another settings file.
    Elsewhere,
    /// The text that was installed is no longer in the settings file as it was written.
    Changed,
    /// A tap on a clash was refused, and nothing was written. It holds why, in one line with no word of the page.
    Tap(String),
    /// The installed `jellymoon` command is not a file the hook may name. It holds why.
    Command(String),
    /// A run's copy of a repo could not be made or cleaned. It holds why, in one line. What git said is in it made
    /// one line and cut short, with credential shapes replaced.
    Copy(String),
    /// A made wish was refused, and nothing was written. It holds why, in one line with no word of the wish.
    Wish(String),
    /// A run was refused, or could not be started. It holds why, in one line with no word of the wish, of a session
    /// log or of the agent.
    Run(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Date => write!(f, "--date must be a day as YYYY-MM-DD"),
            Error::NoLogs(looked) => {
                write!(f, "no Claude Code session logs were found: there is no folder {looked}")
            }
            Error::Logs(e) => write!(f, "reading the session logs: {e}"),
            Error::Skip(why) => write!(
                f,
                "the list of folders you switched off (skip.txt in Jellymoon's folder) could not be read, so nothing was read: {why}"
            ),
            Error::Model => write!(f, "--model must be a model's name: letters, digits, dot, dash and underscore"),
            Error::NoHarness(looked) => write!(
                f,
                "finding Claude Code: no `claude` program in {looked}; install Claude Code, or name the program with JELLYMOON_CLAUDE"
            ),
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
            Error::Home => write!(f, "no home directory: set JELLYMOON_HOME to an absolute path"),
            Error::NotAbsolute => write!(
                f,
                "uninstalling: JELLYMOON_APP_DIR and JELLYMOON_BIN_DIR must be full paths, from the root; nothing was removed"
            ),
            Error::Uninstall(e) => write!(f, "uninstalling: {e}"),
            Error::Ruling => write!(f, "a ruling needs the words that stand and the words they replace"),
            Error::Store(e) => write!(f, "jellymoon's own files: {e}"),
            Error::Settings(e) => write!(f, "the harness settings file: {e}"),
            Error::NotSettings => write!(f, "the harness settings file is not a JSON object; nothing was changed"),
            Error::Elsewhere => {
                write!(f, "the hook is installed in another settings file; uninstall it first. Nothing was changed")
            }
            Error::Changed => {
                write!(f, "the installed hook entry cannot be taken out safely; remove it by hand. Nothing was changed")
            }
            Error::Tap(why) | Error::Copy(why) | Error::Wish(why) | Error::Run(why) => write!(f, "{why}"),
            Error::Command(why) => write!(f, "the installed jellymoon command is not used: {why}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Logs(e) | Error::Call(e) | Error::Uninstall(e) | Error::Store(e) | Error::Settings(e) => Some(e),
            _ => None,
        }
    }
}
