//! A run started apart (F41, `docs/development/run.md`, "A run started apart"): the `jellymoon` program given is
//! started as `jellymoon run`, a process of its own in a process group of its own, and the caller goes on, or
//! ends, while the run goes. Nothing runs in the caller's own process: a run lives as long as the command that
//! holds its lock, and that command must be able to outlive whoever asked for it.
//!
//! The caller is answered once the wish names its run, or once the command has ended without that. The command's
//! output goes nowhere; what it says on stderr is read only until then, 4 KiB of it at most, and its first line is
//! given as the reason through `redact::line`. After that a caller that lives on reads it and drops it, and one
//! that has ended leaves the pipe closed, which the command does not notice.

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::child::stderr_head;
use super::{RunOptions, going, had, no_wish, refused};
use crate::busy::or_test;
use crate::redact::line;
use crate::wish::{self, MadeWish};
use crate::{Error, find};

/// How long the wish is waited for to name its run: longer than a copy of a large repo takes to make. A command
/// still going then is left going, and the answer says so. A test's build reads a shorter wait.
const WAIT: Duration = Duration::from_secs(120);
/// How long what a command that has ended said is waited for.
const SAID: Duration = Duration::from_secs(5);
/// The most characters of a reason the command gave.
const WHY_MAX: usize = 300;

/// What became of a run that was asked for apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Started {
    /// The run the wish names now. None: no run was started.
    pub run: Option<String>,
    /// How the run stands, as the wish has it, or why none was started: core's own words, or the first line the
    /// command said, cleaned.
    pub why: String,
}

/// Starts `program run` for a made wish and returns once the wish names its run, or the command has ended. The
/// program is the `jellymoon` command, named by a full path, and is started as it is given: nothing is looked up.
/// Refused with nothing started: a program not named by a full path, a day that is none, a wish that is not there
/// or has had its run. With no harness nothing is started either, and the answer says where one was looked for.
pub fn run_apart(home: &Path, program: &Path, options: &RunOptions) -> Result<Started, Error> {
    let day = wish::day_of(options.day.as_deref())?;
    if !program.is_absolute() {
        return Err(refused("nothing was started: the jellymoon command is not named by a full path"));
    }
    let wish = named(home, &day, &options.wish)?.ok_or_else(|| no_wish(&day))?;
    if let Some(run) = &wish.run {
        return Err(had(&wish.id, run));
    }
    // Asked here as the command asks it, so that its reason is core's own line and no command is started for it.
    if let Err(e @ Error::NoHarness(_)) = find::named_or_found(options.harness.as_deref()) {
        return Ok(Started { run: None, why: e.to_string() });
    }
    let mut command = Command::new(program);
    command.args(["run", "--wish", &wish.id, "--date", &day]).env("JELLYMOON_HOME", home);
    for (flag, value) in [("--repo", &options.repo), ("--logs", &options.logs)] {
        if let Some(value) = value {
            command.arg(flag).arg(value);
        }
    }
    if !options.skip.is_empty() {
        command.arg("--skip").arg(options.skip.join(","));
    }
    if let Some(harness) = &options.harness {
        command.env("JELLYMOON_CLAUDE", harness);
    }
    command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped());
    own_group(&mut command);
    let child = command.spawn();
    let mut child = child.map_err(|e| refused(format!("nothing was started: jellymoon could not be started: {e}")))?;
    let said = stderr_head(child.stderr.take());
    let deadline = Instant::now() + or_test(WAIT, "JELLYMOON_TEST_APART_WAIT_MS");
    let run_of = || named(home, &day, &wish.id).ok().flatten().filter(|wish| wish.run.is_some());
    loop {
        if let Some(wish) = run_of() {
            left(child);
            return Ok(started(wish));
        }
        match child.try_wait() {
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                left(child);
                let why = "no run has been started yet: jellymoon run is still at work, and the page shows the run \
                           when it has one";
                return Ok(Started { run: None, why: why.to_string() });
            }
            ended => {
                // The command may have named the run, and even ended it, since the wish was last read.
                let fine = matches!(ended, Ok(Some(status)) if status.success());
                let why = || reason(&said.recv_timeout(SAID).unwrap_or_default(), fine);
                return Ok(run_of().map_or_else(|| Started { run: None, why: why() }, started));
            }
        }
    }
}

fn named(home: &Path, day: &str, wish: &str) -> Result<Option<MadeWish>, Error> {
    Ok(wish::kept_of(home, day)?.into_iter().find(|kept| kept.id == wish))
}

fn started(wish: MadeWish) -> Started {
    let why = wish.why.unwrap_or_else(|| wish.run.as_deref().map(going).unwrap_or_default());
    Started { run: wish.run, why }
}

/// Why a command that ended started no run: the first line it said, without its own name, cleaned and cut. One
/// that said nothing is given fixed words.
fn reason(said: &str, fine: bool) -> String {
    let first = said.lines().next().unwrap_or_default();
    let first = line(first.strip_prefix("jellymoon: ").unwrap_or(first), WHY_MAX);
    match (first.is_empty(), fine) {
        (false, _) => first,
        (true, true) => "nothing was started: jellymoon run ended and started no run".to_string(),
        (true, false) => "nothing was started: jellymoon run failed and gave no reason".to_string(),
    }
}

/// A command that is left going is waited for by a thread of its own, so that a caller that lives on (the app)
/// keeps no ended process uncollected. A caller that ends first leaves the command to the system.
fn left(mut child: Child) {
    thread::spawn(move || child.wait());
}

/// The command gets a process group of its own: what is sent to the caller's group (an interrupt typed at its
/// terminal) does not reach the run. On Windows it also gets no window of its own.
fn own_group(command: &mut Command) {
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(command, 0);
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(command, 0x0000_0200 | 0x0800_0000);
}
