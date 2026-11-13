//! A made-up `git` that never returns, for the test of the time limit on a run's copy (F38). It stands first on the
//! test's `PATH` and reads two files in its own folder: `real`, the path of the real git, and `on`, one word. It runs
//! the real git with its own arguments. When one of them is that word it first takes the system's lock on the file
//! `held` there, and after the real git has ended it leaves the file `stalled` and sleeps for a minute. A test that
//! can then take the lock itself knows this program is gone: the system drops a lock only when its holder ends.
//! A file `first` there is something that happens just before that step: the real git is run with its lines,
//! and what it prints goes where this program's own output goes. With a file `returns` there the step is not
//! slept after: it is a step with something done just before it, and nothing more. With a file `says` there, a
//! git asked for its `version` prints that file and nothing else: it is a git of another age (F45). A file
//! `after` there is something that happens just after that step, as `first` does before it; and with a file
//! `fails` there the step is not taken at all: this program ends with the number that file holds.
//! Under any name but git's it is a hook, a filter or a merge driver that someone planted: it leaves the file
//! `ran-<its name>` beside itself and does nothing else, so a test can see that it was never started.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::ffi::OsString;
use std::process::{Command, exit};
use std::time::Duration;
use std::{env, fs, thread};

fn main() {
    let dir = env::current_exe().unwrap().parent().unwrap().to_path_buf();
    let name = env::current_exe().unwrap().file_stem().unwrap().to_string_lossy().into_owned();
    if name != "git" {
        fs::write(dir.join(format!("ran-{name}")), "").unwrap();
        return;
    }
    let read = |name: &str| fs::read_to_string(dir.join(name)).unwrap();
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    if let (Some("version"), Ok(says)) =
        (args.last().and_then(|arg| arg.to_str()), fs::read_to_string(dir.join("says")))
    {
        print!("{says}");
        return;
    }
    let word = fs::read_to_string(dir.join("on")).unwrap_or_default();
    let held = args.iter().any(|arg| arg.to_str() == Some(word.as_str())).then(|| {
        let held = fs::File::create(dir.join("held")).unwrap();
        held.lock().unwrap();
        held
    });
    let beside = |name: &str| {
        if let (true, Ok(lines)) = (held.is_some(), fs::read_to_string(dir.join(name))) {
            Command::new(read("real")).args(lines.lines()).status().unwrap();
        }
    };
    beside("first");
    if let (true, Ok(code)) = (held.is_some(), fs::read_to_string(dir.join("fails"))) {
        exit(code.trim().parse().unwrap());
    }
    let status = Command::new(read("real")).args(&args).status().unwrap();
    beside("after");
    if held.is_some() && !dir.join("returns").exists() {
        fs::write(dir.join("stalled"), "").unwrap();
        thread::sleep(Duration::from_secs(60));
    }
    exit(status.code().unwrap_or(1));
}
