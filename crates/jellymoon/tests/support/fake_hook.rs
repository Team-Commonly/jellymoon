//! A made-up program for git to run, for the black-box tests of a run's copy (F38): a hook, a filter or the program
//! `core.fsmonitor` names. All it does is leave an empty file beside its own program file, named as that file with
//! `.ran` after it. A test that finds no such file knows git never started it.
#![allow(clippy::unwrap_used, clippy::expect_used)]

fn main() {
    let mut mark = std::env::current_exe().unwrap().into_os_string();
    mark.push(".ran");
    std::fs::write(mark, "").unwrap();
}
