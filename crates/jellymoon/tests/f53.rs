//! F53: a follow-up is a made wish that names the wish it follows, run on that work with a fork of its session
//! (ADR 0018, decisions 8 and 9; `docs/development/wish.md`, "A follow-up"; `run.md`, "A follow-up's run"). The
//! commands are run as black boxes against the made-up `fake-agent` in F39's test folder (`f39/world.rs`), in
//! repos made with the real `git`. This file holds a follow-up that is made and run; `f53/refused.rs` every
//! follow-up that is not made or not started, with nothing changed; `f53/session.rs` the fork and a session
//! that is not kept; and `f53/order.rs` the order followed work and its follow-up are kept and tossed in.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f53/gate.rs"]
mod gate;
#[path = "f45/made.rs"]
mod kept;
#[path = "f53/order.rs"]
mod order;
#[path = "f53/refused.rs"]
mod refused;
#[path = "f53/session.rs"]
mod session;
mod support;
#[path = "f39/world.rs"]
mod world;

use std::path::PathBuf;

pub use kept::{finished, said};
use serde_json::{Value, json};
use support::{Ran, tree};
pub use world::*;

/// What a follow-up says, in words no other text of a test holds.
pub const MORE: &str = "and log each retry, plum-lantern";
pub const UNSTARTED: &str = "nothing was started: `jellymoon wish` keeps a wish and starts no run";
pub const FINISHED: &str = "run {run} finished: its work is one commit on the branch jellymoon/run-{run}";
/// What the made-up agent finds in a copy made at the commit a repo starts with, and at the first run's work.
pub const AS_IT_WAS: &str =
    "a.txt: a.txt as it was\nb.txt: b.txt as it was\nc.txt: c.txt as it was\nn.txt: none\nm.txt: none\n";
pub const AS_THE_RUN_LEFT_IT: &str =
    "a.txt: edited by the run\nb.txt: none\nc.txt: c.txt as it was\nn.txt: new from the run\nm.txt: none\n";

/// `jellymoon wish --after` for a wish of the day.
pub fn follow(w: &World, after: &str) -> Ran {
    w.run(&["wish", "--date", DAY, "--say", MORE, "--after", after])
}

/// Makes a follow-up of a wish of the day, and returns its id.
pub fn followed(w: &World, after: &str) -> String {
    printed(&follow(w, after))["wish"]["id"].as_str().unwrap().to_string()
}

/// Everything a refused command could change: every file of the home but the keeps' empty lock, what is the
/// person's in the repo, every ref with what it names, the harness's own store, and how often it was started.
pub type All =
    (Vec<(PathBuf, Vec<u8>)>, (Vec<(PathBuf, Vec<u8>)>, Vec<u8>, Vec<u8>, Vec<String>), String, usize, usize);

impl Made {
    /// `jellymoon run`, or `run <verb>`, for a wish of the day in the repo, with `more` arguments.
    pub fn asked(&self, verb: &[&str], wish: &str, more: &[&str]) -> Ran {
        let named = ["--wish", wish, "--date", DAY, "--repo", self.repo.to_str().unwrap()];
        self.w.run(&[&["run"], verb, &named[..], more].concat())
    }

    /// A follow-up of the first wish that the made-up agent played `script` for: its wish, and what was printed.
    pub fn follows(&self, script: &str, number: usize) -> (String, Value) {
        let wish = followed(&self.w, &self.wish);
        self.w.play(script, number);
        (wish.clone(), printed(&self.asked(&[], &wish, &[])))
    }

    pub fn all(&self) -> All {
        let home = tree(&self.w.at("jm")).into_iter().filter(|(path, _)| !path.ends_with("keep.lock"));
        let refs = self.git(&["for-each-ref", "--format=%(refname) %(objectname) %(symref)"]);
        (home.collect(), self.w.persons(&self.repo), refs, tree(&self.w.at("harness")).len(), self.w.starts())
    }

    /// Fails unless the command was refused in these words, with nothing started and nothing changed.
    pub fn unchanged(&self, ran: &Ran, before: &All, why: &str) {
        assert_eq!(refused(ran), said(&why.replace("{run}", &self.run)));
        assert!(*before == self.all(), "a refused command started or changed something: {why}");
    }
}

/// The command line the made-up agent was given at its start `n`, and what it found in the folder it stood in.
pub fn given(w: &World, n: usize) -> (Vec<String>, String) {
    let line = w.rec(&format!("line{n}")).expect("the agent was not started that often");
    (line.lines().map(str::to_string).collect(), w.rec(&format!("found{n}")).unwrap())
}

/// A run's command line with a fork of `of` under the new session `id` where a run names its session.
pub fn forked(of: &str, id: &str) -> Vec<String> {
    let under = ["--resume", of, "--fork-session", "--session-id", id].map(str::to_string);
    let (head, tail) = (ARGS[..13].iter(), ARGS[15..].iter());
    head.map(|arg| arg.to_string()).chain(under).chain(tail.map(|arg| arg.to_string())).collect()
}

#[test]
fn a_follow_up_is_a_made_wish_that_names_the_wish_it_follows_and_takes_its_project() {
    let made = finished();
    let (w, first) = (&made.w, made.w.kept("m1"));
    let ran = follow(w, "m1");
    let wish = json!({"id": "m2", "said": MORE, "project": "alpha", "state": "working", "stars": [], "after": "m1"});
    assert_eq!(printed(&ran), json!({"wish": wish, "run": null, "why": UNSTARTED}));
    assert_eq!((w.kept("m2"), w.kept("m1")), (wish, first.clone()));
    assert_eq!((first.get("after"), w.starts(), w.branches(&made.repo).len()), (None, 1, 1));
    // A page shows it as any made wish, and what it follows is no part of a page.
    let on_page = w.on_page("m2");
    assert_eq!((&on_page["said"], &on_page["state"], on_page.get("after")), (&json!(MORE), &json!("working"), None));
    // A wish is followed twice, and a follow-up that has had no run has no work to follow.
    let third = printed(&follow(w, "m1"));
    assert_eq!((&third["wish"]["id"], &third["wish"]["after"]), (&json!("m3"), &json!("m1")));
    assert_eq!(refused(&follow(w, "m2")), said("wish m2 has had no run, so there is no work to follow"));
    assert_eq!(w.kept("m2")["after"], "m1");
}

#[test]
fn a_follow_up_runs_in_a_copy_at_the_followed_work_while_it_waits_under_a_fork_of_its_session() {
    let made = finished();
    let (w, repo) = (&made.w, &made.repo);
    let (session, first, end, person) = (w.args().1, made.record(), made.end(), w.persons(repo));
    let (wish, said) = made.follows("follow", 0);
    let run = said["run"].as_str().unwrap().to_string();
    let line = FINISHED.replace("{run}", &run);
    let done = json!({"run": run, "wish": wish, "outcome": "completed", "star": STAR, "denied": 0, "why": line});
    assert_eq!(said, done);
    // The harness was told to fork the followed run's session under a new id of core's, with a run's own list
    // of what it may do, and the follow-up's words were its whole input.
    let (record, (args, found)) = (w.record(&run), given(w, 2));
    let new = record["session"].as_str().unwrap();
    assert!(is_session(new) && new != session, "{new}");
    assert_eq!(args, forked(&session, new));
    assert_eq!((w.rec("stdin").unwrap(), w.starts()), (format!("{MORE}\n"), 2));
    // Its copy was made at the commit of the work it follows, and the record says so and names that run.
    assert_eq!(found, AS_THE_RUN_LEFT_IT);
    let tip = w.commit(repo, &format!("refs/heads/jellymoon/run-{run}"));
    assert_eq!((&record["start"], &record["end"], &record["repo"]), (&json!(end), &json!(tip), &first["repo"]));
    assert_eq!((&record["follows"], &record["forked"]), (&json!(made.run), &json!(true)));
    assert_eq!(w.git(repo, &["rev-parse", &format!("{tip}^")]), end, "one commit on the followed work");
    // The followed run is as it was: its record, its branch and its session.
    assert_eq!((made.record(), w.commit(repo, &made.branch())), (first, end));
    assert!(person == w.persons(repo), "nothing of the person's was changed");
    let kept = json!({"id": wish, "said": MORE, "project": "alpha", "state": "done", "stars": [STAR],
        "run": run, "why": line, "after": "m1"});
    assert_eq!((w.kept(&wish), w.copies()), (kept, vec![]));
    // What is shown of the follow-up is what it adds to the work it follows, and of that work what it made.
    let shown = |wish: &str| -> Vec<(String, String)> {
        let shown = printed(&made.asked(&["show"], wish, &[]));
        let entry =
            |file: &Value| (file["name"].as_str().unwrap().to_string(), file["change"].as_str().unwrap().into());
        shown["files"].as_array().unwrap().iter().map(entry).collect()
    };
    let pair = |name: &str, change: &str| (name.to_string(), change.to_string());
    assert_eq!(shown(&wish), [pair("a.txt", "changed"), pair("m.txt", "made")]);
    assert_eq!(shown("m1"), [pair("a.txt", "changed"), pair("b.txt", "removed"), pair("n.txt", "made")]);
    let text = printed(&made.asked(&["show"], &wish, &["--file", "1"]));
    assert_eq!(text["text"], "edited by the run, and again by the follow-up");
}

#[test]
fn a_follow_up_of_work_that_was_kept_runs_in_a_copy_made_where_the_checkout_stands() {
    let made = finished();
    let (w, repo, session) = (&made.w, &made.repo, made.w.args().1);
    assert_eq!(printed(&made.keep(&[]))["kept"], json!(made.end()));
    let stands = made.moved_on();
    let (wish, said) = made.follows("follow", 0);
    let run = said["run"].as_str().unwrap().to_string();
    assert_eq!(said["outcome"], "completed", "{said}");
    let (record, (args, found)) = (w.record(&run), given(w, 2));
    assert_eq!(args, forked(&session, record["session"].as_str().unwrap()));
    // The copy holds the kept work and the person's commit since: it was made where their checkout stands.
    let held = "a.txt: edited by the run\nb.txt: none\nc.txt: c.txt as the person left it\nn.txt: new from the run\nm.txt: none\n";
    assert_eq!((found.as_str(), &record["start"]), (held, &json!(stands)));
    assert_eq!((&record["follows"], &record["forked"]), (&json!(made.run), &json!(true)));
    // It is kept as any run's work is: the branch had not moved, and is moved forward to its commit.
    let kept = printed(&made.asked(&["keep"], &wish, &[]));
    assert_eq!((&kept["kept"], &kept["onto"]), (&record["end"], &json!("main")));
    assert_eq!(std::fs::read_to_string(repo.join("m.txt")).unwrap(), "more from the follow-up\n");
}

#[test]
fn a_follow_up_made_with_run_starts_its_run_at_once_as_a_wish_made_with_run_does() {
    let made = finished();
    let (w, session) = (&made.w, made.w.args().1);
    w.play("follow", 0);
    let logs = w.at("logs");
    let typed = support::said(&format!("{DAY}T16:00:00.000Z"), made.repo.to_str().unwrap(), "something typed there");
    support::write_log(&logs, "-work-alpha/s1.jsonl", &[typed]);
    let asked = ["wish", "--date", DAY, "--say", MORE, "--after", "m1", "--run", "--logs", logs.to_str().unwrap()];
    let wished = printed(&w.run(&asked));
    let run = wished["run"].as_str().expect("a run was started").to_string();
    assert_eq!((&wished["wish"]["after"], &wished["wish"]["id"]), (&json!("m1"), &json!("m2")));
    // The run goes on after the command: it ends by itself, as a fork of the followed run's session.
    let deadline = std::time::Instant::now() + support::LIMIT;
    while w.kept("m2")["state"] == "working" {
        assert!(std::time::Instant::now() < deadline, "the follow-up's run never ended");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let record = w.record(&run);
    assert_eq!((&w.kept("m2")["state"], &record["start"]), (&json!("done"), &json!(made.end())));
    assert_eq!(given(w, 2).0, forked(&session, record["session"].as_str().unwrap()));
}
