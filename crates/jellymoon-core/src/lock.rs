//! One change at a time: the day's lock (`tap.rs`) and the lock of the list of projects (`project.rs`). A lock is
//! a file only one caller can make, removed by that caller when its change is done
//! (`docs/development/delivery.md`, "The lock"). A lock is removed behind the system's lock on a file beside it,
//! by what was found there: a caller that waits removes a lock it finds left behind, and a holder the lock that
//! holds its own mark. So what is removed is what was looked at.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::busy::{in_use, retried};

/// How long a caller waits for the lock, and a holder for the system's lock when it lets go; and how far from now
/// a lock was last written before it is taken for one left by a command that was killed.
const LOCK_WAIT: Duration = crate::busy::WAIT;
const LOCK_STALE: Duration = Duration::from_secs(30);

/// The file beside a lock on which the system's lock is held while a lock is removed. It is made by the first
/// change in its folder and never removed: the system drops its lock when the holder ends.
const OVER: &str = "take-over.lock";

/// Windows's flag for opening what is at a name and not what a link there leads to.
#[cfg(windows)]
const OPEN_REPARSE_POINT: u32 = 0x0020_0000;

// Where a test holds a thread, in milliseconds: before its first look at a lock it could not make, before it
// asks to take over one it found left behind, before it removes one it found so again, and, letting go, before
// it removes the lock it found to be its own.
#[cfg(test)]
thread_local! {
    static SLOW: std::cell::Cell<[u64; 4]> = const { std::cell::Cell::new([0, 0, 0, 0]) };
}

#[cfg(test)]
fn seam(at: usize) {
    thread::sleep(Duration::from_millis(SLOW.get()[at]));
}

/// The lock, held: the name it was made at, and the mark written into it, which no other lock holds.
pub(crate) struct Lock {
    path: PathBuf,
    mark: String,
}

/// Why the system's lock on [`OVER`] was not had: another caller holds it at this moment, or it is not to be had
/// here at all (its file cannot be made or opened, something that is no plain file is at its name, the system
/// gives no locks there).
enum NotHad {
    Busy,
    Never,
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

/// Opens [`OVER`] at `over`, made for the person alone when it is not there. A link at the name is not followed:
/// on Unix the open fails, and on Windows it is the link itself that is opened.
#[cfg(unix)]
fn open_over(over: &Path) -> io::Result<File> {
    use rustix::fs::{Mode, OFlags, open};
    let flags = OFlags::WRONLY | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    Ok(File::from(open(over, flags, Mode::RUSR | Mode::WUSR)?))
}

#[cfg(not(unix))]
fn open_over(over: &Path) -> io::Result<File> {
    let mut open = OpenOptions::new();
    open.write(true).create(true).truncate(false);
    #[cfg(windows)]
    std::os::windows::fs::OpenOptionsExt::custom_flags(&mut open, OPEN_REPARSE_POINT);
    open.open(over)
}

/// The system's lock on [`OVER`] beside the lock at `path`, held until what is returned is dropped. It is asked
/// for once and never waited for here.
fn over(path: &Path) -> Result<File, NotHad> {
    let over = path.with_file_name(OVER);
    if fs::symlink_metadata(&over).is_ok_and(|meta| !meta.is_file()) {
        return Err(NotHad::Never);
    }
    let over = open_over(&over).map_err(|_| NotHad::Never)?;
    match over.try_lock() {
        Ok(()) => Ok(over),
        Err(TryLockError::WouldBlock) => Err(NotHad::Busy),
        Err(TryLockError::Error(_)) => Err(NotHad::Never),
    }
}

/// Removes the lock at `path` when it is left behind, as one of the callers that found it so. It is removed only
/// behind the system's lock ([`over`]), and only when it is left behind still: by then another caller may have
/// removed it and made its own. Where that lock is not had, nothing is removed.
fn take_over(path: &Path) -> Result<(), NotHad> {
    let _behind = over(path)?;
    let still = left(&fs::symlink_metadata(path));
    #[cfg(test)]
    seam(2);
    if still {
        let _ = fs::remove_file(path);
    }
    Ok(())
}

/// What a caller is told when a lock was left behind and could not be taken over: the two files by their names,
/// never their folder, and the way out.
fn stuck(path: &Path) -> io::Error {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    io::Error::other(format!(
        "{name} is a lock that was left behind, and it could not be taken over, as {OVER} beside it could not be \
         locked; remove both, then try again"
    ))
}

impl Lock {
    /// Waits [`LOCK_WAIT`], by the clock, for the name to be free. A lock that was left behind is removed
    /// ([`take_over`]) and the name asked for again. A folder at the name is never removed: the caller is
    /// refused after the same wait. On Windows a name another process is removing, or looking at, cannot be
    /// made for that moment and says "denied": that is a lock in the way like any other (`busy.rs`). Only when
    /// the wait ends with no lock there is the error itself the answer; and when it ends on a lock left behind
    /// that could not be taken over, the answer says so ([`stuck`]).
    pub(crate) fn take(path: PathBuf) -> io::Result<Lock> {
        let (started, mut unowned) = (Instant::now(), false);
        loop {
            let e = match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(made) => return Lock::marked(made, path),
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
                let held = io::Error::new(ErrorKind::TimedOut, "another tap on this day has not finished");
                let kept = if unowned { stuck(&path) } else { held };
                return Err(if used && none { e } else { kept });
            }
            unowned = left(&there) && {
                #[cfg(test)]
                seam(1);
                matches!(take_over(&path), Err(NotHad::Never))
            };
            thread::sleep(Duration::from_millis(10));
        }
    }

    /// The lock just made, with its mark written into it: this process, this moment and a count, so that no two
    /// locks hold one mark. A lock whose mark could not be written is no lock: its file stays as a command that
    /// was killed would have left it, and the caller has the error.
    fn marked(mut made: File, path: PathBuf) -> io::Result<Lock> {
        static MADE: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_nanos());
        let count = MADE.fetch_add(1, Ordering::Relaxed);
        let lock = Lock { path, mark: format!("{} {nanos} {count}\n", std::process::id()) };
        made.write_all(lock.mark.as_bytes())?;
        Ok(lock)
    }

    /// Whether what a look found at the lock's name is this lock: a plain file that holds its mark.
    fn mine(&self, there: &io::Result<fs::Metadata>) -> bool {
        let mut held = Vec::new();
        let read = |file: File| file.take(self.mark.len() as u64 + 1).read_to_end(&mut held);
        there.as_ref().is_ok_and(fs::Metadata::is_file)
            && File::open(&self.path).and_then(read).is_ok()
            && held == self.mark.as_bytes()
    }
}

/// [`over`] for a holder that lets go: while another caller holds the system's lock it is asked for again, every
/// millisecond for [`LOCK_WAIT`]. A caller holds it for a look and a removal, so the wait ends long before that.
fn over_waited(path: &Path) -> Option<File> {
    let started = Instant::now();
    loop {
        match over(path) {
            Ok(held) => return Some(held),
            Err(NotHad::Busy) if started.elapsed() < LOCK_WAIT => thread::sleep(Duration::from_millis(1)),
            Err(_) => return None,
        }
    }
}

impl Drop for Lock {
    /// The lock goes with its change, when it is this caller's still: one that was taken over is gone, and what
    /// is at its name then is another caller's and stays. The look and the removal are behind the system's lock,
    /// as a take-over's are. Where that lock is not had, a lock of its own that is not left behind is removed
    /// all the same, since nothing else ever would remove it; one that is left behind stays for a take-over.
    fn drop(&mut self) {
        let behind = over_waited(&self.path);
        let there = fs::symlink_metadata(&self.path);
        let mine = self.mine(&there);
        #[cfg(test)]
        seam(3);
        if mine && (behind.is_some() || !left(&there)) {
            let _ = retried(|| fs::remove_file(&self.path));
        }
    }
}

#[cfg(test)]
mod tests;
