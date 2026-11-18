//! A made-up login shell for the black-box tests of F78: what `jellymoon` starts when `JELLYMOON_SHELL` names it,
//! in place of a person's `zsh` or `bash`. It runs nothing it is given. It leaves beside its own program file one
//! line in `starts`, its arguments, the names of its variables, the folder it ran in and its whole input, and then
//! plays the script the test left in `play`, with the test's `path` file as the `PATH` it names:
//!
//! - `answer` (or no `play` file): the file `before`, a mark, the `PATH`, the same mark, the file `after`;
//! - `none`: the `PATH` with no mark; `one`: with one; `two`: two pairs of marks, in one write;
//! - `big`: more than core reads, and then the answer;
//! - `silent`: nothing.
//!
//! With a file `stays` there, it first starts itself once more as a program of its group that keeps this output
//! open, then holds the system's lock on `held` (the other on `left`), and after its script it does not end for a
//! minute: a test that can take both locks knows that both are gone.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;
use std::{env, fs, io, process, thread};

/// The mark core asks for, as it is printed: typed here, never read from the command line.
const MARK: &str = "__jellymoon_path__";

/// Holds the system's lock on a file of that name for as long as this program lives, and says so in `<name>-up`.
fn hold(rec: &Path, name: &str) -> fs::File {
    let held = fs::File::create(rec.join(name)).unwrap();
    held.lock().unwrap();
    fs::write(rec.join(format!("{name}-up")), "").unwrap();
    held
}

fn main() {
    let rec = env::current_exe().unwrap().parent().unwrap().to_path_buf();
    if env::args().nth(1).as_deref() == Some("--left") {
        let _held = hold(&rec, "left");
        thread::sleep(Duration::from_secs(60));
        return;
    }
    let mut starts = fs::OpenOptions::new().create(true).append(true).open(rec.join("starts")).unwrap();
    starts.write_all(b"started\n").unwrap();
    fs::write(rec.join("args"), env::args().skip(1).collect::<Vec<_>>().join("\n")).unwrap();
    let mut names: Vec<String> = env::vars_os().map(|(name, _)| name.to_string_lossy().into_owned()).collect();
    names.sort();
    fs::write(rec.join("env"), names.join("\n")).unwrap();
    fs::write(rec.join("cwd"), env::current_dir().unwrap().to_string_lossy().as_bytes()).unwrap();
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();
    fs::write(rec.join("stdin"), input).unwrap();

    let left = |name: &str| fs::read_to_string(rec.join(name)).unwrap_or_default();
    let stays = rec.join("stays").exists();
    let _held = stays.then(|| {
        // Left on purpose, and never waited for: whoever started this is to end it.
        #[allow(clippy::zombie_processes)]
        process::Command::new(env::current_exe().unwrap()).arg("--left").spawn().unwrap();
        while !rec.join("left-up").exists() {
            thread::sleep(Duration::from_millis(10));
        }
        hold(&rec, "held")
    });
    let path = left("path");
    let answer = format!("{}{MARK}{path}{MARK}\n{}", left("before"), left("after"));
    let said = match left("play").trim() {
        "" | "answer" => answer,
        "none" => format!("{path}\n"),
        "one" => format!("{MARK}{path}\n"),
        "two" => format!("{MARK}{path}{MARK}\n{MARK}{path}{MARK}\n"),
        "big" => format!("{}{answer}", "a line of a profile that says a lot\n".repeat(4096)),
        "silent" => String::new(),
        other => panic!("no script named {other}"),
    };
    let mut stdout = io::stdout().lock();
    // Whoever asked may have read all it reads and let go.
    let _ = stdout.write_all(said.as_bytes()).and_then(|()| stdout.flush());
    if stays {
        thread::sleep(Duration::from_secs(60));
    }
}
