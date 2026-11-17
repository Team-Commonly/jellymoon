//! "Denied" on Windows, which a file being removed says for a moment and a file that may not be written says
//! for good. The rules are shown as values on every system, since the errors themselves are Windows's alone.

use super::*;

// Each wait is what its passes gave, in order: none where a pass took over, or found nothing to take over.
#[test]
fn a_wait_ends_on_a_lock_nobody_can_take_over_when_every_pass_was_denied_or_the_last_can_never_be_locked() {
    use NotHad::{Busy, Denied, Never};
    let nobody_s =
        |passes: &[Option<NotHad>]| passes.iter().fold(Wait::Start, |told, pass| told.then(*pass)).nobody_s();
    assert!(!nobody_s(&[]), "a wait of no pass");
    assert!(nobody_s(&[Some(Denied)]) && nobody_s(&[Some(Denied); 300]), "denied on every pass");
    // One pass that was not denied, wherever it falls, and another caller is at work.
    for other in [None, Some(Busy)] {
        for passes in [
            [other, Some(Denied), Some(Denied)],
            [Some(Denied), other, Some(Denied)],
            [Some(Denied), Some(Denied), other],
        ] {
            assert!(!nobody_s(&passes), "{passes:?}");
        }
    }
    // What can never be locked counts when it is what the wait ends on, as it did before "denied" was told apart.
    assert!(nobody_s(&[Some(Denied), Some(Never)]) && nobody_s(&[None, Some(Busy), Some(Never)]));
    for after in [None, Some(Busy), Some(Denied)] {
        assert!(!nobody_s(&[Some(Never), after]), "{after:?} after what can never be locked");
    }
}

// The look before the open: a read-only file is passed on every system but Windows, where the open would say
// "denied" of it; a folder is passed on none.
#[test]
fn on_windows_a_read_only_file_at_the_take_over_s_name_is_not_to_be_had_at_all() {
    let dir = tempfile::tempdir().unwrap();
    let over = dir.path().join(OVER);
    fs::write(&over, "").unwrap();
    let look = || fs::symlink_metadata(&over).unwrap();
    assert!(lockable(&look(), true) && lockable(&look(), false), "a plain file that may be written");
    let was = look().permissions();
    let mut closed = was.clone();
    closed.set_readonly(true);
    fs::set_permissions(&over, closed).unwrap();
    let (on_windows, elsewhere) = (lockable(&look(), true), lockable(&look(), false));
    fs::set_permissions(&over, was).unwrap();
    assert!(!on_windows && elsewhere, "a read-only file: {on_windows}, {elsewhere}");
    let folder = fs::symlink_metadata(dir.path()).unwrap();
    assert!(!lockable(&folder, true) && !lockable(&folder, false), "a folder");
}

// The same on Windows itself: the caller is told to remove both files, not that another change is at work.
#[cfg(windows)]
#[test]
fn on_windows_a_lock_left_behind_beside_a_read_only_take_over_s_file_is_one_nobody_can_take_over() {
    let dir = tempfile::tempdir().unwrap();
    let (path, over) = (dir.path().join("day.lock"), dir.path().join(OVER));
    leave(&path);
    fs::write(&over, "").unwrap();
    let was = fs::metadata(&over).unwrap().permissions();
    let mut closed = was.clone();
    closed.set_readonly(true);
    fs::set_permissions(&over, closed).unwrap();
    let said = Lock::take(path.clone()).map(drop).map_err(|e| (e.kind(), e.to_string()));
    // Put back, so that the temporary folder can be removed.
    fs::set_permissions(&over, was).unwrap();
    assert_eq!(said, Err((ErrorKind::Other, STUCK.to_string())));
    assert!(path.exists(), "the lock was removed");
}
