//! The `jellymoon` command. `day` prints one local day of what the person typed to Claude Code; `page` prints that
//! day's page data, made with one reading call through their own Claude Code. Each prints one line of JSON on stdout,
//! or an error on stderr and nothing on stdout. Neither talks to a server.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;
use std::{env, io};

use jellymoon_core::{DayOptions, PageOptions};
use lexopt::prelude::*;

const USAGE: &str = "usage:
  jellymoon day [--date <YYYY-MM-DD>] [--logs <Claude Code projects dir>] [--skip <folder names, comma separated>]
  jellymoon page [--date <YYYY-MM-DD>] [--logs <dir>] [--skip <names>] [--model <name>]   (one reading call through your own Claude Code)
";

/// How long the reading call may take. One measured day took 9 seconds.
const READING_LIMIT: Duration = Duration::from_secs(180);

/// Why a command printed nothing: it was not a command (the usage follows the reason), or it could not be done.
enum Failure {
    Usage(String),
    Run(String),
}

enum Command {
    Help,
    Day(DayOptions),
    Page(DayOptions, String),
}

fn main() -> ExitCode {
    let failed = match run() {
        Ok(line) => {
            return if io::stdout().write_all(line.as_bytes()).is_ok() { ExitCode::SUCCESS } else { ExitCode::FAILURE };
        }
        Err(failed) => failed,
    };
    let (text, code) = match failed {
        Failure::Usage(why) => (format!("jellymoon: {why}\n{USAGE}"), 2),
        Failure::Run(why) => (format!("jellymoon: {why}\n"), 1),
    };
    let _ = io::stderr().write_all(text.as_bytes());
    ExitCode::from(code)
}

/// Runs the command and returns what it prints.
fn run() -> Result<String, Failure> {
    let failed = |e: jellymoon_core::Error| Failure::Run(e.to_string());
    match parse(lexopt::Parser::from_env())? {
        Command::Help => Ok(USAGE.to_string()),
        Command::Day(options) => Ok(format!("{}\n", jellymoon_core::day(&options).map_err(failed)?)),
        Command::Page(options, model) => {
            let day = jellymoon_core::day(&options).map_err(failed)?;
            let harness = named("JELLYMOON_CLAUDE");
            let options = PageOptions { model, dir: home()?.join("reading"), limit: READING_LIMIT, harness };
            Ok(format!("{}\n", jellymoon_core::page(&day, &options).map_err(failed)?))
        }
    }
}

/// Reads the command line. A flag's value is whatever follows it, even text that starts with dashes: a model name
/// shaped like a flag is taken here and refused by the check on names, never read as a flag of ours.
fn parse(mut parser: lexopt::Parser) -> Result<Command, Failure> {
    let usage = |e: lexopt::Error| Failure::Usage(e.to_string());
    let command = match parser.next().map_err(usage)? {
        Some(Value(command)) if command == "day" || command == "page" || command == "help" => command,
        Some(Long("help") | Short('h')) => return Ok(Command::Help),
        Some(other) => return Err(usage(other.unexpected())),
        None => return Err(Failure::Usage("no command".to_string())),
    };
    let (mut date, mut logs, mut skip, mut model) = (None, None, String::new(), "sonnet".to_string());
    while let Some(arg) = parser.next().map_err(usage)? {
        match arg {
            Long("help") | Short('h') => return Ok(Command::Help),
            Long("date") => date = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("logs") => logs = Some(PathBuf::from(parser.value().map_err(usage)?)),
            Long("skip") => skip = parser.value().and_then(|v| v.string()).map_err(usage)?,
            Long("model") if command == "page" => model = parser.value().and_then(|v| v.string()).map_err(usage)?,
            other => return Err(usage(other.unexpected())),
        }
    }
    if command == "help" {
        return Ok(Command::Help);
    }
    let logs = match logs {
        Some(logs) => logs,
        None => harness_logs()?,
    };
    let options = DayOptions { logs, date, skip: skip.split(',').map(str::to_string).collect() };
    Ok(if command == "day" { Command::Day(options) } else { Command::Page(options, model) })
}

/// The path a variable names, when it is set to something.
fn named(variable: &str) -> Option<PathBuf> {
    env::var_os(variable).filter(|path| !path.is_empty()).map(PathBuf::from)
}

fn user_home() -> Option<PathBuf> {
    env::home_dir().filter(|home| !home.as_os_str().is_empty())
}

/// Claude Code's own projects directory: under `CLAUDE_CONFIG_DIR`, its own setting for where it keeps its files, or
/// under `.claude` in the user's home.
fn harness_logs() -> Result<PathBuf, Failure> {
    let config = named("CLAUDE_CONFIG_DIR").or_else(|| Some(user_home()?.join(".claude")));
    let config = config.ok_or_else(|| {
        Failure::Run("finding the session logs: no home directory; name them with --logs".to_string())
    })?;
    Ok(config.join("projects"))
}

/// Where jellymoon keeps its own files: `JELLYMOON_HOME`, or `.jellymoon` in the user's home.
fn home() -> Result<PathBuf, Failure> {
    let home = named("JELLYMOON_HOME").or_else(|| Some(user_home()?.join(".jellymoon")));
    home.ok_or_else(|| Failure::Run("no home directory: set JELLYMOON_HOME".to_string()))
}
