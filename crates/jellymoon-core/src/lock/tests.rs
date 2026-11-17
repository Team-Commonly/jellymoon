//! The lock taken from many threads at once. A fault is a caller that came in while another held the lock, or
//! one whose lock was gone from its name before it let go. [`SLOW`] holds a thread at the three places a caller
//! stands between a look at a lock and what it does about it, which is where the faults were.

use std::ffi::OsString;
use std::sync::Barrier;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

const MINUTE: Duration = Duration::from_secs(60);

/// Counts who is behind the lock, and the faults.
#[derive(Default)]
struct Seen {
    inside: AtomicUsize,
    faults: AtomicUsize,
}

impl Seen {
    /// Takes the lock, holds it for `hold` and lets go.
    fn held(&self, path: &Path, hold: Duration) {
        let lock = Lock::take(path.to_path_buf()).unwrap();
        let alone = self.inside.fetch_add(1, Ordering::SeqCst) == 0;
        thread::sleep(hold);
        let mine = fs::symlink_metadata(path).is_ok();
        self.inside.fetch_sub(1, Ordering::SeqCst);
        if !(alone && mine) {
            self.faults.fetch_add(1, Ordering::SeqCst);
        }
        drop(lock);
    }
}

/// Leaves a file at the name as a command killed a minute ago would have left its lock.
fn leave(path: &Path) {
    fs::write(path, "").unwrap();
    File::options().write(true).open(path).unwrap().set_modified(SystemTime::now() - MINUTE).unwrap();
}

/// The faults of eight threads that each take a fresh lock `rounds` times, every thread slowed by `slow` at
/// each of the three places, and what the lock's folder holds when they are done. Each rests a different while
/// between its takes, so that they do not keep step.
fn churned(rounds: usize, slow: [u64; 3]) -> (usize, Vec<OsString>) {
    let (dir, seen) = (tempfile::tempdir().unwrap(), Seen::default());
    let path = dir.path().join("day.lock");
    thread::scope(|scope| {
        for n in 0..8 {
            let (seen, path) = (&seen, &path);
            scope.spawn(move || {
                SLOW.set(slow);
                for _ in 0..rounds {
                    seen.held(path, Duration::from_millis(2));
                    thread::sleep(Duration::from_micros(300 * n));
                }
            });
        }
    });
    (seen.faults.into_inner(), fs::read_dir(dir.path()).unwrap().flatten().map(|entry| entry.file_name()).collect())
}

/// The faults of six threads that meet one lock left a minute ago, `rounds` times over. Thread `n` is slowed
/// by `slow(n)`, and each holds the lock long enough for a slowed thread to come to its removal meanwhile.
fn taken_over(rounds: usize, slow: fn(u64) -> [u64; 3]) -> usize {
    let (dir, seen) = (tempfile::tempdir().unwrap(), Seen::default());
    let path = dir.path().join("day.lock");
    for _ in 0..rounds {
        leave(&path);
        let start = Barrier::new(6);
        thread::scope(|scope| {
            for n in 0..6 {
                let (seen, path, start) = (&seen, &path, &start);
                scope.spawn(move || {
                    SLOW.set(slow(n));
                    start.wait();
                    seen.held(path, Duration::from_millis(100));
                });
            }
        });
        assert!(!path.exists(), "the lock was left");
    }
    seen.faults.into_inner()
}

// The fault of the CI run: a caller that waits finds no lock at the name, as its holder has just let go, and
// "removes" it; by then it is the next caller's. Unslowed it shows rarely, which is how it got through.
#[test]
fn a_lock_that_is_held_is_never_removed_by_a_caller_that_waits_for_it() {
    for slow in [[0, 0, 0], [1, 0, 1]] {
        let (faults, left) = churned(150, slow);
        assert_eq!(faults, 0, "of 1200 takes, each caller slowed by {slow:?}");
        assert!(left.is_empty(), "a lock that was never left behind was taken over, or one was left: {left:?}");
    }
}

// Every caller has seen the lock left behind before the first has removed it. Only one removes it: the others
// find another at work on it, or look again and find the next holder's lock.
#[test]
fn a_lock_left_behind_is_removed_once_however_many_callers_found_it_so() {
    assert_eq!(taken_over(3, |n| [0, 0, 20 + 40 * n]), 0, "each slowed before its removal, one after another");
    assert_eq!(taken_over(3, |n| [0, 30 * n, 0]), 0, "each slowed before it asks to take over, one after another");
}

#[test]
fn a_lock_is_left_behind_only_when_its_time_is_read_and_is_over_thirty_seconds_from_now_either_way() {
    let (now, secs) = (SystemTime::now(), Duration::from_secs);
    let cases = [
        ("just written", Some(now), false),
        ("29 seconds old", Some(now - secs(29)), false),
        ("31 seconds old", Some(now - secs(31)), true),
        ("dated 29 seconds ahead", Some(now + secs(29)), false),
        ("dated 31 seconds ahead", Some(now + secs(31)), true),
        ("a time that cannot be read", None, false),
    ];
    for (case, written, is) in cases {
        assert_eq!(stale(written, now), is, "{case}");
    }
}

#[test]
fn only_what_is_there_is_no_folder_and_is_old_is_looked_at_as_a_lock_left_behind() {
    let dir = tempfile::tempdir().unwrap();
    let at = |name: &str| dir.path().join(name);
    assert!(!left(&fs::symlink_metadata(at("nothing"))), "nothing is at the name");
    fs::write(at("fresh"), "").unwrap();
    assert!(!left(&fs::symlink_metadata(at("fresh"))), "a lock made just now");
    leave(&at("old"));
    assert!(left(&fs::symlink_metadata(at("old"))), "a lock a minute old");
    // Windows opens no folder this way; what a folder at the name does there is the binary's test (F35).
    #[cfg(unix)]
    {
        fs::create_dir(at("folder")).unwrap();
        File::open(at("folder")).unwrap().set_modified(SystemTime::now() - MINUTE).unwrap();
        assert!(!left(&fs::symlink_metadata(at("folder"))), "a folder a minute old");
    }
}

#[test]
fn a_holder_removes_its_own_lock_and_leaves_the_name_alone_once_it_held_it_too_long() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("day.lock");
    let lock = Lock::take(path.clone()).unwrap();
    let written = lock.written.expect("the disk says when the lock was made");
    assert!(!stale(Some(written), SystemTime::now()) && path.exists());
    drop(lock);
    assert!(!path.exists(), "a lock let go of in time is removed");
    // Held for a minute: it was taken over meanwhile, and what is at the name is the lock of whoever did.
    fs::write(&path, "").unwrap();
    drop(Lock { path: path.clone(), written: Some(SystemTime::now() - MINUTE) });
    assert!(path.exists(), "a holder that was too slow removed another caller's lock");
}

// A folder stands where the take-over's own lock would be made: nothing is removed without that lock.
#[test]
fn a_lock_left_behind_stays_when_the_lock_a_take_over_is_done_behind_is_not_to_be_had() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("day.lock");
    leave(&path);
    fs::create_dir(dir.path().join(OVER)).unwrap();
    let refused = Lock::take(path.clone()).map(drop).map_err(|e| e.kind());
    assert_eq!(refused, Err(ErrorKind::TimedOut));
    assert!(path.exists(), "the lock was removed");
}
