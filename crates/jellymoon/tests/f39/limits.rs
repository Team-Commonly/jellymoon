//! A run that never ends, one deaf to the stop signal, a `jellymoon run` that is itself cut off and the agent it
//! leaves, and two runs at once for one wish. The
//! made-up agent holds the system's lock on its `held` file for as long as it lives: a test that can take the lock
//! knows the agent is gone, with no clock in the proof.

use std::ffi::OsString;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use std::{fs, thread};

use serde_json::json;

use super::{DAY, SAID, STAR, World, ran, refused, world};
use crate::support::skipped;

const NO_SIGNAL: &str = "this system has no signal to stop another command's child with, and nothing is stopped";

impl World {
    /// Whether the made-up agent got as far as its wait, and is gone now.
    pub fn agent_came_and_went(&self) -> bool {
        self.at("rec").join("started").is_file() && self.free("held")
    }

    /// Whether nothing holds the system's lock on a file the made-up agent, or what it left, locks while it lives.
    pub fn free(&self, held: &str) -> bool {
        fs::File::open(self.at("rec").join(held)).unwrap().try_lock().is_ok()
    }

    /// Waits for a lock of the made-up agent's to be free, five seconds at most: a kill is not felt at once.
    pub fn freed(&self, held: &str) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !self.free(held) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        self.free(held)
    }

    /// Leaves the file the made-up agent waits for, and waits until the agent is gone, half a minute at most.
    pub fn let_end(&self, file: &str) {
        fs::write(self.at("rec").join(file), "").unwrap();
        let deadline = Instant::now() + Duration::from_secs(30);
        while !self.agent_came_and_went() {
            assert!(Instant::now() < deadline, "the agent left behind never ended");
            thread::sleep(Duration::from_millis(10));
        }
    }

    /// Starts `jellymoon run` and returns at once, with the command still going.
    pub fn start_going(&self, wish: &str, repo: &Path) -> Child {
        let mut command = Command::new(env!("CARGO_BIN_EXE_jellymoon"));
        command.args(["run", "--wish", wish, "--date", DAY, "--repo", repo.to_str().unwrap()]);
        command.current_dir(self.dir.path()).env_clear().envs(self.vars.iter().map(|(name, value)| (name, value)));
        if let Some(root) = std::env::var_os("SystemRoot").filter(|_| cfg!(windows)) {
            command.env("SystemRoot", root); // a Windows program needs it to start
        }
        command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap()
    }
}

/// Waits for a file the made-up agent leaves, half a minute at most.
pub fn wait_for(file: &Path) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !file.exists() {
        assert!(Instant::now() < deadline, "{} never came", file.display());
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn a_run_that_never_ends_is_stopped_at_its_limit_and_has_failed() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    w.play("forever", 0);
    // Three seconds: far more than the agent takes to start, far less than the minute it sleeps.
    let short = [("JELLYMOON_TEST_RUN_LIMIT_MS", OsString::from("3000"))];
    let stopped = w.run_with(&short, &["run", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()]);
    let (run, said) = ran(&stopped);
    let why = format!(
        "run {run} did not finish (it was stopped at its time limit of 3 seconds): its copy is kept and no star was added"
    );
    assert_eq!(said, json!({"wish": wish, "outcome": "failed", "star": null, "denied": 0, "why": why}));
    assert!(w.agent_came_and_went(), "the agent that never ends is still running");
    assert_eq!((w.copies(), &w.kept(&wish)["state"]), (vec![run], &json!("needs_you")));
}

#[test]
fn a_run_cut_off_with_its_command_is_not_shown_as_going() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    w.play("slow", 0);
    let mut command = w.start_going(&wish, &repo);
    wait_for(&w.at("rec").join("started"));
    // While the run goes the wish says so, and the page shows it working.
    let going = w.kept(&wish);
    let run = going["run"].as_str().unwrap().to_string();
    assert_eq!(going["why"], json!(format!("run {run} is going, in a copy of the repo")));
    assert_eq!((&going["state"], &w.on_page(&wish)["state"]), (&json!("working"), &json!("working")));
    assert_eq!(w.record(&run)["outcome"], json!(null));

    command.kill().unwrap();
    command.wait().unwrap();
    // Nothing was written since. The page no longer shows a run going: the wish needs the person.
    assert_eq!(w.kept(&wish), going);
    assert_eq!(w.on_page(&wish)["state"], "needs_you");
    // The agent it left behind is stopped by that page where the system can; elsewhere it ends when it next prints.
    w.let_end("go");

    // The wish has had its run. Asking again says so, and writes down that the run was cut off.
    let again = w.start(&wish, &repo);
    assert_eq!(refused(&again), format!("jellymoon: wish {wish} has had its run, {run}: a made wish gets one run\n"));
    let why = format!("run {run} was cut off before it ended: its copy is kept and no star was added");
    let kept = json!({"id": wish, "said": SAID, "project": "alpha", "state": "needs_you", "stars": [], "run": run, "why": why});
    assert_eq!((w.kept(&wish), &w.record(&run)["outcome"]), (kept, &json!("failed")));
    assert_eq!((w.copies(), w.starts()), (vec![run], 1));
}

/// A `jellymoon run` killed while its agent keeps editing the copy and prints nothing. Returns the run and the file
/// the agent writes, once that file has been seen to grow after the kill.
fn left_editing(w: &World, wish: &str, repo: &Path) -> (String, std::path::PathBuf) {
    w.play("scribble", 0);
    let mut command = w.start_going(wish, repo);
    wait_for(&w.at("rec").join("started"));
    let run = w.kept(wish)["run"].as_str().unwrap().to_string();
    command.kill().unwrap();
    command.wait().unwrap();
    let file = w.jm("copies").join(&run).join("scribble.txt");
    assert!(grows(&file) && !w.agent_came_and_went(), "the agent was to be left editing");
    (run, file)
}

pub fn grows(file: &Path) -> bool {
    let size = || fs::metadata(file).map_or(0, |meta| meta.len());
    let before = size();
    thread::sleep(Duration::from_millis(400));
    size() > before
}

#[test]
fn the_agent_a_killed_run_left_editing_is_stopped_by_the_next_page() {
    if !cfg!(unix) {
        return skipped(NO_SIGNAL);
    }
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (run, file) = left_editing(&w, &wish, &repo);
    let record = w.record(&run);
    assert!(record["pid"].is_u64() && record["mark"].is_string(), "{record}");

    assert_eq!(w.on_page(&wish)["state"], "needs_you");
    // The agent's process is gone, by the lock it held, and the file it was writing has stopped changing.
    assert!(w.freed("held"), "the agent is still running after the page");
    assert!(!grows(&file), "the copy is still being edited");
    let why = format!("run {run} was cut off before it ended: its copy is kept and no star was added");
    let kept = json!({"id": wish, "said": SAID, "project": "alpha", "state": "needs_you", "stars": [], "run": run, "why": why});
    assert_eq!((w.kept(&wish), &w.record(&run)["outcome"]), (kept, &json!("failed")));
    assert_eq!((w.copies(), w.starts()), (vec![run], 1));
}

// A record is a file that whoever can write the home can write. One that says a cut-off run was never started has
// its copy taken back, so the agent that run left is stopped first: no agent outlives its copy, and no second run
// is started beside it.
#[test]
fn the_agent_of_a_cut_off_run_whose_record_says_never_started_is_stopped_with_its_copy_taken_back() {
    if !cfg!(unix) {
        return skipped(NO_SIGNAL);
    }
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (run, _) = left_editing(&w, &wish, &repo);
    let mut record = w.record(&run);
    record["outcome"] = json!("not_started");
    fs::write(w.jm("runs").join(&run).join("run.json"), format!("{record}\n")).unwrap();

    assert_eq!((&w.on_page(&wish)["state"], &w.kept(&wish)["run"]), (&json!("working"), &json!(null)));
    assert!(w.freed("held"), "the agent is still running after its copy was taken back");
    assert_eq!((w.copies(), w.branches(&repo), w.starts()), (vec![], vec![], 1));
}

// A pid is given out again. One whose process did not start when the record says is another process: never signalled.
#[test]
fn a_pid_that_is_another_process_by_now_is_never_signalled() {
    if !cfg!(unix) {
        return skipped(NO_SIGNAL);
    }
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let (run, file) = left_editing(&w, &wish, &repo);
    // The record as it would be had the run's child ended and its pid gone to this process.
    let mut record = w.record(&run);
    record["mark"] = json!("ps Thu Jan  1 00:00:00 1970");
    fs::write(w.jm("runs").join(&run).join("run.json"), format!("{record}\n")).unwrap();

    assert_eq!(w.on_page(&wish)["state"], "needs_you");
    assert_eq!((&w.kept(&wish)["state"], &w.record(&run)["outcome"]), (&json!("needs_you"), &json!("failed")));
    assert!(grows(&file) && !w.agent_came_and_went(), "a process that is not the recorded one was signalled");
    // A record with no mark at all is no better known.
    record["mark"] = json!(null);
    fs::write(w.jm("runs").join(&run).join("run.json"), format!("{record}\n")).unwrap();
    assert!(refused(&w.start(&wish, &repo)).contains("has had its run"));
    assert!(grows(&file) && !w.agent_came_and_went());
    w.let_end("stop");
}

#[test]
fn an_agent_deaf_to_the_stop_signal_is_killed_after_its_five_seconds() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    w.play("deaf", 0);
    // Three seconds, so that the agent has made itself deaf before it is asked, also on a machine that is busy.
    let short = [("JELLYMOON_TEST_RUN_LIMIT_MS", OsString::from("3000"))];
    let from = Instant::now();
    let stopped = w.run_with(&short, &["run", "--wish", &wish, "--date", DAY, "--repo", repo.to_str().unwrap()]);
    let took = from.elapsed();
    if let Some(why) = w.rec("cannot") {
        return skipped(&why);
    }
    let (run, said) = ran(&stopped);
    let why = format!(
        "run {run} did not finish (it was stopped at its time limit of 3 seconds): its copy is kept and no star was added"
    );
    assert_eq!((&said["outcome"], &said["why"]), (&json!("failed"), &json!(why)));
    // It was asked, was given its five seconds, and was then killed: the agent sleeps for a minute.
    assert!(took >= Duration::from_secs(8) && took < Duration::from_secs(30), "{took:?}");
    assert!(w.agent_came_and_went(), "the deaf agent is still running");
}

#[test]
fn two_runs_started_at_once_for_one_wish_are_one_run() {
    let w = world();
    let repo = w.repo("alpha");
    for round in 0..4 {
        let wish = w.wish("alpha");
        let start = || w.start(&wish, &repo);
        let (one, two) = thread::scope(|both| {
            let (one, two) = (both.spawn(start), both.spawn(start));
            (one.join().unwrap(), two.join().unwrap())
        });
        let (done, late) = if one.code == 0 { (one, two) } else { (two, one) };
        let (run, said) = ran(&done);
        assert_eq!((&said["outcome"], &said["star"]), (&json!("completed"), &json!(STAR)), "round {round}");
        assert!(refused(&late).contains("has had its run"), "round {round}: {}", late.stderr);
        // One agent was started, one branch is new, and no copy or record of the other is left.
        assert_eq!((w.starts(), w.branches(&repo).len(), w.copies()), (round + 1, round + 1, vec![]), "round {round}");
        assert_eq!(w.kept(&wish)["run"], json!(run), "round {round}");
        let runs = fs::read_dir(w.jm("runs")).unwrap().map(|entry| entry.unwrap().file_name().into_string().unwrap());
        assert_eq!(runs.filter(|name| name.starts_with('r')).count(), round + 1, "round {round}");
    }
}
