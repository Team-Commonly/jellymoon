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
use crate::redact::{WINDOW, line};
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

/// The most bytes that are kept of what a command says on stderr, whoever reads it: 128 KiB. What is said after
/// that is read and dropped ([`collect`]). Nothing reads further in: the reading call's harness and every git
/// step of a run's copy (`copy/git.rs`) make an error's one line of it with `redact::line`, which looks at the
/// first [`SAID_MAX`] and [`WINDOW`] characters and at nothing after them, and a git step that asks whether
/// anything was said at all asks it of that line. Those characters are at most four bytes each, so they are all
/// within what is kept, and every such line is what it was when all of stderr was kept.
pub(crate) const SAID_KEPT: usize = 128 * 1024;
const _: () = assert!(SAID_KEPT >= 4 * (SAID_MAX + WINDOW));

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

/// Runs the command with `input` on its stdin and returns what it printed, all of it, and the first
/// [`SAID_KEPT`] bytes of what it said on stderr. One still running at `limit` is stopped. With `from`, a file of
/// a test's (`busy::test_file`), the limit is counted from when that file is there.
pub(crate) fn run(command: Command, input: Vec<u8>, limit: Duration, from: Option<&Path>) -> Result<Output, Error> {
    run_most(command, input, limit, from, usize::MAX)
}

/// [`run`], reading no more than `most` bytes of what the command prints on stdout. That pipe is let go of
/// there, so a command that prints more is ended by its own next write: what it exited with then says nothing.
/// Of what it says on stderr no more than `most` bytes are kept either, and never more than [`SAID_KEPT`], but
/// that pipe is read to its end: a command is never ended for what it says beside its answer.
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
    let stdout = collect(child.stdout.take(), most, false);
    let stderr = collect(child.stderr.take(), most.min(SAID_KEPT), true);
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

/// Reads a pipe on a thread of its own, so a command that prints a lot is never left waiting to be read, and
/// hands over its first `most` bytes as soon as it has them, or all of it at its end. With `heard_out` the thread
/// then goes on reading, into nothing, until the pipe ends or fails: the pipe stays open and is never full, so
/// its writer is neither ended by a write nor left waiting on one, on any system. The thread is waited for by
/// nobody, and ends when every holder of the pipe's other end has closed it: the command, by ending or by being
/// killed at its limit, and whatever it started. Without `heard_out` the pipe is let go of at `most` bytes.
fn collect(pipe: Option<impl Read + Send + 'static>, most: usize, heard_out: bool) -> Receiver<Vec<u8>> {
    let (send, receive) = mpsc::channel();
    thread::spawn(move || {
        let (mut read, mut pipe) = (Vec::new(), pipe);
        if let Some(pipe) = pipe.as_mut() {
            let _ = pipe.by_ref().take(u64::try_from(most).unwrap_or(u64::MAX)).read_to_end(&mut read);
        }
        let _ = send.send(read);
        if let (true, Some(mut pipe)) = (heard_out, pipe) {
            let _ = io::copy(&mut pipe, &mut io::sink());
        }
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

    /// Not a test of its own either: a child that says a megabyte on stderr, in 256 writes with a pause after
    /// each, and fails when one of them is not taken. Then it prints one word.
    #[test]
    fn a_child_that_says_a_lot() {
        if env::var_os("JELLYMOON_TEST_SAYS_A_LOT").is_some() {
            for _ in 0..256 {
                std::io::stderr().write_all(&[b'e'; 4096]).expect("what it says is read to its end");
                thread::sleep(Duration::from_millis(1));
            }
            println!("said-it-all");
        }
    }

    // The gate's finding on the follow-up to F48. A read that stops at a size keeps no more than that of what the
    // command says beside its answer, and a call that reads everything keeps `SAID_KEPT`. Neither lets go of the
    // pipe: a command that goes on saying things is heard out, ends as itself, and its answer is whole.
    #[test]
    fn a_command_that_says_far_more_than_is_kept_is_heard_out_and_ends_as_itself() {
        for (most, kept) in [(4096, 4096), (usize::MAX, SAID_KEPT)] {
            let mut command = this_test_program("call::tests::a_child_that_says_a_lot");
            command.env("JELLYMOON_TEST_SAYS_A_LOT", "1");
            let ran = run_most(command, Vec::new(), Duration::from_secs(30), None, most).unwrap();
            let printed = String::from_utf8_lossy(&ran.stdout);
            assert!(ran.status.success(), "read to {most}: the command was ended, {}: {printed}", ran.status);
            assert!(printed.contains("said-it-all") && printed.contains("1 passed"), "read to {most}: {printed}");
            assert_eq!(ran.stderr.len(), kept, "read to {most}");
            assert!(ran.stderr.iter().all(|byte| *byte == b'e'), "what is kept is the start of what was said");
        }
    }

    /// Not a test of its own either: a child that prints until a write is not taken, a minute at most.
    #[test]
    fn a_child_that_prints_without_end() {
        if env::var_os("JELLYMOON_TEST_PRINTS_ON").is_some() {
            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(60) && std::io::stdout().write_all(&[b'o'; 4096]).is_ok() {}
        }
    }

    // What a command prints is read to a size and its pipe let go of there, which is what ends a command that
    // prints more: only what it says beside its answer is heard out.
    #[test]
    fn a_command_that_prints_past_the_size_read_is_ended_by_its_own_next_write() {
        let mut command = this_test_program("call::tests::a_child_that_prints_without_end");
        command.env("JELLYMOON_TEST_PRINTS_ON", "1");
        let start = Instant::now();
        let ran = run_most(command, Vec::new(), Duration::from_secs(20), None, 4096).unwrap();
        assert_eq!(ran.stdout.len(), 4096);
        assert!(start.elapsed() < Duration::from_secs(15), "ended only after {:?}", start.elapsed());
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
