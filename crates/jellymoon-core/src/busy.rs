//! A step on a file that another process is using at that moment (`docs/development/delivery.md`, "On Windows").
//! On unix such a step works: a name being removed can be made again, and a file that is open can be removed,
//! renamed and renamed over. On Windows each of those fails with "Access is denied" (5) or a sharing violation
//! (32) for as long as the other process holds the file, which between two changes to one day is milliseconds.
//! So there, and only there, a step that fails so is tried again for the day's own wait, and then its error stands.

use std::io::{self, ErrorKind};
use std::thread;
use std::time::{Duration, Instant};

/// How long a step that found its file in use is tried again: the day's lock's own wait.
pub(crate) const WAIT: Duration = Duration::from_secs(3);

/// Windows's code for a file another process has open in a way that does not share it.
const SHARING_VIOLATION: i32 = 32;

/// Whether a step's error may be another process's use of the file and nothing wrong: on Windows, permission
/// denied or a sharing violation. On every other system no error is. It is also what a folder nobody may write in
/// gives, which is why a caller tries again for a bounded time and then reports the error it last got.
pub(crate) fn busy(kind: ErrorKind, raw: Option<i32>, windows: bool) -> bool {
    windows && (kind == ErrorKind::PermissionDenied || raw == Some(SHARING_VIOLATION))
}

/// [`busy`] for an error of this system.
pub(crate) fn in_use(error: &io::Error) -> bool {
    busy(error.kind(), error.raw_os_error(), cfg!(windows))
}

/// Does `step`, again every 10 ms while it fails with a file in use and [`WAIT`] has not passed.
pub(crate) fn retried<T>(step: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    retrying(WAIT, in_use, step)
}

/// [`retried`] with the wait and the question given. Any other error, and the last one once the wait is over, is
/// returned as it is.
fn retrying<T>(
    wait: Duration,
    again: fn(&io::Error) -> bool,
    mut step: impl FnMut() -> io::Result<T>,
) -> io::Result<T> {
    let started = Instant::now();
    loop {
        match step() {
            Err(e) if again(&e) && started.elapsed() < wait => thread::sleep(Duration::from_millis(10)),
            done => return done,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACCESS_DENIED: i32 = 5;

    #[test]
    fn only_on_windows_and_only_denied_or_a_sharing_violation_is_a_file_in_use() {
        let kinds = [
            (ErrorKind::PermissionDenied, Some(ACCESS_DENIED), true),
            (ErrorKind::PermissionDenied, None, true),
            (ErrorKind::Other, Some(SHARING_VIOLATION), true),
            (ErrorKind::AlreadyExists, Some(80), false),
            (ErrorKind::NotFound, Some(2), false),
            (ErrorKind::InvalidData, None, false),
            (ErrorKind::TimedOut, None, false),
            (ErrorKind::Other, Some(ACCESS_DENIED + 1), false),
        ];
        for (kind, raw, on_windows) in kinds {
            assert_eq!(busy(kind, raw, true), on_windows, "{kind:?} {raw:?} on Windows");
            assert!(!busy(kind, raw, false), "{kind:?} {raw:?} elsewhere is an error, at once");
        }
        // This system's own errors, through the one question the callers ask.
        let denied = io::Error::from(ErrorKind::PermissionDenied);
        assert_eq!(in_use(&denied), cfg!(windows));
        assert!(!in_use(&io::Error::from(ErrorKind::NotFound)));
    }

    #[test]
    fn a_step_is_tried_again_only_while_its_file_is_in_use_and_only_for_the_wait() {
        let denied = |e: &io::Error| e.kind() == ErrorKind::PermissionDenied;
        let fail = |kind: ErrorKind| Err::<u32, _>(io::Error::from(kind));
        // In use twice, then free.
        let mut calls = 0;
        let done = retrying(Duration::from_secs(5), denied, || {
            calls += 1;
            if calls < 3 { fail(ErrorKind::PermissionDenied) } else { Ok(calls) }
        });
        assert_eq!((done.ok(), calls), (Some(3), 3));
        // Any other error is the answer, at once.
        let mut calls = 0;
        let done = retrying(Duration::from_secs(5), denied, || {
            calls += 1;
            fail(ErrorKind::NotFound)
        });
        assert_eq!((done.map_err(|e| e.kind()), calls), (Err(ErrorKind::NotFound), 1));
        // In use for good: the wait ends it, and the last error is what is said.
        let (mut calls, started) = (0, Instant::now());
        let done = retrying(Duration::from_millis(50), denied, || {
            calls += 1;
            fail(ErrorKind::PermissionDenied)
        });
        assert_eq!(done.map_err(|e| e.kind()), Err(ErrorKind::PermissionDenied));
        assert!(calls > 1 && started.elapsed() >= Duration::from_millis(50) && started.elapsed() < WAIT, "{calls}");
    }
}
