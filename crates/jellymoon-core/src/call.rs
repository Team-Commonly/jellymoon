//! The reading call: one run of the person's own Claude Code that can return text and do nothing else. Every tool, MCP
//! server, slash command, saved session and user-level setting is off. The day goes in on stdin and nowhere else; the
//! arguments are fixed but for the model's name; the environment is a short list of this process's (the harness's own
//! sign-in is under the home or its config dir); the folder is new, empty and removed afterwards.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};
use std::{env, fs, io, path, thread};

use serde::Deserialize;

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

/// The harness's file names, in the order tried in each folder of `PATH`. On Windows it is a program or a command script.
const PROGRAMS: &[&str] = if cfg!(windows) { &["claude.exe", "claude.cmd", "claude"] } else { &["claude"] };

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

struct Output {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// Makes the reading call and returns the model's text.
pub(crate) fn reading(day: &ReaderDay, options: &PageOptions) -> Result<String, Error> {
    let harness = match &options.harness {
        Some(harness) => path::absolute(harness).map_err(Error::Call)?,
        None => on_path()?,
    };
    private_dir(&options.dir).map_err(Error::Call)?;
    let folder = tempfile::Builder::new().prefix("call-").tempdir_in(&options.dir).map_err(Error::Call)?;
    let mut command = Command::new(harness);
    command.args(["-p", "--model", &options.model, "--tools", "", "--strict-mcp-config", "--disable-slash-commands"]);
    command.args([
        "--no-session-persistence",
        "--setting-sources",
        "local",
        "--output-format",
        "json",
        "--system-prompt",
        PROMPT,
    ]);
    command.current_dir(folder.path()).env_clear();
    command.envs(ENVIRONMENT.iter().filter_map(|name| Some((name, env::var_os(name)?))));
    let output = run(command, format!("{day}\n").into_bytes(), options.limit)?;
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

/// Finds `claude` in the folders of `PATH`.
fn on_path() -> Result<PathBuf, Error> {
    let folders = env::var_os("PATH").unwrap_or_default();
    let found = env::split_paths(&folders)
        .filter(|folder| !folder.as_os_str().is_empty())
        .flat_map(|folder| PROGRAMS.iter().map(move |name| folder.join(name)))
        .find(|program| fs::metadata(program).is_ok_and(|meta| meta.is_file() && runnable(&meta)));
    // The child starts in another folder, so a relative entry of PATH must not be read from there.
    path::absolute(found.ok_or(Error::NoHarness)?).map_err(Error::Call)
}

#[cfg(unix)]
fn runnable(meta: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn runnable(_: &fs::Metadata) -> bool {
    true
}

/// Makes the folder the call folders go in, for this user only where the system has file modes. On Windows the folder
/// is under the user's profile, which other users cannot read.
fn private_dir(dir: &Path) -> io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder.create(dir)
}

/// Runs the command with `input` on its stdin and returns what it printed. One still running at `limit` is stopped.
fn run(mut command: Command, input: Vec<u8>, limit: Duration) -> Result<Output, Error> {
    let mut child =
        command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(Error::Call)?;
    let stdin = child.stdin.take();
    // A harness that exits without reading its input leaves this write failing, which is its affair.
    thread::spawn(move || stdin.map(|mut stdin| stdin.write_all(&input)));
    let (stdout, stderr) = (collect(child.stdout.take()), collect(child.stderr.take()));
    let deadline = Instant::now() + limit;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(Error::Call)? {
            break status;
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
fn collect(pipe: Option<impl Read + Send + 'static>) -> Receiver<Vec<u8>> {
    let (send, receive) = mpsc::channel();
    thread::spawn(move || {
        let mut read = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut read);
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
        let stopped = run(command, b"the day\n".to_vec(), Duration::from_millis(300));
        assert!(matches!(stopped, Err(Error::TooLong(_))), "want the call stopped at its limit");
        assert!(start.elapsed() < Duration::from_secs(10), "stopped only after {:?}", start.elapsed());
    }

    #[test]
    fn a_call_that_ends_in_time_gives_its_exit_and_what_it_printed() {
        let ended =
            run(this_test_program("call::tests::a_child_that_never_answers"), Vec::new(), Duration::from_secs(30))
                .unwrap();
        assert!(ended.status.success() && String::from_utf8_lossy(&ended.stdout).contains("1 passed"));
    }

    #[test]
    fn what_a_harness_printed_is_one_line_cut_and_has_credentials_replaced_before_it_is_an_error() {
        let key = format!("sk{}{}", "-ant-", "Zx9Yw8Vu7T".repeat(3));
        assert_eq!(line(&format!("  bad key {key}\u{1b}[0m\nline two\n"), SAID_MAX), "bad key [redacted][0m line two");
        assert_eq!(line(&"x".repeat(400), SAID_MAX).chars().count(), 300);
    }
}
