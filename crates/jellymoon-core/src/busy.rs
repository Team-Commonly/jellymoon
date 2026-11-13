//! A step on a file that another process is using at that moment (`docs/development/delivery.md`, "On Windows").
//! On unix such a step works: a name being removed can be made again, and a file that is open can be removed,
//! renamed and renamed over. On Windows each of those fails with "Access is denied" (5) or a sharing violation
//! (32) for as long as the other process holds the file, which between two changes to one day is milliseconds.
//! So there, and only there, a step that fails so is tried again for the day's own wait, and then its error stands.

use std::io::{self, ErrorKind};
use std::path::PathBuf;
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

/// A wait or a limit, shorter for a test: the variable holds milliseconds, and is read only by a build with debug
/// assertions, which a release is not.
pub(crate) fn or_test(real: Duration, variable: &str) -> Duration {
    let short = std::env::var(variable).ok().filter(|_| cfg!(debug_assertions));
    short.and_then(|ms| ms.parse().ok()).map_or(real, Duration::from_millis)
}

/// The file a test's build names in `variable`, and none in a release. A limit that is under test is counted
/// from the moment that file is there: the made-up program the limit is tested on leaves it when it has got to
/// where it stalls, so nothing it does before that has to be done within the short limit.
pub(crate) fn test_file(variable: &str) -> Option<PathBuf> {
    std::env::var_os(variable).filter(|_| cfg!(debug_assertions)).map(PathBuf::from)
}

/// A place a test's build can hold a command at: for the milliseconds in `variable`, or until the file that
/// `JELLYMOON_TEST_GO` names is there, whichever is first. The variable's name is left in a file of that name
/// with `.at` for its ending, so the test knows that the command has come to the place. A release never waits.
pub(crate) fn pause(variable: &str) {
    let (wait, from) = (or_test(Duration::ZERO, variable), Instant::now());
    let go = test_file("JELLYMOON_TEST_GO").filter(|_| !wait.is_zero());
    if let Some(go) = &go {
        let _ = std::fs::write(go.with_extension("at"), variable);
    }
    while from.elapsed() < wait && !go.as_ref().is_some_and(|go| go.exists()) {
        thread::sleep(Duration::from_millis(10));
    }
}

/// [`pause`] for a test that reads every state a file ever has: the file `JELLYMOON_TEST_GO` names is taken away
/// when the wait is over, so the test's next one lets go of the next step only. No clock is in such a proof.
pub(crate) fn step(variable: &str) {
    pause(variable);
    if let Some(go) = test_file("JELLYMOON_TEST_GO").filter(|_| !or_test(Duration::ZERO, variable).is_zero()) {
        let _ = retried(|| std::fs::remove_file(&go));
    }
}

/// Does `step`, again every 10 ms while it fails with a file in use and [`WAIT`] has not passed.
pub(crate) fn retried<T>(step: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    retrying(WAIT, in_use, step)
}

/// [`retried`] for a wait of the caller's, where the other process is done in less than the day's own wait.
pub(crate) fn retried_for<T>(wait: Duration, step: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    retrying(wait, in_use, step)
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
