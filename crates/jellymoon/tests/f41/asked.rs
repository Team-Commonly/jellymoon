//! A left copy whose repo cannot be asked at a look, and a look that is cut off between its git steps
//! (`docs/development/run.md`, "A copy nothing names"). The git that never returns is F38's made-up `fake-git`,
//! first on the `PATH` of the one look that is to be cut off.

use std::env::consts::EXE_SUFFIX;
use std::ffi::OsString;
use std::path::Path;
use std::{env, fs};

use serde_json::{Value, json};

use super::named::link;
use super::{DAY, World, found_world};
use crate::support::{plant, skipped};

impl World {
    /// A look: `jellymoon page` of the day, with `more` variables after the world's.
    pub(super) fn look(&self, more: &[(&'static str, OsString)]) -> i32 {
        let logs = self.at("logs");
        self.run_with(more, &["page", "--date", DAY, "--logs", logs.to_str().unwrap()]).code
    }

    /// How many working trees a repo lists, its own checkout among them.
    pub(super) fn listed(&self, repo: &Path) -> usize {
        self.git(repo, &["worktree", "list", "--porcelain"]).matches("worktree ").count()
    }

    /// One field of a copy's record, set by hand.
    pub(super) fn noted(&self, run: &str, field: &str, value: Value) {
        let file = self.jm("runs").join(run).join("copy.json");
        let mut record: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        record[field] = value;
        fs::write(&file, record.to_string()).unwrap();
    }

    /// The variables that put `fake-git` first on `PATH` and give a git child one second from when the made-up
    /// git has left `stalled`.
    pub(super) fn fake_git(&self) -> Vec<(&'static str, OsString)> {
        let (folder, name) = (self.at("fake"), format!("git{EXE_SUFFIX}"));
        fs::create_dir(&folder).unwrap();
        plant(env!("CARGO_BIN_EXE_fake-git"), &folder.join(&name));
        let path = &self.vars.iter().find(|(name, _)| *name == "PATH").unwrap().1;
        let real = env::split_paths(path).map(|dir| dir.join(&name)).find(|git| git.is_file()).unwrap();
        fs::write(folder.join("real"), real.to_str().unwrap()).unwrap();
        let first = env::join_paths([folder.clone()].into_iter().chain(env::split_paths(path))).unwrap();
        let stalled = folder.join("stalled").into();
        vec![
            ("PATH", first),
            ("JELLYMOON_TEST_GIT_LIMIT_MS", "1000".into()),
            ("JELLYMOON_TEST_GIT_LIMIT_FROM", stalled),
        ]
    }
}

/// Shuts a folder or a file to everyone, or opens it again. False where a mode keeps nobody out: on Windows,
/// and for an account that may read everything.
#[cfg(unix)]
pub(super) fn shut(at: &Path, shut: bool) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(at, fs::Permissions::from_mode(if shut { 0o000 } else { 0o755 })).unwrap();
    !shut || if at.is_dir() { fs::read_dir(at).is_err() } else { fs::File::open(at).is_err() }
}

#[cfg(not(unix))]
pub(super) fn shut(_: &Path, _: bool) -> bool {
    false
}

// A look does its git steps in this order: is the branch someone's, the branch, the repo's listing of the
// copy. Cut off after any of them, by a git that never returns from it, the next look finishes: no branch stays
// that a later look cannot take back. A third look finds nothing.
#[test]
fn a_look_cut_off_after_any_of_its_git_steps_is_finished_by_the_next_look() {
    let w = found_world();
    let (repo, wish, vars) = (w.repo("alpha"), w.wish("alpha"), w.fake_git());
    let fake = w.at("fake");
    // What the cut-off look leaves in the repo: the branch, and how many working trees it lists.
    for (step, branch, listed) in [("for-each-ref", true, 2), ("update-ref", false, 2), ("remove", false, 1)] {
        let run = w.killed_in_the_claim(&wish, &repo);
        fs::write(fake.join("on"), step).unwrap();
        assert_eq!(w.look(&vars), 0, "{step}");
        assert!(fake.join("stalled").is_file(), "{step}: the made-up git never got to its sleep");
        assert!(fs::File::open(fake.join("held")).unwrap().try_lock().is_ok(), "{step}: that git still runs");
        fs::remove_file(fake.join("stalled")).unwrap();
        let left = if branch { vec![format!("jellymoon/run-{run}")] } else { vec![] };
        assert_eq!((w.branches(&repo), w.listed(&repo), w.kept_for(&run)), (left, listed, true), "{step}: cut off");
        for look in ["the next look", "a third look"] {
            assert_eq!(w.look(&[]), 0);
            assert_eq!(w.left_of(&run, &repo), (vec![], vec![], false), "{step}: {look}");
            assert_eq!(w.listed(&repo), 1, "{step}: {look}");
        }
    }
}

// A repo that is not where its record says cannot be asked. Nothing is done then, in the home or anywhere: the
// copy and its record wait for a look that finds the repo, which takes the copy back whole.
#[test]
fn a_left_copy_whose_repo_was_moved_away_is_kept_and_taken_back_whole_once_the_repo_is_back() {
    let w = found_world();
    let (repo, wish, away) = (w.repo("alpha"), w.wish("alpha"), w.at("work").join("moved"));
    let run = w.killed_in_the_claim(&wish, &repo);
    fs::rename(&repo, &away).unwrap();
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.copies(), w.kept_for(&run)), (vec![run.clone()], true), "a repo that was away lost its record");
    fs::rename(&away, &repo).unwrap();
    assert_eq!((w.branches(&repo), w.listed(&repo)), (vec![format!("jellymoon/run-{run}")], 2));
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.left_of(&run, &repo), w.listed(&repo)), ((vec![], vec![], false), 1));
}

#[test]
fn a_left_copy_whose_repo_cannot_be_read_at_one_look_is_taken_back_whole_by_the_next() {
    let w = found_world();
    let (repo, wish, above) = (w.repo("alpha"), w.wish("alpha"), w.at("work"));
    let run = w.killed_in_the_claim(&wish, &repo);
    if !shut(&above, true) {
        shut(&above, false);
        return skipped("no mode keeps this account out of a folder (Windows, or one that may read everything)");
    }
    let looked = w.look(&[]);
    shut(&above, false);
    assert_eq!(looked, 0);
    assert_eq!((w.copies(), w.kept_for(&run)), (vec![run.clone()], true), "a repo shut for one look lost its record");
    assert_eq!((w.branches(&repo), w.listed(&repo)), (vec![format!("jellymoon/run-{run}")], 2));
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.left_of(&run, &repo), w.listed(&repo)), ((vec![], vec![], false), 1));
}

// A repo removed for good is never asked. Its copy's record takes the time of the first look that could not ask,
// and keeps that time; a look thirty days after it removes what the home holds of the run.
#[test]
fn a_left_copy_whose_repo_is_gone_for_good_is_kept_for_thirty_days_and_then_leaves_the_home() {
    let w = found_world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let run = w.killed_in_the_claim(&wish, &repo);
    fs::remove_dir_all(&repo).unwrap();
    let first = |w: &World| -> Value {
        assert_eq!(w.look(&[]), 0);
        assert_eq!((w.copies(), w.kept_for(&run)), (vec![run.clone()], true), "kept for less than thirty days");
        let record = fs::read_to_string(w.jm("runs").join(&run).join("copy.json")).unwrap();
        serde_json::from_str::<Value>(&record).unwrap()["put_off"].clone()
    };
    let noted = first(&w);
    assert!(noted.as_u64().is_some_and(|at| at > 0), "the first look that could not ask was not noted: {noted}");
    assert_eq!(first(&w), noted, "a later look moved the time of the first");
    // Thirty days, less a minute and more a minute, in nanoseconds before the look.
    let ago = |seconds: u64| json!(noted.as_u64().unwrap() - seconds * 1_000_000_000);
    w.noted(&run, "put_off", ago(30 * 24 * 3600 - 60));
    assert_eq!(first(&w), ago(30 * 24 * 3600 - 60));
    w.noted(&run, "put_off", ago(30 * 24 * 3600 + 60));
    // The look that gives the copy up says so, in one line, and is no failure.
    let logs = w.at("logs");
    let gave_up = w.run(&["page", "--date", DAY, "--logs", logs.to_str().unwrap()]);
    let line = format!(
        "jellymoon: the copy of run {run} could not be taken back for thirty days and is given up: what Jellymoon \
         kept of it is removed, and its branch jellymoon/run-{run} may still be in its repo\n"
    );
    assert_eq!((gave_up.code, gave_up.stderr), (0, line));
    assert_eq!((w.copies(), w.kept_for(&run)), (vec![], false), "a repo gone for good kept its record for ever");
}

// A time later than now, in a record written by hand or before the clock was set back, would keep the copy for
// ever. It is taken as now, and written so.
#[test]
fn a_time_in_the_future_in_a_left_copy_s_record_is_taken_as_now() {
    let w = found_world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let run = w.killed_in_the_claim(&wish, &repo);
    fs::remove_dir_all(&repo).unwrap();
    assert_eq!(w.look(&[]), 0);
    let noted = |w: &World| {
        let record = fs::read_to_string(w.jm("runs").join(&run).join("copy.json")).unwrap();
        serde_json::from_str::<Value>(&record).unwrap()["put_off"].as_u64().unwrap()
    };
    let (now, far) = (noted(&w), u64::MAX / 2);
    w.noted(&run, "put_off", json!(far));
    assert_eq!(w.look(&[]), 0);
    assert!((now..far).contains(&noted(&w)), "a time in the future was kept: {}", noted(&w));
    assert_eq!((w.copies(), w.kept_for(&run)), (vec![run.clone()], true));
}

// A record made by hand names another repo, and a git folder of the copy's inside it that is not there, as a
// look cut off after the repo forgot the copy would leave it. That repo answers and does not list the copy:
// nothing is done there, and its branch of the run's name stays. No look removes a branch on a record's word.
#[test]
fn a_record_made_by_hand_for_a_copy_its_repo_no_longer_lists_removes_no_branch_there() {
    let w = found_world();
    let (repo, beta, wish) = (w.repo("alpha"), w.repo("beta"), w.wish("alpha"));
    let run = w.killed_in_the_claim(&wish, &repo);
    let (git, branch) = (w.git(&beta, &["rev-parse", "--absolute-git-dir"]), format!("jellymoon/run-{run}"));
    w.git(&beta, &["branch", &branch]);
    let before = w.persons(&beta);
    w.renamed(&run, &git, &Path::new(&git).join("worktrees").join(&run));
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.branches(&beta), w.persons(&beta)), (vec![branch], before), "a repo a record named was changed");
    assert_eq!((w.copies(), w.kept_for(&run)), (vec![], false), "what the home held of the run stayed");
}

// `copies` itself is a link to a folder elsewhere, and a plain file of the run's name is in that folder. Removing
// what is at the copy's name would go through the link: the file stays.
#[test]
fn a_plain_file_of_the_run_s_name_in_a_folder_that_copies_is_a_link_to_stays() {
    let w = found_world();
    let (repo, wish, outside) = (w.repo("alpha"), w.wish("alpha"), w.at("outside"));
    let run = w.killed_in_the_claim(&wish, &repo);
    fs::rename(w.jm("copies"), &outside).unwrap();
    fs::remove_dir_all(outside.join(&run)).unwrap();
    fs::write(outside.join(&run), "the person's own\n").unwrap();
    if !link(&outside, &w.jm("copies")) {
        return skipped("no link to a folder can be made here");
    }
    assert_eq!(w.look(&[]), 0);
    assert_eq!(fs::read_to_string(outside.join(&run)).unwrap(), "the person's own\n", "removed through a link");
    assert!(w.kept_for(&run), "the record of a copy that could not be taken back went");
}

// The record names the copy's own repo by a link to it. "Inside" is asked of both folders as the system
// resolves them, so the copy is still the repo's own and is taken back whole.
#[test]
fn a_record_that_names_its_own_repo_through_a_link_is_taken_back_whole() {
    let w = found_world();
    let (repo, wish, alias) = (w.repo("alpha"), w.wish("alpha"), w.at("alias"));
    let run = w.killed_in_the_claim(&wish, &repo);
    if !link(&repo, &alias) {
        return skipped("no link to a folder can be made here");
    }
    let own = w.own_of(&run);
    w.renamed(&run, alias.join(".git").to_str().unwrap(), &own);
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.left_of(&run, &repo), w.listed(&repo)), ((vec![], vec![], false), 1));
}

// git makes a copy's folder, its branch and its listing in one call, and only then is git asked where the copy's
// own git folder is. A command killed between the two left a copy no record named, and a branch for good. The
// record is written before git is called, naming the git folder git will make.
#[test]
fn a_copy_whose_command_was_killed_while_the_copy_was_made_is_taken_back_by_the_next_look() {
    let w = found_world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (mut going, run) = w.held_at("JELLYMOON_TEST_MADE_WAIT_MS", &wish, &repo);
    going.kill().unwrap();
    going.wait().unwrap();
    let made = (vec![run.clone()], vec![format!("jellymoon/run-{run}")], true);
    assert_eq!((w.left_of(&run, &repo), w.listed(&repo)), (made, 2), "git had not made the copy");
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.left_of(&run, &repo), w.listed(&repo)), ((vec![], vec![], false), 1));
}
