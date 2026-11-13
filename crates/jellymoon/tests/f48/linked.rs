//! A repo with a second checkout, a linked worktree (ADR 0017, "Found while building F44"). A run's record names
//! the git folder the checkouts share and never a checkout's own, so it is one repo to a look from either
//! checkout, and it does not say which of them the run started from: that is `start`'s to say.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;

use super::shown::names;
use super::{Made, World, finished, one_folder, printed, ran, world};

const WORK: [&str; 3] = ["a.txt", "b.txt", "n.txt"];

/// A second checkout of the repo at `work/linked/alpha`, on a branch of its own that is one commit past `main`.
fn linked(w: &World, repo: &Path) -> PathBuf {
    let feat = w.at("work").join("linked").join("alpha");
    w.git(repo, &["worktree", "add", "-q", "-b", "feat", feat.to_str().unwrap()]);
    fs::write(feat.join("f.txt"), "only on feat\n").unwrap();
    w.git(&feat, &["add", "f.txt"]);
    w.git(&feat, &["commit", "-q", "-m", "a commit on feat"]);
    feat
}

/// The record names the folder both checkouts share, and not the linked checkout's own git folder inside it.
fn names_the_shared_folder(made: &Made, feat: &Path) {
    let record = made.record();
    let (named, shared) = (record["repo"].as_str().unwrap(), made.repo.join(".git"));
    assert!(one_folder(named, &shared), "{record}");
    let own = made.w.git(feat, &["rev-parse", "--path-format=absolute", "--git-dir"]);
    let inside = fs::canonicalize(shared.join("worktrees")).unwrap();
    assert!(fs::canonicalize(&own).unwrap().starts_with(inside), "{own}");
    assert!(!one_folder(named, Path::new(&own)), "{record}");
}

#[test]
fn a_run_started_from_a_linked_worktree_names_the_shared_git_folder_and_is_shown_from_the_main_checkout_too() {
    let w = world();
    let repo = w.repo("alpha");
    let (feat, wish) = (linked(&w, &repo), w.wish("alpha"));
    let (run, said) = ran(&w.start(&wish, &feat));
    assert_eq!(said["outcome"], "completed", "{said}");
    let made = Made { w, repo, wish, run };
    names_the_shared_folder(&made, &feat);
    // It started from the commit the linked checkout was on, which the main checkout never was.
    let (record, on_feat) = (made.record(), made.w.commit(&feat, "feat"));
    assert_ne!(on_feat, made.w.commit(&made.repo, "main"));
    assert_eq!(record["start"], json!(on_feat), "{record}");
    assert_eq!(made.git(&["rev-list", "--parents", "-n", "1", &made.branch()]).split(' ').nth(1), Some(&*on_feat));
    for checkout in [&feat, &made.repo] {
        assert_eq!(names(&printed(&made.show_in(checkout, &[]))), WORK, "from {}", checkout.display());
    }
}

#[test]
fn a_run_started_from_the_main_checkout_is_shown_from_a_linked_worktree_too() {
    let made = finished();
    let feat = linked(&made.w, &made.repo);
    names_the_shared_folder(&made, &feat);
    assert_eq!(made.record()["start"], json!(made.w.commit(&made.repo, "main")));
    for checkout in [&feat, &made.repo] {
        assert_eq!(names(&printed(&made.show_in(checkout, &[]))), WORK, "from {}", checkout.display());
    }
}
