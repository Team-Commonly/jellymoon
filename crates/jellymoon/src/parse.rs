//! The command line of `jellymoon`, read into a [`Command`]. A flag's value is whatever follows it; what a value
//! may be is core's rule, so any text is taken here.

use std::path::PathBuf;

use jellymoon_core::{DayOptions, HearOptions, KeepOptions, Outcome, RunOptions};
use lexopt::prelude::*;

use crate::{Failure, harness_folder, named, own_folder};

/// What a wish is made of: a project, or with `--after` the work of a made wish (a follow-up).
pub enum WishOf {
    Project(String),
    After(String),
}

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
    /// `apart`: with `--run`, the options of the run to start, but for the wish and the day. `shaped`: kept with
    /// the shape that waits for these words (`shape.rs`).
    Wish {
        date: Option<String>,
        say: String,
        of: WishOf,
        apart: Option<RunOptions>,
        shaped: bool,
    },
    /// `shape`: the options name no wish.
    Shape(RunOptions, String, String),
    Hear(HearOptions),
    /// `hear --settings`.
    Voice,
    Run(RunOptions),
    Stop(RunOptions),
    /// `run answer`: the answer itself is read from the command's input.
    Answer(RunOptions),
    /// `run show`, and the number of the one file to print.
    Show(RunOptions, Option<usize>),
    /// `run keep`, and whether it only asks and whether files the person's tools obey are kept too.
    Keep(RunOptions, KeepOptions),
    Toss(RunOptions),
    /// `run undo`, and whether only asked (`--ask`).
    Undo(RunOptions, bool),
    /// `run waiting`: the options name no wish.
    Waiting(RunOptions),
    RemoveHook,
    HookStatus,
    Uninstall,
    Harness,
    MakeCopy {
        repo: PathBuf,
        run: Option<String>,
    },
    CleanCopy {
        run: String,
        outcome: Outcome,
    },
    Project(Listing),
}

/// A `project` command: `add` and `new` with the day the project is used on.
pub enum Listing {
    Add { repo: PathBuf, date: Option<String> },
    New { name: String, date: Option<String> },
    List,
    Forget { name: String },
}

/// Reads the command line. A flag's value is whatever follows it, even text that starts with dashes: a model name
/// shaped like a flag is taken here and refused by the check on names, never read as a flag of ours.
pub fn parse(mut parser: lexopt::Parser) -> Result<Command, Failure> {
    let usage = |e: lexopt::Error| Failure::Usage(e.to_string());
    let command = match parser.next().map_err(usage)? {
        Some(Value(command)) if command == "day" || command == "page" || command == "help" => command,
        Some(Value(command)) if command == "uninstall" || command == "harness" => {
            return match parser.next().map_err(usage)? {
                None if command == "harness" => Ok(Command::Harness),
                None => Ok(Command::Uninstall),
                Some(Long("help") | Short('h')) => Ok(Command::Help),
                Some(other) => Err(usage(other.unexpected())),
            };
        }
        Some(Value(command)) if command == "rule" => return parse_rule(parser),
        Some(Value(command)) if command == "settle" => return parse_settle(parser),
        Some(Value(command)) if command == "wish" => return parse_wish(parser),
        Some(Value(command)) if command == "shape" => return crate::shape::parse(parser),
        Some(Value(command)) if command == "hear" => return parse_hear(parser),
        Some(Value(command)) if command == "run" => return parse_run(parser),
        Some(Value(command)) if command == "hook" => return parse_hook(parser),
        Some(Value(command)) if command == "copy" => return parse_copy(parser),
        Some(Value(command)) if command == "project" => return parse_project(parser),
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

/// An option's value that is a number: digits and nothing else, so `+1` is not one, though it would parse as one.
fn number(parser: &mut lexopt::Parser, takes: &str) -> Result<usize, Failure> {
    let value = parser.value().and_then(|v| v.string()).map_err(|e| Failure::Usage(e.to_string()))?;
    let digits = !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit());
    let number = digits.then(|| value.parse::<usize>().ok()).flatten();
    number.ok_or_else(|| Failure::Usage(format!("{takes} a number, and {value:?} is not one")))
}

fn parse_settle(mut parser: lexopt::Parser) -> Result<Command, Failure> {
    let usage = |e: lexopt::Error| Failure::Usage(e.to_string());
    let (mut date, mut clash, mut option) = (None, None, None);
    let takes = "--clash and --option take";
    while let Some(arg) = parser.next().map_err(usage)? {
        match arg {
            Long("help") | Short('h') => return Ok(Command::Help),
            Long("date") => date = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("clash") => clash = Some(number(&mut parser, takes)?),
            Long("option") => option = Some(number(&mut parser, takes)?),
            other => return Err(usage(other.unexpected())),
        }
    }
    match (date, clash, option) {
        (Some(date), Some(clash), Some(option)) => Ok(Command::Settle { date, clash, option }),
        _ => Err(Failure::Usage("settle takes --date, --clash and --option".to_string())),
    }
}

const NEITHER: &str = "wish takes --say and --project or --after";

fn parse_wish(mut parser: lexopt::Parser) -> Result<Command, Failure> {
    let usage = |e: lexopt::Error| Failure::Usage(e.to_string());
    let (mut date, mut say, mut project, mut after) = (None, None, None, None);
    let (mut run, mut logs, mut skip, mut shaped) = (false, None, None, false);
    while let Some(arg) = parser.next().map_err(usage)? {
        match arg {
            Long("help") | Short('h') => return Ok(Command::Help),
            Long("date") => date = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("say") => say = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("project") => project = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("after") => after = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("run") => run = true,
            Long("shaped") => shaped = true,
            Long("logs") => logs = Some(PathBuf::from(parser.value().map_err(usage)?)),
            Long("skip") => skip = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            other => return Err(usage(other.unexpected())),
        }
    }
    if !run && (logs.is_some() || skip.is_some()) {
        return Err(Failure::Usage("wish takes --logs and --skip only with --run".to_string()));
    }
    let apart = if run { Some(found(logs, &skip.unwrap_or_default())?) } else { None };
    let of = match (project, after) {
        (Some(project), None) => WishOf::Project(project),
        (None, Some(_)) if shaped => return Err(Failure::Usage("wish takes --shaped with --project".to_string())),
        (None, Some(after)) => WishOf::After(after),
        (Some(_), Some(_)) => {
            let both =
                "wish takes --project or --after, not both: a follow-up is for the project of the wish it follows";
            return Err(Failure::Usage(both.to_string()));
        }
        (None, None) => return Err(Failure::Usage(NEITHER.to_string())),
    };
    match say {
        Some(say) => Ok(Command::Wish { date, say, of, apart, shaped }),
        None => Err(Failure::Usage(NEITHER.to_string())),
    }
}

/// `hear`, and `hear --settings`, which takes nothing else. With `--run` a made wish's run is started as
/// `jellymoon wish --run` starts it: by this same program, with the same logs and names switched off.
fn parse_hear(mut parser: lexopt::Parser) -> Result<Command, Failure> {
    let usage = |e: lexopt::Error| Failure::Usage(e.to_string());
    let (mut date, mut logs, mut skip) = (None, None, String::new());
    let (mut run, mut settings, mut given) = (false, false, 0);
    while let Some(arg) = parser.next().map_err(usage)? {
        match arg {
            Long("help") | Short('h') => return Ok(Command::Help),
            Long("settings") => settings = true,
            Long("date") => date = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("logs") => logs = Some(PathBuf::from(parser.value().map_err(usage)?)),
            Long("skip") => skip = parser.value().and_then(|v| v.string()).map_err(usage)?,
            Long("run") => run = true,
            other => return Err(usage(other.unexpected())),
        }
        given += 1;
    }
    if settings {
        let alone = Failure::Usage("hear --settings takes nothing else".to_string());
        return if given == 1 { Ok(Command::Voice) } else { Err(alone) };
    }
    if logs.as_ref().is_some_and(|logs| logs.to_string_lossy().trim().is_empty()) {
        return Err(Failure::Usage("--logs needs a folder".to_string()));
    }
    let apart = found(logs.clone(), &skip)?;
    let logs = match logs {
        Some(logs) => logs,
        None => harness_folder()?.join("projects"),
    };
    let read = DayOptions { logs, date, skip: apart.skip.clone(), home: own_folder()? };
    let program = || std::env::current_exe().map_err(|e| Failure::Run(format!("finding this program: {e}")));
    let apart = if run { Some((program()?, apart)) } else { None };
    Ok(Command::Hear(HearOptions { read, apart }))
}

/// The options of a run whose repo is found by its name in the session logs: the logs named, or Claude Code's
/// own, the folder names switched off, and the harness `JELLYMOON_CLAUDE` names.
pub fn found(logs: Option<PathBuf>, skip: &str) -> Result<RunOptions, Failure> {
    let logs = logs.or_else(|| harness_folder().ok().map(|folder| folder.join("projects")));
    let skip = jellymoon_core::skip_names(skip).map_err(Failure::Usage)?;
    Ok(RunOptions { logs, harness: named("JELLYMOON_CLAUDE"), skip, ..RunOptions::default() })
}

/// `run`, `run stop`, `run answer`, `run show`, `run keep`, `run undo`, `run toss` and `run waiting`. The logs
/// are read only when no repo is named, to find the one folder of the wish's project. A stop and an answer read
/// the home alone, and take no repo and no logs. A file's number is digits and nothing else.
fn parse_run(mut parser: lexopt::Parser) -> Result<Command, Failure> {
    let usage = |e: lexopt::Error| Failure::Usage(e.to_string());
    let (mut options, mut logs, mut skip) = (RunOptions::default(), None, String::new());
    let (mut wish, mut stop, mut show, mut file, mut first) = (None, false, false, None, true);
    let (mut keep, mut toss, mut waiting, mut answer): (Option<KeepOptions>, bool, bool, bool) = Default::default();
    let mut undo = None;
    while let Some(arg) = parser.next().map_err(usage)? {
        match arg {
            Long("help") | Short('h') => return Ok(Command::Help),
            Value(word) if first && word == "stop" => stop = true,
            Value(word) if first && word == "undo" => undo = Some(false),
            Long("ask") if undo.is_some() => undo = Some(true),
            Value(word) if first && word == "answer" => (stop, answer) = (true, true),
            Value(word) if first && word == "show" => show = true,
            Value(word) if first && word == "keep" => keep = Some(KeepOptions::default()),
            Value(word) if first && word == "toss" => toss = true,
            Value(word) if first && word == "waiting" => waiting = true,
            Long("ask") if keep.is_some() => keep = keep.map(|keep| KeepOptions { ask: true, ..keep }),
            Long("with-settings") if keep.is_some() => {
                keep = keep.map(|keep| KeepOptions { with_settings: true, ..keep })
            }
            Long("file") if show => file = Some(number(&mut parser, "--file takes")?),
            Long("wish") if !waiting => wish = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("date") => options.day = Some(parser.value().and_then(|v| v.string()).map_err(usage)?),
            Long("repo") if !stop => options.repo = Some(PathBuf::from(parser.value().map_err(usage)?)),
            Long("logs") if !stop => logs = Some(PathBuf::from(parser.value().map_err(usage)?)),
            Long("skip") if !stop => skip = parser.value().and_then(|v| v.string()).map_err(usage)?,
            other => return Err(usage(other.unexpected())),
        }
        first = false;
    }
    if waiting {
        let options = RunOptions { day: options.day, repo: options.repo, ..found(logs, &skip)? };
        return Ok(Command::Waiting(options));
    }
    let wish = wish.ok_or_else(|| Failure::Usage("run takes --wish".to_string()))?;
    let options = RunOptions { wish, day: options.day, repo: options.repo, ..found(logs, &skip)? };
    Ok(match (stop, show, keep) {
        _ if answer => Command::Answer(options),
        (true, ..) => Command::Stop(options),
        (_, true, _) => Command::Show(options, file),
        (_, _, Some(keep)) => Command::Keep(options, keep),
        _ if toss => Command::Toss(options),
        _ if undo.is_some() => Command::Undo(options, undo == Some(true)),
        _ => Command::Run(options),
    })
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

/// `project add`, `project new`, `project list` and `project forget`. What a name may be is core's rule: any text is taken.
/// Whatever is wrong with the command line is answered in one fixed line: an argument that is not expected may
/// be a folder with `--repo` forgotten, or a credential, and lexopt's own words would say it back.
fn parse_project(mut parser: lexopt::Parser) -> Result<Command, Failure> {
    let usage = || {
        let is = "project is `project add --repo <folder>`, `project new <name>`, `project list` or `project \
                  forget <name>`";
        Failure::Usage(is.to_string())
    };
    let (mut action, mut repo, mut date, mut names) = (String::new(), None, None, Vec::new());
    while let Some(arg) = parser.next().map_err(|_| usage())? {
        match arg {
            Long("help") | Short('h') => return Ok(Command::Help),
            Value(word) if action.is_empty() && ["add", "new", "list", "forget"].iter().any(|known| word == *known) => {
                action = word.string().map_err(|_| usage())?
            }
            Value(name) if action == "forget" || action == "new" => names.push(name.string().map_err(|_| usage())?),
            Long("repo") if action == "add" => repo = Some(PathBuf::from(parser.value().map_err(|_| usage())?)),
            Long("date") if action == "add" || action == "new" => {
                date = Some(parser.value().and_then(|v| v.string()).map_err(|_| usage())?)
            }
            _ => return Err(usage()),
        }
    }
    match (action.as_str(), repo, names.len()) {
        ("add", Some(repo), _) => Ok(Listing::Add { repo, date }),
        ("new", None, 1) => Ok(Listing::New { name: names.remove(0), date }),
        ("list", ..) => Ok(Listing::List),
        ("forget", _, 1) => Ok(Listing::Forget { name: names.remove(0) }),
        _ => Err(usage()),
    }
    .map(Command::Project)
}
