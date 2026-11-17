//! F76: what another program sent to a headless Claude Code is not on a person's page as their words (ADR 0021,
//! decision 4; `docs/development/reader.md`, "What counts as typed by a person"). `jellymoon day`, `page`, `run`
//! and `hear` are run as black boxes in F39's test folder (`f39/world.rs`), each world in the system's own
//! temporary folder; the reading call of `page` goes to the made-up `fake-claude`, which keeps its input. Every
//! line is typed here from the ADR's rule: none is copied from a log, and no file with both kinds was ever
//! seen. "LEAK" marks text that is on no page.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "f55/heard.rs"]
mod heard;
mod support;
#[path = "f39/world.rs"]
mod world;

use std::fs;
use std::path::Path;

pub use heard::*;
use serde_json::{Value, json};
use support::{install, line, write_log};
pub use world::*;

const ALPHA: &str = "/work/alpha";

/// A line's `entrypoint`: the one that marks a headless session, the one a terminal writes, or none at all.
fn sdk() -> Option<Value> {
    Some(json!("sdk-cli"))
}

fn cli() -> Option<Value> {
    Some(json!("cli"))
}

/// One line at `time` (UTC, on the day) in `cwd`, carrying `entry` as its `entrypoint` and `more` beside it.
fn entered(
    entry: Option<Value>,
    kind: &str,
    origin: &str,
    time: &str,
    cwd: &str,
    content: Value,
    more: Value,
) -> String {
    let mut extra = more;
    if let Some(entry) = entry {
        extra["entrypoint"] = entry;
    }
    line(kind, &format!("{DAY}T{time}.000Z"), cwd, origin, content, extra)
}

/// A line that looks typed by a person.
fn typed(entry: Option<Value>, time: &str, cwd: &str, text: &str, uuid: &str) -> String {
    entered(entry, "user", "human", time, cwd, json!(text), json!({"uuid": uuid}))
}

/// A line the agent said.
fn agent(entry: Option<Value>, time: &str, cwd: &str, text: &str) -> String {
    entered(entry, "assistant", "", time, cwd, json!([{"type": "text", "text": text}]), json!({}))
}

/// What `jellymoon day` prints of the world's logs: each turn's words and its reply, and the whole output.
fn day(w: &World) -> (Vec<(String, String)>, String) {
    let ran = w.run(&["day", "--date", DAY, "--logs", w.logs().to_str().unwrap()]);
    let day = printed(&ran);
    let text = |turn: &Value, field: &str| turn[field].as_str().unwrap().to_string();
    let turns = day["turns"].as_array().unwrap().iter().map(|turn| (text(turn, "said"), text(turn, "reply")));
    assert!(!ran.stdout.contains("LEAK"), "{}", ran.stdout);
    (turns.collect(), ran.stdout)
}

fn turns(w: &World) -> Vec<(String, String)> {
    day(w).0
}

fn pair(said: &str, reply: &str) -> (String, String) {
    (said.to_string(), reply.to_string())
}

#[test]
fn a_headless_session_s_lines_are_no_turn_and_no_reply_and_any_other_entrypoint_is_read_as_before() {
    let w = heard_world();
    let headless = [
        typed(sdk(), "16:00:00", ALPHA, "LEAK sent by a program", "h1"),
        agent(sdk(), "16:00:05", ALPHA, "LEAK said to a program"),
        typed(sdk(), "16:01:00", ALPHA, "LEAK sent by a program again", "h2"),
        agent(sdk(), "16:01:05", ALPHA, "LEAK said to a program again"),
    ];
    write_log(&w.logs(), "-work-alpha/headless.jsonl", &headless);
    assert!(turns(&w).is_empty(), "a session that is all headless");

    // The value is compared whole and exactly, and only a string is a value: anything else is read as before.
    let others = [
        cli(),
        None,
        Some(json!("remote-control")),
        Some(Value::Null),
        Some(json!(7)),
        Some(json!(["sdk-cli"])),
        Some(json!("SDK-CLI")),
        Some(json!("sdk-cli ")),
        Some(json!("sdk")),
        Some(json!("sdk-cli-2")),
        Some(json!("")),
    ];
    let mut lines = Vec::new();
    let mut wanted = Vec::new();
    for (n, entry) in others.into_iter().enumerate() {
        let said = format!("typed with entrypoint {}", entry.as_ref().map_or("left out".to_string(), Value::to_string));
        let reply = format!("answer {n}");
        lines.push(typed(entry.clone(), &format!("17:{n:02}:00"), ALPHA, &said, &format!("o{n}")));
        lines.push(agent(entry, &format!("17:{n:02}:05"), ALPHA, &reply));
        wanted.push((said, reply));
    }
    write_log(&w.logs(), "-work-alpha/others.jsonl", &lines);
    assert_eq!(turns(&w), wanted);
}

#[test]
fn it_is_decided_for_each_line_and_a_left_out_line_still_ends_the_turn_before_it() {
    let w = heard_world();
    let nobody = |time: &str, text: &str| entered(sdk(), "user", "", time, ALPHA, json!(text), json!({}));
    let mixed = [
        typed(cli(), "16:00:00", ALPHA, "one", "u1"),
        agent(cli(), "16:00:05", ALPHA, "one done"),
        // A headless line the agent said is no reply, and ends nothing: the turn takes the next reply.
        agent(sdk(), "16:00:06", ALPHA, "LEAK said headless while a turn was open"),
        typed(cli(), "16:01:00", ALPHA, "two", "u2"),
        agent(sdk(), "16:01:04", ALPHA, "LEAK said headless before the reply"),
        agent(cli(), "16:01:05", ALPHA, "two done"),
        // A typed-looking headless line ends the turn before it: what is said next answers the program.
        typed(sdk(), "16:02:00", ALPHA, "LEAK sent by a program", "u3"),
        agent(cli(), "16:02:05", ALPHA, "LEAK said to the program's words"),
        agent(sdk(), "16:02:06", ALPHA, "LEAK said to the program's words, headless"),
        typed(cli(), "16:03:00", ALPHA, "three", "u4"),
        agent(cli(), "16:03:05", ALPHA, "three done"),
        // So does a headless line that names no origin, as such a line always did.
        nobody("16:04:00", "LEAK sent with no origin"),
        agent(cli(), "16:04:05", ALPHA, "LEAK said to the line with no origin"),
        typed(None, "16:05:00", ALPHA, "four", "u5"),
        agent(None, "16:05:05", ALPHA, "four done"),
        typed(sdk(), "16:06:00", ALPHA, "LEAK sent by a program once more", "u6"),
        // A line with no folder is judged, for a run's copy, as the line before it was: a headless line is in no copy.
        typed(cli(), "16:07:00", "", "five, with no folder on its line", "u7"),
        typed(sdk(), "16:08:00", ALPHA, "LEAK the file's last line", "u8"),
    ];
    write_log(&w.logs(), "-work-alpha/mixed.jsonl", &mixed);
    let wanted = [pair("one", "one done"), pair("two", "two done"), pair("three", "three done")];
    let more = [pair("four", "four done"), pair("five, with no folder on its line", "")];
    assert_eq!(turns(&w), [&wanted[..], &more].concat());
}

#[test]
fn a_left_out_line_s_uuid_is_not_held_so_a_person_s_own_line_of_that_uuid_read_later_is_a_turn() {
    let w = heard_world();
    // Files are read in name order: the headless line is read first, in another file and in the same one.
    write_log(&w.logs(), "-work-alpha/a.jsonl", &[typed(sdk(), "16:00:00", ALPHA, "LEAK sent by a program", "same")]);
    let own = [
        typed(cli(), "16:10:00", ALPHA, "my own words", "same"),
        agent(cli(), "16:10:05", ALPHA, "read and answered"),
        typed(sdk(), "16:20:00", ALPHA, "LEAK sent by a program", "twice"),
        typed(cli(), "16:30:00", ALPHA, "typed after it in one file", "twice"),
    ];
    write_log(&w.logs(), "-work-alpha/b.jsonl", &own);
    // A person's line that was read is held, as before: its headless copy is no turn, and nor is its own copy.
    let copies = [
        typed(sdk(), "16:40:00", ALPHA, "LEAK a headless copy of my own words", "same"),
        typed(cli(), "16:41:00", ALPHA, "LEAK my own words again", "same"),
    ];
    write_log(&w.logs(), "-work-alpha/c.jsonl", &copies);
    assert_eq!(turns(&w), [pair("my own words", "read and answered"), pair("typed after it in one file", "")]);
}

/// The harness's reply for a model that answered with no wish: one JSON object, as `--output-format json` prints.
fn no_wishes() -> String {
    json!({"type": "result", "is_error": false, "result": r#"{"wishes":[],"clashes":[]}"#}).to_string()
}

#[test]
fn a_summary_is_no_turn_copies_are_counted_once_and_page_hands_the_reading_call_what_day_prints() {
    let w = heard_world();
    let summary = |entry: Option<Value>, time: &str, text: &str| {
        entered(entry, "user", "human", time, ALPHA, json!(text), json!({"isCompactSummary": true}))
    };
    let first = [
        typed(cli(), "16:00:00", ALPHA, "add a retry to the uploader", "u1"),
        agent(cli(), "16:00:05", ALPHA, "LEAK an earlier reply in the same turn"),
        // A summary starts no turn and ends none, whatever its entrypoint: the reply after it is the turn's.
        summary(cli(), "16:00:06", "LEAK compaction summary"),
        summary(sdk(), "16:00:07", "LEAK headless compaction summary"),
        summary(None, "16:00:08", "LEAK compaction summary with no entrypoint"),
        agent(cli(), "16:00:09", ALPHA, "retry added, tests pass"),
    ];
    write_log(&w.logs(), "-work-alpha/s1.jsonl", &first);
    // A resume and a fork repeat the typed line under its uuid, in files read later.
    for (file, new) in [("s2", "typed in the resumed session"), ("s3", "typed in the fork")] {
        let copy = [
            typed(cli(), "16:00:00", ALPHA, "add a retry to the uploader", "u1"),
            agent(cli(), "16:00:20", ALPHA, "LEAK reply in the copy"),
            typed(cli(), &format!("17:00:0{}", &file[1..]), ALPHA, new, file),
        ];
        write_log(&w.logs(), &format!("-work-alpha/{file}.jsonl"), &copy);
    }
    // A project whose sessions are all headless is not read, and one file of both kinds gives its typed turn.
    write_log(
        &w.logs(),
        "-work-beta/h.jsonl",
        &[typed(sdk(), "18:00:00", "/work/beta", "LEAK sent by a program", "b1")],
    );
    let both = [
        typed(sdk(), "19:00:00", "/work/gamma", "LEAK sent by a program", "g1"),
        typed(cli(), "19:00:10", "/work/gamma", "typed in gamma", "g2"),
    ];
    write_log(&w.logs(), "-work-gamma/both.jsonl", &both);
    let (turns, printed_day) = day(&w);
    let wanted = [
        pair("add a retry to the uploader", "retry added, tests pass"),
        pair("typed in the resumed session", ""),
        pair("typed in the fork", ""),
        pair("typed in gamma", ""),
    ];
    assert_eq!(turns, wanted);

    // The reading call is made to a harness that keeps its input, in place of the world's made-up agent.
    let rec = w.at("reading");
    install(&rec, "claude");
    fs::write(rec.join("reply"), no_wishes()).unwrap();
    fs::write(rec.join("exit"), "0").unwrap();
    let ran =
        w.run_with(&[("PATH", rec.clone().into())], &["page", "--date", DAY, "--logs", w.logs().to_str().unwrap()]);
    assert_eq!(printed(&ran)["read"], json!({"turns": 4, "projects": ["alpha", "gamma"]}));
    let handed = fs::read_to_string(rec.join("stdin")).unwrap();
    assert_eq!(handed, printed_day, "the reading call's input is what `jellymoon day` prints, byte for byte");
    assert!(!handed.contains("LEAK") && handed.contains("typed in the fork"), "{handed}");
}

/// A session of the day in `folder`, under a name of its own, whose one typed-looking line carries `entry`.
fn session(w: &World, name: &str, entry: Option<Value>, folder: &Path) {
    let lines = [
        typed(entry.clone(), "16:00:00", folder.to_str().unwrap(), "look at the uploader", name),
        agent(entry, "16:00:05", folder.to_str().unwrap(), "looked"),
    ];
    write_log(&w.logs(), &format!("{name}/s.jsonl"), &lines);
}

/// Makes a wish for `alpha` and asks for its run with no folder named, over the world's logs.
fn run_found(w: &World) -> support::Ran {
    let wish = w.wish("alpha");
    w.run(&["run", "--wish", &wish, "--date", DAY, "--logs", w.logs().to_str().unwrap()])
}

/// What `jellymoon hear` shows for a hold of the usual words whose four seconds have passed: its last line.
fn heard_for(w: &World) -> Value {
    let mut events = hold(0, SAID);
    events.push(tick(4999));
    w.hear(&[], &events).pop().unwrap()
}

#[test]
fn a_project_not_on_the_list_whose_sessions_are_all_headless_is_not_found_by_a_run_and_a_listed_one_is() {
    // A day nothing was typed: the logs are there and hold no session.
    let nothing = heard_world();
    nothing.repo("alpha");
    fs::create_dir_all(nothing.logs()).unwrap();
    let said_of_nothing = refused(&run_found(&nothing)).to_string();
    let none =
        format!("jellymoon: no folder of the name the wish gives was typed in on {DAY}; name the folder with --repo\n");
    assert_eq!(said_of_nothing, none);

    // Not listed, and every session of the day headless: it says the same, starts nothing and offers nothing.
    let w = heard_world();
    let repo = w.repo("alpha");
    session(&w, "-work-alpha", sdk(), &repo);
    session(&w, "-work-alpha-again", sdk(), &repo);
    assert_eq!(heard_for(&w), cannot(NO_PROJECT));
    let ran = run_found(&w);
    assert_eq!(refused(&ran), said_of_nothing);
    assert_eq!((w.starts(), w.copies(), w.branches(&repo)), (0, vec![], vec![]));

    // Listed, it is found as before, on the same logs.
    printed(&w.run(&["project", "add", "--repo", repo.to_str().unwrap(), "--date", DAY]));
    assert_eq!(heard_for(&w), heard(SAID, "alpha", 1));
    assert_eq!(printed(&run_found(&w))["outcome"], "completed");
    assert_eq!((w.starts(), w.branches(&repo).len()), (1, 1));
}

#[test]
fn a_folder_typed_in_at_a_terminal_is_found_by_a_run_whatever_was_sent_headless_beside_it() {
    // The same sessions with a terminal's entrypoint, with another value and with none: found, as before.
    for entry in [cli(), Some(json!("remote-control")), None] {
        let w = heard_world();
        let repo = w.repo("alpha");
        session(&w, "-work-alpha", entry.clone(), &repo);
        // A headless session in another folder of the name is not a second folder typed in.
        session(&w, "-elsewhere-alpha", sdk(), &w.at("elsewhere").join("alpha"));
        assert_eq!(heard_for(&w), heard(SAID, "alpha", 1), "{entry:?}");
        assert_eq!(printed(&run_found(&w))["outcome"], "completed", "{entry:?}");
        assert_eq!((w.starts(), w.branches(&repo).len()), (1, 1), "{entry:?}");
    }
}
