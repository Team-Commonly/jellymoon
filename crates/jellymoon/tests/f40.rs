//! F40: a run can be stopped, and nothing it asks for is allowed unless listed (ADR 0014,
//! `docs/development/run.md`, "A stop the person asks for"). `jellymoon run stop` is run as a black box beside a
//! `jellymoon run` that is going, in F39's test folder (`f39/world.rs`) with the made-up `fake-agent` as the
//! harness, so the real Claude Code is never started. `f40/deaf.rs` holds the agents that do not hear, and what
//! is asked meanwhile; `f40/nothing.rs` a stop with nothing going, a stop left by hand and a locked day;
//! `f40/races.rs` a stop against the run's own finish; `f40/denied.rs` what is counted as denied.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f40/deaf.rs"]
mod deaf;
#[path = "f40/denied.rs"]
mod denied;
#[path = "f40/nothing.rs"]
mod nothing;
#[path = "f40/pipe.rs"]
mod pipe;
#[path = "f40/races.rs"]
mod races;
mod support;
#[path = "f39/world.rs"]
mod world;

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use support::Ran;
pub use world::*;

/// How long a build with debug assertions gives a child after each time it is asked to stop.
pub const GRACE: &str = "JELLYMOON_TEST_GRACE_MS";

pub fn ms(variable: &'static str, ms: u64) -> (&'static str, OsString) {
    (variable, ms.to_string().into())
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
    ) -> (Child, String) {
        self.play(script.0, script.1);
        let going = self.going(more, &["run", "--wish", wish, "--date", DAY, "--repo", repo.to_str().unwrap()]);
        wait_for(&self.at("rec").join("started"));
        (going, self.kept(wish)["run"].as_str().unwrap().to_string())
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
pub fn within(mut going: Child, seconds: u64) -> Option<Ran> {
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
    // Fifteen seconds after each asking: an agent that is gone sooner heard the first one, the interrupt.
    let (going, run) = w.run_going(("lingers", 0), &[ms(GRACE, 15_000)], &wish, &repo);
    assert_eq!(w.kept(&wish)["state"], "working");

    let from = Instant::now();
    let stopped = w.stop(&wish);
    let took = from.elapsed();
    // The stop and the run it stopped print the same line.
    assert_eq!(printed(&stopped), said_cancelled(&run, &wish, 0));
    assert_eq!(printed(&ended(going)), said_cancelled(&run, &wish, 0));
    assert!(w.agent_came_and_went(), "the agent is still going after the stop");
    if cfg!(unix) {
        assert!(took < Duration::from_secs(10), "the agent was not interrupted: {took:?}");
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
