//! F63: a day nothing was typed on has a page of its made wishes (ADR 0020, decision 5; `docs/development/page.md`).
//! `jellymoon page` is run as a black box in F39's test folder (`f39/world.rs`): the made-up `fake-agent` stands
//! as `claude` on `PATH` and records every start, so a reading call that was made is seen, and the real Claude
//! Code is never started. Every world is in the system's own temporary folder, as F62's are. The logs are not
//! there, hold nothing, or hold only another day. What the app places of such a day is `app/tests/f63.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f55/heard.rs"]
mod heard;
mod support;
#[path = "f39/world.rs"]
mod world;

use std::fs;
use std::path::{Path, PathBuf};

pub use heard::*;
use serde_json::json;
use support::{Ran, said, started, write_log};
pub use world::*;

/// What `jellymoon page` prints of a day whose logs were read and hold no turn of the day.
const EMPTY: &str = "{\"day\":\"2026-03-10\",\"wishes\":[],\"clashes\":[],\"read\":{\"turns\":0,\"projects\":[]}}\n";
/// The same of a day with no logs at all, when something is made or on offer.
const NO_LOGS: &str =
    "{\"day\":\"2026-03-10\",\"wishes\":[],\"clashes\":[],\"read\":{\"turns\":0,\"projects\":[],\"no_logs\":true}}\n";
const QUESTION: &str = "Which of the two uploaders did you mean?";

/// What the day's session logs are.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Logs {
    Absent,
    Empty,
    OtherDay,
}

const EVERY: [Logs; 3] = [Logs::Absent, Logs::Empty, Logs::OtherDay];

/// The logs folder a command is given, made as `logs` says: none, one that holds nothing, or one whose only
/// session was typed in `work/alpha` two days before.
fn logs(w: &World, logs: Logs) -> PathBuf {
    match logs {
        // The folder nobody named: where Claude Code keeps its logs, by the world's `CLAUDE_CONFIG_DIR`.
        Logs::Absent => return w.logs(),
        Logs::Empty => {}
        Logs::OtherDay => {
            let folder = w.at("work").join("alpha");
            let line = said("2026-03-08T16:00:00.000Z", folder.to_str().unwrap(), "look at the uploader");
            write_log(&w.at("logs"), "-work-alpha/s.jsonl", &[line]);
        }
    }
    w.at("logs")
}

fn page(w: &World, logs: &Path) -> Ran {
    w.run(&["page", "--date", DAY, "--logs", logs.to_str().unwrap()])
}

/// A world with the repo `work/alpha`, listed as a project Jellymoon knows.
fn listed() -> (World, PathBuf) {
    let w = heard_world();
    let repo = w.repo("alpha");
    printed(&w.run(&["project", "add", "--repo", repo.to_str().unwrap(), "--date", DAY]));
    (w, repo)
}

#[test]
fn a_day_with_nothing_typed_nothing_made_and_no_project_on_offer_answers_as_it_did() {
    for how in [Logs::Empty, Logs::OtherDay] {
        let w = heard_world();
        w.repo("alpha"); // a repo that is on no list is not on offer
        let ran = page(&w, &logs(&w, how));
        assert_eq!((ran.code, ran.stdout.as_str(), ran.stderr.as_str()), (0, EMPTY, ""), "{how:?}");
        assert_eq!(w.starts(), 0, "{how:?}: a reading call was made");
    }
    // No logs at all: the one line that says so, as before, whether the list is empty or holds a folder that
    // is gone.
    let (w, repo) = listed();
    fs::remove_dir_all(&repo).unwrap();
    for w in [heard_world(), w] {
        let ran = page(&w, &logs(&w, Logs::Absent));
        let why = refused(&ran);
        assert!(why.starts_with("jellymoon: no Claude Code session logs were found: there is no folder "), "{why}");
        assert!(why.ends_with("or name its projects folder with --logs\n"), "{why}");
        assert_eq!(w.starts(), 0);
    }
    // Made wishes that cannot be used are no made wish: with no project on offer the answer is the reader's.
    let w = heard_world();
    fs::create_dir_all(w.jm("pages")).unwrap();
    fs::write(w.jm("pages").join(format!("{DAY}.wishes.json")), "not the made wishes of any day\n").unwrap();
    let ran = page(&w, &logs(&w, Logs::Absent));
    let why = refused(&ran);
    assert!(why.starts_with("jellymoon: no Claude Code session logs were found: there is no folder "), "{why}");
}

#[test]
fn a_project_on_offer_gives_the_day_a_page_with_no_wish_and_no_call() {
    for how in EVERY {
        let (w, _) = listed();
        let logs = logs(&w, how);
        let before = w.home();
        let ran = page(&w, &logs);
        let wanted = if how == Logs::Absent { NO_LOGS } else { EMPTY };
        assert_eq!((ran.code, ran.stdout.as_str(), ran.stderr.as_str()), (0, wanted, ""), "{how:?}");
        // Nothing is kept of it, so the next call is the same bytes.
        assert_eq!(page(&w, &logs).stdout, wanted, "{how:?}");
        assert_eq!(w.home(), before, "{how:?}: `page` wrote into Jellymoon's home");
        assert_eq!(w.starts(), 0, "{how:?}: a reading call was made");
    }
}

#[test]
fn a_made_wish_alone_gives_the_day_its_page() {
    let (w, _) = listed();
    w.wish("alpha");
    printed(&w.run(&["project", "forget", "alpha"]));
    let page = printed(&page(&w, &logs(&w, Logs::Absent)));
    assert_eq!(page["wishes"].as_array().map(Vec::len), Some(1), "{page}");
    assert_eq!(page["read"], json!({"turns": 0, "projects": [], "no_logs": true}));
    // Only "no logs" is answered with a page: a list of folders switched off that cannot be read still stops it.
    fs::write(w.jm("skip.txt"), b"alpha\n\xff").unwrap();
    let ran = self::page(&w, &logs(&w, Logs::Absent));
    let why = refused(&ran);
    assert!(why.contains("skip.txt") && !why.contains("no_logs"), "{why}");
    assert_eq!(w.starts(), 0);
}

/// Makes a wish for `alpha` with the agent playing `script`, runs it over `logs` when an outcome is given, and
/// returns its id.
fn wished(w: &World, logs: &Path, script: &str, outcome: Option<&str>) -> String {
    w.play(script, 0);
    let id = w.wish("alpha");
    if let Some(outcome) = outcome {
        let ran = printed(&w.run(&["run", "--wish", &id, "--date", DAY, "--logs", logs.to_str().unwrap()]));
        assert_eq!(ran["outcome"], outcome, "{ran}");
    }
    id
}

#[test]
fn the_made_wishes_of_such_a_day_are_printed_in_their_states_as_on_a_day_whose_logs_were_read() {
    let (w, _) = listed();
    let none = logs(&w, Logs::Absent);
    // Five wishes, each run found by the list over logs that are not there: one whose work waits, one with no
    // run, one that asks, one whose work was tossed and one whose work was kept.
    wished(&w, &none, "edits", Some("completed"));
    wished(&w, &none, "edits", None);
    wished(&w, &none, "asks", Some("asks"));
    for (verb, wish) in [("toss", "m4"), ("keep", "m5")] {
        assert_eq!(wished(&w, &none, "edits", Some("completed")), wish);
        printed(&w.run(&["run", verb, "--wish", wish, "--date", DAY, "--logs", none.to_str().unwrap()]));
    }
    let (starts, before) = (w.starts(), w.home());
    let ran = page(&w, &none);
    let page_none = printed(&ran);
    let wish = |id: &str, state: &str, stars: usize| {
        let stars = vec![STAR; stars];
        json!({"id": id, "label": SAID, "said": SAID, "project": "alpha", "state": state, "stars": stars})
    };
    let mut asks = wish("m3", "needs_you", 0);
    asks["question"] = QUESTION.into();
    let wishes =
        json!([wish("m1", "done", 1), wish("m2", "working", 0), asks, wish("m4", "let_go", 1), wish("m5", "done", 1)]);
    let read = json!({"turns": 0, "projects": [], "no_logs": true});
    assert_eq!(page_none, json!({"day": DAY, "wishes": wishes, "clashes": [], "read": read}));
    // A day whose logs were read and hold nothing prints the same wishes, and does not say that no logs were found.
    for how in [Logs::Empty, Logs::OtherDay] {
        let read = printed(&page(&w, &logs(&w, how)));
        assert_eq!(read["wishes"], page_none["wishes"], "{how:?}");
        assert_eq!(read["read"], json!({"turns": 0, "projects": []}), "{how:?}");
    }
    assert_eq!(page(&w, &none).stdout, ran.stdout, "a later call is the same bytes");
    assert_eq!((w.starts(), w.home()), (starts, before), "`page` started the agent, or wrote into the home");
}

#[test]
fn a_model_that_is_no_name_is_refused_on_such_a_day_as_on_any() {
    let (w, _) = listed();
    let args = |logs: &PathBuf| {
        let logs = logs.to_str().unwrap().to_string();
        ["page", "--date", DAY, "--model", "-p", "--logs"].map(str::to_string).into_iter().chain([logs])
    };
    let on = |how: Logs| {
        let args: Vec<String> = args(&logs(&w, how)).collect();
        w.run(&args.iter().map(String::as_str).collect::<Vec<_>>())
    };
    let (none, empty) = (on(Logs::Absent), on(Logs::Empty));
    assert_eq!(refused(&none), refused(&empty));
    assert_eq!(w.starts(), 0);
}

#[test]
fn delivery_and_the_hook_are_as_they_were_on_such_a_day() {
    let (w, _) = listed();
    let none = logs(&w, Logs::Absent);
    w.wish("alpha");
    let status = || printed(&w.run(&["hook", "status"]));
    let (was, before) = (status(), w.home());
    assert_eq!(was, json!({"installed": false, "changed": false}));
    assert_eq!(printed(&page(&w, &none))["wishes"].as_array().map(Vec::len), Some(1));
    // A step of a session on that day is told nothing: no ruling waits, and a page is no ruling.
    let step = json!({"session_id": "s1", "transcript_path": "/made/up.jsonl", "cwd": "/work/alpha",
        "hook_event_name": "PostToolUse"});
    let vars: Vec<_> = w.vars.iter().map(|(name, value)| (*name, value.as_os_str())).collect();
    let told = started(w.dir.path(), &vars, &["hook"], &step.to_string());
    assert_eq!((told.code, told.stdout.as_str(), told.stderr.as_str()), (0, "", ""));
    assert_eq!((status(), w.home()), (was, before));
}
