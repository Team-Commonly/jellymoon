//! The harness child of a run (`docs/development/run.md`). The wish's words go in on its input and nowhere else.
//! What it prints is untrusted, and read by `stream.rs`. A child still going at its limit, or one that prints past
//! the cap, is asked to stop and then killed; one whose stream does not start as a run's is killed at once; one the
//! person stopped (`stop.rs`) is interrupted first, then asked, then killed. Where
//! there are process groups the child is started in one of its own, and when it has ended, however it ended, that
//! group is killed: nothing the harness started outlives the run.

use std::io::{self, Read, Write};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use super::stream::{self, Stream};
use crate::busy::{or_test, test_file};

/// How much of what the child said on stderr is looked at, for the one fixed sign of a harness that is too old.
const STDERR_HEAD: u64 = 4096;
/// How long a child that was asked to stop is given before it is killed.
const GRACE: Duration = Duration::from_secs(5);
/// How long a child's output is waited for once it is gone: a program it moved to another group may hold the pipe.
const DRAIN: Duration = Duration::from_secs(5);

/// Why the wait for the child ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum End {
    Exited,
    /// It was still going at its time limit, and was stopped.
    Limit,
    /// It printed more than the stream's cap, and was stopped.
    Over,
    /// Its stream did not start as a run's does, and it was killed.
    NotARun,
    /// The person asked for the run to stop while the child was still going, or before there was one.
    Stopped,
}

/// What is known of a child that is gone.
pub(super) struct Ended {
    pub end: End,
    /// Whether it exited with 0.
    pub exit_0: bool,
    pub read: Stream,
    /// It printed no line at all and refused its command line as one with an option it does not know: a harness
    /// too old for runs.
    pub old: bool,
}

/// Starts the command with `said` as its whole input, and waits for it to end, `limit` at most. `started` is told
/// the child's pid as soon as there is one, and `began` is called once its stream has started as a run's does.
/// `stop` says whether the person has asked for the run to stop: it is
/// asked before the child is started, which then never is, and every 10 ms while the child goes. A child that has
/// ended by itself when it is asked has ended, and was not stopped.
pub(super) fn run(
    mut command: Command,
    said: String,
    limit: Duration,
    started: impl FnOnce(u32),
    began: impl FnOnce(),
    stop: impl Fn() -> bool,
) -> io::Result<Ended> {
    if stop() {
        return Ok(Ended { end: End::Stopped, exit_0: false, read: Stream::default(), old: false });
    }
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    let mut child = command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    started(child.id());
    let stdin = child.stdin.take();
    // A harness that exits without reading its input leaves this write failing, which is its affair.
    thread::spawn(move || stdin.map(|mut stdin| stdin.write_all(said.as_bytes())));
    let ((stream, so_far), head) = (stream::reader(child.stdout.take()), stderr_head(child.stderr.take()));
    let (mut deadline, grace) = (Instant::now() + limit, or_test(GRACE, "JELLYMOON_TEST_GRACE_MS"));
    // A test's limit is counted from when its made-up agent has left its mark (`busy::test_file`).
    let from = test_file("JELLYMOON_TEST_RUN_LIMIT_FROM");
    let (mut read, mut began) = (None, Some(began));
    let (end, grace) = loop {
        if let Some(began) = began.take_if(|_| so_far.lock().is_ok_and(|so_far| so_far.start == Some(true))) {
            began();
        }
        if gone(&mut child) {
            break (End::Exited, Duration::ZERO);
        }
        read = read.or_else(|| stream.try_recv().ok());
        if from.as_ref().is_some_and(|from| !from.exists()) {
            deadline = Instant::now() + limit;
        }
        match read {
            Some(Stream { over: true, .. }) => break (End::Over, grace),
            Some(Stream { start: Some(false), .. }) => break (End::NotARun, Duration::ZERO),
            _ if stop() => break (End::Stopped, grace),
            _ if Instant::now() >= deadline => break (End::Limit, grace),
            _ => thread::sleep(Duration::from_millis(10)),
        }
    };
    let status = halt(&mut child, end == End::Stopped, grace);
    // A reader that does not finish has read what it has so far: its stream is never taken for an empty one.
    let whole = read.or_else(|| stream.recv_timeout(DRAIN).ok());
    let read = whole.or_else(|| so_far.lock().ok().map(|so_far| *so_far)).unwrap_or_default();
    let exit_0 = end == End::Exited && status.is_some_and(|status| status.success());
    let head = head.recv_timeout(DRAIN).unwrap_or_default();
    let unknown = head.contains("unknown option") && head.contains("--restricted");
    let old = end == End::Exited && !exit_0 && whole.is_some() && read.lines == 0 && unknown;
    // A child the person stopped was stopped, whatever it printed while it was being ended.
    let end = if read.over && end != End::Stopped { End::Over } else { end };
    Ok(Ended { end, exit_0, read, old })
}

/// The start of what the child said on stderr, as text; the rest is read and dropped, so the child never waits.
pub(super) fn stderr_head(pipe: Option<impl Read + Send + 'static>) -> Receiver<String> {
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

/// Whether the child has ended. Where there are process groups it is not collected yet, so its pid, and the group
/// of that number, stay its own until [`halt`] has killed the group. A child that cannot be asked about has ended.
fn gone(child: &mut Child) -> bool {
    #[cfg(unix)]
    {
        use rustix::process::{Pid, WaitId, WaitIdOptions, waitid};
        let how = WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT;
        !matches!(waitid(WaitId::Pid(Pid::from_child(child)), how), Ok(None))
    }
    #[cfg(not(unix))]
    !matches!(child.try_wait(), Ok(None))
}

/// Ends the child and collects it. Where there are process groups the group, which is the child's own and named by
/// its pid, is asked first when there is a `grace` (`SIGINT` when the person stopped it, each time `SIGTERM`), given
/// that long after each, and then killed (`dead::stop`): so is the group of a child that has ended by itself, with
/// what it left going. Windows has no such asking and no group: the child alone is ended at once
/// (`TerminateProcess`), and what it started goes on.
fn halt(child: &mut Child, interrupt: bool, grace: Duration) -> Option<ExitStatus> {
    #[cfg(unix)]
    super::dead::stop(child.id(), interrupt, grace, || true, || gone(child));
    let _ = (interrupt, grace, child.kill());
    child.wait().ok()
}
