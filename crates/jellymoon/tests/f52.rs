//! F52: `jellymoon run answer` resumes a run that asks with the person's words (ADR 0018, decision 4;
//! `docs/development/run.md`, "An answer"). The command is run as a black box against the made-up `fake-agent`
//! in F39's test folder (`f39/world.rs`), in repos made with the real `git`. This file holds an answer that
//! finishes the run and one that is asked again; `f52/refused.rs` every answer that is refused, with nothing
//! started and nothing written; `f52/ends.rs` the other ends of an answered run, a harness that no longer keeps
//! the session among them; `f52/words.rs` where an answer's words are and are not; and `f52/gate.rs` what F51's
//! gate left.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f52/ends.rs"]
mod ends;
#[path = "f52/gate.rs"]
mod gate;
#[path = "f45/made.rs"]
mod kept;
#[path = "f52/refused.rs"]
mod refused;
mod support;
#[path = "f52/words.rs"]
mod words;
#[path = "f39/world.rs"]
mod world;

use std::ffi::OsString;
use std::fs;
use std::io::{Seek, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub use kept::said;
use serde_json::{Value, json};
use support::{Ran, started, tree};
pub use world::*;

/// What the made-up agent asks when its question is a plain one, and the second question it can ask.
pub const QUESTION: &str = "Which of the two uploaders did you mean?";
pub const SECOND: &str = "Did you mean 'retry' or 'resume'?";
/// An answer in words no other text of a test holds.
pub const ANSWER: &str = "the old one, quince-harbour, in src/upload.rs";
pub const ASKS: &str = "run {run} asks a question: its copy is kept as it is, and nothing was committed";
pub const GOING: &str = "run {run} is going, in a copy of the repo";
pub const FINISHED: &str = "run {run} finished: its work is one commit on the branch jellymoon/run-{run}";
/// The line of a run that did not finish, around why.
pub const FAILED: (&str, &str) = ("run {run} did not finish (", "): its copy is kept and no star was added");

/// A run that asks the plain question, with nothing done in its copy.
pub fn asking() -> Made {
    let w = world();
    w.play("asks", 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let said = printed(&w.start(&wish, &repo));
    assert_eq!((&said["outcome"], &said["question"]), (&json!("asks"), &json!(QUESTION)), "{said}");
    let run = said["run"].as_str().unwrap().to_string();
    Made { w, repo, wish, run }
}

/// What an answered run that did not finish for `kind` prints.
pub fn failed(made: &Made, kind: &str) -> Value {
    let why = format!("{}{kind}{}", FAILED.0.replace("{run}", &made.run), FAILED.1);
    json!({"run": made.run, "wish": made.wish, "outcome": "failed", "star": null, "denied": 0, "why": why})
}

/// Waits for `done`, half a minute at most.
pub fn until(why: &str, mut done: impl FnMut() -> bool) {
    let from = Instant::now();
    while !done() {
        assert!(from.elapsed() < Duration::from_secs(30), "{why}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

impl Made {
    pub fn copy(&self) -> PathBuf {
        self.w.jm("copies").join(&self.run)
    }

    fn asked(&self) -> [&str; 6] {
        ["run", "answer", "--wish", &self.wish, "--date", DAY]
    }

    /// `jellymoon run answer` for the wish, with `words` as its input, and waits for it.
    pub fn answer(&self, words: &str) -> Ran {
        let vars = self.w.vars.iter().map(|(name, value)| (*name, value.as_os_str()));
        started(self.w.dir.path(), &vars.collect::<Vec<_>>(), &self.asked(), words)
    }

    /// The same with `more` variables, started and not waited for. Dropped while it goes, it is killed.
    pub fn answering(&self, more: &[(&'static str, OsString)], words: &str) -> Going {
        let mut fed = tempfile::tempfile().unwrap();
        fed.write_all(words.as_bytes()).unwrap();
        fed.rewind().unwrap();
        Going::new(self.fed(more, Stdio::from(fed)))
    }

    /// The same, with its input from wherever `input` says.
    pub fn fed(&self, more: &[(&'static str, OsString)], input: Stdio) -> Child {
        let mut command = Command::new(env!("CARGO_BIN_EXE_jellymoon"));
        let vars = self.w.vars.iter().chain(more).map(|(name, value)| (name, value));
        command.args(self.asked()).current_dir(self.w.dir.path()).env_clear().envs(vars);
        if let Some(root) = std::env::var_os("SystemRoot").filter(|_| cfg!(windows)) {
            command.env("SystemRoot", root); // a Windows program needs it to start
        }
        command.stdin(input).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap()
    }

    /// The command line the made-up agent was last started with, to go on with a session, with `<id>` where
    /// that session's id stood, and that id.
    pub fn resumed(&self) -> (Vec<String>, String) {
        let mut args: Vec<String> = (0..).map_while(|n| self.w.rec(&format!("arg{n}"))).collect();
        let at = args.iter().position(|arg| arg == "--resume").expect("the agent was not told to resume") + 1;
        let id = std::mem::replace(&mut args[at], "<id>".to_string());
        (args, id)
    }

    /// Every file of the home but the empty file an answer, a keep or a toss makes for the lock they share.
    pub fn home(&self) -> Vec<(PathBuf, Vec<u8>)> {
        tree(&self.w.at("jm")).into_iter().filter(|(path, _)| !path.ends_with("keep.lock")).collect()
    }

    /// Fails unless the answer was refused in these words, with nothing started and nothing in the home or the
    /// repo changed.
    pub fn unanswered(&self, words: &str, why: &str) {
        let before = (self.home(), self.w.persons(&self.repo), self.w.branches(&self.repo), self.w.starts());
        let ran = self.answer(words);
        assert_eq!(refused(&ran), said(&why.replace("{run}", &self.run)), "{words:?}");
        let after = (self.home(), self.w.persons(&self.repo), self.w.branches(&self.repo), self.w.starts());
        assert!(before == after, "an answer that was refused started or changed something: {why}");
    }

    pub fn copy_failed(&self) -> bool {
        let copy = fs::read_to_string(self.w.jm("runs").join(&self.run).join("copy.json")).unwrap();
        serde_json::from_str::<Value>(&copy).unwrap()["failed"].is_u64()
    }
}

#[test]
fn an_answer_goes_on_with_the_run_in_its_copy_under_its_session_and_the_run_finishes() {
    let made = asking();
    let (w, run) = (&made.w, &made.run);
    let (first, session) = w.args();
    let (before, person, start) = (made.record(), w.persons(&made.repo), w.commit(&made.repo, "main"));
    w.play("edits", 0);
    let ran = made.answer(&format!("{ANSWER}\n"));
    let line = FINISHED.replace("{run}", run);
    let done = json!({"run": run, "wish": made.wish, "outcome": "completed", "star": STAR, "denied": 0, "why": line});
    assert_eq!(printed(&ran), done);
    w.never_says_the_agent(&ran);
    // The command line is a run's own, with the session to go on with where the session to start under stood.
    let (args, resumed) = made.resumed();
    let want: Vec<String> = ARGS.iter().map(|arg| arg.replace("--session-id", "--resume")).collect();
    assert_eq!((args.as_slice(), &resumed), (want.as_slice(), &session));
    assert_eq!(first, ARGS, "the first start was a run's");
    // The answer is the harness's whole input, and it was started in the run's own copy.
    assert_eq!(w.rec("stdin").unwrap(), format!("{ANSWER}\n"));
    // The copy is gone by now, as a finished run's is: its folder is named by the folder it was in.
    let stood = PathBuf::from(w.rec("cwd").unwrap());
    let within = fs::canonicalize(stood.parent().unwrap()).unwrap();
    assert_eq!((within, stood.file_name()), (fs::canonicalize(w.jm("copies")).unwrap(), Some(run.as_ref())));
    assert_eq!(w.starts(), 2);
    // The same run: its id, its branch and its record, which names the session and the start it named before.
    let record = made.record();
    for name in ["wish", "day", "started", "repo", "start", "session"] {
        assert_eq!(record[name], before[name], "{name}");
    }
    let end = w.commit(&made.repo, &made.branch());
    assert_eq!((&record["outcome"], &record["why"], &record["end"]), (&json!("completed"), &json!(line), &json!(end)));
    assert_eq!((record.get("question"), record.get("answer")), (None, None));
    assert_eq!(w.git(&made.repo, &["rev-parse", &format!("{end}^")]), start, "one commit on the run's start");
    assert_eq!((w.copies(), w.branches(&made.repo).len()), (vec![], 1));
    // The wish is done, with its star and the answer it was given, and the page shows no answer.
    let wish = json!({"id": made.wish, "said": SAID, "project": "alpha", "state": "done", "stars": [STAR],
        "run": run, "why": line, "answer": ANSWER});
    assert_eq!(w.kept(&made.wish), wish);
    let on_page = w.on_page(&made.wish);
    assert_eq!((&on_page["state"], on_page.get("question"), on_page.get("answer")), (&json!("done"), None, None));
    assert!(!w.jm("runs").join(run).join("run.lock").exists(), "the answer left the run's lock");
    assert!(person == w.persons(&made.repo), "nothing of the person's was changed");
    // Its work is kept as any finished run's is.
    let kept = printed(&made.keep(&[]));
    assert_eq!((&kept["kept"], &kept["onto"]), (&json!(end), &json!("main")));
}

#[test]
fn an_answered_run_that_asks_again_keeps_the_new_question_and_takes_a_second_answer() {
    let made = asking();
    let (w, run) = (&made.w, &made.run);
    let session = w.args().1;
    w.play("asks", 11);
    let ran = made.answer(ANSWER);
    let line = ASKS.replace("{run}", run);
    let asks = json!({"run": run, "wish": made.wish, "outcome": "asks", "star": null, "denied": 0, "why": line,
        "question": SECOND});
    assert_eq!(printed(&ran), asks);
    let record = made.record();
    assert_eq!(
        (&record["outcome"], &record["question"], &record["session"]),
        (&json!("asks"), &json!(SECOND), &json!(session))
    );
    let wish = json!({"id": made.wish, "said": SAID, "project": "alpha", "state": "needs_you", "stars": [],
        "run": run, "why": line, "question": SECOND, "answer": ANSWER});
    assert_eq!(w.kept(&made.wish), wish);
    assert_eq!(w.on_page(&made.wish)["question"], SECOND);
    assert!(made.keeps(QUESTION).is_none(), "the first question is kept nowhere any more");
    assert!(!made.copy_failed() && w.copies() == [run.clone()] && !w.jm("runs").join(run).join("run.lock").exists());
    assert_eq!(w.commit(&made.repo, &made.branch()), w.commit(&made.repo, "main"), "nothing was committed");
    // The second answer goes on with the same session, and takes the first one's place with the wish.
    w.play("edits", 0);
    let second = "the new one, saffron-ledger";
    assert_eq!(printed(&made.answer(second))["outcome"], "completed");
    assert_eq!((made.resumed().1, w.rec("stdin").unwrap(), w.starts()), (session, format!("{second}\n"), 3));
    assert_eq!(w.kept(&made.wish)["answer"], second);
    assert!(made.keeps("quince-harbour").is_none(), "the first answer is kept nowhere any more");
}

impl Made {
    /// The file in the home that holds `words`, if any does. The made-up agent's own folders are not the home.
    pub fn keeps(&self, words: &str) -> Option<PathBuf> {
        let found = |bytes: &[u8]| bytes.windows(words.len()).any(|window| window == words.as_bytes());
        tree(&self.w.at("jm")).into_iter().find(|(_, bytes)| found(bytes)).map(|(path, _)| path)
    }
}
