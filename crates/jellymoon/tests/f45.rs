//! F45: keeping a run's work lands it on the branch the person stands on, as one commit by them (ADR 0017,
//! decisions 4 to 6 and 8; `docs/development/keep.md`). `jellymoon run keep` is run as a black box on runs the
//! made-up `fake-agent` finished in F39's test folder (`f39/world.rs`), in repos made with the real `git` under a
//! made-up identity and no config of this machine. This file holds what a keep does; `f45/refused.rs` what is
//! refused with nothing changed, `f45/record.rs` the records and branches a keep does not go by, `f45/settings.rs`
//! the files the person's tools obey, other repos' entries and what a repo's config could start, `f45/cut.rs` a
//! keep cut off after each of its steps, `f45/disk.rs` what a disk does with the work's names, `f45/linked.rs` a
//! repo with a second checkout, `f45/started.rs` what a keep never starts beside what git does start,
//! `f45/raced.rs` a checkout or a branch changed between two steps, `f45/names.rs` the branch names a record
//! holds, `f45/late.rs` a step git stops, a file that only looks changed and what a question writes, and
//! `f45/attrs.rs` which attributes git goes by when the work removes, changes or adds a `.gitattributes`.
//! `f45/made.rs` is what these tests share with a toss's (F46).
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f45/attrs.rs"]
mod attrs;
#[path = "f45/cut.rs"]
mod cut;
#[path = "f45/disk.rs"]
mod disk;
#[path = "f45/made.rs"]
mod kept;
#[path = "f45/late.rs"]
mod late;
#[path = "f45/linked.rs"]
mod linked;
#[path = "f45/names.rs"]
mod names;
#[path = "f45/raced.rs"]
mod raced;
#[path = "f45/record.rs"]
mod record;
#[path = "f45/refused.rs"]
mod refused;
#[path = "f45/settings.rs"]
mod settings;
#[path = "f45/started.rs"]
mod started;
mod support;
#[path = "f39/world.rs"]
mod world;

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

pub use kept::{Held, finished, said};
use serde_json::json;
use support::{Ran, tree};
pub use world::*;

/// One file of made-up work: its mode as git writes it (`0` takes the file away), its path and what it holds.
pub type Entry<'a> = (&'a str, &'a str, &'a [u8]);

impl Made {
    /// The real git in the repo with `input` on its input and an index file of the test's own.
    fn stored(&self, args: &[&str], input: &[u8], index: &Path) -> String {
        let mut git = Command::new("git");
        git.args(args).current_dir(&self.repo).env_clear().envs(self.w.vars.iter().map(|(name, value)| (name, value)));
        git.env("GIT_CEILING_DIRECTORIES", self.w.dir.path()).env("GIT_INDEX_FILE", index);
        for who in ["AUTHOR", "COMMITTER"] {
            git.env(format!("GIT_{who}_NAME"), "Test Setup").env(format!("GIT_{who}_EMAIL"), "setup@example.test");
        }
        git.envs(std::env::var_os("SystemRoot").map(|root| ("SystemRoot", root)));
        let mut git = git.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        git.stdin.take().unwrap().write_all(input).unwrap();
        let ran = git.wait_with_output().unwrap();
        assert!(ran.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&ran.stderr));
        String::from_utf8(ran.stdout).unwrap().trim_end().to_string()
    }

    /// Puts made-up work where the run's own was: one commit of `entries` over the run's start, with the message
    /// core gives a run's, made in the repo's store only, which the run's record is made to name as its end.
    pub fn rework(&self, entries: &[Entry]) -> String {
        let (index, start) =
            (self.w.at("tmp").join("made-up-index"), self.record()["start"].as_str().unwrap().to_string());
        let _ = fs::remove_file(&index);
        self.stored(&["read-tree", &start], b"", &index);
        let (mut info, mut blobs) = (Vec::new(), HashMap::new());
        for (mode, path, holds) in entries {
            let store = || self.stored(&["hash-object", "-w", "--stdin"], holds, &index);
            let blob = match *mode {
                "0" => "0".repeat(40),
                "160000" => start.clone(),
                _ => blobs.entry(*holds).or_insert_with(store).clone(),
            };
            info.extend(format!("{mode} {blob}\t{path}\0").into_bytes());
        }
        self.stored(&["update-index", "-z", "--index-info"], &info, &index);
        let tree = self.stored(&["write-tree"], b"", &index);
        let made = self.git(&["commit-tree", "-p", &start, "-m", &format!("Jellymoon run {}", self.run), &tree]);
        self.git(&["update-ref", &self.branch(), &made]);
        let mut record = self.record();
        record["end"] = json!(made);
        self.write(&record);
        made
    }

    /// The files of the checkout as text, by name.
    pub fn files(&self) -> Vec<(String, String)> {
        let git = |path: &Path| path.components().any(|part| part.as_os_str() == ".git");
        let files = tree(&self.repo).into_iter().filter(|(path, _)| !git(path) && !path.is_dir());
        let named = |path: &Path| path.strip_prefix(&self.repo).unwrap().to_string_lossy().replace('\\', "/");
        files.map(|(path, bytes)| (named(&path), String::from_utf8(bytes).unwrap())).collect()
    }
}

/// The files the made-up agent's work leaves in a checkout that was at the run's start.
pub fn worked() -> Vec<(String, String)> {
    let files = [("a.txt", "edited by the run\n"), ("c.txt", "c.txt as it was\n"), ("n.txt", "new from the run\n")];
    files.iter().map(|(name, holds)| (name.to_string(), holds.to_string())).collect()
}

/// A program planted where git would start it, which leaves `ran-<name>` beside itself if it ever is.
pub fn planted(folder: &Path, name: &str) -> std::path::PathBuf {
    let program = folder.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    support::plant(env!("CARGO_BIN_EXE_fake-git"), &program);
    program
}

/// A planted program as a config names one that git hands to a shell (a filter, a merge driver): in quotes, and
/// with the one slash a shell keeps on every system. A Windows path as it is written starts nothing there.
pub fn shell(program: &Path) -> String {
    format!("'{}'", program.to_str().unwrap().replace('\\', "/"))
}

/// The planted programs that were started, by the files they left.
pub fn started(folder: &Path) -> Vec<String> {
    let left = fs::read_dir(folder).unwrap().map(|entry| entry.unwrap().file_name().into_string().unwrap());
    left.filter(|name| name.starts_with("ran-")).collect()
}

/// Makes a file look changed to git, with what it holds left as it is: its time is set back twenty years.
pub fn touch(file: &Path) {
    touch_back(file, 20);
}

/// [`touch`], to a time so many years back: a second touch of one file takes another time than the first.
pub fn touch_back(file: &Path, years: u64) {
    let then = std::time::SystemTime::now() - std::time::Duration::from_secs(years * 365 * 24 * 3600);
    fs::File::options().write(true).open(file).unwrap().set_modified(then).unwrap();
}

pub const STEPPED: &str = "git would not move the branch forward to the work of run {run}: git stopped the step, \
                           files of the work may already be written to the checkout, and the next `run keep` begins \
                           again";
pub const FORWARD: &str = "run {run} was kept: the branch was moved forward to the run's commit";
pub const JOINED: &str = "run {run} was kept: one new commit on top of the branch's own holds its work and the run's";
pub const BEFORE: &str = "the work of run {run} was kept before: nothing was changed";
pub const UNCHANGED: &str = "the work of run {run} was not kept, and nothing was changed";

/// What a keep that worked printed, with its five names and no other.
pub fn kept(made: &Made, ran: &Ran, commit: &str, onto: &str, why: &str) {
    let want = json!({"run": made.run, "wish": made.wish, "kept": commit, "onto": onto, "why": why.replace("{run}", &made.run)});
    assert_eq!(printed(ran), want, "{}", ran.stderr);
    made.w.never_says_the_agent(ran);
}

#[test]
fn work_is_kept_by_moving_the_branch_forward_when_the_checkout_is_where_the_run_started() {
    let made = finished();
    let (end, start) = (made.end(), made.w.commit(&made.repo, "main"));
    assert_eq!(made.files().len(), 3, "the checkout is as the run found it");
    let ran = made.keep(&[]);
    kept(&made, &ran, &end, "main", FORWARD);
    // The branch is at the run's own commit, and the checkout stands on it still.
    assert_eq!(made.w.commit(&made.repo, "main"), end);
    assert_eq!(made.git(&["symbolic-ref", "HEAD"]), "refs/heads/main");
    assert_eq!(made.git(&["rev-list", "main"]), format!("{end}\n{start}"));
    // The files are in the working tree, and git finds nothing to say of it.
    assert_eq!(made.files(), worked());
    assert_eq!(made.status(), "");
    // The run's branch is gone, and nothing else was added to the repo's names.
    assert!(made.w.branches(&made.repo).is_empty());
    assert_eq!(made.git(&["for-each-ref", "--format=%(refname)"]), "refs/heads/main");
    // The record says kept, with the commit and the branch, after everything a run wrote.
    let record = made.record();
    assert_eq!(
        (&record["work"], &record["kept"], &record["onto"]),
        (&json!("kept"), &json!(end), &json!("refs/heads/main"))
    );
    assert_eq!((&record["end"], &record["outcome"]), (&json!(end), &json!("completed")));
    // The wish is as its run left it: done, with its one star.
    assert_eq!(
        (&made.w.kept(&made.wish)["state"], made.w.kept(&made.wish)["stars"].as_array().unwrap().len()),
        (&json!("done"), 1)
    );
}

#[test]
fn work_is_kept_as_one_new_commit_by_the_person_on_top_of_their_own_commits_since() {
    let made = finished();
    let (end, start, theirs) = (made.end(), made.w.commit(&made.repo, "main"), made.moved_on());
    let ran = made.keep(&[]);
    let commit = made.w.commit(&made.repo, "main");
    kept(&made, &ran, &commit, "main", JOINED);
    // One new commit, whose one parent is the person's last: no merge commit, and the run's own commit is in
    // nobody's history.
    assert_eq!(made.git(&["rev-list", "--parents", "main"]), format!("{commit} {theirs}\n{theirs} {start}\n{start}"));
    assert!(commit != end && !made.git(&["rev-list", "main"]).contains(&end));
    // It is theirs: author and committer are who the repo says they are, and the message is a run's.
    let who = made.git(&["log", "-1", "--format=%an <%ae> / %cn <%ce> / %s%n%b", "main"]);
    assert_eq!(
        who.trim_end(),
        format!("Ada Example <ada@example.test> / Ada Example <ada@example.test> / Jellymoon run {}", made.run)
    );
    // It holds both: their change and their file, and the run's edit, its new file and its removal.
    let mut both = worked();
    both[1].1 = "c.txt as the person left it\n".to_string();
    both.insert(2, ("mine.txt".to_string(), "the person's own\n".to_string()));
    assert_eq!(made.files(), both);
    assert_eq!(made.status(), "");
    assert!(made.w.branches(&made.repo).is_empty());
    let record = made.record();
    assert_eq!(
        (&record["work"], &record["kept"], &record["onto"]),
        (&json!("kept"), &json!(commit), &json!("refs/heads/main"))
    );
}

#[test]
fn what_the_person_has_unsaved_beside_the_work_stays_as_it_is() {
    let made = finished();
    made.w.work_in_progress(&made.repo);
    fs::write(made.repo.join(".gitignore"), "build/\n").unwrap();
    fs::create_dir(made.repo.join("build")).unwrap();
    fs::write(made.repo.join("build").join("out.bin"), "built\n").unwrap();
    let stash = made.git(&["rev-parse", "refs/stash"]);
    let ran = made.keep(&[]);
    kept(&made, &ran, &made.end(), "main", FORWARD);
    // The staged file is still staged, the edit still unsaved, and the files git never saw are still there.
    assert_eq!(made.status(), " M c.txt\nA  staged.txt\n?? .gitignore\n?? note.txt\n!! build/");
    let files: HashMap<String, String> = made.files().into_iter().collect();
    assert_eq!(files["c.txt"], "half an edit\n");
    assert_eq!((files["note.txt"].as_str(), files["build/out.bin"].as_str()), ("git has never seen this\n", "built\n"));
    assert_eq!((files["a.txt"].as_str(), files["n.txt"].as_str()), ("edited by the run\n", "new from the run\n"));
    assert_eq!(made.git(&["rev-parse", "refs/stash"]), stash);
}

#[test]
fn a_second_keep_changes_nothing_and_says_so_and_a_look_says_the_work_was_kept() {
    let made = finished();
    let end = made.end();
    kept(&made, &made.keep(&[]), &end, "main", FORWARD);
    let before = made.held();
    for more in [&[][..], &["--with-settings"]] {
        kept(&made, &made.keep(more), &end, "main", BEFORE);
        assert!(before == made.held());
    }
    let asked = printed(&made.keep(&["--ask"]));
    assert_eq!(
        (&asked["kept"], &asked["fits"], &asked["why"]),
        (&json!(end), &json!(false), &json!(BEFORE.replace("{run}", &made.run)))
    );
    let show = ["run", "show", "--wish", &made.wish, "--date", DAY, "--repo", made.repo.to_str().unwrap()];
    let shown = printed(&made.w.run(&show));
    assert_eq!((&shown["work"], shown["files"].as_array().unwrap().len()), (&json!("kept"), 0));
    assert!(before == made.held());
}

#[test]
fn asked_first_a_keep_says_what_it_would_do_and_changes_nothing() {
    let made = finished();
    let (end, before) = (made.end(), made.held());
    let would = |why: &str| why.replace("{run}", &made.run);
    let asked = made.keep(&["--ask"]);
    let forward = json!({"run": made.run, "wish": made.wish, "kept": end, "onto": "main", "fits": true,
        "why": would("a keep of run {run} would move the branch forward to the run's commit")});
    assert_eq!(printed(&asked), forward);
    assert!(before == made.held());
    // With commits of the person's own it would make a commit, which has no id yet.
    made.moved_on();
    let before = made.held();
    let asked = printed(&made.keep(&["--ask"]));
    let why = would(
        "a keep of run {run} would land one new commit on top of the branch's own, holding its work and the run's",
    );
    assert_eq!(
        asked,
        json!({"run": made.run, "wish": made.wish, "kept": null, "onto": "main", "fits": true, "why": why})
    );
    assert!(before == made.held());
    // What would be refused is printed, in the words of the refusal, and is no failure of the question.
    fs::write(made.repo.join("n.txt"), "the person's own\n").unwrap();
    let before = made.held();
    let asked = made.keep(&["--ask"]);
    let why = would(&format!(
        "an unsaved change, or a file that is not in the branch, is where the work would go: {UNCHANGED}"
    ));
    assert_eq!(
        printed(&asked),
        json!({"run": made.run, "wish": made.wish, "kept": null, "onto": null, "fits": false, "why": why})
    );
    assert!(before == made.held());
    made.refuses(&made.keep(&[]), &before, &why);
}

#[test]
fn a_file_of_any_name_is_kept_and_no_name_is_read_as_an_option_or_a_pattern() {
    let made = finished();
    let entries: [Entry; 4] = [
        ("100644", "-rf", b"a name like an option\n"),
        ("100644", "--exec=x", b"another\n"),
        ("100644", "dir/*.txt", b"a name like a pattern\n"),
        ("100755", "dir/run me.sh", b"a program\n"),
    ];
    // A star is no name on Windows: there git itself refuses the file, and the keep is refused whole.
    let entries = if cfg!(windows) { &entries[..2] } else { &entries[..] };
    let end = made.rework(entries);
    kept(&made, &made.keep(&[]), &end, "main", FORWARD);
    assert_eq!(made.status(), "");
    let files: HashMap<String, String> = made.files().into_iter().collect();
    assert_eq!(files["-rf"], "a name like an option\n");
    assert_eq!(files.len(), 3 + entries.len());
}
