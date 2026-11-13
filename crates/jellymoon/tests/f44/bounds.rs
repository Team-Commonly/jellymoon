//! Where a look's reads stop (`docs/development/show.md`, "What is read, and how far"): a commit longer than is
//! read, an answer about the branch longer than a line, a text that cleaning makes longer than is kept, a cut
//! with no whole word before it, a home that git cannot be started in, and one file read without the others.

use std::fs;

use serde_json::{Value, json};

use super::{DAY, Made, WORK, finished, printed, refused};

/// Stores a commit typed by hand, with `pad` bytes of a header no git writes and `after` below the run's message.
fn typed_commit(made: &Made, parent: &str, pad: usize, after: &str) -> (String, usize) {
    let tree = made.git(&["rev-parse", &format!("{}^{{tree}}", made.branch())], b"", None);
    let who = "Test Setup <setup@example.test> 1700000000 +0000";
    let pad = "p".repeat(pad);
    let head = format!("tree {tree}\nparent {parent}\nauthor {who}\ncommitter {who}\npad {pad}\n\n");
    let text = format!("{head}Jellymoon run {}\n", made.run);
    let id = made.git(&["hash-object", "-t", "commit", "-w", "--stdin"], format!("{text}{after}").as_bytes(), None);
    (id, text.len())
}

/// Puts a commit under the run's branch and in its record, as the run's end.
fn ended_as(made: &Made, commit: &str) {
    made.git(&["update-ref", &made.branch(), commit], b"", None);
    let mut record = made.record();
    record["end"] = json!(commit);
    made.write(&record);
}

// What is read of a commit stops at 65,536 bytes. A commit whose first 65,536 bytes end just where a run's
// message ends was a run's by all that was read, with anything at all below.
#[test]
fn a_commit_longer_than_is_read_is_not_judged_by_what_was_read_of_it() {
    let made = finished();
    let (main, shown) = (made.w.commit(&made.repo, "main"), printed(&made.show(&[])));
    let (_, bare) = typed_commit(&made, &main, 0, "");
    let (long, read) = typed_commit(&made, &main, 65536 - bare, &format!("and more: {WORK}\n"));
    assert_eq!(read, 65536, "the run's message ends with the last byte that is read");
    ended_as(&made, &long);
    for more in [&[][..], &["--file", "1"]] {
        assert!(refused(&made.show(more)).contains("is not as the run left it"));
    }
    made.write(&made.as_before());
    assert!(refused(&made.show(&[])).contains("is not as the run left it"), "by its shape alone, too");
    // One byte shorter it was read whole, and is a run's commit with a header more: it is shown.
    let (whole, read) = typed_commit(&made, &main, 65535 - bare, "");
    assert_eq!(read, 65535);
    made.git(&["update-ref", &made.branch(), &whole], b"", None);
    assert_eq!(printed(&made.show(&[]))["files"], shown["files"]);
}

/// `jellymoon run show` with the made-up git first on `PATH`, which does `first` just before the step `on`.
fn shown_through(made: &Made, on: &str, first: Option<&[&str]>, more: &[&str]) -> crate::support::Ran {
    let (vars, fake) = (made.w.fake_git(), made.w.at("fake"));
    for left in ["first", "returns"] {
        let _ = fs::remove_file(fake.join(left));
    }
    if let Some(first) = first {
        let git = format!("--git-dir={}", made.repo.join(".git").display());
        fs::write(fake.join("first"), format!("{git}\n{}", first.join("\n"))).unwrap();
        fs::write(fake.join("returns"), "").unwrap();
    }
    fs::write(fake.join("on"), on).unwrap();
    let asked = ["run", "show", "--wish", &made.wish, "--date", DAY, "--repo", made.repo.to_str().unwrap()];
    made.w.run_with(&vars, &[&asked[..], more].concat())
}

// The answer about the run's branch is a line. One that goes on for longer than is read is no answer, though
// the branch's own line stands somewhere in it.
#[test]
fn an_answer_about_the_branch_that_is_longer_than_is_read_is_no_answer() {
    let made = finished();
    let shown = printed(&made.show(&[]));
    let flood = "a line git printed before its answer\n".repeat(1000).into_bytes();
    let flood = made.git(&["hash-object", "-w", "--stdin"], &flood, None);
    let said = shown_through(&made, "for-each-ref", Some(&["cat-file", "blob", &flood]), &[]);
    assert!(refused(&said).contains("could not be read from the repo"), "{}", said.stderr);
    // With nothing printed before it the same made-up git's answer is taken.
    assert_eq!(printed(&shown_through(&made, "for-each-ref", Some(&["version"]), &["--file", "3"]))["n"], 3);
    assert_eq!(printed(&shown_through(&made, "no step", None, &[])), shown);
}

/// The text of the work's one file, as `--file 1` prints it.
fn text_of(made: &Made, holds: &str) -> Value {
    made.rework(&[("100644", "notes.txt", holds.as_bytes())]);
    printed(&made.show(&["--file", "1"]))
}

// Cleaning can make a text longer: a token gains an ellipsis, a password becomes the mark. A file of 65,536
// bytes or fewer is read whole, and what is printed of it can still be less than all of it.
#[test]
fn a_text_that_cleaning_makes_longer_than_is_kept_is_cut_and_says_so() {
    let made = finished();
    let token = format!("cm{} ", "_");
    let link = format!("x://{}@h ", ":a");
    for (holds, starts) in [(token.repeat(16384), "cm_… cm_… "), (link.repeat(7281), "x://:[redacted]@h x://")] {
        assert!(holds.len() <= 65536, "all of it is read");
        let file = text_of(&made, &holds);
        let text = file["text"].as_str().unwrap();
        assert_eq!((&file["cut"], &file["size"], &file["most"]), (&json!(true), &json!(holds.len()), &json!(65536)));
        assert!(text.starts_with(starts) && text.ends_with('…'), "{:?}", text.chars().take(40).collect::<String>());
        assert_eq!(text.chars().count(), 65536, "no more characters are kept than bytes are read");
    }
    // A text that cleaning makes longer, and no longer than is kept, is all there.
    let file = text_of(&made, &token.repeat(3));
    assert_eq!((&file["cut"], &file["text"]), (&json!(false), &json!("cm_… cm_… cm_…")));
}

// A cut text is shown up to its last whole word. With no break in what was read there is none.
#[test]
fn a_text_over_the_stated_size_with_no_whole_word_in_what_is_read_is_refused() {
    let made = finished();
    made.rework(&[
        ("100644", "long.txt", "a".repeat(65537).as_bytes()),
        ("100644", "space.txt", format!("{}{WORK}", " ".repeat(65536)).as_bytes()),
        ("100644", "whole.txt", "a".repeat(65536).as_bytes()),
    ]);
    for n in ["1", "2"] {
        let said = made.show(&["--file", n]);
        let line = format!(
            "jellymoon: file {n} of run {} has no whole word in what is read of it, so it is not shown\n",
            made.run
        );
        assert_eq!(refused(&said), line);
    }
    let whole = printed(&made.show(&["--file", "3"]));
    assert_eq!((&whole["cut"], whole["text"].as_str().unwrap().len()), (&json!(false), 65536));
}

// The folder git is started in, and told to look for hooks in, is Jellymoon's own and must be empty. A fault
// there, or no git at all, is not said of the person's repo.
#[test]
fn a_home_that_git_cannot_be_started_in_and_a_machine_with_no_git_have_a_line_of_their_own() {
    let made = finished();
    let shown = printed(&made.show(&[]));
    let line = format!(
        "jellymoon: the work of run {} could not be read: no `git` program was found on PATH, or the folder \
         Jellymoon keeps empty for git (runs/no-hooks) is not an empty folder\n",
        made.run
    );
    let planted = made.w.jm("runs").join("no-hooks").join("reference-transaction");
    fs::write(&planted, "a hook somebody left").unwrap();
    for more in [&[][..], &["--file", "1"]] {
        assert_eq!(refused(&made.show(more)), line);
    }
    fs::remove_file(&planted).unwrap();
    let asked = ["run", "show", "--wish", &made.wish, "--date", DAY, "--repo", made.repo.to_str().unwrap()];
    let no_git = made.w.run_with(&[("PATH", made.w.at("bin").into())], &asked);
    assert_eq!(refused(&no_git), line);
    assert_eq!(printed(&made.show(&[])), shown);
}

// One file is printed with no read of any other file's bytes: a file git cannot be made to give does not keep
// the others from being shown. The list says of every file whether it is text, so it reads them all.
#[test]
fn one_file_is_printed_without_reading_any_other_files_bytes() {
    let made = finished();
    let (one, two) = ("one ".repeat(20000), "two ".repeat(20000));
    made.rework(&[("100644", "big-1.txt", one.as_bytes()), ("100644", "big-2.txt", two.as_bytes())]);
    let second = made.git(&["rev-parse", &format!("{}:big-2.txt", made.branch())], b"", None);
    // The made-up git never returns from the read of the second file.
    let file = printed(&shown_through(&made, &second, None, &["--file", "1"]));
    assert_eq!((&file["name"], &file["cut"], &file["size"]), (&json!("big-1.txt"), &json!(true), &json!(80000)));
    assert!(!made.w.at("fake").join("stalled").exists(), "the second file was not asked for");
    let said = shown_through(&made, &second, None, &[]);
    assert!(refused(&said).contains("could not be read from the repo"), "{}", said.stderr);
    assert!(made.w.at("fake").join("stalled").exists(), "the list asked for it");
}
