//! What ends with a test, however the test ends: a panic unwinds through both. A `jellymoon` command a test
//! started and has not seen end is killed when its [`Going`] is dropped. Killed so, a `jellymoon run` leaves its
//! agent, which nothing signals; and a run started apart is no child of the test at all. So when the test's
//! [`super::World`] is dropped it leaves the file `over` in `rec`, at which every made-up agent ends itself
//! (`support/fake_agent.rs`, `minded`), and waits until no agent and no `jellymoon run` of its folder is left:
//! each holds the system's lock on a file there while it lives. Only then is the folder removed, which Windows
//! refuses while a program still stands in it.

use std::fs::{self, File};
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::process::Child;
use std::thread;
use std::time::{Duration, Instant};

/// How long a dropped world waits for what it started to be gone. An agent looks for `over` every 20 ms.
const WAIT: Duration = Duration::from_secs(10);

/// A `jellymoon` command that is still going. Dropped before it was seen to end, it is killed and waited for.
pub struct Going(Option<Child>);

impl Going {
    pub fn new(child: Child) -> Self {
        Self(Some(child))
    }

    /// The command itself, for a caller that waits for it: nothing is killed any more when this is dropped.
    pub fn take(&mut self) -> Child {
        self.0.take().unwrap()
    }
}

impl Deref for Going {
    type Target = Child;
    fn deref(&self) -> &Child {
        self.0.as_ref().unwrap()
    }
}

impl DerefMut for Going {
    fn deref_mut(&mut self) -> &mut Child {
        self.0.as_mut().unwrap()
    }
}

impl Drop for Going {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Whether something holds the system's lock on a file. One that is not there, or cannot be opened, is not held,
/// and neither is what a test put at the name that is no plain file: a named pipe is never opened.
fn held(file: &Path) -> bool {
    let plain = fs::symlink_metadata(file).is_ok_and(|meta| meta.is_file());
    plain && File::open(file).is_ok_and(|file| file.try_lock_shared().is_err())
}

/// Whether an agent or a `jellymoon run` of a world's folder still lives: an agent holds `rec/live-<pid>`, and a
/// `jellymoon run` its `start.lock` from before its copy is made until it ends.
pub fn alive(dir: &Path) -> bool {
    let names = |folder: &Path| fs::read_dir(folder).into_iter().flatten().flatten().map(|entry| entry.path());
    let agent = |file: &Path| file.file_name().is_some_and(|name| name.to_string_lossy().starts_with("live-"));
    names(&dir.join("rec")).any(|file| agent(&file) && held(&file))
        || names(&dir.join("jm").join("runs")).any(|run| held(&run.join("start.lock")) || held(&run.join("run.lock")))
}

/// Ends what a world started: called as the world is dropped. Nothing here panics, since a test that is
/// unwinding from a panic of its own would be ended by a second one with nothing said.
pub fn end(dir: &Path) {
    let _ = fs::write(dir.join("rec").join("over"), "");
    let from = Instant::now();
    while alive(dir) && from.elapsed() < WAIT {
        thread::sleep(Duration::from_millis(10));
    }
    if alive(dir) {
        eprintln!("left going after a test: an agent or a jellymoon run of {}", dir.display());
    }
}
