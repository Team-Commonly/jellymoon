//! F40: a run can be stopped, and nothing it asks for is allowed unless listed (ADR 0014,
//! `docs/development/run.md`, "A stop the person asks for"). `jellymoon run stop` is run as a black box beside a
//! `jellymoon run` that is going, in F39's test folder (`f39/world.rs`) with the made-up `fake-agent` as the
//! harness, so the real Claude Code is never started. `f40/deaf.rs` holds the agents that do not hear, and what
//! is asked meanwhile; `f40/nothing.rs` a stop with nothing going, a stop left by hand and a locked day;
//! `f40/races.rs` a stop against the run's own finish; `f40/denied.rs` what is counted as denied; `f40/obeys.rs`
//! an agent that ends by itself at the interrupt; `f40/hand.rs` a record and a wish changed by hand;
//! `f40/request.rs` a request that is no plain file; `f40/pipe.rs` a named pipe at a file of the home;
//! `f40/closed.rs` a lock nobody can open; `f40/theirs.rs` wishes made to name each other's live runs;
//! `f40/late.rs` a look between the end of a run's copy and the end in its record.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f40/closed.rs"]
mod closed;
#[path = "f40/deaf.rs"]
mod deaf;
#[path = "f40/denied.rs"]
mod denied;
#[path = "f40/hand.rs"]
mod hand;
#[path = "f40/late.rs"]
mod late;
#[path = "f40/nothing.rs"]
mod nothing;
#[path = "f40/obeys.rs"]
mod obeys;
#[path = "f40/pipe.rs"]
mod pipe;
#[path = "f40/races.rs"]
mod races;
#[path = "f40/request.rs"]
mod request;
mod support;
#[path = "f40/theirs.rs"]
mod theirs;
#[path = "f39/world.rs"]
mod world;

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use support::Ran;
pub use world::*;

/// How long a build with debug assertions gives a child after each time it is asked to stop.
pub const GRACE: &str = "JELLYMOON_TEST_GRACE_MS";

pub fn ms(variable: &'static str, ms: u64) -> (&'static str, OsString) {
    (variable, ms.to_string().into())
}

/// The variable that names the file a command held at a place waits for (`busy::pause`): `name`, in `rec`.
pub fn until(w: &World, name: &str) -> (&'static str, OsString) {
    ("JELLYMOON_TEST_GO", w.at("rec").join(name).into())
}

pub fn cancelled(run: &str) -> String {
    format!("run {run} was cancelled by a stop: its copy is kept and no star was added")
}

/// What a cancelled run prints, from `jellymoon run` and from `jellymoon run stop` alike.
pub fn said_cancelled(run: &str, wish: &str, denied: usize) -> Value {
    json!({"run": run, "wish": wish, "outcome": "cancelled", "star": null, "denied": denied, "why": cancelled(run)})
}

impl World {
    /// `jellymoon run stop` for a wish of the day, with `more` variables.
    pub fn stop_with(&self, more: &[(&'static str, OsString)], wish: &str) -> Ran {
        self.run_with(more, &["run", "stop", "--wish", wish, "--date", DAY])
    }

    pub fn stop(&self, wish: &str) -> Ran {
        self.stop_with(&[], wish)
    }

    /// Starts `jellymoon run` on a script of the made-up agent's that waits, and returns once the agent has got
    /// as far as its wait: the command, still going, and the run.
    pub fn run_going(
        &self,
        script: (&str, usize),
        more: &[(&'static str, OsString)],
        wish: &str,
        repo: &Path,
    ) -> (Going, String) {
        self.play(script.0, script.1);
        let going = self.going(more, &["run", "--wish", wish, "--date", DAY, "--repo", repo.to_str().unwrap()]);
        wait_for(&self.at("rec").join("started"));
        (going, self.kept(wish)["run"].as_str().unwrap().to_string())
    }

    /// A run whose agent has done something about signals first: a script of the made-up agent's that starts it
    /// again through `perl`. None where no such agent can be started, with the reason said.
    pub fn minded(&self, script: (&str, usize), grace: u64, wish: &str, repo: &Path) -> Option<(Going, String)> {
        self.play(script.0, script.1);
        let args = ["run", "--wish", wish, "--date", DAY, "--repo", repo.to_str().unwrap()];
        let going = self.going(&[ms(GRACE, grace)], &args);
        let deadline = Instant::now() + Duration::from_secs(30);
        while !self.at("rec").join("started").exists() {
            if let Some(why) = self.rec("cannot") {
                assert_eq!(printed(&ended(going))["outcome"], "failed");
                support::skipped(&why);
                return None;
            }
            assert!(Instant::now() < deadline, "the agent never started");
            std::thread::sleep(Duration::from_millis(10));
        }
        Some((going, self.kept(wish)["run"].as_str().unwrap().to_string()))
    }

    /// A run that was cut off: its `jellymoon run` is killed while the made-up agent goes on writing a file in the
    /// copy, which is returned after the run. The agent ends when the test leaves `stop`, if nothing ends it first.
    pub fn cut_off(&self, wish: &str, repo: &Path) -> (String, PathBuf) {
        let (mut going, run) = self.run_going(("scribble", 0), &[], wish, repo);
        going.kill().unwrap();
        going.wait().unwrap();
        let file = self.jm("copies").join(&run).join("scribble.txt");
        assert!(grows(&file), "the agent was to be left editing");
        (run, file)
    }

    /// A cancelled run as it is left: the wish needs the person and has no star, on the page too; the record says
    /// cancelled; the copy is kept, and nothing is committed on the run's branch.
    pub fn is_cancelled(&self, wish: &str, run: &str, repo: &Path, denied: usize) {
        let why = cancelled(run);
        let kept = json!({"id": wish, "said": SAID, "project": "alpha", "state": "needs_you", "stars": [], "run": run, "why": why});
        assert_eq!(self.kept(wish), kept);
        let on_page = self.on_page(wish);
        assert_eq!((&on_page["state"], &on_page["stars"]), (&json!("needs_you"), &json!([])));
        let record = self.record(run);
        assert_eq!(
            (&record["outcome"], &record["why"], &record["denied"]),
            (&json!("cancelled"), &json!(why), &json!(denied))
        );
        assert!(self.jm("copies").join(run).is_dir(), "a stopped run keeps its copy");
        assert_eq!(self.commit(repo, &format!("jellymoon/run-{run}")), self.commit(repo, "HEAD"));
    }
}

/// What a command that is still going printed, if it ends within `seconds`. One that does not is killed: None.
pub fn within(mut going: Going, seconds: u64) -> Option<Ran> {
    let deadline = Instant::now() + Duration::from_secs(seconds);
    while going.try_wait().unwrap().is_none() {
        if Instant::now() > deadline {
            going.kill().unwrap();
            going.wait().unwrap();
            return None;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Some(ended(going))
}

#[test]
fn a_stop_ends_the_run_as_cancelled_with_no_star_and_its_copy_kept() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    w.work_in_progress(&repo);
    let before = w.persons(&repo);
    // A minute after each asking: an agent that is gone sooner heard the first asking. This agent ends at
    // the interrupt and at the stop signal alike, so that the interrupt is the first is not shown here:
    // `deaf::an_agent_deaf_to_the_stop_signal_is_gone_at_the_interrupt_which_comes_first` shows it.
    let (going, run) = w.run_going(("lingers", 0), &[ms(GRACE, 60_000)], &wish, &repo);
    assert_eq!(w.kept(&wish)["state"], "working");

    let from = Instant::now();
    let stopped = w.stop(&wish);
    let took = from.elapsed();
    // The stop and the run it stopped print the same line.
    assert_eq!(printed(&stopped), said_cancelled(&run, &wish, 0));
    assert_eq!(printed(&ended(going)), said_cancelled(&run, &wish, 0));
    assert!(w.agent_came_and_went(), "the agent is still going after the stop");
    if cfg!(unix) {
        assert!(took < Duration::from_secs(40), "the agent was not gone at the first asking: {took:?}");
    }

    w.is_cancelled(&wish, &run, &repo, 0);
    // The copy holds what the agent had done, the person's checkout is as it was, and no other copy is kept.
    let copy = w.jm("copies").join(&run);
    assert_eq!(fs::read_to_string(copy.join("a.txt")).unwrap(), "edited by the run\n");
    assert_eq!((w.persons(&repo), w.copies()), (before, vec![run.clone()]));
    // The request is a file with nothing in it, in the run's own folder.
    assert_eq!(fs::read(w.jm("runs").join(&run).join("stop")).unwrap(), b"");
    w.never_says_the_agent(&stopped);

    // The wish has had its one run: it is not run again.
    assert!(refused(&w.start(&wish, &repo)).contains("has had its run"));
    assert_eq!(w.starts(), 1);
}

// Nothing in a stop allows more than a run was started with: the agent of a run that is being stopped was started
// with the measured command line and nothing else, and no second agent is ever started.
#[test]
fn the_agent_of_a_stopped_run_was_started_with_exactly_the_measured_command_line() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (going, run) = w.run_going(("lingers", 0), &[], &wish, &repo);
    let args = || (0..).map_while(|n| w.rec(&format!("arg{n}"))).collect::<Vec<String>>();
    assert_eq!(args(), ARGS, "while the run goes");
    assert_eq!(printed(&w.stop(&wish)), said_cancelled(&run, &wish, 0));
    assert_eq!(printed(&ended(going))["outcome"], "cancelled");
    assert_eq!((args(), w.starts()), (ARGS.map(str::to_string).to_vec(), 1), "after the stop");
    assert_eq!(w.rec("stdin").unwrap(), format!("{SAID}\n"));
}

#[test]
fn run_stop_takes_a_wish_and_a_date_and_nothing_else() {
    let w = world();
    for args in
        [&["run", "stop"][..], &["run", "stop", "--wish", "m1", "--repo", "x"], &["run", "--wish", "m1", "stop"]]
    {
        let ran = w.run(args);
        assert_eq!((ran.code, ran.stdout.as_str()), (2, ""), "{args:?}: {}", ran.stderr);
        assert!(ran.stderr.contains("jellymoon run stop --wish <id> [--date <YYYY-MM-DD>]"), "{args:?}");
    }
    assert!(!w.at("jm").exists(), "a command that is not one makes nothing");
}
