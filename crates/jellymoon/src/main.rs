//! The `jellymoon` command. `day` prints one local day of what the person typed to Claude Code; `page` prints that
//! day's page data, made with one reading call through their own Claude Code; `rule` keeps a ruling, and `hook`, run
//! by Claude Code, hands it to a session. `settle` is a tap on a clash of the day's kept page: a ruling of its two
//! statements. Each prints one line of JSON on stdout (`hook` nothing when no ruling
//! waits), or an error on stderr and nothing on stdout. None talks to a server. `uninstall` removes what `install.sh`
//! added, and the hook.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;
use std::{env, io};

use jellymoon_core::{DayOptions, Error, Outcome, PageOptions, ReaderDay, RunOptions, UninstallOptions, user_home};
use lexopt::prelude::*;

const USAGE: &str = "usage:
  jellymoon day [--date <YYYY-MM-DD>] [--logs <Claude Code projects dir>] [--skip <folder names, comma separated>]
  jellymoon page [--date <YYYY-MM-DD>] [--logs <dir>] [--skip <names>] [--model <name>]   (one reading call through your own Claude Code)
  jellymoon rule --stands <the words that stand> --over <the words they replace>
  jellymoon hook                                 (run by Claude Code: hands waiting rulings to the session on stdin)
  jellymoon settle --date <YYYY-MM-DD> --clash <n> --option <1|2>   (a tap on a clash of the kept page: that statement stands)
  jellymoon wish --say <your words> --project <folder name> [--date <YYYY-MM-DD>]   (keeps a wish you make on the day's page; starts nothing)
  jellymoon run --wish <id> [--date <YYYY-MM-DD>] [--repo <folder>]   (starts your Claude Code on a made wish, in a copy of its repo, and waits)
  jellymoon hook install [--settings <Claude Code settings file>] [--installed]   (--installed: for the installed command, as the app does)
  jellymoon hook uninstall
  jellymoon hook status                          (whether the hook is in the settings now; changes nothing)
  jellymoon copy make --repo <folder> [--run <id>]   (a run's own copy of a repo: a git worktree on a branch of its own)
  jellymoon copy clean --run <id> --outcome <completed|failed>   (completed: committed on its branch, the folder removed; failed: kept)
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
    InstallHook { settings: Option<PathBuf>, installed: bool },
    Settle { date: String, clash: usize, option: usize },
    Wish { date: Option<String>, say: String, project: String },
    Run(RunOptions),
    RemoveHook,
    HookStatus,
    Uninstall,
    MakeCopy { repo: PathBuf, run: Option<String> },
    CleanCopy { run: String, outcome: Outcome },
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

/// `hook install`: for this program, or with `--installed` for the checked command in the install folder.
fn install_hook(settings: Option<PathBuf>, installed: bool) -> Result<jellymoon_core::HookState, Failure> {
    let settings = match settings {
        Some(settings) => settings,
        None => harness_folder()?.join("settings.json"),
    };
    if !settings.is_absolute() && named("CLAUDE_CONFIG_DIR").is_some_and(|folder| settings.starts_with(folder)) {
        return Err(Failure::Run("CLAUDE_CONFIG_DIR must be an absolute path; nothing was changed".to_string()));
    }
    let state = if installed {
        let bin = bin().ok_or_else(|| Failure::Run("no home directory: set JELLYMOON_BIN_DIR".to_string()))?;
        jellymoon_core::turn_on_delivery(&settings, &home()?, &bin)
    } else {
        let program = env::current_exe().map_err(|e| Failure::Run(format!("finding this program: {e}")))?;
        jellymoon_core::install_hook(&settings, &home()?, &program)
    };
    state.map_err(|e| Failure::Run(e.to_string()))
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
            let page = jellymoon_core::page(&day, &options).map_err(failed)?;
            Ok(format!("{}\n", jellymoon_core::with_made(&home()?, page)))
        }
        Command::Uninstall => {
            // The same two folders, under the same two names, that install.sh puts the pieces in.
            let app = named("JELLYMOON_APP_DIR").or_else(|| Some(user_home()?.join("Applications")));
            let home = own_folder()?;
            let (Some(app), Some(bin)) = (app, bin()) else {
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
        Command::InstallHook { settings, installed } => Ok(format!("{}\n", install_hook(settings, installed)?)),
        Command::Settle { date, clash, option } => {
            let settled = jellymoon_core::settle_kept(&home()?, &date, clash, option).map_err(failed)?;
            if let Some(why) = &settled.unkept {
                // The ruling was made, so this is not a failure: the line says what the page will not show.
                let line = format!("jellymoon: the ruling was made, but the page could not keep it: {why}\n");
                let _ = io::stderr().write_all(line.as_bytes());
            }
            Ok(format!("{settled}\n"))
        }
        Command::Wish { date, say, project } => {
            Ok(format!("{}\n", jellymoon_core::wish(&home()?, date.as_deref(), &say, &project).map_err(failed)?))
        }
        Command::Run(options) => Ok(format!("{}\n", jellymoon_core::run(&home()?, &options).map_err(failed)?)),
        Command::HookStatus => {
            let installed = home().is_ok_and(|home| jellymoon_core::hook_installed(&home));
            Ok(format!("{}\n", jellymoon_core::HookState { installed, changed: false }))
        }
        Command::RemoveHook => Ok(format!("{}\n", jellymoon_core::remove_hook(&home()?).map_err(failed)?)),
        Command::MakeCopy { repo, run } => {
            Ok(format!("{}\n", jellymoon_core::make_copy(&home()?, &repo, run.as_deref()).map_err(failed)?))
        }
        Command::CleanCopy { run, outcome } => {
            Ok(format!("{}\n", jellymoon_core::clean_copy(&home()?, &run, outcome).map_err(failed)?))
        }
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
        Some(Value(command)) if command == "settle" => return parse_settle(parser),
        Some(Value(command)) if command == "wish" => return parse_wish(parser),
        Some(Value(command)) if command == "run" => return parse_run(parser),
        Some(Value(command)) if command == "hook" => return parse_hook(parser),
        Some(Value(command)) if command == "copy" => return parse_copy(parser),
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

fn parse_settle(mut parser: lexopt::Parser) -> Result<Command, Failure> {
    let usage = |e: lexopt::Error| Failure::Usage(e.to_string());
    let (mut date, mut clash, mut option) = (None, None, None);
    // Digits and nothing else: `+1` is not a number here, though it would parse as one.
    let number = |parser: &mut lexopt::Parser| {
        let value = parser.value().and_then(|v| v.string()).map_err(usage)?;
        let digits = !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit());
        let number = digits.then(|| value.parse::<usize>().ok()).flatten();
        number.ok_or_else(|| Failure::Usage(format!("--clash and --option take a number, and {value:?} is not one")))
    };
    while let Some(arg) = parser.next().map_err(usage)? {
        match arg {
            Long("help") | Short('h') => return Ok(Command::Help),
            Long("date") => date = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("clash") => clash = Some(number(&mut parser)?),
            Long("option") => option = Some(number(&mut parser)?),
            other => return Err(usage(other.unexpected())),
        }
    }
    match (date, clash, option) {
        (Some(date), Some(clash), Some(option)) => Ok(Command::Settle { date, clash, option }),
        _ => Err(Failure::Usage("settle takes --date, --clash and --option".to_string())),
    }
}

fn parse_wish(mut parser: lexopt::Parser) -> Result<Command, Failure> {
    let usage = |e: lexopt::Error| Failure::Usage(e.to_string());
    let (mut date, mut say, mut project) = (None, None, None);
    while let Some(arg) = parser.next().map_err(usage)? {
        match arg {
            Long("help") | Short('h') => return Ok(Command::Help),
            Long("date") => date = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("say") => say = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("project") => project = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            other => return Err(usage(other.unexpected())),
        }
    }
    match (say, project) {
        (Some(say), Some(project)) => Ok(Command::Wish { date, say, project }),
        _ => Err(Failure::Usage("wish takes --say and --project".to_string())),
    }
}

/// `run`. The logs are read only when no repo is named, to find the one folder of the wish's project.
fn parse_run(mut parser: lexopt::Parser) -> Result<Command, Failure> {
    let usage = |e: lexopt::Error| Failure::Usage(e.to_string());
    let logs = harness_folder().ok().map(|folder| folder.join("projects"));
    let mut options = RunOptions { logs, harness: named("JELLYMOON_CLAUDE"), ..RunOptions::default() };
    let mut wish = None;
    while let Some(arg) = parser.next().map_err(usage)? {
        match arg {
            Long("help") | Short('h') => return Ok(Command::Help),
            Long("wish") => wish = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("date") => options.day = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("repo") => options.repo = Some(PathBuf::from(parser.value().map_err(usage)?)),
            other => return Err(usage(other.unexpected())),
        }
    }
    options.wish = wish.ok_or_else(|| Failure::Usage("run takes --wish".to_string()))?;
    Ok(Command::Run(options))
}

fn parse_hook(mut parser: lexopt::Parser) -> Result<Command, Failure> {
    let usage = |e: lexopt::Error| Failure::Usage(e.to_string());
    let install = |action: &Command| matches!(action, Command::InstallHook { .. });
    let (mut action, mut settings, mut installed) = (Command::Hook, None, false);
    while let Some(arg) = parser.next().map_err(usage)? {
        match arg {
            Long("help") | Short('h') => return Ok(Command::Help),
            Value(word) if matches!(action, Command::Hook) && word == "install" => {
                action = Command::InstallHook { settings: None, installed: false };
            }
            Value(word) if matches!(action, Command::Hook) && word == "uninstall" => action = Command::RemoveHook,
            Value(word) if matches!(action, Command::Hook) && word == "status" => action = Command::HookStatus,
            Long("settings") if install(&action) => settings = Some(PathBuf::from(parser.value().map_err(usage)?)),
            Long("installed") if install(&action) => installed = true,
            other => return Err(usage(other.unexpected())),
        }
    }
    Ok(if install(&action) { Command::InstallHook { settings, installed } } else { action })
}

/// `copy make` and `copy clean`. What a run id may be is core's rule, so any text is taken here.
fn parse_copy(mut parser: lexopt::Parser) -> Result<Command, Failure> {
    let usage = |e: lexopt::Error| Failure::Usage(e.to_string());
    let (mut action, mut repo, mut run, mut outcome) = (String::new(), None, None, None);
    while let Some(arg) = parser.next().map_err(usage)? {
        match arg {
            Long("help") | Short('h') => return Ok(Command::Help),
            Value(word) if action.is_empty() && (word == "make" || word == "clean") => {
                action = word.string().map_err(usage)?
            }
            Long("repo") if action == "make" => repo = Some(PathBuf::from(parser.value().map_err(usage)?)),
            Long("run") if !action.is_empty() => run = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("outcome") if action == "clean" => {
                let named = parser.value().and_then(|v| v.string()).map_err(usage)?;
                let known = Outcome::named(&named);
                outcome = Some(
                    known.ok_or_else(|| Failure::Usage(format!("--outcome is completed or failed, not {named:?}")))?,
                );
            }
            other => return Err(usage(other.unexpected())),
        }
    }
    match (action.as_str(), repo, run, outcome) {
        ("make", Some(repo), run, _) => Ok(Command::MakeCopy { repo, run }),
        ("clean", _, Some(run), Some(outcome)) => Ok(Command::CleanCopy { run, outcome }),
        _ => Err(Failure::Usage(
            "copy is `copy make --repo <folder>` or `copy clean --run <id> --outcome <completed|failed>`".to_string(),
        )),
    }
}

/// The path a variable names, when it is set to something.
fn named(variable: &str) -> Option<PathBuf> {
    env::var_os(variable).filter(|path| !path.is_empty()).map(PathBuf::from)
}

/// The folder `install.sh` puts the `jellymoon` command in.
fn bin() -> Option<PathBuf> {
    named("JELLYMOON_BIN_DIR").or_else(|| Some(user_home()?.join(".local").join("bin")))
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
