//! A take-back deletes the run's own name and never what a name points at, only while the name is where it was
//! asked about, and asks what git will still hold tomorrow (`docs/development/run.md`, "What is kept").

use std::fs;
use std::path::Path;

use super::shapes::NONE;
use super::{World, found_world};

impl World {
    /// A commit on top of `main`'s that no ref has.
    fn loose_commit(&self, repo: &Path, parent: &str) -> String {
        self.git(repo, &["commit-tree", "main^{tree}", "-p", parent, "-m", "landed"])
    }
}

// The fourth gate's finding. The run's name was made a symbolic ref to another branch: `main` checked out, a
// branch that holds its only commit, a branch checked out in another worktree. The name is no longer the run's
// branch. It stays, the branch it points at stays with its commit, and every `HEAD` still names a branch.
#[test]
fn a_run_s_name_made_a_symbolic_ref_is_kept_and_the_branch_it_points_at_is_not_deleted() {
    for target in ["main", "feature", "topic"] {
        let w = found_world();
        let (repo, other) = (w.repo("alpha"), w.at("work").join("other"));
        let (branch, run) = w.left_copy(&repo);
        if target == "feature" {
            w.git(&repo, &["checkout", "-q", "-b", target]);
            w.git(&repo, &["commit", "-q", "--allow-empty", "-m", "its only commit"]);
            w.git(&repo, &["checkout", "-q", "main"]);
        } else if target == "topic" {
            w.git(&repo, &["worktree", "add", "-q", "-b", target, other.to_str().unwrap()]);
            w.git(&other, &["commit", "-q", "--allow-empty", "-m", "its only commit"]);
        }
        let (name, at) = (format!("refs/heads/{branch}"), format!("refs/heads/{target}"));
        let was = w.commit(&repo, &at);
        w.git(&repo, &["symbolic-ref", &name, &at]);
        assert_eq!(w.look(&[]), 0, "{target}");
        assert_eq!(w.commit(&repo, &at), was, "{target}");
        assert_eq!(w.git(&repo, &["symbolic-ref", &name]), at, "{target}: the name itself");
        for checkout in [&repo, &other].into_iter().filter(|checkout| checkout.is_dir()) {
            let head = w.git(checkout, &["symbolic-ref", "HEAD"]);
            assert_eq!(w.commit(checkout, "HEAD"), w.commit(&repo, &head), "{target}: {head}");
        }
        assert_eq!(w.home_of(&run), NONE, "{target}");
    }
}

// A name made a symbolic ref to a branch that is not there names no commit. It is not the run's branch: the look
// leaves it as it is, asks nothing more about it, and does not put the take-back off for it.
#[test]
fn a_run_s_name_that_points_at_no_branch_is_left_and_the_take_back_is_not_put_off() {
    let w = found_world();
    let repo = w.repo("alpha");
    let (branch, run) = w.left_copy(&repo);
    let name = format!("refs/heads/{branch}");
    w.git(&repo, &["symbolic-ref", &name, "refs/heads/nowhere"]);
    assert_eq!(w.look(&[]), 0);
    assert_eq!(w.git(&repo, &["symbolic-ref", &name]), "refs/heads/nowhere");
    assert_eq!((w.listed(&repo), w.home_of(&run)), (1, NONE));
}

// A commit lands on the run's branch between the look's question and its delete: the made-up git moves the
// branch just before the real `update-ref` runs. git is told where the branch was, and refuses. The next look
// asks again, and keeps the branch for its commit.
#[test]
fn a_commit_that_lands_between_the_question_and_the_delete_keeps_the_branch() {
    let w = found_world();
    let (repo, vars) = (w.repo("alpha"), w.fake_git());
    let (branch, run) = w.left_copy(&repo);
    let landed = w.loose_commit(&repo, "main");
    let git = format!("--git-dir={}", repo.join(".git").display());
    fs::write(w.at("fake").join("first"), format!("{git}\nupdate-ref\nrefs/heads/{branch}\n{landed}")).unwrap();
    fs::write(w.at("fake").join("on"), "update-ref").unwrap();
    assert_eq!(w.look(&vars), 0);
    assert_eq!((w.commit(&repo, &branch), w.kept_for(&run)), (landed.clone(), true), "the delete went through");
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.commit(&repo, &branch), w.home_of(&run)), (landed, NONE));
}

// A replace ref, or a line in `info/grafts`, that gives `main`'s commit the run's own commit as a parent makes
// the commit look held, until the replace ref or the line is removed. The question is asked with both off. git
// is told its grafts are in a file that is never there, by its whole path: a name alone is read from inside the
// repo's git folder, where anyone can leave a file of that name.
#[test]
fn a_replace_ref_or_a_graft_does_not_stand_for_a_commit() {
    for way in ["replace", "graft", "no-grafts"] {
        let w = found_world();
        let repo = w.repo("alpha");
        let (branch, run) = w.left_copy(&repo);
        let mine = w.commit_of_its_own(&repo, &run);
        let main = w.commit(&repo, "main");
        if way == "replace" {
            w.git(&repo, &["replace", &main, &w.loose_commit(&repo, &mine)]);
        } else {
            let file = if way == "graft" { Path::new("info").join("grafts") } else { way.into() };
            fs::write(repo.join(".git").join(file), format!("{main} {mine}\n")).unwrap();
        }
        assert_eq!(w.look(&[]), 0, "{way}");
        assert_eq!((w.branches(&repo), w.commit(&repo, &branch)), (vec![branch], mine), "{way}");
        assert_eq!(w.home_of(&run), NONE, "{way}");
    }
}

// A question git cannot answer is not "nothing of its own": a broken ref among the branches makes `rev-list`
// fail, and the take-back is put off. Once the ref is gone the branch is kept for its commit.
#[test]
fn a_question_git_cannot_answer_puts_the_take_back_off() {
    let w = found_world();
    let repo = w.repo("alpha");
    let (branch, run) = w.left_copy(&repo);
    let mine = w.commit_of_its_own(&repo, &run);
    let broken = repo.join(".git").join("refs").join("heads").join("broken");
    fs::write(&broken, "nothing git can read\n").unwrap();
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.branches(&repo), w.kept_for(&run)), (vec![branch.clone()], true));
    fs::remove_file(&broken).unwrap();
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.commit(&repo, &branch), w.home_of(&run)), (mine, NONE));
}

// The repo's `worktrees` folder cannot be listed, though what is in it can be reached: its other checkouts
// could not be asked, and that is not "no other checkout". Only where a mode keeps someone out.
#[cfg(unix)]
#[test]
fn a_worktrees_folder_that_cannot_be_listed_puts_the_take_back_off() {
    use std::os::unix::fs::PermissionsExt;
    let w = found_world();
    let repo = w.repo("alpha");
    let (branch, run) = w.left_copy(&repo);
    let folder = repo.join(".git").join("worktrees");
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o300)).unwrap();
    let shut = fs::read_dir(&folder).is_err();
    let looked = w.look(&[]);
    let kept = (w.branches(&repo), w.kept_for(&run));
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o755)).unwrap();
    if !shut {
        return crate::support::skipped("nothing can be shut to this account here");
    }
    assert_eq!((looked, kept), (0, (vec![branch], true)));
    assert_eq!(w.look(&[]), 0);
    assert_eq!((w.branches(&repo), w.listed(&repo), w.home_of(&run)), (vec![], 1, NONE));
}

// The command's own take-back asks nobody whose the branch is, so only the way it deletes stands between a
// name made to point at `main` meanwhile and `main` itself. The harness is too old for runs here.
#[test]
fn the_command_s_own_take_back_deletes_its_name_and_not_the_branch_the_name_was_made_to_point_at() {
    let w = found_world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    w.play("old", 0);
    let (going, run) = w.held_at("JELLYMOON_TEST_MADE_WAIT_MS", &wish, &repo);
    let was = w.commit(&repo, "main");
    w.git(&repo, &["symbolic-ref", &format!("refs/heads/jellymoon/run-{run}"), "refs/heads/main"]);
    fs::write(w.at("rec").join("claimed"), "").unwrap();
    super::ended(going);
    assert_eq!((w.commit(&repo, "main"), w.branches(&repo), w.copies()), (was, vec![], vec![]));
}
