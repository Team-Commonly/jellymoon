//! What the tests of `jellymoon hear` share (F55, and F30's own file): the events as lines, the lines it prints,
//! and a world whose project is a folder something was typed in. Every world is in the system's own temporary
//! folder: Cargo's may be inside a harness worktree of this repo, and a folder there is read as the repo's own.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::{fs, thread};

use serde_json::{Value, json};

use crate::support::{Ran, said, started, tree, write_log};
use crate::world::{DAY, World, world_in};

pub const DOWN: &str = r#"{"key":"down"}"#;
pub const UP: &str = r#"{"key":"up"}"#;
/// What a heard wish says with its words.
pub const TAKE_BACK: &str = "press the key again to take it back";
/// What is said when no project can be picked.
pub const NO_PROJECT: &str = "nothing was kept: make the day's first wish on the page, so that Jellymoon knows which \
                              project a wish said aloud is for";

pub fn tick(ms: u64) -> String {
    json!({ "tick": ms }).to_string()
}

pub fn partial(words: &str) -> String {
    json!({"said": "partial", "words": words}).to_string()
}

pub fn fin(words: &str) -> String {
    json!({"said": "final", "words": words}).to_string()
}

pub fn ear_cannot(why: &str) -> String {
    json!({"said": "cannot", "why": why}).to_string()
}

/// A line `jellymoon hear` prints: the five names every line has.
pub fn jelly(state: &str, says: &str, words: &str, project: &str, left: u64) -> Value {
    json!({"state": state, "says": says, "words": words, "project": project, "left": left})
}

pub fn quiet() -> Value {
    jelly("quiet", "", "", "", 0)
}

pub fn listening(words: &str) -> Value {
    jelly("listening", "", words, "", 0)
}

pub fn heard(words: &str, project: &str, left: u64) -> Value {
    jelly("heard", TAKE_BACK, words, project, left)
}

pub fn cannot(says: &str) -> Value {
    jelly("cannot", says, "", "", 0)
}

/// `line` with one more name.
pub fn with(mut line: Value, name: &str, value: Value) -> Value {
    line[name] = value;
    line
}

/// A hold of a second that ends with `words` as its final words, on a clock that starts at `from`, and the tick
/// that follows them, at the same time: what was heard or refused is counted from it.
pub fn hold(from: u64, words: &str) -> Vec<String> {
    vec![tick(from), DOWN.to_string(), tick(from + 1000), UP.to_string(), fin(words), tick(from + 1000)]
}

/// A world in the system's temporary folder, where nothing was typed yet.
pub fn heard_world() -> World {
    let mut world = world_in(tempfile::tempdir().unwrap());
    // Whoever asks for a run apart is told that it goes after a tenth of a second of a harness's silence.
    world.vars.push(("JELLYMOON_TEST_APART_QUIET_MS", "100".into()));
    world
}

/// A world where something was typed in the folder `work/alpha`, which is no repo.
pub fn alpha_world() -> World {
    let world = heard_world();
    world.typed("alpha");
    world
}

impl World {
    pub fn logs(&self) -> PathBuf {
        self.at("harness").join("projects")
    }

    /// The folder `work/<name>`, made.
    pub fn folder(&self, name: &str) -> PathBuf {
        let folder = self.at("work").join(name);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    /// A session log of one thing typed on the day in `folder`, under a name of its own.
    pub fn typed_at(&self, session: &str, folder: &Path) {
        let line = said("2026-03-10T16:00:00.000Z", folder.to_str().unwrap(), "look at the uploader");
        write_log(&self.logs(), &format!("{session}/s.jsonl"), &[line]);
    }

    /// Something typed on the day in the folder `work/<name>`.
    pub fn typed(&self, name: &str) -> PathBuf {
        let folder = self.folder(name);
        self.typed_at(&format!("-work-{name}"), &folder);
        folder
    }

    /// `jellymoon hear` for the day over the world's logs, with `more` arguments, fed `events` a line each.
    pub fn hear_ran<S: AsRef<str>>(&self, more: &[&str], events: &[S]) -> Ran {
        let input: String = events.iter().map(|event| format!("{}\n", event.as_ref())).collect();
        self.hear_fed(more, &input)
    }

    /// The same, fed `input` as it is.
    pub fn hear_fed(&self, more: &[&str], input: &str) -> Ran {
        let logs = self.logs();
        let args = ["hear", "--date", DAY, "--logs", logs.to_str().unwrap()];
        let vars: Vec<_> = self.vars.iter().map(|(name, value)| (*name, value.as_os_str())).collect();
        started(self.dir.path(), &vars, &[&args[..], more].concat(), input)
    }

    /// The lines `jellymoon hear` printed for `events`: it exited 0 and said nothing else.
    pub fn hear<S: AsRef<str>>(&self, more: &[&str], events: &[S]) -> Vec<Value> {
        lines(&self.hear_ran(more, events))
    }

    /// The day's made wishes as they are kept. None when no file of them is.
    pub fn made_wishes(&self) -> Option<Value> {
        let kept = fs::read_to_string(self.jm("pages").join(format!("{DAY}.wishes.json"))).ok()?;
        Some(serde_json::from_str::<Value>(&kept).unwrap()["wishes"].clone())
    }

    /// Waits for a run's record to have its end and for its command to be gone, half a minute at most.
    pub fn run_ended(&self, wish: &str) -> String {
        let (from, run) = (Instant::now(), self.kept(wish)["run"].as_str().unwrap().to_string());
        let folder = self.jm("runs").join(&run);
        let ended = || {
            let record = fs::read_to_string(folder.join("run.json")).unwrap_or_default();
            !serde_json::from_str::<Value>(&record).unwrap_or_default()["outcome"].is_null()
        };
        // The lock's file of a run that asks stays when its command has gone: gone is a lock nothing holds.
        let held = || fs::File::open(folder.join("run.lock")).is_ok_and(|lock| lock.try_lock().is_err());
        while !ended() || held() {
            assert!(from.elapsed() < Duration::from_secs(30), "run {run} never ended");
            thread::sleep(Duration::from_millis(10));
        }
        run
    }

    /// Jellymoon's home, every file with its bytes. Empty when there is no home.
    pub fn home(&self) -> Vec<(PathBuf, Vec<u8>)> {
        if self.at("jm").exists() { tree(&self.at("jm")) } else { Vec::new() }
    }
}

/// The lines of a `jellymoon hear` that exited 0 and said nothing on stderr.
pub fn lines(ran: &Ran) -> Vec<Value> {
    assert_eq!((ran.code, ran.stderr.as_str()), (0, ""), "{}", ran.stdout);
    assert!(ran.stdout.ends_with('\n'), "{:?}", ran.stdout);
    ran.stdout.lines().map(|line| serde_json::from_str(line).unwrap()).collect()
}

/// A made wish as it is kept with no run.
pub fn made(id: &str, said: &str, project: &str) -> Value {
    json!({"id": id, "said": said, "project": project, "state": "working", "stars": []})
}
