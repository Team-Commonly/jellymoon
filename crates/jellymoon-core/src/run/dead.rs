//! The harness child of a `jellymoon run` that was cut off (`docs/development/run.md`, "A run that was cut off").
//! Nobody waits for that child any more, so all that is known of it is the pid in the run's record, and a pid is
//! given out again once its process is gone. [`mark`] is what tells the two apart: when the process started, as the
//! system says it. [`kill`] signals the pid only while its mark is the one recorded, and does not wait: a dead run's
//! agent has nobody to report to, and a page is not kept waiting for it. [`stop`] is the one stop sequence, which
//! the limit inside a live `jellymoon run` uses with its five seconds, and a stop the person asked for with an
//! interrupt before them (`child.rs`).
//!
//! Where the system has no signals (Windows) there is no mark and nothing is stopped.

#[cfg(unix)]
use std::{
    fs,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

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

/// The one stop sequence. The child was started in a process group of its own, named by its pid, so each signal
/// goes to the group, and only while `same` says the pid is still that child. With a `grace` the group is asked
/// first and given that long, or until the child is `gone`: with `interrupt` as Ctrl-C asks (`SIGINT`), and then,
/// if it is still there, as the system asks a program to stop (`SIGTERM`), with the same wait again. Then it is
/// killed, gone or not: what the child started is in the group too.
#[cfg(unix)]
pub(super) fn stop(
    pid: u32,
    interrupt: bool,
    grace: Duration,
    same: impl Fn() -> bool,
    mut gone: impl FnMut() -> bool,
) {
    use rustix::process::{Pid, Signal, kill_process_group};
    let Some(group) = i32::try_from(pid).ok().filter(|pid| *pid > 1).and_then(Pid::from_raw) else { return };
    let asks = [Signal::INT, Signal::TERM].into_iter().skip(usize::from(!interrupt));
    for ask in asks.filter(|_| !grace.is_zero()) {
        if !gone() && same() && kill_process_group(group, ask).is_ok() {
            let deadline = Instant::now() + grace;
            while Instant::now() < deadline && !gone() {
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
    if same() {
        let _ = kill_process_group(group, Signal::KILL);
    }
}

/// Kills the process of that pid, and what it started, if it is still the one that was recorded: at once, with
/// nothing asked and nothing waited for. The mark is read just before the signal, so a pid that is another process
/// by then is not signalled.
#[cfg(unix)]
pub(super) fn kill(pid: u32, recorded: &str) {
    stop(pid, false, Duration::ZERO, || mark(pid).as_deref() == Some(recorded), || true);
}

#[cfg(not(unix))]
pub(super) fn mark(_pid: u32) -> Option<String> {
    None
}

#[cfg(not(unix))]
pub(super) fn kill(_pid: u32, _recorded: &str) {}
