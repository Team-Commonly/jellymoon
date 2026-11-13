//! Shapes of work the first tests of F44 left out: a link and another repo's entry that the work removed, a
//! commit whose parent is no commit, and a file moved from one name to another in a repo told to find renames.

use std::fs;

use serde_json::json;

use super::{Entry, finished, listed, printed, refused};

#[test]
fn a_link_and_another_repos_entry_that_the_work_removed_are_listed_as_what_they_were() {
    let made = finished();
    // By a record from before, which names no start: the work is the branch's last commit against its one parent.
    made.write(&made.as_before());
    let held: [Entry; 2] = [("120000", "link", b"elsewhere/secret-target"), ("160000", "sub", b"")];
    let base = made.commit_of(&held, &["main"], "a commit with a link and another repo's entry");
    let work = made.commit_of(&[], &[&base], &format!("Jellymoon run {}", made.run));
    made.git(&["update-ref", &made.branch(), &work], b"", None);
    let shown = printed(&made.show(&[]));
    let files = json!([listed(1, "link", "removed", 23, "link", false), listed(2, "sub", "removed", 0, "repo", false)]);
    assert_eq!(shown["files"], files);
    for n in ["1", "2"] {
        let said = made.show(&["--file", n]);
        assert!(refused(&said).contains("was removed by the work"), "{}", said.stderr);
        assert!(!said.stderr.contains("secret-target"));
    }
}

// git is asked for the change from the parent a commit names. A parent that is no commit is a question git
// fails at, and what it printed until then, nothing, is not taken for work that changed no file.
#[test]
fn a_commit_whose_parent_is_no_commit_is_refused_and_never_shown_as_work_of_no_files() {
    let made = finished();
    made.write(&made.as_before());
    let tree = made.git(&["rev-parse", &format!("{}^{{tree}}", made.branch())], b"", None);
    let blob = made.git(&["rev-parse", "main:a.txt"], b"", None);
    let who = "Test Setup <setup@example.test> 1700000000 +0000";
    let typed = format!("tree {tree}\nparent {blob}\nauthor {who}\ncommitter {who}\n\nJellymoon run {}\n", made.run);
    let odd = made.git(&["hash-object", "-t", "commit", "-w", "--literally", "--stdin"], typed.as_bytes(), None);
    made.git(&["update-ref", &made.branch(), &odd], b"", None);
    for more in [&[][..], &["--file", "1"]] {
        let said = made.show(more);
        assert!(refused(&said).contains("could not be read from the repo"), "{}", said.stderr);
    }
}

// A look asks git's plumbing, which finds a rename only when told to. The repo's config and the person's can
// say to find them, and a moved file is still one file removed and one made.
#[test]
fn a_moved_file_is_one_removed_and_one_made_whatever_the_config_says_of_renames() {
    let made = finished();
    made.w.git(&made.repo, &["config", "diff.renames", "copies"]);
    fs::write(made.w.at("gitconfig"), "[diff]\n\trenames = copies\n\trenameLimit = 1000\n").unwrap();
    made.rework(&[
        ("0", "a.txt", b""),
        ("100644", "moved/a.txt", b"a.txt as it was\n"),
        ("100644", "moved/copy-of-c.txt", b"c.txt as it was\n"),
    ]);
    let files = json!([
        listed(1, "a.txt", "removed", 16, "text", false),
        listed(2, "moved/a.txt", "made", 16, "text", false),
        listed(3, "moved/copy-of-c.txt", "made", 16, "text", false),
    ]);
    assert_eq!(printed(&made.show(&[]))["files"], files);
    assert_eq!(printed(&made.show(&["--file", "2"]))["text"], "a.txt as it was");
}
