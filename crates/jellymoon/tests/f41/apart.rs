//! A run that outlives the command that asked for it, and how it ends with that command gone.

use std::ffi::OsString;
use std::fs;
use std::path::Path;

use serde_json::json;

use super::{DAY, SAID, STAR, World, found_world, grows, made, printed, wait_for};
use crate::support::Ran;

impl World {
    /// Starts the run of a wish through `fake-asker` (`support/fake_asker.rs`): a program that calls core's
    /// `run_apart` and exits, and that core starts in place of `jellymoon`, where it starts the real `jellymoon`
    /// with what it was given and leaves that command's exit code in `rec/exit`. What the asker printed.
    fn asked(&self, wish: &str) -> Ran {
        let asker = OsString::from(env!("CARGO_BIN_EXE_fake-asker"));
        let more = [
            ("FAKE_ASKER_JELLYMOON", OsString::from(env!("CARGO_BIN_EXE_jellymoon"))),
            ("FAKE_ASKER_REC", self.at("rec").into()),
        ];
        let vars = self.vars.iter().chain(&more).map(|(name, value)| (*name, value.as_os_str()));
        let logs = self.at("harness").join("projects");
        let mut command = std::process::Command::new(asker);
        command.args(["ask", wish, DAY]).arg(self.at("jm")).arg(logs).env_clear().envs(vars);
        if let Some(root) = std::env::var_os("SystemRoot").filter(|_| cfg!(windows)) {
            command.env("SystemRoot", root); // a Windows program needs it to start
        }
        let done = command.current_dir(self.dir.path()).output().unwrap();
        let text = |bytes: Vec<u8>| String::from_utf8(bytes).unwrap();
        Ran { stdout: text(done.stdout), stderr: text(done.stderr), code: done.status.code().unwrap_or(-1) }
    }

    /// The run a wish names.
    fn run_of(&self, wish: &str) -> String {
        self.kept(wish)["run"].as_str().unwrap().to_string()
    }
}

/// The exit code `fake-asker` left of the `jellymoon run` it started, once that command has ended.
fn exit_of(w: &World) -> String {
    wait_for(&w.at("rec").join("exit"));
    fs::read_to_string(w.at("rec").join("exit")).unwrap()
}

/// What a failed test says of a run: the exit its command left, if it has ended, and the run's record.
fn seen(w: &World, run: &str) -> String {
    let exit = fs::read_to_string(w.at("rec").join("exit")).unwrap_or_else(|_| "none yet".to_string());
    format!("the command's exit: {exit}, the record: {}", w.record(run))
}

fn repo_typed_in(w: &World) -> std::path::PathBuf {
    let repo = w.repo("alpha");
    w.typed_in("-work-alpha", &repo);
    repo
}

#[test]
fn a_wish_made_with_run_starts_a_run_that_goes_on_after_the_command_and_adds_its_star() {
    let w = found_world();
    let repo = repo_typed_in(&w);
    w.play("slow", 0);
    let said = w.wish_run("alpha", &[]);
    // The command has exited, and the run it started goes: the wish names it, and its agent waits.
    let run = said["run"].as_str().expect("no run was started").to_string();
    let why = format!("run {run} is going, in a copy of the repo");
    assert_eq!(said, json!({"wish": made("m1", "alpha"), "run": run, "why": why}));
    wait_for(&w.at("rec").join("started"));
    assert!(!w.free("held") && w.record(&run)["outcome"].is_null(), "the run ended with the command");
    let kept =
        json!({"id": "m1", "said": SAID, "project": "alpha", "state": "working", "stars": [], "run": run, "why": why});
    assert_eq!((w.kept("m1"), &w.on_page("m1")["state"]), (kept, &json!("working")));
    assert_eq!(w.rec("stdin").unwrap(), format!("{SAID}\n"));

    fs::write(w.at("rec").join("go"), "").unwrap();
    assert_eq!(w.ended_as(&run)["outcome"], "completed");
    w.command_gone(&run);
    let on_page = w.on_page("m1");
    assert_eq!((&on_page["state"], &on_page["stars"]), (&json!("done"), &json!([STAR])));
    assert_eq!((w.branches(&repo), w.copies(), w.starts()), (vec![format!("jellymoon/run-{run}")], vec![], 1));
}

// With its asker gone nobody reads what the run's command says: it ends all the same, with the exit it would have
// had, and its record and the wish say how.
#[test]
fn the_command_of_a_run_started_apart_exits_0_when_the_run_completes_after_its_asker_is_gone() {
    let w = found_world();
    repo_typed_in(&w);
    let wish = w.wish("alpha");
    w.play("slow", 0);
    let asked = w.asked(&wish);
    let run = w.run_of(&wish);
    assert_eq!((asked.code, asked.stdout), (0, format!("run {run}\nrun {run} is going, in a copy of the repo\n")));
    assert!(!w.at("rec").join("exit").exists(), "the run's command ended before its asker: {}", seen(&w, &run));
    fs::write(w.at("rec").join("go"), "").unwrap();
    assert_eq!(exit_of(&w), "0");
    let (kept, record) = (w.kept(&wish), w.record(&run));
    assert_eq!(
        (&record["outcome"], &kept["state"], &kept["stars"]),
        (&json!("completed"), &json!("done"), &json!([STAR]))
    );
    assert!(!w.jm("runs").join(&run).join("run.lock").exists(), "the command left its lock");
}

// A failed run's command exits 0 too, as it does with a terminal: the outcome is in what it prints and keeps.
#[test]
fn a_run_started_apart_that_does_not_finish_is_written_down_by_its_own_command() {
    let w = found_world();
    repo_typed_in(&w);
    let wish = w.wish("alpha");
    w.play("not-success", 0);
    let asked = w.asked(&wish);
    assert_eq!(asked.code, 0, "{}", asked.stderr);
    assert_eq!(exit_of(&w), "0");
    let (run, kept) = (w.run_of(&wish), w.kept(&wish));
    assert_eq!(
        (&w.record(&run)["outcome"], &kept["state"], &kept["stars"]),
        (&json!("failed"), &json!("needs_you"), &json!([]))
    );
    assert_eq!(w.copies(), vec![run]);
}

#[test]
fn a_stop_ends_a_run_started_apart_and_everything_it_started() {
    let w = found_world();
    let repo = repo_typed_in(&w);
    w.play("lingers", 0);
    let run = w.wish_run("alpha", &[])["run"].as_str().unwrap().to_string();
    wait_for(&w.at("rec").join("started"));
    let why = format!("run {run} was cancelled by a stop: its copy is kept and no star was added");
    let said = json!({"run": run, "wish": "m1", "outcome": "cancelled", "star": null, "denied": 0, "why": why});
    assert_eq!(printed(&w.run(&["run", "stop", "--wish", "m1", "--date", DAY])), said);
    assert!(w.agent_came_and_went(), "the agent is still going after the stop");
    w.command_gone(&run);
    let (kept, record) = (w.kept("m1"), w.record(&run));
    assert_eq!((&kept["state"], &kept["stars"], &kept["why"]), (&json!("needs_you"), &json!([]), &json!(why)));
    assert_eq!((&record["outcome"], w.copies()), (&json!("cancelled"), vec![run.clone()]));
    assert_eq!(w.commit(&repo, &format!("jellymoon/run-{run}")), w.commit(&repo, "HEAD"));
}

// The command of a run started apart is killed while its agent edits: the next look ends the agent and writes the
// run down as cut off, as it does for a run started with a terminal.
#[test]
fn a_run_started_apart_and_cut_off_is_written_down_by_the_next_look_and_its_agent_ended() {
    let w = found_world();
    repo_typed_in(&w);
    let wish = w.wish("alpha");
    w.play("scribble", 0);
    assert_eq!(w.asked(&wish).code, 0);
    let run = w.run_of(&wish);
    let file: &Path = &w.jm("copies").join(&run).join("scribble.txt");
    wait_for(file);
    assert!(grows(file), "the agent was to be editing: {}", seen(&w, &run));
    fs::write(w.at("rec").join("cut"), "").unwrap(); // `fake-asker` kills the command it started
    assert_ne!(exit_of(&w), "0");
    assert_eq!(w.on_page(&wish)["state"], "needs_you");
    assert!(w.freed("held") && !grows(file), "the agent the run left is still editing");
    let why = format!("run {run} was cut off before it ended: its copy is kept and no star was added");
    assert_eq!((&w.kept(&wish)["why"], &w.record(&run)["outcome"]), (&json!(why), &json!("failed")));
}
