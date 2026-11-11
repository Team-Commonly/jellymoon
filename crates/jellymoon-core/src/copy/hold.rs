//! The system's lock on `runs/clean.lock`, behind which a copy is made, cleaned and taken back. git reads every
//! worktree of a repo while it adds one, and fails on one half made or half removed, so those are done one at a
//! time. The system drops the lock when the process that holds it ends, however it ends, and no git child outlasts
//! its limit (`git.rs`), so a wait for it ends.

use std::fs::{File, OpenOptions, TryLockError};
use std::io::{self, ErrorKind};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

/// The lock, taken: the file is `made` when it is not there, or must be there. With no `wait` it is waited for as
/// long as it is held, which is as long as the git children of the command that holds it take. With one it is
/// asked for every 10 ms until the wait has passed, and then the error is one that timed out ([`timed_out`]).
pub(super) fn held(file: &Path, made: bool, wait: Option<Duration>) -> io::Result<File> {
    let held = OpenOptions::new().write(true).create(made).truncate(false).open(file)?;
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

/// Whether an error is the lock not had within its wait, and nothing wrong with the copy.
pub(crate) fn timed_out(error: &crate::Error) -> bool {
    matches!(error, crate::Error::Store(e) if e.kind() == ErrorKind::TimedOut)
}
