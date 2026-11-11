//! The harness child of a run (`docs/development/run.md`, "What is read of the stream"). The wish's words go in on its
//! input and nowhere else. What it prints is untrusted: a line is read up to [`LINE_MAX`] bytes and the whole stream up
//! to [`STREAM_MAX`], and of all of it four things are taken: that a result line came, its `subtype`, its `is_error`
//! and how many `permission_denials` it lists. No text of the agent's is kept. A child still going at its limit, or
//! one that prints past the cap, is stopped. Where there are process groups the child is started in one of its
//! own, so a stop reaches what the harness itself started.

use std::io::{self, BufReader, Read, Write};
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde::de::IgnoredAny;

use crate::session::next_line;

/// The longest line of the stream that is read, with its line break. A longer one is counted and skipped.
const LINE_MAX: usize = 1 << 20;
/// The most bytes of a whole stream that are read. A child that prints more is stopped, and its run has failed.
const STREAM_MAX: u64 = 64 << 20;
/// How much of what the child said on stderr is looked at, for the one fixed sign of a harness that is too old.
const STDERR_HEAD: u64 = 4096;
/// How long a child that was asked to stop is given before it is killed.
#[cfg(unix)]
const GRACE: Duration = Duration::from_secs(5);
/// How long a child's output is waited for once it is gone: something it started may still hold the pipe.
const DRAIN: Duration = Duration::from_secs(5);

/// Why the wait for the child ended. F40's stop is one more of these, and ends the child with [`stop`] too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum End {
    Exited,
    /// It was still going at its time limit, and was stopped.
    Limit,
    /// It printed more than [`STREAM_MAX`], and was stopped.
    Over,
}

/// What is known of a child that is gone.
pub(super) struct Ended {
    pub end: End,
    /// Whether it exited with 0.
    pub exit_0: bool,
    /// The last result line: whether it says success and no error. None when no result line came.
    pub result: Option<bool>,
    pub denied: usize,
    /// The lines that were not the stream's.
    pub skipped: usize,
    /// It refused its command line as one with an option it does not know: a harness too old for runs.
    pub old: bool,
}

/// What the reader of the stream has, when the stream ends.
#[derive(Default, Clone, Copy)]
struct Stream {
    result: Option<(bool, usize)>,
    skipped: usize,
    over: bool,
}

/// The parts of a line of the stream that are looked at. A line that is not an object with a `type` that is text,
/// or that has one of the other three in another shape, is not a line of the stream. A list is not one either,
/// though it could be read as these four in order: [`line`] takes only what starts as an object.
#[derive(Deserialize)]
struct Line {
    #[serde(rename = "type")]
    kind: String,
    subtype: Option<String>,
    is_error: Option<bool>,
    #[serde(default)]
    permission_denials: Vec<IgnoredAny>,
}

fn line(raw: &[u8]) -> Option<Line> {
    let object = raw.trim_ascii_start().first() == Some(&b'{');
    serde_json::from_slice(raw).ok().filter(|_| object)
}

/// Starts the command with `said` as its whole input, and waits for it to end, `limit` at most. `started` is told
/// the child's pid as soon as there is one.
pub(super) fn run(mut command: Command, said: String, limit: Duration, started: impl FnOnce(u32)) -> io::Result<Ended> {
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    let mut child = command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    started(child.id());
    let stdin = child.stdin.take();
    // A harness that exits without reading its input leaves this write failing, which is its affair.
    thread::spawn(move || stdin.map(|mut stdin| stdin.write_all(said.as_bytes())));
    let (stream, head) = (reader(child.stdout.take()), stderr_head(child.stderr.take()));
    let deadline = Instant::now() + limit;
    let mut read = None;
    let (end, status) = loop {
        if let Some(status) = child.try_wait()? {
            break (End::Exited, Some(status));
        }
        read = read.or_else(|| stream.try_recv().ok());
        if read.is_some_and(|read: Stream| read.over) {
            break (End::Over, stop(&mut child));
        }
        if Instant::now() >= deadline {
            break (End::Limit, stop(&mut child));
        }
        thread::sleep(Duration::from_millis(10));
    };
    let read = read.or_else(|| stream.recv_timeout(DRAIN).ok()).unwrap_or_default();
    let exit_0 = end == End::Exited && status.is_some_and(|status| status.success());
    let head = head.recv_timeout(DRAIN).unwrap_or_default();
    let unknown = head.contains("unknown option") && head.contains("--restricted");
    let old = end == End::Exited && !exit_0 && read.result.is_none() && unknown;
    let end = if read.over { End::Over } else { end };
    Ok(Ended {
        end,
        exit_0,
        result: read.result.map(|(fine, _)| fine),
        denied: read.result.map_or(0, |r| r.1),
        skipped: read.skipped,
        old,
    })
}

/// Reads the stream to its end, or to the cap, on a thread of its own, and sends what it found.
fn reader(stdout: Option<ChildStdout>) -> Receiver<Stream> {
    let (send, receive) = mpsc::channel();
    thread::spawn(move || {
        let mut found = Stream::default();
        if let Some(stdout) = stdout {
            let mut pipe = BufReader::with_capacity(1 << 16, stdout.take(STREAM_MAX + 1));
            let mut raw = Vec::new();
            while let Ok(true) = next_line(&mut pipe, LINE_MAX, &mut raw) {
                match line(&raw) {
                    Some(line) if line.kind == "result" => {
                        let fine = line.subtype.as_deref() == Some("success") && line.is_error == Some(false);
                        found.result = Some((fine, line.permission_denials.len()));
                    }
                    Some(_) => {}
                    None => found.skipped += 1,
                }
            }
            // One byte past the cap was asked for: when it came, the stream is over the cap, whatever was in it.
            found.over = pipe.get_ref().limit() == 0;
        }
        let _ = send.send(found);
    });
    receive
}

/// The start of what the child said on stderr, as text; the rest is read and dropped, so the child never waits.
fn stderr_head(pipe: Option<impl Read + Send + 'static>) -> Receiver<String> {
    let (send, receive) = mpsc::channel();
    thread::spawn(move || {
        let mut head = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.by_ref().take(STDERR_HEAD).read_to_end(&mut head);
            let _ = send.send(String::from_utf8_lossy(&head).into_owned());
            let _ = io::copy(&mut pipe, &mut io::sink());
        }
    });
    receive
}

/// Stops the child and, where there are process groups, what it started: the group is the child's own, and is
/// named by the child's pid. It is asked first (`SIGTERM`), given [`GRACE`], and then killed. Windows has no such
/// asking and no group: the child alone is ended at once (`TerminateProcess`).
fn stop(child: &mut Child) -> Option<ExitStatus> {
    #[cfg(unix)]
    {
        use rustix::process::{Pid, Signal, kill_process_group};
        // The child has not been waited for, so its pid, and the group of that number, are still its own.
        let group = Pid::from_child(child);
        let asked = kill_process_group(group, Signal::TERM).is_ok();
        let deadline = Instant::now() + GRACE;
        while asked && Instant::now() < deadline {
            if let Ok(Some(status)) = child.try_wait() {
                return Some(status);
            }
            thread::sleep(Duration::from_millis(10));
        }
        let _ = kill_process_group(group, Signal::KILL);
    }
    let _ = child.kill();
    child.wait().ok()
}
