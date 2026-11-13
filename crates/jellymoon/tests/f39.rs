//! F39: a wish starts a run with the person's harness, and its star appears when the run reports it finished
//! (ADR 0014, `docs/development/run.md`). `jellymoon run` is run as a black box. The harness it finds is the made-up
//! `fake-agent` (`support/fake_agent.rs`), first on a `PATH` whose only other folder holds git alone, so the real
//! Claude Code is never started. Every repo is made here with the real `git`, under an empty global config, and both
//! homes and the harness's folder are scratch folders. `f39/failed.rs` holds the runs that do not finish and the
//! caps on the stream, `f39/stream.rs` the star's rule and the stream's first line, `f39/refused.rs` what is refused
//! or not started, `f39/limits.rs` the time limit, a run cut off and two runs at once, `f39/safe.rs` what never leaks,
//! `f39/world.rs` the test's folder, which F40's tests use too, and `f39/guard.rs` what ends with a test, which
//! `f39/panics.rs` proves.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f39/failed.rs"]
mod failed;
#[path = "f39/limits.rs"]
mod limits;
#[path = "f39/panics.rs"]
mod panics;
#[path = "f39/refused.rs"]
mod refused;
#[path = "f39/safe.rs"]
mod safe;
#[path = "f39/stream.rs"]
mod stream;
mod support;
#[path = "f39/world.rs"]
mod world;

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;
pub use world::*;

#[test]
fn the_agent_is_started_with_the_measured_command_line_in_the_copy_and_the_wish_on_its_input() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (run, _) = ran(&w.start(&wish, &repo));
    let (args, _) = w.args();
    assert_eq!(args, ARGS, "the arguments, in order");
    assert_eq!(w.rec("stdin").unwrap(), format!("{SAID}\n"), "the wish's words go in on the input");
    assert!(args.iter().all(|arg| !arg.contains(SAID) && !arg.contains("uploader")), "and in no argument");
    // The copy is gone by now, so both are named from the folder that held it, each by that folder's real path:
    // the same folder is then the same text on every system.
    let cwd = PathBuf::from(w.rec("cwd").unwrap());
    let held = |folder: &Path| fs::canonicalize(folder).unwrap();
    assert_eq!(held(cwd.parent().unwrap()).join(cwd.file_name().unwrap()), held(&w.jm("copies")).join(&run));
    assert_eq!(w.starts(), 1);
}

#[test]
fn the_agent_sees_only_the_allow_listed_variables() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    // Names shaped like the ones a token is kept under, joined here so no scanner reads one in this file.
    let planted = [format!("GH_{}", "TOKEN"), format!("ANTHROPIC_{}_{}", "API", "KEY"), "GIT_DIR".to_string()];
    let planted: Vec<&'static str> = planted.into_iter().map(|name| &*name.leak()).collect();
    let more: Vec<(&'static str, OsString)> = planted.iter().map(|name| (*name, "planted".into())).collect();
    let args = ["run", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()];
    assert_eq!(ran(&w.run_with(&more, &args)).1["outcome"], "completed");
    let seen = w.rec("env").unwrap();
    let seen: Vec<&str> = seen.lines().collect();
    assert!(planted.iter().all(|name| !seen.contains(name)), "{seen:?}");
    if cfg!(windows) {
        let allowed = "PATH PATHEXT SystemRoot SystemDrive ComSpec USERPROFILE HOMEDRIVE HOMEPATH APPDATA \
                       LOCALAPPDATA TEMP TMP USERNAME CLAUDE_CONFIG_DIR";
        let allowed: Vec<String> = allowed.split(' ').map(str::to_lowercase).collect();
        assert!(seen.iter().all(|name| allowed.contains(&name.to_lowercase())), "{seen:?}");
    } else {
        assert_eq!(seen, ["CLAUDE_CONFIG_DIR", "HOME", "LANG", "LC_ALL", "PATH", "TERM", "TMPDIR", "USER"]);
    }
}

#[test]
fn a_finished_run_adds_one_star_and_leaves_its_work_as_one_commit_on_its_branch() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    w.work_in_progress(&repo);
    let (base, before) = (w.commit(&repo, "HEAD"), w.persons(&repo));

    let done = w.start(&wish, &repo);
    let (run, said) = ran(&done);
    let branch = format!("jellymoon/run-{run}");
    let why = format!("run {run} finished: its work is one commit on the branch {branch}");
    assert_eq!(said, json!({"wish": wish, "outcome": "completed", "star": STAR, "denied": 0, "why": why}));

    // The branch holds exactly the agent's edits, as one commit on the commit the checkout was on.
    let log = w.git(&repo, &["log", "-1", "--format=%an <%ae>|%s|%P", "--name-status", &branch]);
    let log: Vec<&str> = log.lines().filter(|line| !line.is_empty()).collect();
    let first = format!("Ada Example <ada@example.test>|Jellymoon run {run}|{base}");
    assert_eq!(log, [first.as_str(), "M\ta.txt", "D\tb.txt", "A\tn.txt"]);
    assert_eq!(w.git(&repo, &["show", &format!("{branch}:a.txt")]), "edited by the run");
    // Nothing of the person's is changed, the copy's folder is gone, and only the run's branch is new.
    assert_eq!(w.persons(&repo), before);
    assert_eq!((w.copies(), w.branches(&repo)), (vec![], vec![branch.clone()]));
    assert_eq!(w.git(&repo, &["worktree", "list", "--porcelain"]).matches("worktree ").count(), 1);

    let kept =
        json!({"id": wish, "said": SAID, "project": "alpha", "state": "done", "stars": [STAR], "run": run, "why": why});
    assert_eq!(w.kept(&wish), kept);
    let on_page = w.on_page(&wish);
    assert_eq!((&on_page["state"], &on_page["stars"]), (&json!("done"), &json!([STAR])), "the page shows the star");
    let record = w.record(&run);
    assert_eq!((&record["wish"], &record["day"], &record["outcome"]), (&json!(wish), &json!(DAY), &json!("completed")));
    assert_eq!((&record["denied"], &record["skipped"]), (&json!(0), &json!(0)));
    w.never_says_the_agent(&done);
}

#[test]
fn a_run_that_changed_nothing_is_still_finished_and_says_so() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    w.play("nothing", 0);
    let (run, said) = ran(&w.start(&wish, &repo));
    let branch = format!("jellymoon/run-{run}");
    let why = format!("run {run} finished and changed nothing: the branch {branch} is where the checkout was");
    assert_eq!(said, json!({"wish": wish, "outcome": "completed", "star": STAR, "denied": 0, "why": why}));
    assert_eq!(w.commit(&repo, &branch), w.commit(&repo, "HEAD"));
    assert_eq!((w.copies(), &w.kept(&wish)["state"]), (vec![], &json!("done")));
}

// The stream is untrusted: what is not a line of it is counted and skipped, and what was denied is only counted.
#[test]
fn lines_that_are_not_the_stream_s_are_skipped_and_denials_are_counted() {
    let w = world();
    let repo = w.repo("alpha");
    w.play("garbage", 0);
    let garbage = w.start(&w.wish("alpha"), &repo);
    let (run, said) = ran(&garbage);
    assert_eq!((&said["outcome"], &said["denied"]), (&json!("completed"), &json!(0)));
    assert_eq!(w.record(&run)["skipped"], 8, "eight lines that are not the stream's");
    w.never_says_the_agent(&garbage);

    w.play("denials", 2);
    let denied = w.start(&w.wish("alpha"), &repo);
    let (run, said) = ran(&denied);
    assert_eq!((&said["outcome"], &said["denied"], &said["star"]), (&json!("completed"), &json!(2), &json!(STAR)));
    assert_eq!(w.record(&run)["denied"], 2);
    w.never_says_the_agent(&denied);
}
