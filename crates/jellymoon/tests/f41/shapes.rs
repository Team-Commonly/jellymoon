//! The shapes of a left copy's record, of its repo and of the repo's checkouts that a look must tell apart
//! before it removes a branch (`docs/development/run.md`, "A copy nothing names"). A left copy is made here by
//! `jellymoon copy make` and an empty `start.lock` put beside its record: what a killed `jellymoon run` leaves.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::named::link;
use super::{DAY, World, found_world, printed};
use crate::support::skipped;

/// What is left of a run in the home: its copies, and whether anything is kept for it.
pub(super) type Home = (Vec<String>, bool);
pub(super) const NONE: Home = (Vec::new(), false);

impl World {
    /// A copy of a repo that a killed command left, and its branch.
    pub(super) fn left_copy(&self, repo: &Path) -> (String, String) {
        let made = printed(&self.run(&["copy", "make", "--repo", repo.to_str().unwrap()]));
        let run = made["run"].as_str().unwrap().to_string();
        fs::write(self.jm("runs").join(&run).join("start.lock"), "").unwrap();
        (format!("jellymoon/run-{run}"), run)
    }

    pub(super) fn home_of(&self, run: &str) -> Home {
        (self.copies(), self.kept_for(run))
    }

    /// A commit on the copy's branch that nothing else has, made in the copy.
    pub(super) fn commit_of_its_own(&self, repo: &Path, run: &str) -> String {
        self.git(&self.jm("copies").join(run), &["commit", "-q", "--allow-empty", "-m", "the person's own"]);
        self.commit(repo, &format!("jellymoon/run-{run}"))
    }

    /// A repo whose git folder is inside another repo's, at `modules/s`, as a submodule's is.
    fn inside(&self, outer: &Path) -> PathBuf {
        let (repo, git) = (self.at("work").join("s"), outer.join(".git").join("modules").join("s"));
        fs::create_dir_all(&repo).unwrap();
        fs::create_dir_all(git.parent().unwrap()).unwrap();
        self.git(&repo, &["init", "-q", "-b", "main", "--separate-git-dir", git.to_str().unwrap()]);
        fs::write(repo.join("a.txt"), "a.txt as it was\n").unwrap();
        self.git(&repo, &["add", "."]);
        self.git(&repo, &["commit", "-q", "-m", "the first commit"]);
        repo
    }
}

// "Inside the repo" was true of the copy's own git folder named as the repo. git takes a worktree's git folder
// for a repo, and its `HEAD` there is the copy's own, which is the branch: the branch looked reached.
#[test]
fn a_record_whose_repo_is_the_copy_s_own_git_folder_removes_no_branch() {
    let w = found_world();
    let repo = w.repo("alpha");
    let (branch, run) = w.left_copy(&repo);
    let (mine, own) = (w.commit_of_its_own(&repo, &run), w.own_of(&run));
    w.renamed(&run, own.to_str().unwrap(), &own);
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.branches(&repo), w.commit(&repo, &branch)), (vec![branch], mine), "the commit was lost");
    assert_eq!(w.home_of(&run), NONE, "what the home held of the run stayed");
}

// The copy's git folder is inside the git folder of a repo that encloses its own, and the record names that
// one: its branch of the run's name is not the copy's.
#[test]
fn a_record_that_names_a_repo_enclosing_the_copy_s_own_removes_no_branch_there() {
    let w = found_world();
    let outer = w.repo("alpha");
    let (branch, run) = w.left_copy(&w.inside(&outer));
    w.git(&outer, &["branch", &branch]);
    let before = w.persons(&outer);
    w.renamed(&run, outer.join(".git").to_str().unwrap(), &w.own_of(&run));
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.branches(&outer), w.persons(&outer)), (vec![branch], before), "a repo a record named was changed");
    assert_eq!(w.home_of(&run), NONE, "what the home held of the run stayed");
}

// The record names the git folder of a worktree of the repo, and the copy's git folder has been put in a
// `worktrees` of that one's own. git takes a worktree's git folder for the repo; a look does not.
#[test]
fn a_record_whose_repo_is_a_worktree_s_git_folder_removes_no_branch() {
    let w = found_world();
    let (repo, side) = (w.repo("alpha"), w.at("work").join("side"));
    w.git(&repo, &["worktree", "add", "-q", "-b", "side", side.to_str().unwrap()]);
    let (branch, run) = w.left_copy(&repo);
    let within = repo.join(".git").join("worktrees").join("side");
    let own = within.join("worktrees").join(&run);
    fs::create_dir(own.parent().unwrap()).unwrap();
    fs::rename(w.own_of(&run), &own).unwrap();
    w.renamed(&run, within.to_str().unwrap(), &own);
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.branches(&repo), w.home_of(&run)), (vec![branch], NONE));
}

// The copy's git folder put one folder deeper in the repo's `worktrees`: git keeps none there.
#[test]
fn a_record_whose_own_is_deeper_in_the_repo_s_worktrees_removes_no_branch() {
    let w = found_world();
    let repo = w.repo("alpha");
    let (branch, run) = w.left_copy(&repo);
    let own = repo.join(".git").join("worktrees").join("deep").join(&run);
    fs::create_dir(own.parent().unwrap()).unwrap();
    fs::rename(w.own_of(&run), &own).unwrap();
    w.renamed(&run, repo.join(".git").to_str().unwrap(), &own);
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.branches(&repo), w.home_of(&run)), (vec![branch], NONE));
}

#[test]
fn a_copy_of_a_repo_whose_git_folder_is_inside_another_repo_s_is_taken_back_whole() {
    let w = found_world();
    let repo = w.inside(&w.repo("alpha"));
    let (_, run) = w.left_copy(&repo);
    assert_eq!((w.branches(&repo).len(), w.listed(&repo)), (1, 2));
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.branches(&repo), w.listed(&repo), w.home_of(&run)), (vec![], 1, NONE));
}

#[test]
fn a_copy_of_a_checkout_that_is_itself_a_worktree_of_a_repo_is_taken_back_whole() {
    let w = found_world();
    let (repo, side) = (w.repo("alpha"), w.at("work").join("side"));
    w.git(&repo, &["worktree", "add", "-q", "-b", "side", side.to_str().unwrap()]);
    let (_, run) = w.left_copy(&side);
    assert_eq!((w.branches(&repo).len(), w.listed(&repo)), (1, 3));
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.branches(&repo), w.listed(&repo), w.home_of(&run)), (vec![], 2, NONE));
}

// A checkout on a branch with no commit yet has a `HEAD` that names no commit: it is left out of the question,
// and is no reason the copy cannot be taken back.
#[test]
fn a_checkout_left_on_a_branch_with_no_commit_does_not_keep_a_left_copy() {
    let w = found_world();
    let repo = w.repo("alpha");
    let (_, run) = w.left_copy(&repo);
    w.git(&repo, &["checkout", "-q", "--orphan", "fresh"]);
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.branches(&repo), w.listed(&repo), w.home_of(&run)), (vec![], 1, NONE));
}

// The person's checkout was switched onto the run's branch, and a commit made there. `HEAD` reaches that commit,
// and is this branch: the branch is kept, and the rest is taken back.
#[test]
fn a_branch_the_person_s_checkout_stands_on_is_kept() {
    let w = found_world();
    let repo = w.repo("alpha");
    let (branch, run) = w.left_copy(&repo);
    w.git(&repo, &["checkout", "-q", "--ignore-other-worktrees", &branch]);
    w.git(&repo, &["commit", "-q", "--allow-empty", "-m", "the person's own"]);
    let mine = w.commit(&repo, "HEAD");
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.branches(&repo), w.commit(&repo, &branch)), (vec![branch], mine), "the checkout lost its branch");
    assert_eq!((w.listed(&repo), w.home_of(&run)), (1, NONE));
}

// Another worktree of the person's stands on the branch, with no commit of its own made there.
#[test]
fn a_branch_another_worktree_of_the_repo_stands_on_is_kept() {
    let w = found_world();
    let (repo, other) = (w.repo("alpha"), w.at("work").join("other"));
    let (branch, run) = w.left_copy(&repo);
    w.git(&repo, &["worktree", "add", "-q", "--force", other.to_str().unwrap(), &branch]);
    assert_eq!(w.look(&[]), 0);
    assert_eq!(w.git(&other, &["symbolic-ref", "HEAD"]), format!("refs/heads/{branch}"));
    assert_eq!(w.branches(&repo), [branch], "a worktree lost the branch it stands on");
    assert_eq!((w.listed(&repo), w.home_of(&run)), (2, NONE));
}

// Another ref is a name for the branch: it reaches whatever the branch holds, and would be left naming nothing.
#[test]
fn a_branch_another_ref_is_a_name_for_is_kept() {
    let w = found_world();
    let repo = w.repo("alpha");
    let (branch, run) = w.left_copy(&repo);
    let mine = w.commit_of_its_own(&repo, &run);
    w.git(&repo, &["symbolic-ref", "refs/heads/also", &format!("refs/heads/{branch}")]);
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.commit(&repo, &branch), w.commit(&repo, "also")), (mine.clone(), mine), "the commit was lost");
    assert_eq!((w.listed(&repo), w.home_of(&run)), (1, NONE));
}

// The `gitdir` file in the copy's git folder must name this run's copy, and be a plain file no longer than a
// path. One that names another run's place in `copies`, one with the right path and nine thousand line breaks
// after it, a link to a file that holds the right path, and one that names a folder of the run's name in a
// folder that is there and is not the home's `copies`: the repo does not list this copy.
#[test]
fn a_gitdir_file_that_names_another_run_or_is_too_long_or_is_a_link_has_nothing_done_in_its_repo() {
    for shape in ["another run", "too long", "a link", "another folder"] {
        let w = found_world();
        let repo = w.repo("alpha");
        let (branch, run) = w.left_copy(&repo);
        let file = w.own_of(&run).join("gitdir");
        let wrote = fs::read_to_string(&file).unwrap();
        match shape {
            "another run" => fs::write(&file, wrote.replace(&run, "r00000000000000aa")).unwrap(),
            "too long" => fs::write(&file, wrote + &"\n".repeat(9000)).unwrap(),
            "another folder" => {
                let named = w.at("work").join(&run).join(".git");
                fs::write(&file, format!("{}\n", named.display())).unwrap();
            }
            _ => {
                let real = file.with_extension("real");
                fs::rename(&file, &real).unwrap();
                #[cfg(unix)]
                let linked = std::os::unix::fs::symlink(&real, &file).is_ok();
                #[cfg(windows)]
                let linked = std::os::windows::fs::symlink_file(&real, &file).is_ok();
                if !linked {
                    return skipped("no link to a file can be made here");
                }
            }
        }
        assert_eq!(w.look(&[]), 0, "{shape}");
        assert_eq!((w.branches(&repo), w.home_of(&run)), (vec![branch], NONE), "{shape}");
    }
}

// The thirty days are over for a copy whose repo is gone, and `copies` has been made a link to a folder
// elsewhere that holds a folder of the run's name: nothing is removed through the link.
#[test]
fn a_copy_given_up_after_thirty_days_is_not_removed_through_a_link_at_copies() {
    let w = found_world();
    let (repo, outside) = (w.repo("alpha"), w.at("outside"));
    let (_, run) = w.left_copy(&repo);
    fs::remove_dir_all(&repo).unwrap();
    fs::rename(w.jm("copies"), &outside).unwrap();
    if !link(&outside, &w.jm("copies")) {
        return skipped("no link to a folder can be made here");
    }
    w.noted(&run, "put_off", 1.into());
    assert_eq!(w.look(&[]), 0);
    assert!(outside.join(&run).join("a.txt").is_file(), "a folder was removed through a link");
    assert!(!w.kept_for(&run), "what the home held of the run stayed");
}

// A copy that could not be made is taken back in the order every take-back has: the branch, then the repo's
// listing of the copy. Here git never returns from making it, and the command is killed between the two steps
// (`JELLYMOON_TEST_UNMADE_WAIT_MS`). The repo still lists the copy, so the next look finishes.
#[test]
fn a_command_killed_while_it_takes_back_a_copy_it_could_not_make_leaves_no_branch_for_good() {
    let w = found_world();
    let (repo, wish, mut vars) = (w.repo("alpha"), w.wish("alpha"), w.fake_git());
    let (go, there) = (w.at("rec").join("claimed"), w.at("rec").join("claimed.at"));
    vars.extend([("JELLYMOON_TEST_UNMADE_WAIT_MS", "60000".into()), ("JELLYMOON_TEST_GO", go.into())]);
    fs::write(w.at("fake").join("on"), "add").unwrap();
    let mut going = w.going(&vars, &["run", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()]);
    let from = Instant::now();
    while !there.is_file() {
        assert!(from.elapsed() < Duration::from_secs(30), "the command never came to take its copy back");
        std::thread::sleep(Duration::from_millis(10));
    }
    going.kill().unwrap();
    going.wait().unwrap();
    assert!(fs::File::open(w.at("fake").join("held")).unwrap().try_lock().is_ok(), "the made-up git still runs");
    assert_eq!((w.branches(&repo), w.listed(&repo)), (vec![], 2), "the branch goes first, the listing last");
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.branches(&repo), w.listed(&repo), w.copies()), (vec![], 1, vec![]));
    assert_eq!(fs::read_dir(w.jm("runs")).unwrap().filter(|run| run.as_ref().unwrap().path().is_dir()).count(), 1);
}
