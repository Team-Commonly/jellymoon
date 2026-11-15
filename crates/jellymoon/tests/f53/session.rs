//! The session of a follow-up's run: a fork of the followed run's, asked of the harness under a new id of
//! core's; a new session when the harness keeps the followed one no more, or its record names none, with the
//! record saying which; a fork that starts under another id than the one given; a harness too old to fork; a
//! follow-up that asks, and is answered under its own session; the reader, which reads no session kept in a
//! follow-up's copy; and where a follow-up's words are.

use std::fs;

use serde_json::{Value, json};

use super::{ARGS, AS_THE_RUN_LEFT_IT, DAY, MORE, STAR, World, finished, forked, given, is_session, printed};
use crate::support::{A, Ran, reply, said as typed, started, tree, write_log};

/// A run's own command line, under the session `id`.
fn plain(id: &str) -> Vec<String> {
    ARGS.iter().map(|arg| arg.replace("<id>", id)).collect()
}

fn text(value: &Value) -> String {
    value.as_str().unwrap().to_string()
}

#[test]
fn a_followed_session_that_is_not_kept_starts_a_new_session_and_the_record_says_which() {
    let made = finished();
    let (w, session) = (&made.w, made.w.args().1);
    let (wish, said) = made.follows("unkept", 0);
    let run = text(&said["run"]);
    assert_eq!((&said["outcome"], &said["star"], &said["denied"]), (&json!("completed"), &json!(STAR), &json!(0)));
    // The harness was told to fork, said it keeps no such session, and was started once more: a run's own
    // command line, under a third id, in the same copy with nothing done in it yet.
    let (record, (fork, first), (again, second)) = (w.record(&run), given(w, 2), given(w, 3));
    let (new, tried) = (text(&record["session"]), fork[17].clone());
    assert_eq!((fork, again), (forked(&session, &tried), plain(&new)));
    assert!(is_session(&new) && is_session(&tried) && new != tried && new != session, "{new} {tried}");
    assert_eq!((first.as_str(), second.as_str(), w.starts()), (AS_THE_RUN_LEFT_IT, AS_THE_RUN_LEFT_IT, 3));
    assert_eq!(w.rec("stdin").unwrap(), format!("{MORE}\n"));
    // The record names the run it follows, the session the work was done under, and that it is no fork.
    assert_eq!(
        (&record["follows"], &record["forked"], &record["start"]),
        (&json!(made.run), &json!(false), &json!(made.end()))
    );
    assert_eq!((&w.kept(&wish)["state"], w.copies()), (&json!("done"), vec![]));
    // Its work is its own commit on the followed work, shown and kept as a fork's is.
    let shown = printed(&made.asked(&["show"], &wish, &[]));
    assert_eq!(shown["files"].as_array().unwrap().len(), 2, "{shown}");
}

#[test]
fn a_new_session_that_fails_is_a_failed_run_and_the_harness_is_not_started_a_third_time() {
    for number in [1, 2] {
        let made = finished();
        let (wish, said) = made.follows("unkept", number);
        let (run, why) = (text(&said["run"]), text(&said["why"]));
        assert_eq!((&said["outcome"], made.w.starts()), (&json!("failed"), 3), "{said}");
        // The words of a session that is not kept are an answered run's: a run started under a new session
        // was told to go on with none, whatever its harness says.
        assert!(why.starts_with(&format!("run {run} did not finish (")) && !why.contains("no longer keeps"), "{why}");
        let record = made.w.record(&run);
        assert_eq!((&record["forked"], &record["outcome"]), (&json!(false), &json!("failed")));
        assert_eq!((&made.w.kept(&wish)["state"], made.w.copies()), (&json!("needs_you"), vec![run]));
    }
}

#[test]
fn a_followed_record_that_names_no_session_of_core_s_is_not_forked() {
    for odd in [None, Some("--dangerously-skip-permissions"), Some("0123ABCD-4567-4abc-8def-0123456789ab")] {
        let made = finished();
        let mut record = made.record();
        match odd {
            Some(odd) => record["session"] = json!(odd),
            None => _ = record.as_object_mut().unwrap().remove("session"),
        }
        made.write(&record);
        let (_, said) = made.follows("follow", 0);
        let record = made.w.record(said["run"].as_str().unwrap());
        // No word of the followed record reaches the harness: it is started as any run, under a new session.
        assert_eq!(given(&made.w, 2), (plain(record["session"].as_str().unwrap()), AS_THE_RUN_LEFT_IT.to_string()));
        assert_eq!((&said["outcome"], &record["forked"], made.w.starts()), (&json!("completed"), &json!(false), 2));
    }
}

#[test]
fn a_fork_that_starts_under_another_session_than_the_new_one_is_stopped_and_has_failed() {
    for (script, number) in [("follow", 3), ("session", 0)] {
        let made = finished();
        let (wish, said) = made.follows(script, number);
        let run = text(&said["run"]);
        let why = format!(
            "run {run} did not finish (it did not start under the session id it was given): its copy is kept and no \
             star was added"
        );
        assert_eq!((&said["outcome"], &said["why"]), (&json!("failed"), &json!(why)), "{script}");
        assert!(made.w.agent_came_and_went() && made.w.starts() == 2);
        // Nothing was committed, and the followed work is as it was.
        let tip = made.w.commit(&made.repo, &format!("refs/heads/jellymoon/run-{run}"));
        assert_eq!((tip, &made.w.kept(&wish)["state"]), (made.end(), &json!("needs_you")));
    }
}

#[test]
fn a_harness_too_old_to_fork_a_session_starts_no_run_and_the_follow_up_can_be_run_again() {
    let made = finished();
    let (w, before) = (&made.w, (made.w.branches(&made.repo), made.record()));
    let (wish, said) = made.follows("old-option", 4);
    let old = "nothing was started: this Claude Code is too old for runs, as it does not know --fork-session; update \
               Claude Code and run the wish again";
    let none = json!({"run": null, "wish": wish, "outcome": "not_started", "star": null, "denied": 0, "why": old});
    assert_eq!(said, none);
    let kept = json!({"id": wish, "said": MORE, "project": "alpha", "state": "needs_you", "stars": [], "why": old,
        "after": "m1"});
    assert_eq!((w.kept(&wish), w.copies(), (w.branches(&made.repo), made.record())), (kept, vec![], before));
    w.play("follow", 0);
    assert_eq!(printed(&made.asked(&[], &wish, &[]))["outcome"], "completed");
}

#[test]
fn a_follow_up_that_asks_is_answered_under_its_own_session_and_not_the_followed_one() {
    let made = finished();
    let (w, session) = (&made.w, made.w.args().1);
    let (wish, said) = made.follows("asks", 0);
    let run = text(&said["run"]);
    assert_eq!(said["outcome"], "asks", "{said}");
    let own = text(&w.record(&run)["session"]);
    w.play("follow", 0);
    let vars = w.vars.iter().map(|(name, value)| (*name, value.as_os_str()));
    let asked = ["run", "answer", "--wish", &wish, "--date", DAY];
    let answered: Ran = started(w.dir.path(), &vars.collect::<Vec<_>>(), &asked, "the first one\n");
    assert_eq!(printed(&answered)["outcome"], "completed");
    // The answer goes on with the fork, in the follow-up's own copy: nothing is forked a second time.
    let (args, found) = given(w, 3);
    let resumed: Vec<String> = plain(&own).iter().map(|arg| arg.replace("--session-id", "--resume")).collect();
    assert_eq!((args, found.as_str()), (resumed, AS_THE_RUN_LEFT_IT));
    let record = w.record(&run);
    assert_eq!(
        (&record["session"], &record["forked"], &record["start"]),
        (&json!(own), &json!(true), &json!(made.end()))
    );
    assert!(own != session);
}

impl World {
    /// What `jellymoon day` prints of the day's turns, and everything it printed.
    fn day(&self) -> (Vec<String>, String) {
        let logs = self.at("logs");
        let ran = self.run(&["day", "--date", DAY, "--logs", logs.to_str().unwrap()]);
        assert_eq!((ran.code, ran.stderr.as_str()), (0, ""), "{}", ran.stdout);
        let day: Value = serde_json::from_str(&ran.stdout).expect(&ran.stdout);
        (day["turns"].as_array().unwrap().iter().map(|turn| text(&turn["said"])).collect(), ran.stdout)
    }
}

#[test]
fn the_session_kept_in_a_follow_up_s_copy_is_no_turn_of_the_day() {
    const LEFT: &str = "damson-ferry-typed-in-a-copy";
    let made = finished();
    let w = &made.w;
    let (_, said) = made.follows("follow", 0);
    let (run, at) = (text(&said["run"]), |minute: usize| format!("{DAY}T16:{minute:02}:00.000Z"));
    w.log_of("-work-alpha/s1.jsonl", &[typed(&at(0), A, "add a retry to the uploader"), reply(&at(1), A, "added")]);
    let before = w.day();
    assert_eq!(before.0, ["add a retry to the uploader"]);
    // A fork's file repeats the session it forked, the followed wish with it, and then holds the follow-up:
    // every line of it names the follow-up's copy, as the harness resolves it or as Jellymoon names it.
    let session = text(&w.record(&run)["session"]);
    let name = format!("-jm-copies-{run}/{session}.jsonl");
    let copy = w.jm("copies").join(&run);
    for copy in [copy.clone(), fs::canonicalize(w.jm("copies")).unwrap().join(&run)] {
        let copy = copy.to_str().unwrap();
        let lines = [
            typed(&at(10), copy, &format!("{LEFT}: make the uploader retry twice")),
            reply(&at(11), copy, &format!("{LEFT} done")),
            typed(&at(12), copy, &format!("{LEFT}: {MORE}")),
        ];
        w.log_of(&name, &lines);
        let after = w.day();
        assert!(after == before && !after.1.contains(LEFT), "{}", after.1);
    }
    // The control: the same lines in a folder that is no copy are read.
    let work = w.at("work").join("alpha");
    w.log_of(&name, &[typed(&at(10), work.to_str().unwrap(), &format!("{LEFT}: {MORE}"))]);
    assert!(w.day().1.contains(LEFT));
}

impl World {
    fn log_of(&self, name: &str, lines: &[String]) {
        write_log(&self.at("logs"), name, lines);
    }
}

#[test]
fn a_follow_up_s_words_are_the_harness_s_input_and_the_wish_s_own_and_are_on_no_command_line() {
    let made = finished();
    let (w, words) = (&made.w, "plum-lantern");
    let (wish, said) = made.follows("follow", 0);
    assert!(!said.to_string().contains(words));
    assert!(given(w, 2).0.iter().all(|arg| !arg.contains(words)), "the words are no argument");
    let holds = |bytes: &[u8]| bytes.windows(words.len()).any(|window| window == words.as_bytes());
    let home = tree(&w.at("jm")).into_iter().filter(|(_, bytes)| holds(bytes));
    let held: Vec<_> = home.map(|(path, _)| path.file_name().unwrap().to_str().unwrap().to_string()).collect();
    assert_eq!(held, [format!("{DAY}.wishes.json")]);
    // No command that looks at the follow-up or at the work it follows prints them.
    for (verb, wish) in [("show", wish.as_str()), ("show", "m1"), ("keep", wish.as_str()), ("toss", "m1")] {
        let ran = made.asked(&[verb], wish, &[]);
        assert!(!ran.stdout.contains(words) && !ran.stderr.contains(words), "{verb} {wish}: {}", ran.stderr);
    }
}
