//! F48: a run's record names its repo, the commit it started from and the commit it ended as (ADR 0017, "Found
//! while building F44"; `docs/development/run.md`, "What is kept"; `docs/development/show.md`). `jellymoon run` is
//! run as a black box in F39's test folder (`f39/world.rs`), with the made-up `fake-agent` as the harness and repos
//! made with the real `git`. This file holds what a run writes; `f48/shown.rs` holds what `jellymoon run show`
//! takes from the record and what it refuses, `f48/cut.rs` a run cut off between its steps, `f48/moved.rs` a run
//! whose branch was moved while it worked, `f48/raced.rs` one whose branch was moved between two git steps of its
//! end, and `f48/linked.rs` a repo with a second checkout.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f48/cut.rs"]
mod cut;
#[path = "f48/linked.rs"]
mod linked;
#[path = "f48/moved.rs"]
mod moved;
#[path = "f48/named.rs"]
mod named;
#[path = "f48/raced.rs"]
mod raced;
#[path = "f48/shown.rs"]
mod shown;
mod support;
#[path = "f39/world.rs"]
mod world;

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use support::{Ran, install, said, tree, write_log};
pub use world::*;

/// A run the made-up agent finished: it edited `a.txt`, removed `b.txt` and made `n.txt`.
pub fn finished() -> Made {
    played("edits", "completed")
}

/// A run of the made-up agent's script, which ended as `outcome`.
pub fn played(script: &str, outcome: &str) -> Made {
    let w = world();
    w.play(script, 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (run, said) = ran(&w.start(&wish, &repo));
    assert_eq!(said["outcome"], outcome, "{said}");
    Made { w, repo, wish, run }
}

impl Made {
    pub fn git(&self, args: &[&str]) -> String {
        self.w.git(&self.repo, args)
    }

    /// `jellymoon run show` for the wish, in `repo`, with `more` arguments.
    pub fn show_in(&self, repo: &Path, more: &[&str]) -> Ran {
        let asked = ["run", "show", "--wish", &self.wish, "--date", DAY, "--repo", repo.to_str().unwrap()];
        self.w.run(&[&asked[..], more].concat())
    }

    pub fn show(&self) -> Ran {
        self.show_in(&self.repo, &[])
    }

    /// A commit made by hand in the repo's store, of the tree of `like`, on `parent`: no checkout is touched.
    pub fn by_hand(&self, parent: &str, message: &str, like: &str) -> String {
        self.git(&["commit-tree", "-p", parent, "-m", message, &format!("{like}^{{tree}}")])
    }

    /// The message core gives a run's commit.
    pub fn message(&self) -> String {
        format!("Jellymoon run {}", self.run)
    }
}

/// Whether a record's word is that folder as the system resolves it: the one spelling a run writes.
pub fn one_folder(one: &str, other: &Path) -> bool {
    Path::new(one) == fs::canonicalize(other).unwrap()
}

#[test]
fn a_run_that_completes_writes_its_repo_its_start_and_its_end_in_its_record() {
    let made = finished();
    let (record, main) = (made.record(), made.w.commit(&made.repo, "main"));
    let end = made.w.commit(&made.repo, &made.branch());
    assert!(one_folder(record["repo"].as_str().unwrap(), &made.repo.join(".git")), "{record}");
    assert_eq!((&record["start"], &record["end"]), (&json!(main), &json!(end)), "{record}");
    assert_ne!(main, end);
    assert_eq!(made.git(&["rev-list", "--parents", "-n", "1", &end]), format!("{end} {main}"));

    // A run that changed nothing ended as the commit it started from. Its branch is still at that commit, or
    // the run would have failed (`f48/moved.rs`): core compares the two before it commits anything.
    let nothing = played("nothing", "completed");
    let (record, main) = (nothing.record(), nothing.w.commit(&nothing.repo, "main"));
    assert!(record["why"].as_str().unwrap().contains("changed nothing"), "{record}");
    assert!(one_folder(record["repo"].as_str().unwrap(), &nothing.repo.join(".git")), "{record}");
    assert_eq!((&record["start"], &record["end"]), (&json!(main), &json!(main)), "{record}");
    assert_eq!(nothing.w.commit(&nothing.repo, &nothing.branch()), main);
}

#[test]
fn a_run_that_failed_or_was_cancelled_writes_its_repo_and_its_start_and_no_end() {
    let failed = played("exit-3", "failed");
    let w = world();
    w.play("forever", 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let going = w.start_going(&wish, &repo);
    wait_for(&w.at("rec").join("started"));
    let run = w.kept(&wish)["run"].as_str().unwrap().to_string();
    // While it goes the record names the repo and the start already: both were known when the copy was made.
    let record = w.record(&run);
    assert!(record["outcome"].is_null() && record["start"].is_string() && record["repo"].is_string(), "{record}");
    printed(&w.run(&["run", "stop", "--wish", &wish, "--date", DAY]));
    assert_eq!(printed(&ended(going))["outcome"], "cancelled");
    let cancelled = Made { w, repo, wish, run };

    for (made, outcome) in [(&failed, "failed"), (&cancelled, "cancelled")] {
        let (record, main) = (made.record(), made.w.commit(&made.repo, "main"));
        assert_eq!(record["outcome"], outcome);
        assert!(one_folder(record["repo"].as_str().unwrap(), &made.repo.join(".git")), "{record}");
        assert_eq!(record["start"], json!(main), "{record}");
        assert!(record.get("end").is_none(), "a run that did not finish ended as no commit: {record}");
        assert_eq!(made.w.commit(&made.repo, &made.branch()), main, "nothing was committed on its branch");
    }
}

#[test]
fn a_run_that_was_never_started_leaves_no_record_at_all() {
    let w = world();
    w.play("old", 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let said = printed(&w.start(&wish, &repo));
    assert_eq!((&said["outcome"], &said["run"]), (&json!("not_started"), &Value::Null), "{said}");
    let records = tree(&w.jm("runs")).into_iter().filter(|(path, _)| path.ends_with("run.json")).count();
    assert_eq!(records, 0, "the run's folder went with its copy");
    assert!(w.branches(&repo).is_empty() && w.copies().is_empty());
}

/// The names of a record's fields, in the order they are written in its one line.
fn fields(line: &str) -> Vec<String> {
    let record: Value = serde_json::from_str(line).unwrap();
    let mut names: Vec<&String> = record.as_object().unwrap().keys().collect();
    names.sort_by_key(|name| line.find(&format!("\"{name}\":")).unwrap());
    names.into_iter().cloned().collect()
}

// Run before F48 was built, this test passed as it stands but for the three names in `fields`: it is what a run
// wrote then.
#[test]
fn nothing_else_a_run_writes_changes_and_no_step_of_a_run_in_the_persons_repo_does() {
    let w = world();
    let repo = w.repo("alpha");
    w.work_in_progress(&repo);
    let (wish, before) = (w.wish("alpha"), w.persons(&repo));
    let main = w.commit(&repo, "main");
    let (run, said) = ran(&w.start(&wish, &repo));
    let made = Made { w, repo, wish, run };
    let (w, run) = (&made.w, &made.run);
    let why = format!("run {run} finished: its work is one commit on the branch jellymoon/run-{run}");
    let want = json!({"wish": "m1", "outcome": "completed", "star": STAR, "denied": 0, "why": why});
    assert_eq!(said, want);

    // The record: one line, the fields a run wrote before, then the three, then the session (F51), and every
    // earlier value as it was.
    let line = fs::read_to_string(w.jm("runs").join(run).join("run.json")).unwrap();
    assert!(line.ends_with("}\n") && line.matches('\n').count() == 1, "{line:?}");
    let old = ["wish", "day", "started", "outcome", "why", "denied", "skipped", "pid", "mark", "began"];
    assert_eq!(fields(&line), [&old[..], &NAMED[..], &["session"]].concat());
    let mut record = made.as_before();
    assert!(record["started"].as_u64().is_some_and(|at| at > 1_600_000_000), "{record}");
    assert!(record["pid"].as_u64().is_some() && (record["mark"].is_string() || cfg!(windows)), "{record}");
    for at in ["started", "pid", "mark"] {
        record.as_object_mut().unwrap().remove(at);
    }
    let want = json!({
        "wish": "m1", "day": DAY, "outcome": "completed", "why": why, "denied": 0, "skipped": 0, "began": true
    });
    assert_eq!(record, want);

    // The home: the files a run left before, and no other.
    let home = w.at("jm");
    let name = |path: PathBuf| path.strip_prefix(&home).unwrap().to_string_lossy().replace('\\', "/");
    let mut left: Vec<String> = tree(&home).into_iter().map(|(path, _)| name(path)).collect();
    left.sort();
    let mut want = vec![
        "copies".to_string(),
        "pages".to_string(),
        format!("pages/{DAY}.wishes.json"),
        "runs".to_string(),
        "runs/clean.lock".to_string(),
        "runs/going.lock".to_string(),
        "runs/no-hooks".to_string(),
        format!("runs/{run}"),
        format!("runs/{run}/run.json"),
    ];
    want.sort();
    assert_eq!(left, want);
    let kept = w.kept(&made.wish);
    assert_eq!((&kept["run"], &kept["state"], &kept["why"]), (&json!(run), &json!("done"), &json!(why)));
    assert_eq!(kept["stars"], json!([STAR]));

    // The repo: the person's files, index, `HEAD` and refs as they were, and of the run one branch of one commit.
    assert_eq!(w.persons(&made.repo), before);
    assert_eq!(w.branches(&made.repo), [format!("jellymoon/run-{run}")]);
    let end = w.commit(&made.repo, &made.branch());
    let shape = made.git(&["log", "-1", "--format=%P %an <%ae> %cn <%ce> %s", &end]);
    assert_eq!(
        shape,
        format!("{main} Ada Example <ada@example.test> Ada Example <ada@example.test> {}", made.message())
    );
    let files = made.git(&["diff-tree", "-r", "--no-renames", "--name-status", &main, &end]);
    assert_eq!(files, "M\ta.txt\nD\tb.txt\nA\tn.txt");
    assert_eq!(made.git(&["worktree", "list", "--porcelain"]).matches("worktree ").count(), 1);
    let kept_for_copies = fs::read_dir(made.repo.join(".git").join("worktrees")).map_or(0, |held| held.count());
    assert_eq!(kept_for_copies, 0, "git's registration of the copy is gone, as before");
}

#[test]
fn the_repos_path_is_in_the_runs_record_and_in_nothing_else_a_run_or_a_page_writes_prints_or_sends() {
    let made = finished();
    let (w, repo) = (&made.w, made.repo.to_str().unwrap());
    // A session typed in the repo that day, so the page makes its reading call, to a made-up harness that keeps
    // what it was given.
    write_log(&w.at("logs"), "alpha/s.jsonl", &[said("2026-03-10T16:00:00.000Z", repo, "look at the uploader")]);
    let reader = install(&w.at("reader"), "claude");
    let reply = json!({"type": "result", "is_error": false, "result": r#"{"wishes":[],"clashes":[]}"#});
    fs::write(w.at("reader").join("reply"), reply.to_string()).unwrap();
    fs::write(w.at("reader").join("exit"), "0").unwrap();
    let logs = w.at("logs");
    let page =
        w.run_with(&[("JELLYMOON_CLAUDE", reader.into())], &["page", "--date", DAY, "--logs", logs.to_str().unwrap()]);
    printed(&page);
    let sent = fs::read(w.at("reader").join("stdin")).expect("the reading call was made");
    assert!(!sent.is_empty());

    let printed_by = [
        page,
        made.show(),
        made.show_in(&made.repo, &["--file", "3"]),
        w.run(&["run", "stop", "--wish", &made.wish, "--date", DAY]),
        w.run(&["day", "--date", DAY, "--logs", logs.to_str().unwrap()]),
    ];
    // The path as it was typed and as the system resolves it, with either separator, and as JSON spells it:
    // there every `\` of a Windows path is two, in the record and in whatever is printed.
    let real = fs::canonicalize(&made.repo).unwrap();
    let spelled = [repo.to_string(), real.to_string_lossy().into_owned()];
    let in_json = |path: &String| serde_json::to_string(path).unwrap().trim_matches('"').to_string();
    let spellings = |path: &String| [path.clone(), path.replace('\\', "/"), in_json(path)];
    let needles: Vec<String> = spelled.iter().flat_map(spellings).collect();
    let holds = |bytes: &[u8]| needles.iter().any(|path| bytes.windows(path.len()).any(|at| at == path.as_bytes()));
    for ran in &printed_by {
        assert_eq!(ran.code, 0, "{}", ran.stderr);
        assert!(!holds(ran.stdout.as_bytes()) && !holds(ran.stderr.as_bytes()), "{} {}", ran.stdout, ran.stderr);
    }
    let record = w.jm("runs").join(&made.run).join("run.json");
    let held: Vec<PathBuf> =
        tree(&w.at("jm")).into_iter().filter(|(_, bytes)| holds(bytes)).map(|(path, _)| path).collect();
    assert_eq!(held, [record], "the one file of the home that holds the repo's path");
    for (path, bytes) in tree(&w.at("reader")) {
        let given = path.file_name().is_some_and(|name| name != "env" && name != "cwd" && name != "ls");
        assert!(!given || !holds(&bytes), "the reading call was given the repo's path in {}", path.display());
    }
}
