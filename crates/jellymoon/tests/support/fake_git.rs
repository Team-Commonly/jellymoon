//! A made-up `git` that never returns, for the test of the time limit on a run's copy (F38). It stands first on the
//! test's `PATH` and reads two files in its own folder: `real`, the path of the real git, and `on`, one word. It runs
//! the real git with its own arguments. When one of them is that word it first takes the system's lock on the file
//! `held` there, and after the real git has ended it leaves the file `stalled` and sleeps for a minute. A test that
//! can then take the lock itself knows this program is gone: the system drops a lock only when its holder ends.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::ffi::OsString;
use std::process::{Command, exit};
use std::time::Duration;
use std::{env, fs, thread};

fn main() {
    let dir = env::current_exe().unwrap().parent().unwrap().to_path_buf();
    let read = |name: &str| fs::read_to_string(dir.join(name)).unwrap();
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    let word = read("on");
    let held = args.iter().any(|arg| arg.to_str() == Some(word.as_str())).then(|| {
        let held = fs::File::create(dir.join("held")).unwrap();
        held.lock().unwrap();
        held
    });
    let status = Command::new(read("real")).args(&args).status().unwrap();
    if held.is_some() {
        fs::write(dir.join("stalled"), "").unwrap();
        thread::sleep(Duration::from_secs(60));
    }
    exit(status.code().unwrap_or(1));
}
