//! The `jellymoon` command. `day` prints one local day of what the person typed to Claude Code; `page` prints that
//! day's page data, made with one reading call through their own Claude Code; `rule` keeps a ruling, and `hook`, run
//! by Claude Code, hands it to a session. Each prints one line of JSON on stdout (`hook` nothing when no ruling
//! waits), or an error on stderr and nothing on stdout. None talks to a server. `uninstall` removes what `install.sh`
//! added, and the hook.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;
use std::{env, io};

use jellymoon_core::{DayOptions, Error, PageOptions, ReaderDay, UninstallOptions, user_home};
use lexopt::prelude::*;

const USAGE: &str = "usage:
  jellymoon day [--date <YYYY-MM-DD>] [--logs <Claude Code projects dir>] [--skip <folder names, comma separated>]
  jellymoon page [--date <YYYY-MM-DD>] [--logs <dir>] [--skip <names>] [--model <name>]   (one reading call through your own Claude Code)
  jellymoon rule --stands <the words that stand> --over <the words they replace>
  jellymoon hook                                 (run by Claude Code: hands waiting rulings to the session on stdin)
  jellymoon hook install [--settings <Claude Code settings file>]
  jellymoon hook uninstall
  jellymoon uninstall   (removes Jellymoon.app, this command and the delivery hook; your journal's own files stay)
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
    Rule { stands: String, over: String },
    Hook,
    InstallHook(Option<PathBuf>),
    RemoveHook,
    Uninstall,
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
        Command::Day(options) => Ok(format!("{}\n", day(&options)?)),
        Command::Page(options, model) => {
            let day = day(&options)?;
            let harness = named("JELLYMOON_CLAUDE");
            let options = PageOptions { model, dir: home()?.join("reading"), limit: READING_LIMIT, harness };
            Ok(format!("{}\n", jellymoon_core::page(&day, &options).map_err(failed)?))
        }
        Command::Uninstall => {
            // The same two folders, under the same two names, that install.sh puts the pieces in.
            let app = named("JELLYMOON_APP_DIR").or_else(|| Some(user_home()?.join("Applications")));
            let bin = named("JELLYMOON_BIN_DIR").or_else(|| Some(user_home()?.join(".local").join("bin")));
            let home = own_folder()?;
            let (Some(app), Some(bin)) = (app, bin) else {
                return Err(Failure::Run("no home directory: set JELLYMOON_APP_DIR and JELLYMOON_BIN_DIR".to_string()));
            };
            let binary = bin.join(format!("jellymoon{}", env::consts::EXE_SUFFIX));
            let options = UninstallOptions { app: app.join("Jellymoon.app"), binary, home };
            Ok(format!("{}\n", jellymoon_core::uninstall(&options).map_err(failed)?))
        }
        Command::Rule { stands, over } => {
            Ok(format!("{}\n", jellymoon_core::rule(&home()?, &stands, &over).map_err(failed)?))
        }
        Command::Hook => {
            // With no home there is no ruling, and a hook that fails would show an error at every step of a session:
            // a home that is refused is no home here, and nothing is said.
            let home = jellymoon_core::own_folder().ok().flatten();
            let told = home.and_then(|home| jellymoon_core::deliver(&home, io::stdin().lock()));
            Ok(told.map(|told| told + "\n").unwrap_or_default())
        }
        Command::InstallHook(settings) => {
            let settings = match settings {
                Some(settings) => settings,
                None => harness_folder()?.join("settings.json"),
            };
            let program = env::current_exe().map_err(|e| Failure::Run(format!("finding this program: {e}")))?;
            Ok(format!("{}\n", jellymoon_core::install_hook(&settings, &home()?, &program).map_err(failed)?))
        }
        Command::RemoveHook => Ok(format!("{}\n", jellymoon_core::remove_hook(&home()?).map_err(failed)?)),
    }
}

/// Reads the command line. A flag's value is whatever follows it, even text that starts with dashes: a model name
/// shaped like a flag is taken here and refused by the check on names, never read as a flag of ours.
fn parse(mut parser: lexopt::Parser) -> Result<Command, Failure> {
    let usage = |e: lexopt::Error| Failure::Usage(e.to_string());
    let command = match parser.next().map_err(usage)? {
        Some(Value(command)) if command == "day" || command == "page" || command == "help" => command,
        Some(Value(command)) if command == "uninstall" => {
            return match parser.next().map_err(usage)? {
                None => Ok(Command::Uninstall),
                Some(Long("help") | Short('h')) => Ok(Command::Help),
                Some(other) => Err(usage(other.unexpected())),
            };
        }
        Some(Value(command)) if command == "rule" => return parse_rule(parser),
        Some(Value(command)) if command == "hook" => return parse_hook(parser),
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
        Some(logs) if logs.to_string_lossy().trim().is_empty() => {
            return Err(Failure::Usage("--logs needs a folder".to_string()));
        }
        Some(logs) => logs,
        None => harness_folder()?.join("projects"),
    };
    let skip = jellymoon_core::skip_names(&skip).map_err(Failure::Usage)?;
    let options = DayOptions { logs, date, skip, home: own_folder()? };
    Ok(if command == "day" { Command::Day(options) } else { Command::Page(options, model) })
}

fn parse_rule(mut parser: lexopt::Parser) -> Result<Command, Failure> {
    let usage = |e: lexopt::Error| Failure::Usage(e.to_string());
    let (mut stands, mut over) = (String::new(), String::new());
    while let Some(arg) = parser.next().map_err(usage)? {
        match arg {
            Long("help") | Short('h') => return Ok(Command::Help),
            Long("stands") => stands = parser.value().and_then(|v| v.string()).map_err(usage)?,
            Long("over") => over = parser.value().and_then(|v| v.string()).map_err(usage)?,
            other => return Err(usage(other.unexpected())),
        }
    }
    Ok(Command::Rule { stands, over })
}

fn parse_hook(mut parser: lexopt::Parser) -> Result<Command, Failure> {
    let usage = |e: lexopt::Error| Failure::Usage(e.to_string());
    let (mut action, mut settings) = (Command::Hook, None);
    while let Some(arg) = parser.next().map_err(usage)? {
        match arg {
            Long("help") | Short('h') => return Ok(Command::Help),
            Value(word) if matches!(action, Command::Hook) && word == "install" => action = Command::InstallHook(None),
            Value(word) if matches!(action, Command::Hook) && word == "uninstall" => action = Command::RemoveHook,
            Long("settings") if matches!(action, Command::InstallHook(_)) => {
                settings = Some(PathBuf::from(parser.value().map_err(usage)?));
            }
            other => return Err(usage(other.unexpected())),
        }
    }
    Ok(if matches!(action, Command::InstallHook(_)) { Command::InstallHook(settings) } else { action })
}

/// The path a variable names, when it is set to something.
fn named(variable: &str) -> Option<PathBuf> {
    env::var_os(variable).filter(|path| !path.is_empty()).map(PathBuf::from)
}

/// Claude Code's own folder, which holds its `projects` and its `settings.json`: `CLAUDE_CONFIG_DIR`, its own setting
/// for where it keeps its files, or `.claude` in the user's home.
fn harness_folder() -> Result<PathBuf, Failure> {
    let config = named("CLAUDE_CONFIG_DIR").or_else(|| Some(user_home()?.join(".claude")));
    config.ok_or_else(|| {
        Failure::Run("finding Claude Code's files: no home directory; name them with --logs or --settings".to_string())
    })
}

/// Where jellymoon keeps its own files, by core's one rule; none when no home is known.
fn own_folder() -> Result<Option<PathBuf>, Failure> {
    jellymoon_core::own_folder().map_err(|e| Failure::Run(e.to_string()))
}

fn home() -> Result<PathBuf, Failure> {
    own_folder()?.ok_or_else(|| Failure::Run("no home directory: set JELLYMOON_HOME".to_string()))
}

/// Reads the day. When the logs folder that is not there is the one nobody named, the line says what to do.
fn day(options: &DayOptions) -> Result<ReaderDay, Failure> {
    jellymoon_core::day(options).map_err(|e| match &e {
        Error::NoLogs(_) if harness_folder().is_ok_and(|folder| folder.join("projects") == options.logs) => {
            Failure::Run(format!(
                "{e}. Use Claude Code once and they will be there, or name its projects folder with --logs"
            ))
        }
        _ => Failure::Run(e.to_string()),
    })
}
