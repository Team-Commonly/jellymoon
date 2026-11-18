//! The lock taken from many threads at once. A fault is a caller that came in while another held the lock, or
//! one whose lock was gone from its name before it let go. [`SLOW`] holds a thread at the four places a caller
//! stands between a look at a lock and what it does about it, which is where the faults were. `tests/corners.rs`
//! holds a holder that lets go while another caller is at a take-over.

use std::ffi::OsString;
use std::sync::Barrier;
use std::sync::atomic::AtomicUsize;

use super::*;

mod corners;
mod denied;

const MINUTE: Duration = Duration::from_secs(60);

/// Counts who is behind the lock, and the faults.
#[derive(Default)]
struct Seen {
    inside: AtomicUsize,
    faults: AtomicUsize,
}

impl Seen {
    /// Takes the lock, holds it for `hold` and lets go. Nothing makes the callers take turns, so one can be
    /// passed over for the lock's whole wait: it asks again, as a made wish does, and that is no fault. A lock
    /// wrongly left at its name is one: it is there 30 seconds, and a caller here gives up well before that.
    fn held(&self, path: &Path, hold: Duration) {
        let started = Instant::now();
        let lock = loop {
            match Lock::take(path.to_path_buf()) {
                Ok(lock) => break lock,
                Err(e) if e.kind() == ErrorKind::TimedOut && started.elapsed() < LOCK_STALE / 2 => {}
                Err(e) => panic!("the lock was not to be had: {e}"),
            }
        };
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

/// Dates what is at the name: `at` is when the disk now says it was last written.
fn date(path: &Path, at: SystemTime) {
    File::options().write(true).open(path).unwrap().set_modified(at).unwrap();
}

/// Leaves a file at the name as a command killed a minute ago would have left its lock.
fn leave(path: &Path) {
    fs::write(path, "").unwrap();
    date(path, SystemTime::now() - MINUTE);
}

/// The names in a folder.
fn names(folder: &Path) -> Vec<OsString> {
    fs::read_dir(folder).unwrap().flatten().map(|entry| entry.file_name()).collect()
}

/// The faults of eight threads that each take a fresh lock `rounds` times, every thread slowed by `slow` at
/// each of the places, and what the lock's folder holds when they are done. Each rests a different while
/// between its takes, so that they do not keep step.
fn churned(rounds: usize, slow: [u64; 4]) -> (usize, Vec<OsString>) {
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
    (seen.faults.into_inner(), names(dir.path()))
}

/// The faults of six threads that meet one lock left a minute ago, `rounds` times over. Thread `n` is slowed
/// by `slow(n)`, and each holds the lock long enough for a slowed thread to come to its removal meanwhile.
fn taken_over(rounds: usize, slow: fn(u64) -> [u64; 4]) -> usize {
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
    // Eight callers take the lock with no turns kept, so one can wait for all the rest: a round is short, and
    // a caller refused after its 3 seconds asks again (`Seen::held`).
    for slow in [[0, 0, 0, 0], [1, 0, 1, 0], [1, 0, 0, 1]] {
        let (faults, left) = churned(60, slow);
        assert_eq!(faults, 0, "of 480 takes, each caller slowed by {slow:?}");
        assert_eq!(left, [OVER], "a lock was left, or something that is no lock: {left:?}");
    }
}

// Every caller has seen the lock left behind before the first has removed it. Only one removes it: the others
// find another at work on it, or look again and find the next holder's lock.
#[test]
fn a_lock_left_behind_is_removed_once_however_many_callers_found_it_so() {
    assert_eq!(taken_over(3, |n| [0, 0, 20 + 40 * n, 0]), 0, "each slowed before its removal, one after another");
    assert_eq!(taken_over(3, |n| [0, 30 * n, 0, 0]), 0, "each slowed before it asks to take over, one after another");
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

// A lock that holds its holder's mark is that holder's, however long ago the disk says it was written.
#[test]
fn a_holder_removes_its_own_lock_however_long_it_held_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("day.lock");
    drop(Lock::take(path.clone()).unwrap());
    assert!(!path.exists(), "a lock let go of at once is removed");
    let lock = Lock::take(path.clone()).unwrap();
    date(&path, SystemTime::now() - MINUTE);
    drop(lock);
    assert!(!path.exists(), "a lock held for a minute and never taken over is removed");
    assert_eq!(names(dir.path()), [OVER], "it was removed behind the system's lock");
}

// Something that is no plain file is not looked into: a link there is another's, whatever it leads to.
#[cfg(unix)]
#[test]
fn a_link_at_the_lock_s_name_is_no_holder_s_own_even_when_it_leads_to_its_mark() {
    let dir = tempfile::tempdir().unwrap();
    let (path, marked) = (dir.path().join("day.lock"), dir.path().join("marked"));
    fs::write(&marked, "a mark\n").unwrap();
    std::os::unix::fs::symlink(&marked, &path).unwrap();
    drop(Lock { path: path.clone(), mark: "a mark\n".to_string() });
    assert!(fs::symlink_metadata(&path).is_ok(), "the link was removed");
}

const STUCK: &str = "day.lock is a lock that was left behind, and it could not be taken over, as take-over.lock \
                     beside it could not be locked; remove both, then try again";

/// What a caller is told that meets a lock left behind where `spoil` has been at the take-over's own lock; the
/// lock must still be there. None when `spoil` could not spoil it on this system, for this user.
fn refused_where(spoil: fn(&Path) -> bool) -> Option<(ErrorKind, String)> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("day.lock");
    leave(&path);
    if !spoil(&dir.path().join(OVER)) {
        return None;
    }
    let refused = Lock::take(path.clone()).map(drop).unwrap_err();
    assert!(path.exists(), "the lock was removed");
    Some((refused.kind(), refused.to_string()))
}

// A folder stands where the take-over's own lock would be made: nothing is removed without that lock, and the
// caller is told what is true of it, not that another change is at work.
#[test]
fn a_lock_left_behind_stays_when_the_lock_a_take_over_is_done_behind_is_not_to_be_had() {
    let said = refused_where(|over| fs::create_dir(over).is_ok());
    assert_eq!(said, Some((ErrorKind::Other, STUCK.to_string())));
}

// A plain file that cannot be opened: the look that wants a plain file passes it, and the open alone refuses.
#[cfg(unix)]
#[test]
fn a_lock_left_behind_stays_when_the_take_over_s_file_is_a_plain_file_that_cannot_be_opened() {
    let closed = |over: &Path| {
        fs::write(over, "").unwrap();
        fs::set_permissions(over, std::os::unix::fs::PermissionsExt::from_mode(0o000)).unwrap();
        // Whoever may open every file (root) is not refused, and has nothing to show here.
        File::options().write(true).open(over).is_err()
    };
    if let Some(said) = refused_where(closed) {
        assert_eq!(said, (ErrorKind::Other, STUCK.to_string()));
    }
}

// The open follows no link: nothing is made where a link at the name leads, even with the look before it away.
#[cfg(unix)]
#[test]
fn a_link_at_the_take_over_s_name_is_not_followed() {
    let dir = tempfile::tempdir().unwrap();
    let (over, away) = (dir.path().join(OVER), dir.path().join("away"));
    std::os::unix::fs::symlink(&away, &over).unwrap();
    assert!(open_over(&over).is_err(), "a link was opened");
    assert!(!away.exists(), "a file was made where the link leads");
    let said = refused_where(|over| std::os::unix::fs::symlink(over.with_file_name("away"), over).is_ok());
    assert_eq!(said, Some((ErrorKind::Other, STUCK.to_string())));
}

// Another caller is behind the take-over's lock for the whole wait: that is a change at work, and is said so.
#[test]
fn a_lock_left_behind_while_another_caller_holds_the_take_over_s_lock_is_another_change_at_work() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("day.lock");
    leave(&path);
    let _behind = over(&path).expect("the system's lock");
    let refused = Lock::take(path.clone()).map(drop).map_err(|e| e.kind());
    assert_eq!(refused, Err(ErrorKind::TimedOut));
    assert!(path.exists(), "the lock was removed");
}

// The lock that could not be taken over is gone by the end of the wait, and a fresh one is at the name: what is
// said is of the lock that is there.
#[test]
fn a_lock_that_is_fresh_when_the_wait_ends_is_another_change_at_work_whatever_was_there_before() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("day.lock");
    leave(&path);
    fs::create_dir(dir.path().join(OVER)).unwrap();
    let refused = thread::scope(|scope| {
        scope.spawn(|| {
            thread::sleep(LOCK_WAIT - Duration::from_millis(500));
            // Written and dated: on Windows a file that was empty and is written empty keeps its time.
            fs::write(&path, "another caller's mark").unwrap();
            date(&path, SystemTime::now());
        });
        Lock::take(path.clone()).map(drop).map_err(|e| e.kind())
    });
    assert_eq!(refused, Err(ErrorKind::TimedOut));
}

// Without the system's lock a holder still removes its own lock, which nothing else would ever remove; but not
// one that counts as left behind, which a caller that waits may be removing.
#[test]
fn a_holder_that_cannot_have_the_system_s_lock_removes_its_own_lock_unless_it_is_left_behind() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("day.lock");
    fs::create_dir(dir.path().join(OVER)).unwrap();
    drop(Lock::take(path.clone()).unwrap());
    assert!(!path.exists(), "a fresh lock was left for ever");
    let lock = Lock::take(path.clone()).unwrap();
    date(&path, SystemTime::now() - MINUTE);
    drop(lock);
    assert!(path.exists(), "a lock left behind was removed with no system's lock held");
}

// Another caller keeps the system's lock: the holder waits for it three seconds, never for ever, and then
// removes its own fresh lock.
#[test]
fn a_holder_waits_three_seconds_for_the_system_s_lock_and_then_removes_its_own_fresh_lock() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("day.lock");
    let lock = Lock::take(path.clone()).unwrap();
    let _behind = over(&path).expect("the system's lock");
    let started = Instant::now();
    drop(lock);
    let waited = started.elapsed();
    assert!(waited >= LOCK_WAIT && waited < 2 * LOCK_WAIT, "waited {waited:?}");
    assert!(!path.exists(), "a fresh lock was left");
}

// What Windows says of the take-over's file while another process has it open or locked is that caller at
// work, never a file that cannot be locked. "Denied" there is an answer of its own (`tests/denied.rs`).
#[test]
fn on_windows_a_take_over_s_file_in_use_is_another_caller_at_work_and_elsewhere_an_error_is_what_it_says() {
    let cases = [
        ("access is denied", ErrorKind::PermissionDenied, Some(5), NotHad::Denied),
        ("a sharing violation that reads as denied", ErrorKind::PermissionDenied, Some(32), NotHad::Busy),
        ("a sharing violation", ErrorKind::Other, Some(32), NotHad::Busy),
        ("a lock violation", ErrorKind::Other, Some(33), NotHad::Busy),
        ("the path is not found", ErrorKind::NotFound, Some(3), NotHad::Never),
        ("not supported", ErrorKind::Unsupported, Some(50), NotHad::Never),
        ("an invalid function, as a disk with no locks says", ErrorKind::Other, Some(1), NotHad::Never),
        ("too many open files", ErrorKind::Other, Some(4), NotHad::Busy),
    ];
    for (case, kind, raw, on_windows) in cases {
        assert_eq!(not_had(kind, raw, true), on_windows, "{case}, on Windows");
        assert_eq!(not_had(kind, raw, false), NotHad::Never, "{case}, elsewhere");
    }
    // A process, or the system, with no file handle left, as Unix numbers them; 4 there is a call cut short.
    for raw in [23, 24] {
        assert_eq!(not_had(ErrorKind::Other, Some(raw), false), NotHad::Busy, "{raw} on Unix");
        assert_eq!(not_had(ErrorKind::Other, Some(raw), true), NotHad::Never, "{raw} on Windows");
    }
    // What this system says of a folder and of a link at the name is never a caller at work.
    #[cfg(unix)]
    for raw in [21, 62, 40, 6] {
        assert_eq!(not_had(ErrorKind::Other, Some(raw), false), NotHad::Never, "{raw}");
    }
}

// The file a lock is made as is the caller's for a moment before its mark is in it. A mark that cannot be
// written, as on a full disk, leaves no lock: here the file is handed over opened for reading only.
#[test]
fn a_lock_whose_mark_cannot_be_written_is_removed_and_refuses_nobody() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("day.lock");
    fs::write(&path, "").unwrap();
    let refused = Lock::marked(File::open(&path).unwrap(), path.clone()).map(drop);
    assert!(refused.is_err(), "a mark was written through a file opened for reading");
    assert!(!path.exists(), "the file of a lock that was never one was left");
    let started = Instant::now();
    drop(Lock::take(path.clone()).unwrap());
    assert!(started.elapsed() < LOCK_WAIT, "the next caller waited");
}

// A pipe at the take-over's name: an open for writing waits until someone reads it, which may be never.
#[cfg(unix)]
#[test]
fn the_open_of_the_take_over_s_file_never_waits_on_a_pipe() {
    let dir = tempfile::tempdir().unwrap();
    let pipe = dir.path().join(OVER);
    let made = std::process::Command::new("mkfifo").arg(&pipe).status();
    if !made.is_ok_and(|status| status.success()) {
        return; // No `mkfifo` here: nothing to show.
    }
    let (opened, done) = std::sync::mpsc::channel();
    thread::scope(|scope| {
        scope.spawn(|| opened.send(open_over(&pipe).is_err()));
        let failed = done.recv_timeout(Duration::from_secs(2));
        // A reader lets a writer that waits go on, so the test ends; with no writer the reader would wait.
        if failed.is_err() {
            drop(File::open(&pipe));
        }
        assert_eq!(failed, Ok(true), "the open waited on the pipe, or opened it");
    });
}
