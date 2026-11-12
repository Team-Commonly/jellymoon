//! Whose a left copy's branch is when its copy is taken back (`docs/development/run.md`, "What is kept"): a
//! checkout detached at its commit, a rebase or a bisect stopped on it, a tag that holds its commit, and a
//! checkout that cannot be asked. Which refs hold a commit is `held.rs`.

use std::fs;
use std::path::{Path, PathBuf};

use super::asked::shut;
use super::named::link;
use super::shapes::NONE;
use super::{World, ended, found_world};
use crate::support::skipped;

impl World {
    /// The person's checkout on the run's branch, with a commit to `a.txt` there that `backup` holds too, and a
    /// commit to `a.txt` on `main` that it cannot be put on top of. Returns the commit on the run's branch.
    fn two_sides(&self, repo: &Path, on: &Path, branch: &str) -> String {
        fs::write(repo.join("a.txt"), "main's\n").unwrap();
        self.git(repo, &["commit", "-q", "-a", "-m", "main's"]);
        if on == repo {
            self.git(repo, &["checkout", "-q", "--ignore-other-worktrees", branch]);
        } else {
            self.git(repo, &["worktree", "add", "-q", "--force", on.to_str().unwrap(), branch]);
        }
        fs::write(on.join("a.txt"), "the person's\n").unwrap();
        self.git(on, &["commit", "-q", "-a", "-m", "the person's"]);
        self.git(on, &["branch", "backup"]);
        self.commit(on, "HEAD")
    }

    /// Whether git says `HEAD` is detached in a checkout.
    fn detached(&self, checkout: &Path) -> bool {
        !self.git_ran(checkout, &["symbolic-ref", "--quiet", "HEAD"]).status.success()
    }

    pub(super) fn holders(&self, repo: &Path, commit: &str) -> String {
        self.git(repo, &["for-each-ref", "--contains", commit, "--format=%(refname:short)"])
    }
}

// Scene A of PR 148's second gate. A detached `HEAD` is no ref: a commit only the run's branch holds is the
// branch's own, whatever stands detached at it.
#[test]
fn a_branch_whose_own_commit_a_checkout_is_detached_at_is_kept() {
    let w = found_world();
    let repo = w.repo("alpha");
    let (branch, run) = w.left_copy(&repo);
    let mine = w.commit_of_its_own(&repo, &run);
    w.git(&repo, &["checkout", "-q", "--detach", &mine]);
    assert_eq!(w.look(&[]), 0);
    assert_eq!(w.holders(&repo, &mine), branch, "the commit was left with no ref");
    assert_eq!((w.listed(&repo), w.home_of(&run)), (1, NONE));
}

// Scene B. A rebase of the run's branch stopped in the person's checkout: `HEAD` is detached, and git keeps
// the branch's name in `rebase-apply/head-name`. Another branch holds the commit, so nothing but that name
// says the branch is stood on.
#[test]
fn a_branch_a_rebase_stopped_in_the_person_s_checkout_names_is_kept() {
    let w = found_world();
    let repo = w.repo("alpha");
    let (branch, run) = w.left_copy(&repo);
    w.two_sides(&repo, &repo, &branch);
    assert!(!w.git_ran(&repo, &["rebase", "-q", "--apply", "main"]).status.success(), "the rebase did not stop");
    assert!(w.detached(&repo));
    assert_eq!(w.look(&[]), 0);
    assert_eq!(w.branches(&repo), vec![branch.clone()], "a branch was removed in the middle of its rebase");
    fs::write(repo.join("a.txt"), "both\n").unwrap();
    w.git(&repo, &["add", "a.txt"]);
    w.git(&repo, &["rebase", "--continue"]);
    assert_eq!(w.git(&repo, &["symbolic-ref", "HEAD"]), format!("refs/heads/{branch}"));
    assert_eq!((w.listed(&repo), w.home_of(&run)), (1, NONE));
}

// The same in another worktree of the repo, by the rebase that keeps the name in `rebase-merge/head-name`.
#[test]
fn a_branch_a_rebase_stopped_in_another_worktree_names_is_kept() {
    let w = found_world();
    let (repo, other) = (w.repo("alpha"), w.at("work").join("other"));
    let (branch, run) = w.left_copy(&repo);
    w.two_sides(&repo, &other, &branch);
    assert!(!w.git_ran(&other, &["rebase", "-q", "--merge", "main"]).status.success(), "the rebase did not stop");
    assert!(w.detached(&other));
    assert_eq!(w.look(&[]), 0);
    assert_eq!(w.branches(&repo), vec![branch.clone()], "a branch was removed in the middle of its rebase");
    w.git(&other, &["rebase", "--abort"]);
    assert_eq!(w.git(&other, &["symbolic-ref", "HEAD"]), format!("refs/heads/{branch}"));
    assert_eq!((w.listed(&repo), w.home_of(&run)), (2, NONE));
}

// A bisect started on the run's branch: `HEAD` is detached at a commit between, and git keeps the branch's
// name in `BISECT_START`, to go back to.
#[test]
fn a_branch_a_bisect_was_started_on_is_kept() {
    let w = found_world();
    let repo = w.repo("alpha");
    for count in ["second", "third"] {
        w.git(&repo, &["commit", "-q", "--allow-empty", "-m", count]);
    }
    let (branch, run) = w.left_copy(&repo);
    w.git(&repo, &["checkout", "-q", "--ignore-other-worktrees", &branch]);
    w.git(&repo, &["bisect", "start", "HEAD", "HEAD~2"]);
    assert!(w.detached(&repo));
    assert_eq!(w.look(&[]), 0);
    assert_eq!(w.branches(&repo), vec![branch.clone()], "a branch was removed in the middle of a bisect");
    w.git(&repo, &["bisect", "reset"]);
    assert_eq!(w.git(&repo, &["symbolic-ref", "HEAD"]), format!("refs/heads/{branch}"));
    assert_eq!((w.listed(&repo), w.home_of(&run)), (1, NONE));
}

// A tag holds the commit, so the branch has nothing of its own, and goes.
#[test]
fn a_branch_whose_commit_a_tag_holds_goes_and_the_tag_keeps_the_commit() {
    let w = found_world();
    let repo = w.repo("alpha");
    let (_, run) = w.left_copy(&repo);
    let mine = w.commit_of_its_own(&repo, &run);
    w.git(&repo, &["tag", "held", &mine]);
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.branches(&repo), w.holders(&repo, &mine)), (vec![], "held".to_string()));
    assert_eq!((w.listed(&repo), w.home_of(&run)), (1, NONE));
}

// A worktree of the repo whose `HEAD` cannot be read may stand on the branch: nothing is removed in the repo,
// and the record waits for a look that can ask.
#[test]
fn a_worktree_that_cannot_be_asked_keeps_the_branch_and_the_record() {
    let w = found_world();
    let (repo, other) = (w.repo("alpha"), w.at("work").join("other"));
    let (branch, run) = w.left_copy(&repo);
    w.git(&repo, &["worktree", "add", "-q", "-b", "side", other.to_str().unwrap()]);
    let head: PathBuf = repo.join(".git").join("worktrees").join("other").join("HEAD");
    fs::write(&head, "nothing git can read\n").unwrap();
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.branches(&repo), w.kept_for(&run)), (vec![branch], true));
    fs::write(&head, "ref: refs/heads/side\n").unwrap();
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.branches(&repo), w.home_of(&run)), (vec![], NONE), "the look that could ask did not finish");
}

// The command that made a copy takes it back without asking whose the branch is: nothing but that command has
// had the copy. Here the harness is too old for runs, and a commit was made in the copy meanwhile.
#[test]
fn the_command_s_own_take_back_removes_the_branch_without_asking() {
    let w = found_world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    w.play("old", 0);
    let (going, run) = w.held_at("JELLYMOON_TEST_MADE_WAIT_MS", &wish, &repo);
    w.commit_of_its_own(&repo, &run);
    fs::write(w.at("rec").join("claimed"), "").unwrap();
    ended(going);
    assert_eq!((w.branches(&repo), w.listed(&repo), w.copies()), (vec![], 1, vec![]));
}

// A path in a record that does not start at a root would be read from wherever the look was started, and by
// git from another folder. Nothing is done for such a record, in the repo or in the home.
#[test]
fn a_record_whose_repo_or_own_is_not_a_full_path_has_nothing_done_for_it() {
    for field in ["repo", "own"] {
        let w = found_world();
        let repo = w.repo("alpha");
        let (branch, run) = w.left_copy(&repo);
        // From the folder every command of a world is started in.
        let git = Path::new("work").join("alpha").join(".git");
        let short = if field == "repo" { git } else { git.join("worktrees").join(&run) };
        assert!(w.dir.path().join(&short).is_dir());
        w.noted(&run, field, short.to_str().unwrap().into());
        assert_eq!(w.look(&[]), 0);
        assert_eq!((w.branches(&repo), w.listed(&repo)), (vec![branch], 2), "{field}");
        assert_eq!(w.home_of(&run), (vec![run.clone()], true), "{field}");
    }
}

// What git keeps of a rebase or a bisect is read only as a plain file no longer than a path, in a real folder.
// Nine thousand line breaks after the branch's name, a link to a file elsewhere that holds it, and a link to a
// folder elsewhere where git keeps a rebase's folder: that checkout could not be asked, so nothing is removed
// in the repo and the record waits.
#[test]
fn something_else_where_git_keeps_a_branch_stood_on_puts_the_take_back_off() {
    for shape in ["too long", "a link", "a linked folder"] {
        let w = found_world();
        let repo = w.repo("alpha");
        let (branch, run) = w.left_copy(&repo);
        let (file, real) = (repo.join(".git").join("BISECT_START"), w.at("work").join("elsewhere"));
        let made = match shape {
            "too long" => fs::write(&file, format!("{branch}{}", "\n".repeat(9000))).is_ok(),
            "a link" => {
                fs::write(&real, format!("{branch}\n")).unwrap();
                #[cfg(unix)]
                let linked = std::os::unix::fs::symlink(&real, &file).is_ok();
                #[cfg(windows)]
                let linked = std::os::windows::fs::symlink_file(&real, &file).is_ok();
                linked
            }
            _ => {
                fs::create_dir(&real).unwrap();
                fs::write(real.join("head-name"), format!("refs/heads/{branch}\n")).unwrap();
                link(&real, &repo.join(".git").join("rebase-merge"))
            }
        };
        if !made {
            return skipped("no link can be made here");
        }
        assert_eq!(w.look(&[]), 0, "{shape}");
        assert_eq!((w.branches(&repo), w.kept_for(&run)), (vec![branch], true), "{shape}");
    }
}

// A file git keeps a branch's name in that cannot be read, or whose folder cannot, is not taken as not there:
// the checkout could not be asked. Opened again, it names the branch, which is kept as stood on.
#[test]
fn a_file_where_git_keeps_a_branch_stood_on_that_cannot_be_read_puts_the_take_back_off() {
    for shape in ["the file", "its folder"] {
        let w = found_world();
        let repo = w.repo("alpha");
        let (branch, run) = w.left_copy(&repo);
        let closed = repo.join(".git").join(if shape == "the file" { "BISECT_START" } else { "rebase-merge" });
        if shape == "the file" {
            fs::write(&closed, format!("{branch}\n")).unwrap();
        } else {
            fs::create_dir(&closed).unwrap();
            fs::write(closed.join("head-name"), format!("refs/heads/{branch}\n")).unwrap();
        }
        if !shut(&closed, true) {
            shut(&closed, false);
            return skipped("nothing can be shut to this account here");
        }
        assert_eq!(w.look(&[]), 0, "{shape}");
        let kept = (w.branches(&repo), w.kept_for(&run));
        shut(&closed, false);
        assert_eq!(kept, (vec![branch.clone()], true), "{shape}");
        assert_eq!(w.look(&[]), 0, "{shape}");
        assert_eq!((w.branches(&repo), w.home_of(&run)), (vec![branch], NONE), "{shape}");
    }
}
