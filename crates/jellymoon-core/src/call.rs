//! The reading call: one run of the person's own Claude Code that can return text and do nothing else. Every tool, MCP
//! server, slash command, saved session and user-level setting is off. The day goes in on stdin and nowhere else; the
//! arguments are fixed but for the model's name; the environment is a short list of this process's (the harness's own
//! sign-in is under the home or its config dir), with a longer `PATH` when the harness was not found on it (`find.rs`); the folder is new, empty and removed afterwards.

use std::ffi::{OsStr, OsString};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};
use std::{env, fs, io, thread};

use serde::Deserialize;

use crate::find::{self, Found};
use crate::redact::line;
use crate::{Error, PageOptions, ReaderDay};

/// The whole system prompt of the reading call: a fixed text, so nothing in a day can change it.
const PROMPT: &str = include_str!("prompt.md");

/// What the harness may see of this process's environment: enough to find its own sign-in and to start at all.
#[cfg(not(windows))]
const ENVIRONMENT: &[&str] = &["PATH", "HOME", "TMPDIR", "USER", "LANG", "LC_ALL", "TERM", "CLAUDE_CONFIG_DIR"];
#[cfg(windows)]
const ENVIRONMENT: &[&str] = &[
    "PATH",
    "PATHEXT",
    "SystemRoot",
    "SystemDrive",
    "ComSpec",
    "USERPROFILE",
    "HOMEDRIVE",
    "HOMEPATH",
    "APPDATA",
    "LOCALAPPDATA",
    "TEMP",
    "TMP",
    "USERNAME",
    "CLAUDE_CONFIG_DIR",
];

/// The variables that name one folder or one file, here and in git's list (`copy/git.rs`).
const PLACES: &[&str] = &[
    "HOME",
    "TMPDIR",
    "CLAUDE_CONFIG_DIR",
    "XDG_CONFIG_HOME",
    "GIT_CONFIG_GLOBAL",
    "USERPROFILE",
    "APPDATA",
    "LOCALAPPDATA",
    "ProgramData",
    "TEMP",
    "TMP",
];
const NO_PATH: &str =
    "PATH names no folder by its full path, and a folder named any other way is not passed on to Claude Code";

/// How long a stopped or finished harness's output is waited for: something it started may still hold the pipe.
const DRAIN: Duration = Duration::from_secs(1);

/// How much of what a harness printed is kept in an error, in characters.
const SAID_MAX: usize = 300;

/// `--output-format json`: one object, the model's text in `result`.
#[derive(Deserialize)]
struct Reply {
    is_error: bool,
    #[serde(default)]
    result: String,
}

pub(crate) struct Output {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Makes the reading call and returns the model's text.
pub(crate) fn reading(day: &ReaderDay, options: &PageOptions) -> Result<String, Error> {
    let harness = find::named_or_found(options.harness.as_deref())?;
    private_dir(&options.dir).map_err(Error::Call)?;
    let folder = tempfile::Builder::new().prefix("call-").tempdir_in(&options.dir).map_err(Error::Call)?;
    let mut command = harness_of(harness).map_err(Error::Call)?;
    command.current_dir(folder.path()).args([
        "-p",
        "--model",
        &options.model,
        "--tools",
        "",
        "--strict-mcp-config",
        "--disable-slash-commands",
    ]);
    command.args([
        "--no-session-persistence",
        "--setting-sources",
        "local",
        "--output-format",
        "json",
        "--system-prompt",
        PROMPT,
    ]);
    let output = run(command, format!("{day}\n").into_bytes(), options.limit, None)?;
    if !output.status.success() {
        return Err(Error::Failed {
            status: output.status.to_string(),
            said: line(&String::from_utf8_lossy(&output.stderr), SAID_MAX),
        });
    }
    match serde_json::from_slice::<Reply>(&output.stdout) {
        Ok(reply) if reply.is_error => Err(Error::Harness(line(&reply.result, SAID_MAX))),
        Ok(reply) => Ok(reply.result),
        Err(_) => Err(Error::NotAResult),
    }
}

/// The harness, with the short list of this process's variables and nothing else: the one way Jellymoon starts a
/// harness, for the reading call and for a run. The caller names the folder it starts in. With no folder of `PATH`
/// that is a full path there is no command: the harness would look for its programs in the folder it starts in.
pub(crate) fn harness_of(harness: Found) -> io::Result<Command> {
    let mut command = Command::new(harness.program);
    command.env_clear().envs(passed_on(ENVIRONMENT));
    let path = harness.path.or_else(|| env::var_os("PATH")).unwrap_or_default();
    let path = env::join_paths(full_folders(&path)).ok().filter(|path| !path.is_empty());
    command.env("PATH", path.ok_or_else(|| io::Error::other(NO_PATH))?);
    Ok(command)
}

/// The folders of a `PATH` that are named by a full path, in their order.
pub(crate) fn full_folders(path: &OsStr) -> Vec<PathBuf> {
    env::split_paths(path).filter(|folder| folder.is_absolute()).collect()
}

/// This process's variables of the names given, for a child that starts in another folder. A path that is not a
/// full one would be read from that folder, which for a run is a copy the agent writes: of `PATH` only the folders
/// named by a full path are passed on, and a variable that names one folder or file ([`PLACES`]) only when it is one.
pub(crate) fn passed_on(names: &[&'static str]) -> Vec<(&'static str, OsString)> {
    let full = |name: &&'static str| {
        let value = env::var_os(name)?;
        if *name == "PATH" {
            return env::join_paths(full_folders(&value))
                .ok()
                .filter(|path| !path.is_empty())
                .map(|path| (*name, path));
        }
        (!PLACES.contains(name) || Path::new(&value).is_absolute()).then_some((*name, value))
    };
    names.iter().filter_map(full).collect()
}

/// Makes the folder the call folders go in, for this user only where the system has file modes; one that is already
/// there is closed to others too. On Windows the folder is under the user's profile, which other users cannot read.
pub(crate) fn private_dir(dir: &Path) -> io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder.create(dir)?;
    #[cfg(unix)]
    fs::set_permissions(dir, std::os::unix::fs::PermissionsExt::from_mode(0o700))?;
    Ok(())
}

/// Runs the command with `input` on its stdin and returns what it printed. One still running at `limit` is stopped.
/// With `from`, a file of a test's (`busy::test_file`), the limit is counted from when that file is there.
pub(crate) fn run(command: Command, input: Vec<u8>, limit: Duration, from: Option<&Path>) -> Result<Output, Error> {
    run_most(command, input, limit, from, usize::MAX)
}

/// [`run`], reading no more than `most` bytes of what the command prints on stdout, and then no more than that
/// of what it says on stderr either. A pipe is let go of there, so a command that prints more is ended by its own
/// next write: what it exited with then says nothing. Only a caller that reads everything ([`run`]) is given all
/// that was said: it makes an error's words of it, and its command must not be ended for saying much.
pub(crate) fn run_most(
    mut command: Command,
    input: Vec<u8>,
    limit: Duration,
    from: Option<&Path>,
    most: usize,
) -> Result<Output, Error> {
    let mut child =
        command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(Error::Call)?;
    let stdin = child.stdin.take();
    // A harness that exits without reading its input leaves this write failing, which is its affair.
    thread::spawn(move || stdin.map(|mut stdin| stdin.write_all(&input)));
    let (stdout, stderr) = (collect(child.stdout.take(), most), collect(child.stderr.take(), most));
    let mut deadline = Instant::now() + limit;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(Error::Call)? {
            break status;
        }
        if from.is_some_and(|from| !from.exists()) {
            deadline = Instant::now() + limit;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::TooLong(limit));
        }
        thread::sleep(Duration::from_millis(10));
    };
    let drained = |pipe: Receiver<Vec<u8>>| pipe.recv_timeout(DRAIN).unwrap_or_default();
    Ok(Output { status, stdout: drained(stdout), stderr: drained(stderr) })
}

/// Reads a pipe to its end on a thread of its own, so a harness that prints a lot is never left waiting to be read.
fn collect(pipe: Option<impl Read + Send + 'static>, most: usize) -> Receiver<Vec<u8>> {
    let (send, receive) = mpsc::channel();
    thread::spawn(move || {
        let mut read = Vec::new();
        if let Some(pipe) = pipe {
            let _ = pipe.take(u64::try_from(most).unwrap_or(u64::MAX)).read_to_end(&mut read);
        }
        let _ = send.send(read);
    });
    receive
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Not a test of its own: the child of the test below, which runs this test program again with the variable set.
    #[test]
    fn a_child_that_never_answers() {
        if env::var_os("JELLYMOON_TEST_NEVER_ANSWERS").is_some() {
            thread::sleep(Duration::from_secs(60));
        }
    }

    fn this_test_program(test: &str) -> Command {
        let mut command = Command::new(env::current_exe().unwrap());
        command.args(["--exact", test, "--nocapture"]);
        command
    }

    // Item 37. The command's own limit is three minutes; here it is made short.
    #[test]
    fn a_call_that_runs_too_long_is_stopped() {
        let mut command = this_test_program("call::tests::a_child_that_never_answers");
        command.env("JELLYMOON_TEST_NEVER_ANSWERS", "1");
        let start = Instant::now();
        let stopped = run(command, b"the day\n".to_vec(), Duration::from_millis(300), None);
        assert!(matches!(stopped, Err(Error::TooLong(_))), "want the call stopped at its limit");
        assert!(start.elapsed() < Duration::from_secs(10), "stopped only after {:?}", start.elapsed());
    }

    /// Not a test of its own either: a child that says more than a megabyte on stderr and then one word on stdout.
    #[test]
    fn a_child_that_says_a_lot() {
        if env::var_os("JELLYMOON_TEST_SAYS_A_LOT").is_some() {
            let said = std::io::stderr().write_all(&vec![b'e'; 1 << 20]);
            println!("{}", if said.is_ok() { "said-it-all" } else { "was-not-heard" });
        }
    }

    // A read that stops at a size stops there for what the command says beside its answer, too.
    #[test]
    fn a_bounded_call_keeps_no_more_of_what_the_command_says_than_of_what_it_prints() {
        let child = || {
            let mut command = this_test_program("call::tests::a_child_that_says_a_lot");
            command.env("JELLYMOON_TEST_SAYS_A_LOT", "1");
            command
        };
        let bounded = run_most(child(), Vec::new(), Duration::from_secs(30), None, 4096).unwrap();
        assert_eq!(bounded.stderr.len(), 4096);
        // A call that reads everything is given everything, and its command goes on to its end.
        let whole = run(child(), Vec::new(), Duration::from_secs(30), None).unwrap();
        assert_eq!(whole.stderr.len(), 1 << 20);
        assert!(whole.status.success() && String::from_utf8_lossy(&whole.stdout).contains("said-it-all"));
    }

    #[test]
    fn a_call_that_ends_in_time_gives_its_exit_and_what_it_printed() {
        let ended = run(
            this_test_program("call::tests::a_child_that_never_answers"),
            Vec::new(),
            Duration::from_secs(30),
            None,
        )
        .unwrap();
        assert!(ended.status.success() && String::from_utf8_lossy(&ended.stdout).contains("1 passed"));
    }

    #[test]
    fn what_a_harness_printed_is_one_line_cut_and_has_credentials_replaced_before_it_is_an_error() {
        let key = format!("sk{}{}", "-ant-", "Zx9Yw8Vu7T".repeat(3));
        assert_eq!(
            line(&format!("  bad key \u{1b}[31m{key}\u{1b}[0m\nline two\n"), SAID_MAX),
            "bad key [redacted] line two"
        );
        assert_eq!(line(&"x".repeat(400), SAID_MAX).chars().count(), 300);
    }
}
