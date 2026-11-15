//! The repo of a project Jellymoon makes (F65; `docs/development/project.md`, "A new project"): a folder that
//! is there and empty is made a repo with one first commit that holds no file. git is run as a run's copy runs
//! it (`git.rs`): the person's config starts no program, and no template is copied, so the repo holds no hook.
//! The commit is written with git's plumbing, as a run's own commit is (`clean.rs`), and is by whom a run's is.

use std::path::Path;

use super::Places;
use super::clean::{by, who};
use super::git::{Git, on_path};
use crate::call::private_dir;

/// The branch a made project's first commit is on, whatever the person's config names for a new repo.
const BRANCH: &str = "refs/heads/main";

/// What is said of a git that did not make the repo. git's own words name the folder, and are left out.
const FAILED: &str = "git could not make the new project's repo (what git said is not repeated, as it names the \
                      folder)";

/// Whether a `git` program is on `PATH`, as a copy finds one. Nothing is written.
pub(crate) fn git_on_path() -> bool {
    on_path().is_some()
}

/// Makes `folder` a repo on [`BRANCH`] with one commit of no file and no parent, under `message`. The error is
/// fixed words: what the folder git is started in is refused with, or [`FAILED`].
pub(crate) fn fresh_repo(home: &Path, folder: &Path, message: &str) -> Result<(), String> {
    let at = Places::of(home);
    private_dir(&at.runs).map_err(|e| format!("the folder Jellymoon keeps empty for git (runs/no-hooks): {e}"))?;
    let git = Git::find(&at.runs, true)?;
    first_commit(&git, folder, message).map_err(|_| FAILED.to_string())
}

fn first_commit(git: &Git, folder: &Path, message: &str) -> Result<(), String> {
    git.init(folder)?;
    let repo = git.open(&folder.join(".git"))?;
    repo.run(None, &[], &["symbolic-ref", "HEAD", BRANCH])?;
    let tree = repo.run(None, &[], &["write-tree"])?;
    let (name_is, address) = who(&repo)?;
    let made = repo.run(None, &by(&name_is, &address), &["commit-tree", "-m", message, &tree])?;
    // An old value of nothing is git's own check that the branch is not there yet.
    repo.run(None, &[], &["update-ref", "--no-deref", BRANCH, &made, ""]).map(|_| ())
}
