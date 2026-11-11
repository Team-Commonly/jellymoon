//! The harness child of a `jellymoon run` that was cut off (`docs/development/run.md`, "A run that was cut off").
//! Nobody waits for that child any more, so all that is known of it is the pid in the run's record, and a pid is
//! given out again once its process is gone. [`mark`] is what tells the two apart: when the process started, as the
//! system says it. [`stop`] signals the pid only while its mark is the one recorded.
//!
//! Where the system has no signals (Windows) there is no mark and nothing is stopped.

#[cfg(unix)]
use std::{
    fs,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

/// How long a child that was asked to stop is given before it is killed, and how long the kill is given.
#[cfg(unix)]
const WAITS: (Duration, Duration) = (Duration::from_secs(5), Duration::from_secs(2));

/// When the process of that pid started. None when there is no such process, when it has ended and only waits to be
/// collected, or when the system cannot be asked. Linux says it in `/proc`; elsewhere `ps` is asked, by its full path
/// and with no variable of the caller's.
#[cfg(unix)]
pub(super) fn mark(pid: u32) -> Option<String> {
    if let Ok(stat) = fs::read_to_string(format!("/proc/{pid}/stat")) {
        // After the name, which can hold anything: the state, then eighteen fields, then the start.
        let mut rest = stat.rsplit_once(')')?.1.split_ascii_whitespace();
        let (state, started) = (rest.next()?, rest.nth(18)?);
        return (state != "Z").then(|| format!("proc {started}"));
    }
    let mut ps = Command::new("/bin/ps");
    ps.args(["-o", "stat=", "-o", "lstart=", "-p", &pid.to_string()]).env_clear().env("LC_ALL", "C").env("TZ", "UTC");
    let said = ps.stdin(Stdio::null()).stderr(Stdio::null()).output().ok().filter(|said| said.status.success())?;
    let said = String::from_utf8(said.stdout).ok()?;
    let (state, started) = said.trim().split_once(' ')?;
    (!state.starts_with('Z')).then(|| format!("ps {}", started.trim()))
}

/// Stops the process of that pid, and what it started, if it is still the one that was recorded: asked first
/// (`SIGTERM`), then killed. The mark is read again just before each signal, so a pid that is another process by
/// then is not signalled. The child was started in a process group of its own, so the signal goes to the group.
#[cfg(unix)]
pub(super) fn stop(pid: u32, recorded: &str) {
    use rustix::process::{Pid, Signal, kill_process_group};
    let same = || mark(pid).as_deref() == Some(recorded);
    let Some(group) = i32::try_from(pid).ok().filter(|pid| *pid > 1).and_then(Pid::from_raw) else { return };
    for (signal, wait) in [(Signal::TERM, WAITS.0), (Signal::KILL, WAITS.1)] {
        if !same() || kill_process_group(group, signal).is_err() {
            return;
        }
        let deadline = Instant::now() + wait;
        while Instant::now() < deadline && same() {
            thread::sleep(Duration::from_millis(50));
        }
    }
}

#[cfg(not(unix))]
pub(super) fn mark(_pid: u32) -> Option<String> {
    None
}

#[cfg(not(unix))]
pub(super) fn stop(_pid: u32, _recorded: &str) {}
