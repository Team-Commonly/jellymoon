//! One change at a time: the day's lock (`tap.rs`) and the lock of the list of projects (`project.rs`). A lock is
//! a file only one caller can make, removed by that caller when its change is done
//! (`docs/development/delivery.md`, "The lock"). A caller that waits removes nothing but a lock left behind, and
//! that only behind the system's lock on a file beside it, where it has looked at the lock again.

use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use crate::busy::{in_use, retried};

/// How long a caller waits for the lock, and how far from now a lock was last written before it is taken for one
/// left by a command that was killed.
const LOCK_WAIT: Duration = crate::busy::WAIT;
const LOCK_STALE: Duration = Duration::from_secs(30);

/// The file beside a lock on which the system's lock is held while a lock left behind is removed. It is made by
/// the first take-over in its folder and never removed: the system drops its lock when the holder ends.
const OVER: &str = "take-over.lock";

// Where a test holds a thread, in milliseconds: before its first look at a lock it could not make, before it
// asks to take over one it found left behind, and before it removes one it found so again.
#[cfg(test)]
thread_local! {
    static SLOW: std::cell::Cell<[u64; 3]> = const { std::cell::Cell::new([0, 0, 0]) };
}

#[cfg(test)]
fn seam(at: usize) {
    thread::sleep(Duration::from_millis(SLOW.get()[at]));
}

/// The lock, held: the name it was made at, and when the disk says it was written.
pub(crate) struct Lock {
    path: PathBuf,
    written: Option<SystemTime>,
}

/// Whether a lock last written at `written` is left behind at `now`: the two are more than [`LOCK_STALE`] apart,
/// either way round, since a clock that was set back dates a lock ahead. A time that cannot be read says nothing.
fn stale(written: Option<SystemTime>, now: SystemTime) -> bool {
    let apart = |at: SystemTime| now.duration_since(at).unwrap_or_else(|ahead| ahead.duration());
    written.is_some_and(|at| apart(at) > LOCK_STALE)
}

/// Whether what a look found at a lock's name is a lock left behind. Nothing there is not, and neither is
/// what could not be looked at, or a folder.
fn left(there: &io::Result<fs::Metadata>) -> bool {
    there.as_ref().is_ok_and(|meta| !meta.is_dir() && stale(meta.modified().ok(), SystemTime::now()))
}

/// Removes the lock at `path` when it is left behind, as one of the callers that found it so. It is removed only
/// behind the system's lock on [`OVER`], and only when it is left behind still: by then another caller may have
/// removed it and made its own. Where that lock is not to be had (another holds it, its file cannot be made, the
/// system gives no locks there, something that is no plain file is at its name) nothing is removed.
fn take_over(path: &Path) {
    let over = path.with_file_name(OVER);
    if fs::symlink_metadata(&over).is_ok_and(|meta| !meta.is_file()) {
        return;
    }
    let mut open = OpenOptions::new();
    open.write(true).create(true).truncate(false);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut open, 0o600);
    let Ok(over) = open.open(over) else { return };
    if over.try_lock().is_err() {
        return;
    }
    let still = left(&fs::symlink_metadata(path));
    #[cfg(test)]
    seam(2);
    if still {
        let _ = fs::remove_file(path);
    }
}

impl Lock {
    /// Waits [`LOCK_WAIT`], by the clock, for the name to be free. A lock that was left behind is removed
    /// ([`take_over`]) and the name asked for again. A folder at the name is never removed: the caller is
    /// refused after the same wait. On Windows a name another process is removing, or looking at, cannot be
    /// made for that moment and says "denied": that is a lock in the way like any other (`busy.rs`). Only when
    /// the wait ends with no lock there is the error itself the answer.
    pub(crate) fn take(path: PathBuf) -> io::Result<Lock> {
        let started = Instant::now();
        loop {
            let e = match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(made) => return Ok(Lock { written: written(&made), path }),
                Err(e) => e,
            };
            #[cfg(test)]
            seam(0);
            let there = fs::symlink_metadata(&path);
            let used = in_use(&e);
            if e.kind() != ErrorKind::AlreadyExists && !there.as_ref().is_ok_and(fs::Metadata::is_dir) && !used {
                return Err(e);
            }
            if started.elapsed() >= LOCK_WAIT {
                let none = matches!(&there, Err(gone) if gone.kind() == ErrorKind::NotFound);
                let held = || io::Error::new(ErrorKind::TimedOut, "another tap on this day has not finished");
                return Err(if used && none { e } else { held() });
            }
            if left(&there) {
                #[cfg(test)]
                seam(1);
                take_over(&path);
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

/// When the disk says a lock just made was written: what a caller that waits will read of it.
fn written(made: &File) -> Option<SystemTime> {
    made.metadata().and_then(|meta| meta.modified()).ok()
}

impl Drop for Lock {
    /// The lock goes with its change. One held until it counts as left behind may have been taken over, and
    /// what is at its name is then another caller's: it is left there, for the next caller to look at.
    fn drop(&mut self) {
        if !stale(self.written, SystemTime::now()) {
            let _ = retried(|| fs::remove_file(&self.path));
        }
    }
}

#[cfg(test)]
mod tests;
