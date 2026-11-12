//! The command line of `jellymoon`, read into a [`Command`]. A flag's value is whatever follows it; what a value
//! may be is core's rule, so any text is taken here.

use std::path::PathBuf;

use jellymoon_core::{DayOptions, Outcome, RunOptions};
use lexopt::prelude::*;

use crate::{Failure, harness_folder, named, own_folder};

pub enum Command {
    Help,
    Day(DayOptions),
    Page(DayOptions, String),
    Rule {
        stands: String,
        over: String,
    },
    Hook,
    InstallHook {
        settings: Option<PathBuf>,
        installed: bool,
    },
    Settle {
        date: String,
        clash: usize,
        option: usize,
    },
    /// `apart`: with `--run`, the options of the run to start, but for the wish and the day.
    Wish {
        date: Option<String>,
        say: String,
        project: String,
        apart: Option<RunOptions>,
    },
    Run(RunOptions),
    Stop(RunOptions),
    RemoveHook,
    HookStatus,
    Uninstall,
    MakeCopy {
        repo: PathBuf,
        run: Option<String>,
    },
    CleanCopy {
        run: String,
        outcome: Outcome,
    },
}

/// Reads the command line. A flag's value is whatever follows it, even text that starts with dashes: a model name
/// shaped like a flag is taken here and refused by the check on names, never read as a flag of ours.
pub fn parse(mut parser: lexopt::Parser) -> Result<Command, Failure> {
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
    let (mut run, mut logs, mut skip) = (false, None, None);
    while let Some(arg) = parser.next().map_err(usage)? {
        match arg {
            Long("help") | Short('h') => return Ok(Command::Help),
            Long("date") => date = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("say") => say = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("project") => project = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("run") => run = true,
            Long("logs") => logs = Some(PathBuf::from(parser.value().map_err(usage)?)),
            Long("skip") => skip = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            other => return Err(usage(other.unexpected())),
        }
    }
    if !run && (logs.is_some() || skip.is_some()) {
        return Err(Failure::Usage("wish takes --logs and --skip only with --run".to_string()));
    }
    let apart = if run { Some(found(logs, &skip.unwrap_or_default())?) } else { None };
    match (say, project) {
        (Some(say), Some(project)) => Ok(Command::Wish { date, say, project, apart }),
        _ => Err(Failure::Usage("wish takes --say and --project".to_string())),
    }
}

/// The options of a run whose repo is found by its name in the session logs: the logs named, or Claude Code's
/// own, the folder names switched off, and the harness `JELLYMOON_CLAUDE` names.
fn found(logs: Option<PathBuf>, skip: &str) -> Result<RunOptions, Failure> {
    let logs = logs.or_else(|| harness_folder().ok().map(|folder| folder.join("projects")));
    let skip = jellymoon_core::skip_names(skip).map_err(Failure::Usage)?;
    Ok(RunOptions { logs, harness: named("JELLYMOON_CLAUDE"), skip, ..RunOptions::default() })
}

/// `run` and `run stop`. The logs are read only when no repo is named, to find the one folder of the wish's project.
fn parse_run(mut parser: lexopt::Parser) -> Result<Command, Failure> {
    let usage = |e: lexopt::Error| Failure::Usage(e.to_string());
    let (mut options, mut logs, mut skip) = (RunOptions::default(), None, String::new());
    let (mut wish, mut stop, mut first) = (None, false, true);
    while let Some(arg) = parser.next().map_err(usage)? {
        match arg {
            Long("help") | Short('h') => return Ok(Command::Help),
            Value(word) if first && word == "stop" => stop = true,
            Long("wish") => wish = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("date") => options.day = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("repo") if !stop => options.repo = Some(PathBuf::from(parser.value().map_err(usage)?)),
            Long("logs") if !stop => logs = Some(PathBuf::from(parser.value().map_err(usage)?)),
            Long("skip") if !stop => skip = parser.value().and_then(|v| v.string()).map_err(usage)?,
            other => return Err(usage(other.unexpected())),
        }
        first = false;
    }
    let wish = wish.ok_or_else(|| Failure::Usage("run takes --wish".to_string()))?;
    let options = RunOptions { wish, day: options.day, repo: options.repo, ..found(logs, &skip)? };
    Ok(if stop { Command::Stop(options) } else { Command::Run(options) })
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
