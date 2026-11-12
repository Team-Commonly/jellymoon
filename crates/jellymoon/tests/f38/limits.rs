//! The ways a copy's command could be left waiting or doubled: a git that never returns, copies made at once, and
//! two `copy clean` of one run at once. The git that never returns is the made-up `fake-git`, first on the `PATH` these runs are
//! given. It runs the real git, then leaves the file `stalled` and sleeps for a minute holding a lock of the
//! system's. The short limit is counted from when that file is there (`JELLYMOON_TEST_GIT_LIMIT_FROM`): no git
//! call that is not meant to stall, and no part of the one that is, has to be done within it.

use std::env::consts::EXE_SUFFIX;
use std::ffi::OsString;
use std::path::Path;
use std::{env, fs, thread};

use serde_json::{Value, json};

use super::{World, refused, said, world};
use crate::support::{Ran, plant};

/// A folder with `fake-git` in it as `git`, told where the real one is, and the variables that put it first on
/// `PATH` and give a git child one second once the made-up git has left `stalled`: far less than the minute slept.
fn fake(w: &World) -> Vec<(&'static str, OsString)> {
    let (folder, name) = (w.at("fake"), format!("git{EXE_SUFFIX}"));
    fs::create_dir(&folder).unwrap();
    plant(env!("CARGO_BIN_EXE_fake-git"), &folder.join(&name));
    let path = env::var_os("PATH").unwrap();
    let real = env::split_paths(&path).map(|dir| dir.join(&name)).find(|git| git.is_absolute() && git.is_file());
    fs::write(folder.join("real"), real.unwrap().to_str().unwrap()).unwrap();
    let first = env::join_paths([folder].into_iter().chain(env::split_paths(&path))).unwrap();
    let stalled = w.at("fake").join("stalled").into();
    vec![("PATH", first), ("JELLYMOON_TEST_GIT_LIMIT_MS", "1000".into()), ("JELLYMOON_TEST_GIT_LIMIT_FROM", stalled)]
}

/// Runs `jellymoon` with a git that never returns from the call that has `word` among its arguments. The command
/// must be refused for that. The fake git must have got as far as its sleep, and must be gone: the lock it held
/// until it ended can be taken.
fn stalled_on(w: &World, vars: &[(&'static str, OsString)], word: &str, args: &[&str]) -> Ran {
    let fake = w.at("fake");
    fs::write(fake.join("on"), word).unwrap();
    let ran = w.run_with(vars, args);
    assert!(refused(&ran).contains("git did not finish in 1 seconds and was stopped"), "{}", ran.stderr);
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

// A git stopped at its limit can leave a child of its own still writing the copy's files after core removed them.
#[test]
fn a_copies_folder_no_run_stands_for_is_removed_by_the_next_clean_and_one_being_made_is_not() {
    let w = world();
    let made = w.make(&w.repo("alpha"));
    let (copies, runs) = (w.at("jm").join("copies"), w.at("jm").join("runs"));
    let (late, making, other) = ("r00000000000000a1", "r00000000000000a2", "notes");
    for name in [late, making, other] {
        fs::create_dir_all(copies.join(name).join("deep")).unwrap();
        fs::write(copies.join(name).join("deep").join("f.txt"), "written late\n").unwrap();
    }
    fs::create_dir(runs.join(making)).unwrap(); // a `make` that has taken its id and not yet written its record
    assert!(said(&w.clean(&made.run, "failed")).kept);
    let mut left = vec![making.to_string(), other.to_string(), made.run.clone()];
    left.sort();
    assert_eq!(names(&copies), left, "only the folder under a run id that no run stands for is removed");
    assert!(copies.join(making).join("deep").join("f.txt").is_file());
}

// "Newest" is the order the runs failed in, which the records hold: a clock set back does not make the run that
// just failed the oldest. Here the five before it say they failed far in the future.
#[test]
fn the_run_that_just_failed_is_kept_whatever_the_clock_says_and_kept_is_said_after_the_old_ones_went() {
    let w = world();
    let repo = w.repo("alpha");
    let failed = |w: &World| {
        let made = w.make(&repo);
        (said(&w.clean(&made.run, "failed")).kept, made)
    };
    let old: Vec<_> = (0..5).map(|_| failed(&w).1).collect();
    for (n, made) in old.iter().enumerate() {
        let file = w.at("jm").join("runs").join(&made.run).join("copy.json");
        let mut record: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        record["failed"] = json!(u64::MAX / 2 + n as u64);
        fs::write(&file, record.to_string()).unwrap();
    }
    let (kept, last) = failed(&w);
    assert!(kept && last.folder.is_dir(), "the run that just failed is one of the newest five");
    assert!(!old[0].folder.exists() && old[1..].iter().all(|made| made.folder.is_dir()), "the oldest went");
    assert_eq!(w.listed(&repo), 6);
}

// git looks at every working tree of a repo while it adds one, and one half made, or half removed, stops it. So
// copies of one repo are made one at a time, and never while one is cleaned: eight at once, with the round
// before's eight cleaned at the same moment, are each made.
#[test]
fn copies_made_at_once_in_one_repo_while_others_are_cleaned_are_each_made() {
    let w = world();
    let repo = w.repo("alpha");
    let mut before: Vec<String> = Vec::new();
    for round in 0..5 {
        let make = || w.run(&["copy", "make", "--repo", repo.to_str().unwrap()]);
        let (made, cleaned) = thread::scope(|all| {
            let made: Vec<_> = (0..8).map(|_| all.spawn(make)).collect();
            let cleaned: Vec<_> = before.iter().map(|run| all.spawn(|| w.clean(run, "completed"))).collect();
            let ended = |going: Vec<thread::ScopedJoinHandle<Ran>>| -> Vec<Ran> {
                going.into_iter().map(|one| one.join().unwrap()).collect()
            };
            (ended(made), ended(cleaned))
        });
        for ran in made.iter().chain(&cleaned) {
            assert_eq!((ran.code, ran.stderr.as_str()), (0, ""), "round {round}");
        }
        before = made.iter().map(|ran| said(ran).run).collect();
        assert_eq!(w.listed(&repo), 9, "round {round}: the checkout and this round's eight copies");
    }
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
