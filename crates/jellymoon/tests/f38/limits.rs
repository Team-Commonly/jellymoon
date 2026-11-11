//! The two ways a copy's command could be left waiting or doubled: a git that never returns, and two `copy clean`
//! of one run at once. The git that never returns is the made-up `fake-git`, first on the `PATH` these runs are
//! given. It runs the real git, then sleeps for a minute holding a lock of the system's.

use std::env::consts::EXE_SUFFIX;
use std::ffi::OsString;
use std::path::Path;
use std::{env, fs, thread};

use super::{World, refused, said, world};
use crate::support::{Ran, plant};

/// A folder with `fake-git` in it as `git`, told where the real one is, and the variables that put it first on
/// `PATH` and give a git child three seconds: far more than a git call here takes, far less than the minute slept.
fn fake(w: &World) -> Vec<(&'static str, OsString)> {
    let (folder, name) = (w.at("fake"), format!("git{EXE_SUFFIX}"));
    fs::create_dir(&folder).unwrap();
    plant(env!("CARGO_BIN_EXE_fake-git"), &folder.join(&name));
    let path = env::var_os("PATH").unwrap();
    let real = env::split_paths(&path).map(|dir| dir.join(&name)).find(|git| git.is_absolute() && git.is_file());
    fs::write(folder.join("real"), real.unwrap().to_str().unwrap()).unwrap();
    let first = env::join_paths([folder].into_iter().chain(env::split_paths(&path))).unwrap();
    vec![("PATH", first), ("JELLYMOON_TEST_GIT_LIMIT_MS", "3000".into())]
}

/// Runs `jellymoon` with a git that never returns from the call that has `word` among its arguments. The command
/// must be refused for that. The fake git must have got as far as its sleep, and must be gone: the lock it held
/// until it ended can be taken.
fn stalled_on(w: &World, vars: &[(&'static str, OsString)], word: &str, args: &[&str]) -> Ran {
    let fake = w.at("fake");
    fs::write(fake.join("on"), word).unwrap();
    let ran = w.run_with(vars, args);
    assert!(refused(&ran).contains("git did not finish in 3 seconds and was stopped"), "{}", ran.stderr);
    assert!(fake.join("stalled").is_file(), "the made-up git never got to its sleep");
    let held = fs::File::open(fake.join("held")).unwrap();
    assert!(held.try_lock().is_ok(), "the git that never returns is still running");
    fs::remove_file(fake.join("stalled")).unwrap();
    ran
}

fn names(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> =
        fs::read_dir(folder).unwrap().map(|entry| entry.unwrap().file_name().into_string().unwrap()).collect();
    names.sort();
    names
}

#[test]
fn a_git_that_never_returns_is_killed_and_no_half_made_copy_is_left() {
    let w = world();
    let (repo, vars) = (w.repo("alpha"), fake(&w));
    let (base, before) = (w.commit(&repo, "HEAD"), w.persons(&repo, &[]));
    let (copies, runs) = (w.at("jm").join("copies"), w.at("jm").join("runs"));

    // `worktree add` made the copy and its branch, and then never returned: all of it is taken back.
    stalled_on(&w, &vars, "add", &["copy", "make", "--repo", repo.to_str().unwrap()]);
    assert_eq!((names(&copies), names(&runs)), (vec![], vec!["clean.lock".to_string(), "no-hooks".to_string()]));
    assert_eq!((w.listed(&repo), w.git(&repo, &["branch", "--list", "jellymoon/*"])), (1, String::new()));
    assert_eq!(w.persons(&repo, &[]), before);

    // A clean that could not commit keeps the copy and its record, and a later clean ends it.
    let made = w.make(&repo);
    fs::write(made.folder.join("a.txt"), "edited by the run\n").unwrap();
    stalled_on(&w, &vars, "write-tree", &["copy", "clean", "--run", &made.run, "--outcome", "completed"]);
    assert!(made.folder.join("a.txt").is_file() && w.commit(&repo, &made.branch) == base);
    assert!(!said(&w.clean(&made.run, "completed")).kept && !made.folder.exists());
    assert_eq!(w.git(&repo, &["rev-list", "--count", &format!("main..{}", made.branch)]), "1");
}

#[test]
fn two_cleans_of_one_run_at_once_leave_one_commit_and_the_second_says_there_is_no_copy() {
    let w = world();
    let repo = w.repo("alpha");
    for round in 0..6 {
        let made = w.make(&repo);
        fs::write(made.folder.join("a.txt"), format!("round {round}\n")).unwrap();
        let clean = || w.clean(&made.run, "completed");
        let (one, two) = thread::scope(|both| {
            let (one, two) = (both.spawn(clean), both.spawn(clean));
            (one.join().unwrap(), two.join().unwrap())
        });
        let (done, late) = if one.code == 0 { (one, two) } else { (two, one) };
        assert!(!said(&done).kept && !made.folder.exists(), "round {round}");
        assert!(refused(&late).contains("no copy is kept for run"), "round {round}: {}", late.stderr);
        let commits = w.git(&repo, &["rev-list", "--count", &format!("main..{}", made.branch)]);
        assert_eq!((commits.as_str(), w.listed(&repo)), ("1", 1), "round {round}");
        assert_eq!(w.git(&repo, &["show", &format!("{}:a.txt", made.branch)]), format!("round {round}"));
    }
}
