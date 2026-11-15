//! The run `--run` starts for a heard wish, and what the day's runs give the jellyfish to show with no hearing
//! at all. A hearing that goes on is fed an event at a time here, and each line is read as it is printed.

use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};
use std::{fs, thread};

use serde_json::{Value, json};

use super::*;

const ASKS: &str = "a run asks, and waits for your answer on the page";
const WAITS: &str = "a run's work waits for you on the page";

fn working(going: usize) -> Value {
    with(jelly("working", "", "", "", 0), "going", json!(going))
}

/// A `jellymoon hear` that goes on: events are written to it one at a time.
pub struct Hearing {
    going: Going,
    events: Option<ChildStdin>,
    printed: Receiver<String>,
}

impl World {
    /// A world whose project `alpha` is a repo something was typed in.
    fn with_repo(self) -> World {
        let repo = self.repo("alpha");
        self.typed_at("-work-alpha", &repo);
        self
    }

    pub fn hearing(&self, more: &[&str]) -> Hearing {
        let logs = self.logs();
        let mut command = Command::new(env!("CARGO_BIN_EXE_jellymoon"));
        command.args(["hear", "--date", DAY, "--logs", logs.to_str().unwrap()]).args(more);
        command.current_dir(self.dir.path()).env_clear().envs(self.vars.iter().map(|(name, value)| (name, value)));
        if let Some(root) = std::env::var_os("SystemRoot").filter(|_| cfg!(windows)) {
            command.env("SystemRoot", root); // a Windows program needs it to start
        }
        let mut child = command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
        let (events, lines) = (child.stdin.take(), BufReader::new(child.stdout.take().unwrap()));
        let (send, printed) = channel();
        thread::spawn(move || lines.lines().map_while(Result::ok).all(|line| send.send(line).is_ok()));
        Hearing { going: Going::new(child), events, printed }
    }

    /// The first line a new `jellymoon hear` prints: what the day's made wishes give, with no event at all.
    fn at_rest(&self) -> Value {
        let printed = self.hear(&[], &[] as &[&str]);
        assert_eq!(printed.len(), 1);
        printed[0].clone()
    }
}

impl Hearing {
    pub fn feed(&mut self, event: &str) {
        writeln!(self.events.as_mut().unwrap(), "{event}").unwrap();
    }

    /// The next line it printed. It is waited for half a minute.
    pub fn line(&self) -> Value {
        let line = self.printed.recv_timeout(Duration::from_secs(30)).expect("no line was printed");
        serde_json::from_str(&line).unwrap()
    }

    /// Ends its input: it exits 0, having printed nothing more.
    pub fn end(mut self) {
        drop(self.events.take());
        let from = Instant::now();
        let status = loop {
            if let Some(status) = self.going.try_wait().unwrap() {
                break status;
            }
            assert!(from.elapsed() < Duration::from_secs(30), "jellymoon hear did not end with its input");
            thread::sleep(Duration::from_millis(5));
        };
        assert!(status.success());
        assert_eq!(self.printed.recv_timeout(Duration::from_secs(30)).ok(), None, "a line nobody asked for");
    }
}

#[test]
fn with_run_the_wish_s_run_is_started_when_the_window_has_passed_and_not_before() {
    let w = heard_world().with_repo();
    w.play("slow", 0);
    let mut hearing = w.hearing(&["--run"]);
    assert_eq!(hearing.line(), quiet());
    for event in hold(0, SAID) {
        hearing.feed(&event);
    }
    assert_eq!((hearing.line(), hearing.line()), (listening(""), heard(SAID, "alpha", 4000)));
    hearing.feed(&tick(4999));
    assert_eq!(hearing.line(), heard(SAID, "alpha", 1));
    assert!(!w.at("jm").exists() && w.starts() == 0, "something was made inside the window");
    hearing.feed(&tick(5000));
    assert_eq!(hearing.line(), with(working(1), "made", json!("m1")));
    wait_for(&w.at("rec").join("started"));
    let kept = w.kept("m1");
    assert_eq!((w.starts(), &kept["said"], &kept["state"]), (1, &json!(SAID), &json!("working")));
    // The run goes on as one `jellymoon wish --run` started does: it is the wish's own, in a copy of the repo.
    let run = kept["run"].as_str().unwrap();
    assert_eq!((w.copies(), w.record(run)["wish"].as_str()), (vec![run.to_string()], Some("m1")));
    // It is looked at again once a second of the ticks, and at no other time: the run ends, and a tick says so.
    w.let_end("go");
    let ended = w.run_ended("m1");
    assert_eq!(ended, run);
    hearing.feed(&tick(5999));
    hearing.feed(&tick(6000));
    assert_eq!(hearing.line(), jelly("waits", WAITS, "", "", 0));
    assert_eq!(w.kept("m1")["stars"], json!([STAR]));
    hearing.end();
}

#[test]
fn without_run_no_harness_is_started_and_with_no_harness_the_wish_is_kept_and_the_line_says_why() {
    let mut w = heard_world().with_repo();
    fs::create_dir(w.at("empty")).unwrap();
    w.vars.push(("PATH", w.at("empty").into()));
    let mut events = hold(0, SAID);
    events.push(tick(5000));
    // A sentence of the hearing's own, short enough for the bubble: where the program was looked for is what
    // `jellymoon wish --run` says.
    let why = "the wish is kept, but no run was started: no `claude` program was found (JELLYMOON_CLAUDE names one)";
    let printed = w.hear(&["--run"], &events);
    assert_eq!(printed[3..], [with(cannot(why), "made", json!("m1"))]);
    assert_eq!(w.made_wishes(), Some(json!([made("m1", SAID, "alpha")])), "the wish is kept as it was made");
    // What the tick that made the wish refused is said for six seconds from that tick.
    assert!(w.starts() == 0 && !w.jm("copies").exists());
    let said_at = |at: u64| {
        let mut w = heard_world().with_repo();
        fs::create_dir(w.at("empty")).unwrap();
        w.vars.push(("PATH", w.at("empty").into()));
        w.hear(&["--run"], &[&events[..], &[tick(at)]].concat())[4..].to_vec()
    };
    assert_eq!((said_at(10_999), said_at(11_000)), (vec![], vec![quiet()]));
}

// A run that was named and had ended by the time its asker was answered is said as `jellymoon wish --run` says
// it: the wish is kept, and the line carries how its run ended. A run is said to be going once its harness began
// (`run/apart.rs`), so the made-up agent's stream here does not start as a run's: it never began, and it is ended.
#[test]
fn with_run_a_run_that_ended_at_once_is_said_as_it_ended() {
    let mut events = hold(0, SAID);
    events.push(tick(5000));
    let mut w = heard_world().with_repo();
    // No answer comes for a run that goes and has said nothing, so one that ended is not said to go.
    w.vars.push(("JELLYMOON_TEST_APART_QUIET_MS", "30000".into()));
    w.play("first", 13);
    let printed = w.hear(&["--run"], &events);
    w.run_ended("m1");
    let kept = w.kept("m1");
    let why = kept["why"].as_str().expect("the wish says how its run ended");
    assert_eq!((&kept["state"], why.contains("run r")), (&json!("needs_you"), true), "{why}");
    assert_eq!(printed[3..], [with(cannot(why), "made", json!("m1"))]);
}

#[test]
fn the_day_s_runs_give_working_asks_and_waits_with_no_hearing_at_all() {
    let w = heard_world().with_repo();
    assert_eq!(w.at_rest(), quiet());
    // A run that goes.
    w.play("slow", 0);
    let wished = ["wish", "--date", DAY, "--say", SAID, "--project", "alpha", "--run"];
    let logs = w.logs();
    let wish_run = || printed(&w.run(&[&wished[..], &["--logs", logs.to_str().unwrap()]].concat()));
    assert_eq!(wish_run()["wish"]["id"], "m1");
    assert_eq!(w.at_rest(), working(1));
    // A run that finished with work of its own: the work waits, until it is tossed.
    w.let_end("go");
    w.run_ended("m1");
    let before = w.home();
    assert_eq!(w.at_rest(), jelly("waits", WAITS, "", "", 0));
    assert!(w.home() == before, "a look wrote something");
    let toss = ["run", "toss", "--wish", "m1", "--date", DAY, "--logs", logs.to_str().unwrap()];
    assert_eq!(w.run(&toss).code, 0);
    assert_eq!(w.at_rest(), quiet());
    // A run that asks is said before it.
    w.play("asks", 0);
    assert_eq!(wish_run()["wish"]["id"], "m2");
    w.run_ended("m2");
    let question = w.kept("m2")["question"].clone();
    assert_eq!(question, "Which of the two uploaders did you mean?");
    assert_eq!(w.at_rest(), with(jelly("asks", ASKS, "", "", 0), "question", question.clone()));
    // What is heard is shown before any of it, and what is refused too.
    let mut events = vec![tick(0), DOWN.to_string(), tick(1000), UP.to_string(), fin(SAID), ear_cannot("failed")];
    events.extend([tick(1000), tick(5000), tick(7000)]);
    let asks = with(jelly("asks", ASKS, "", "", 0), "question", question);
    let want = [
        asks.clone(),
        listening(""),
        heard(SAID, "alpha", 4000),
        with(cannot("nothing was heard: the recognizer failed"), "made", json!("m3")),
        asks,
    ];
    assert_eq!(w.hear(&[], &events), want);
}

#[test]
fn runs_going_are_counted_only_work_of_a_run_s_own_that_was_not_kept_waits_and_the_first_question_is_shown() {
    let w = heard_world().with_repo();
    let logs = w.logs();
    let logs = logs.to_str().unwrap();
    let wish_run = || {
        let wished = ["wish", "--date", DAY, "--say", SAID, "--project", "alpha", "--run", "--logs", logs];
        printed(&w.run(&wished))["wish"]["id"].clone()
    };
    // A run that finished and changed nothing left no work to wait.
    w.play("nothing", 0);
    assert_eq!(wish_run(), "m1");
    w.run_ended("m1");
    assert_eq!((&w.kept("m1")["state"], w.at_rest()), (&json!("done"), quiet()));
    // Two runs that go are two.
    w.play("slow", 0);
    assert_eq!((wish_run(), wish_run()), (json!("m2"), json!("m3")));
    assert_eq!(w.at_rest(), working(2));
    fs::write(w.at("rec").join("go"), "").unwrap();
    w.run_ended("m2");
    w.run_ended("m3");
    // Work that was kept does not wait, and work that was tossed does not either.
    assert_eq!(w.run(&["run", "keep", "--wish", "m2", "--date", DAY, "--logs", logs]).code, 0);
    assert_eq!((&w.kept("m2")["state"], w.at_rest()), (&json!("done"), jelly("waits", WAITS, "", "", 0)));
    assert_eq!(w.run(&["run", "toss", "--wish", "m3", "--date", DAY, "--logs", logs]).code, 0);
    assert_eq!(w.at_rest(), quiet());
    // The first run that asks is the one whose question is shown: a later run that failed has none.
    w.play("asks", 0);
    assert_eq!(wish_run(), "m4");
    w.run_ended("m4");
    w.play("not-success", 0);
    assert_eq!(wish_run(), "m5");
    w.run_ended("m5");
    let (asked, failed) = (w.kept("m4"), w.kept("m5"));
    assert_eq!((&failed["state"], &failed["question"]), (&json!("needs_you"), &Value::Null));
    assert_eq!(w.at_rest(), with(jelly("asks", ASKS, "", "", 0), "question", asked["question"].clone()));
}
