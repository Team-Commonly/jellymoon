//! F92: a finished change is kept by itself, and can be undone (ADR 0028; `docs/development/keep.md`, "Kept by
//! itself" and "Undone"). `jellymoon run` and `run undo` are run as black boxes on runs the made-up `fake-agent`
//! finished in F39's test folder (`f39/world.rs`), in repos made with the real `git` under a made-up identity and
//! no config of this machine. Every other test's world says that every change waits, as before F92
//! (`support/door/world.rs`); the worlds here say that a finished change is kept by itself, as it is outside the
//! tests. This file holds when a change is kept by itself and when it waits; `f92/undone.rs` what an undo does
//! and what it refuses, `f92/stands.rs` what a page is told of where a kept change is now, and `f92/cut.rs` an undo cut off after each of its steps and one whose checkout moved
//! just before its step.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f92/cut.rs"]
mod cut;
#[path = "f45/made.rs"]
mod kept;
#[path = "f92/stands.rs"]
mod stands;
mod support;
#[path = "f92/undone.rs"]
mod undone;
#[path = "f39/world.rs"]
mod world;

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

pub use kept::{Held, finished, said};
use serde_json::{Value, json};
use support::Ran;
pub use world::*;

/// A world where a finished change is kept by itself, and a keep that finds the lock held waits a moment only.
pub fn itself() -> World {
    let mut w = world();
    w.vars.extend([("JELLYMOON_TEST_WAITS", "0".into()), ("JELLYMOON_TEST_KEEP_WAIT_MS", "200".into())]);
    w
}

/// A wish of `date` for `alpha` whose run the made-up agent played `script` for: the wish, and what was printed.
pub fn wished(w: &World, repo: &Path, date: &str, script: &str) -> (String, Value) {
    w.play(script, 0);
    let made = printed(&w.run(&["wish", "--date", date, "--say", SAID, "--project", "alpha"]));
    let wish = made["wish"]["id"].as_str().unwrap().to_string();
    let said = printed(&w.run(&["run", "--wish", &wish, "--date", date, "--repo", repo.to_str().unwrap()]));
    assert_eq!(said["outcome"], "completed", "{said}");
    (wish, said)
}

/// One run that finished, the agent playing `script`, in a world where a finished change is kept by itself.
pub fn played(script: &str) -> (Made, Value) {
    let w = itself();
    let repo = w.repo("alpha");
    let (wish, said) = wished(&w, &repo, DAY, script);
    let run = said["run"].as_str().unwrap().to_string();
    (Made { w, repo, wish, run }, said)
}

/// A run whose change was kept by itself: the made-up agent edited `a.txt`, removed `b.txt` and made `n.txt`.
pub fn landed() -> Made {
    let (made, said) = played("edits");
    assert_eq!(said["kept_itself"], json!(true), "{said}");
    made
}

/// A run of the first wish that has done its work and has not ended yet: the person does something meanwhile.
struct Open {
    w: World,
    repo: PathBuf,
    wish: String,
    going: Going,
}

fn open() -> Open {
    let w = itself();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    w.play("worked", 0);
    let going = w.start_going(&wish, &repo);
    wait_for(&w.at("rec").join("started"));
    Open { w, repo, wish, going }
}

impl Open {
    fn git(&self, args: &[&str]) -> String {
        self.w.git(&self.repo, args)
    }

    /// Lets the run end.
    fn close(self) -> (Made, Value) {
        fs::write(self.w.at("rec").join("go"), "").unwrap();
        let said = printed(&ended(self.going));
        assert_eq!(said["outcome"], "completed", "{said}");
        let run = said["run"].as_str().unwrap().to_string();
        (Made { w: self.w, repo: self.repo, wish: self.wish, run }, said)
    }
}

impl Made {
    /// `jellymoon run undo` for the wish, with `more` arguments and `vars`.
    pub fn undo_with(&self, more: &[&str], vars: &[(&'static str, OsString)]) -> Ran {
        let asked = ["run", "undo", "--wish", &self.wish, "--date", DAY, "--repo", self.repo.to_str().unwrap()];
        self.w.run_with(vars, &[&asked[..], more].concat())
    }

    pub fn undo(&self) -> Ran {
        self.undo_with(&[], &[])
    }

    /// The day's changes that were kept by itself, as `jellymoon run waiting` prints them.
    pub fn stood(&self) -> Value {
        let asked = ["run", "waiting", "--date", DAY, "--repo", self.repo.to_str().unwrap()];
        printed(&self.w.run(&asked))["kept"].clone()
    }

    /// Writes `before` into the run's record, as no keep did.
    pub fn forge(&self, before: &Value) {
        let file = self.w.jm("runs").join(&self.run).join("run.json");
        let mut record = self.record();
        record["before"] = before.clone();
        fs::write(file, serde_json::to_string(&record).unwrap()).unwrap();
    }

    /// What the index holds, and every file of the checkout with its bytes.
    pub fn folder(&self) -> (String, Vec<(PathBuf, Vec<u8>)>) {
        (self.git(&["ls-files", "-s"]), self.w.persons(&self.repo).0)
    }
}

/// What is the person's in a repo, as the world reads it.
type Persons = (Vec<(PathBuf, Vec<u8>)>, Vec<u8>, Vec<u8>, Vec<String>);

/// Fails unless the run's change waits: nothing says it was kept, the record says neither kept nor tossed and
/// names no commit before, the run's branch is there, and what is the person's is as it was at `before`.
fn waits(made: &Made, said: &Value, before: &Persons) {
    assert_eq!(said.get("kept_itself"), None, "{said}");
    let record = made.record();
    for word in ["work", "kept", "onto", "before", "back"] {
        assert_eq!(record.get(word), None, "{record}");
    }
    assert_eq!(made.w.branches(&made.repo), [format!("jellymoon/run-{}", made.run)]);
    assert!(*before == made.w.persons(&made.repo), "a change that waits changed something of the person's");
}

#[test]
fn a_clean_run_is_kept_by_itself_and_the_runs_branch_stays() {
    let w = itself();
    let repo = w.repo("alpha");
    let start = w.commit(&repo, "main");
    let (wish, said) = wished(&w, &repo, DAY, "edits");
    assert_eq!(said["kept_itself"], json!(true), "{said}");
    let run = said["run"].as_str().unwrap().to_string();
    let made = Made { w, repo, wish, run };
    // The person's branch names the run's own commit, and the checkout has its files.
    let end = made.end();
    assert_eq!(
        (made.w.commit(&made.repo, "main"), made.git(&["symbolic-ref", "HEAD"])),
        (end.clone(), "refs/heads/main".into())
    );
    assert_eq!(made.git(&["rev-list", "main"]), format!("{end}\n{start}"));
    assert_eq!(made.status(), "");
    assert_eq!(fs::read_to_string(made.repo.join("n.txt")).unwrap(), "new from the run\n");
    assert!(!made.repo.join("b.txt").exists());
    // The run's branch is still there, and the record names the commit the branch named before.
    assert_eq!(made.w.branches(&made.repo), [format!("jellymoon/run-{}", made.run)]);
    let record = made.record();
    let words = ["work", "kept", "onto", "before"].map(|word| record[word].clone());
    assert_eq!(words, [json!("kept"), json!(end), json!("refs/heads/main"), json!(start)], "{record}");
    assert_eq!(record.get("back"), None, "{record}");

    // A world that says every change waits, as every other test's does, keeps nothing by itself.
    let made = finished();
    assert_eq!(made.w.commit(&made.repo, "main"), made.record()["start"].as_str().unwrap());
    assert_eq!(made.record().get("work"), None);
}

#[test]
fn where_the_person_moved_on_it_is_kept_by_itself_as_one_new_commit_on_top_of_theirs() {
    let open = open();
    open.git(&["config", "user.name", "Ada Example"]);
    open.git(&["config", "user.email", "ada@example.test"]);
    fs::write(open.repo.join("mine.txt"), "the person's own\n").unwrap();
    open.git(&["add", "mine.txt"]);
    open.git(&["commit", "-q", "-m", "the person's own commit"]);
    let mine = open.w.commit(&open.repo, "main");
    let (made, said) = open.close();
    assert_eq!(said["kept_itself"], json!(true), "{said}");
    let tip = made.w.commit(&made.repo, "main");
    assert_eq!(made.git(&["rev-list", "--first-parent", &format!("{mine}..main")]), tip);
    assert_eq!(made.git(&["rev-parse", "main^1"]), mine);
    let record = made.record();
    assert_eq!((&record["kept"], &record["before"]), (&json!(tip), &json!(mine)), "{record}");
    assert_eq!((made.status(), made.w.branches(&made.repo).len()), (String::new(), 1));
    assert!(made.repo.join("mine.txt").exists() && made.repo.join("n.txt").exists());
}

#[test]
fn a_change_of_a_file_the_persons_tools_obey_or_of_a_gitattributes_waits() {
    for script in ["rules", "attrs"] {
        let w = itself();
        let repo = w.repo("alpha");
        let before = w.persons(&repo);
        let (wish, said) = wished(&w, &repo, DAY, script);
        let run = said["run"].as_str().unwrap().to_string();
        waits(&Made { w, repo, wish, run }, &said, &before);
    }
}

#[test]
fn it_waits_where_a_keep_would_be_refused_and_the_folder_is_untouched() {
    type Meanwhile = fn(&Open);
    let cases: [(&str, Meanwhile); 4] = [
        ("an untracked file in the way", |open| fs::write(open.repo.join("n.txt"), "the person's own\n").unwrap()),
        ("an unsaved change in a file of the change", |open| {
            fs::write(open.repo.join("a.txt"), "half an edit\n").unwrap();
        }),
        ("a detached HEAD", |open| drop(open.git(&["checkout", "-q", "--detach"]))),
        ("history rewritten", |open| {
            drop(open.git(&["commit", "-q", "--amend", "-m", "the first commit, said again"]))
        }),
    ];
    for (case, meanwhile) in cases {
        let open = open();
        meanwhile(&open);
        let before = open.w.persons(&open.repo);
        let (made, said) = open.close();
        waits(&made, &said, &before);
        println!("waits: {case}");
    }
}

/// Why a change was not kept by itself is written with its run's record, as core's name of the reason and no
/// sentence (ADR 0030, decision 10); a change that landed, and one no keep was tried for, have none.
#[test]
fn why_a_change_was_not_kept_by_itself_is_in_its_record_and_a_change_that_landed_has_none() {
    type Meanwhile = fn(&Open);
    let cases: [(&str, Meanwhile); 3] = [
        // The person committed another first line of `a.txt` while the run worked: the two do not fit.
        ("clash", |open| {
            fs::write(open.repo.join("a.txt"), "the person's own first line\n").unwrap();
            let who = ["-c", "user.name=Ada Example", "-c", "user.email=ada@example.test"];
            open.git(&[&who[..], &["commit", "-q", "-am", "the person's own commit"][..]].concat());
        }),
        ("in_way", |open| fs::write(open.repo.join("a.txt"), "half an edit\n").unwrap()),
        ("no_branch", |open| drop(open.git(&["checkout", "-q", "--detach"]))),
    ];
    for (reason, meanwhile) in cases {
        let open = open();
        meanwhile(&open);
        let before = open.w.persons(&open.repo);
        let (made, said) = open.close();
        waits(&made, &said, &before);
        assert_eq!(made.record()["unkept"], json!(reason), "{}", made.record());
    }
    assert_eq!(landed().record().get("unkept"), None);
    // In a world where every change waits no keep is tried, so there is no reason to write.
    assert_eq!(finished().record().get("unkept"), None);
}

#[test]
fn it_waits_while_the_lock_is_busy_and_where_the_folder_is_not_found() {
    // Another keep, toss or undo is at work in this home.
    let open = open();
    let before = open.w.persons(&open.repo);
    fs::create_dir_all(open.w.jm("runs")).unwrap();
    let lock = fs::File::create(open.w.jm("runs").join("keep.lock")).unwrap();
    lock.lock().unwrap();
    let (made, said) = open.close();
    drop(lock);
    waits(&made, &said, &before);

    // The run was started in a second checkout, and that folder is gone from its place when the run ends.
    let w = itself();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (feat, away) = (w.at("work").join("linked").join("alpha"), w.at("work").join("linked").join("moved"));
    w.git(&repo, &["worktree", "add", "-q", "-b", "feat", feat.to_str().unwrap()]);
    let before = (w.persons(&repo), support::tree(&feat));
    w.play("worked", 0);
    let going = w.start_going(&wish, &feat);
    wait_for(&w.at("rec").join("started"));
    fs::rename(&feat, &away).unwrap();
    fs::write(w.at("rec").join("go"), "").unwrap();
    let said = printed(&ended(going));
    fs::rename(&away, &feat).unwrap();
    assert_eq!((said.get("kept_itself"), &said["outcome"]), (None, &json!("completed")), "{said}");
    assert!(before == (w.persons(&repo), support::tree(&feat)), "a change that waits changed something");
    assert_eq!(w.commit(&repo, "feat"), w.commit(&repo, "main"));
}

#[test]
fn a_follow_up_whose_followed_work_waits_waits_too() {
    // The first run's work touches a file the person's tools obey, so it waits.
    let (made, first) = played("rules");
    assert_eq!(first.get("kept_itself"), None, "{first}");
    let before = made.w.persons(&made.repo);
    made.w.play("follow", 0);
    let follow = printed(&made.w.run(&["wish", "--date", DAY, "--say", "and once more", "--after", &made.wish]));
    let wish = follow["wish"]["id"].as_str().unwrap();
    let said = printed(&made.w.run(&["run", "--wish", wish, "--date", DAY, "--repo", made.repo.to_str().unwrap()]));
    assert_eq!((said.get("kept_itself"), &said["outcome"]), (None, &json!("completed")), "{said}");
    assert!(before == made.w.persons(&made.repo), "a follow-up that waits changed something of the person's");
    assert_eq!(made.w.record(said["run"].as_str().unwrap()).get("work"), None);
    assert_eq!(made.w.branches(&made.repo).len(), 2);
}
