//! The reader and a run's kept session (`docs/development/reader.md`, "A run's copy is never read"): since F51 the
//! harness keeps one, in the person's own store, and `jellymoon day` reads that store. Every line of it that a
//! person could have typed stands in the run's copy, so none of it is a turn. And what F50's gate left open of a
//! line with no working directory. The session files here are typed, never a harness's own.

use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use super::{DAY, World, asking, world};
use crate::support::{A, line, reply, said as typed, write_log};

/// What every text of a copy's session holds.
const LEFT: &str = "quince-heron-typed-in-a-copy";

fn at(minute: usize) -> String {
    format!("{DAY}T16:{minute:02}:00.000Z")
}

/// A typed line with no working directory on it, under a uuid when one is given.
fn bare(minute: usize, words: &str, uuid: &str) -> String {
    let more = if uuid.is_empty() { json!({}) } else { json!({ "uuid": uuid }) };
    let mut line: Value = serde_json::from_str(&line("user", &at(minute), "", "human", json!(words), more)).unwrap();
    line.as_object_mut().unwrap().remove("cwd");
    line.to_string()
}

impl World {
    fn log(&self, name: &str, lines: &[String]) {
        write_log(&self.at("logs"), name, lines);
    }

    /// What `jellymoon day` prints of the day's turns, each with its reply, and everything it printed.
    fn day(&self) -> (Vec<(String, String)>, String) {
        let logs = self.at("logs");
        let ran = self.run(&["day", "--date", DAY, "--logs", logs.to_str().unwrap()]);
        assert_eq!((ran.code, ran.stderr.as_str()), (0, ""), "{}", ran.stdout);
        let day: Value = serde_json::from_str(&ran.stdout).expect(&ran.stdout);
        let word = |turn: &Value, key: &str| turn[key].as_str().unwrap().to_string();
        let turns = day["turns"].as_array().unwrap().iter().map(|turn| (word(turn, "said"), word(turn, "reply")));
        (turns.collect(), ran.stdout)
    }
}

/// A run's kept session as the harness writes one: the wish, what the agent said and did, and its last word.
fn kept_session(copy: &Path) -> Vec<String> {
    let copy = copy.to_str().unwrap();
    let result = json!([{"type": "tool_result", "tool_use_id": "t1", "content": format!("{LEFT} a file's lines")}]);
    vec![
        typed(&at(10), copy, &format!("{LEFT}: make the uploader retry twice")),
        reply(&at(11), copy, &format!("{LEFT} reading the uploaders")),
        line("user", &at(12), copy, "", result, json!({})),
        reply(&at(13), copy, &format!("{LEFT} which of the two uploaders did you mean?")),
    ]
}

#[test]
fn the_kept_session_of_a_run_that_asks_is_no_turn_of_the_day() {
    let made = asking();
    let alpha = [typed(&at(0), A, "add a retry to the uploader"), reply(&at(1), A, "retry added")];
    made.w.log("-work-alpha/s1.jsonl", &alpha);
    let (before, printed) = made.w.day();
    assert_eq!(before, [("add a retry to the uploader".to_string(), "retry added".to_string())]);
    // The session is kept under the id core chose, in the folder the harness names after the run's copy, and the
    // harness logs the copy as the system resolves it.
    let (session, copy) = (made.record()["session"].as_str().unwrap().to_string(), made.copy());
    let name = format!("-jm-copies-{}/{session}.jsonl", made.run);
    for copy in [copy.clone(), fs::canonicalize(&copy).unwrap()] {
        let lines = kept_session(&copy);
        made.w.log(&name, &lines);
        // Every line of it a person could have typed names the copy as its working directory.
        let kept = fs::read_to_string(made.w.at("logs").join(&name)).unwrap();
        let users =
            kept.lines().map(|line| serde_json::from_str::<Value>(line).unwrap()).filter(|l| l["type"] == "user");
        assert_eq!(users.filter(|line| line["cwd"] == json!(copy.to_str().unwrap())).count(), 2);
        let (after, said) = made.w.day();
        assert!(after == before && said == printed && !said.contains(LEFT), "{said}");
    }
    // The control: the same session logged in a folder that is no copy is read, with what was typed in it.
    made.w.log(&name, &kept_session(&made.w.at("work").join("alpha")));
    let (after, said) = made.w.day();
    assert!(after.len() == 2 && said.contains(LEFT), "{said}");
}

#[test]
fn every_line_with_no_folder_after_a_copy_s_line_is_left_out_not_only_the_first() {
    let w = world();
    let copy = w.jm("copies").join("r0123456789abcdef");
    let copy = copy.to_str().unwrap();
    let lines = [
        typed(&at(0), copy, &format!("{LEFT} typed")),
        bare(1, &format!("{LEFT} first with no folder"), ""),
        bare(2, &format!("{LEFT} second with no folder"), ""),
        // An empty working directory is none.
        typed(&at(3), "", &format!("{LEFT} third, with an empty folder")),
        bare(4, &format!("{LEFT} fourth with no folder"), ""),
        typed(&at(5), A, "back in alpha"),
        bare(6, "first after it", ""),
        typed(&at(7), "", "second after it"),
        bare(8, "third after it", ""),
    ];
    w.log("-mixed/s.jsonl", &lines);
    let (turns, said) = w.day();
    let kept: Vec<String> = turns.into_iter().map(|(said, _)| said).collect();
    assert_eq!(kept, ["back in alpha", "first after it", "second after it", "third after it"]);
    assert!(!said.contains(LEFT), "{said}");
}

#[test]
fn a_line_with_no_folder_that_is_left_out_hides_no_line_of_its_uuid_typed_elsewhere() {
    for (first, second) in [("-a", "-b"), ("-b", "-a")] {
        let w = world();
        let copy = w.jm("copies").join("r0123456789abcdef");
        let in_copy = [
            typed(&at(0), copy.to_str().unwrap(), &format!("{LEFT} typed")),
            bare(1, &format!("{LEFT} typed with no folder"), "u-twice"),
        ];
        w.log(&format!("{first}/s.jsonl"), &in_copy);
        let own = line("user", &at(2), A, "human", json!("typed in alpha"), json!({"uuid": "u-twice"}));
        w.log(&format!("{second}/s.jsonl"), &[own, reply(&at(3), A, "done in alpha")]);
        let (turns, said) = w.day();
        assert_eq!(turns, [("typed in alpha".to_string(), "done in alpha".to_string())], "the copy's file is {first}");
        assert!(!said.contains(LEFT), "{said}");
    }
    // The control: a line with no folder that is read is remembered, so the one read first is the one kept.
    let w = world();
    w.log("-a/s.jsonl", &[bare(1, "typed with no folder", "u-twice")]);
    w.log("-b/s.jsonl", &[line("user", &at(2), A, "human", json!("typed in alpha"), json!({"uuid": "u-twice"}))]);
    assert_eq!(w.day().0, [("typed with no folder".to_string(), String::new())]);
}
