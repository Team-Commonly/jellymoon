//! The system's lock on `runs/clean.lock`, behind which a copy is made, cleaned and taken back. git reads every
//! worktree of a repo while it adds one, and fails on one half made or half removed, so those are done one at a
//! time. The system drops the lock when the process that holds it ends, however it ends, and no git child outlasts
//! its limit (`git.rs`), so a wait for it ends.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

/// The lock, taken: the file is `made` when it is not there, or must be there. With no `wait` it is waited for as
/// long as it is held, which is as long as the git children of the command that holds it take. With one it is
/// asked for every 10 ms until the wait has passed, and then the error is one that timed out ([`timed_out`]).
pub(super) fn held(file: &Path, made: bool, wait: Option<Duration>) -> io::Result<File> {
    let held = super::record::lock_of(file, made)?;
    let Some(wait) = wait else { return held.lock().map(|()| held) };
    let started = Instant::now();
    loop {
        match held.try_lock() {
            Ok(()) => return Ok(held),
            Err(TryLockError::Error(e)) => return Err(e),
            Err(TryLockError::WouldBlock) if started.elapsed() < wait => thread::sleep(Duration::from_millis(10)),
            Err(TryLockError::WouldBlock) => return Err(ErrorKind::TimedOut.into()),
        }
    }
}

/// The file a `jellymoon run` holds the system's lock on from before its copy is made until it ends,
/// `runs/<id>/start.lock`. It takes the file away at its end, so one that is there with nothing holding it is
/// of a command that was killed. A copy made by `jellymoon copy make` has none.
pub(crate) fn start_lock(home: &Path, run: &str) -> PathBuf {
    home.join("runs").join(run).join("start.lock")
}

/// Makes that file in the run's new folder, which holds nothing yet, and takes its lock.
pub(super) fn start(runs: &Path, run: &str) -> io::Result<File> {
    let lock = OpenOptions::new().write(true).create_new(true).open(runs.join(run).join("start.lock"))?;
    lock.lock().map(|()| lock)
}

/// The runs whose command was killed: those with a plain file at their start lock's name that nothing holds.
/// A lock that cannot be asked about is taken as held, and so is anything there that is not a plain file,
/// which is never opened: of such a run nothing is taken back.
pub(crate) fn left(home: &Path) -> Vec<String> {
    let runs = fs::read_dir(home.join("runs")).into_iter().flatten().flatten();
    let runs = runs.filter_map(|entry| entry.file_name().into_string().ok()).filter(|run| super::is_run_id(run));
    runs.filter(|run| let_go(home, run)).collect()
}

/// Whether a run's start lock is a plain file that nothing holds. It is asked a second time once the copies'
/// lock is had (`take_back_left`): a command makes the file and takes its lock in two steps, behind the copies'
/// lock, so one that was between the two when the runs left were listed holds its lock by then.
pub(super) fn let_go(home: &Path, run: &str) -> bool {
    let lock = start_lock(home, run);
    let plain = fs::symlink_metadata(&lock).is_ok_and(|meta| meta.is_file());
    plain && File::open(&lock).is_ok_and(|lock| lock.try_lock_shared().is_ok())
}

/// Whether an error is the lock not had within its wait, and nothing wrong with the copy.
pub(crate) fn timed_out(error: &crate::Error) -> bool {
    matches!(error, crate::Error::Store(e) if e.kind() == ErrorKind::TimedOut)
}
