//! A holder that lets go while another caller is at a take-over: the three ways the two once acted on different
//! readings of one lock, each shown with a caller held where it stands between its look and its removal.

use super::*;

/// A lock taken now that the disk says was last written at `at`, as a clock that was set would have it.
fn held_since(path: &Path, at: SystemTime) -> Lock {
    let lock = Lock::take(path.to_path_buf()).unwrap();
    date(path, at);
    lock
}

/// The faults of two callers that come for the lock while its holder lets go: one from another thread, slowed by
/// `slow`, and one from this thread once the holder, slowed by `letting_go` and letting go after `after`, is done.
fn faults(at: SystemTime, slow: [u64; 4], letting_go: [u64; 4], after: u64) -> usize {
    let (dir, seen) = (tempfile::tempdir().unwrap(), Seen::default());
    let path = dir.path().join("day.lock");
    let holder = held_since(&path, at);
    thread::scope(|scope| {
        let (seen, path) = (&seen, &path);
        scope.spawn(move || {
            SLOW.set(slow);
            seen.held(path, Duration::from_millis(50));
        });
        thread::sleep(Duration::from_millis(after));
        SLOW.set(letting_go);
        drop(holder);
        SLOW.set([0; 4]);
        seen.held(path, Duration::from_millis(400));
    });
    assert!(!path.exists(), "a lock was left");
    seen.faults.into_inner()
}

// The lock turns thirty seconds old 100 ms from now. Its holder finds it in time now and removes it 200 ms from
// now; a caller that waits looks at it 150 ms from now, when it is left behind, and would remove it 300 ms later,
// when the name is the next caller's.
#[test]
fn a_hold_that_ends_at_the_thirty_second_mark_is_judged_once_by_its_holder_and_a_caller_that_waits() {
    let at = SystemTime::now() - LOCK_STALE + Duration::from_millis(100);
    assert_eq!(faults(at, [150, 0, 300, 0], [0, 0, 0, 200], 0), 0, "the next caller's lock was removed");
}

// A lock dated ahead is left behind less as time passes: this one for 150 ms more. A caller that waits finds
// it so at once and would remove it 300 ms later; its holder lets go 200 ms from now, when it is in time again.
#[test]
fn a_lock_dated_ahead_is_judged_once_by_its_holder_and_a_caller_that_waits() {
    let at = SystemTime::now() + LOCK_STALE + Duration::from_millis(150);
    assert_eq!(faults(at, [0, 0, 300, 0], [0; 4], 200), 0, "the next caller's lock was removed");
}

// The clock was set a minute on while a holder was at work: its lock was seen a minute old and taken over. By
// the holder's own reading no time has passed. What is at the name is not its lock.
#[test]
fn a_holder_whose_lock_was_taken_over_leaves_the_lock_of_the_caller_that_took_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("day.lock");
    let holder = Lock::take(path.clone()).unwrap();
    date(&path, SystemTime::now() - MINUTE);
    let taker = Lock::take(path.clone()).unwrap();
    drop(holder);
    assert!(path.exists(), "a holder removed the lock of the caller that took its own over");
    drop(taker);
    assert!(!path.exists(), "the caller that took the lock over left it");
    // The same while the caller that takes it over is between its second look and its removal.
    let at = SystemTime::now() - MINUTE;
    assert_eq!(faults(at, [0, 0, 300, 0], [0; 4], 100), 0, "the next caller's lock was removed");
}
