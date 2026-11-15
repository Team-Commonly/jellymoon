//! Every answer that is refused: nothing is started, and nothing in the home or the repo is changed. A wish that
//! is not there or has had no run, a run that does not ask, a run that is going (an answer is going for it, or
//! another command holds it), an answer with no words or too many, a copy that is not kept, a record that is not
//! core's, a home with three runs going, and a machine with no harness.

use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::process::Stdio;

use serde_json::{Value, json};

use super::{ANSWER, DAY, Made, asking, ended, printed, refused, said, until, world};
use crate::support::map_key;

const NOT_ASKING: &str = "run {run} does not ask a question, so there is nothing to answer";
const IS_GOING: &str = "run {run} is going and asks nothing by now: nothing was started";
const HELD: &str = "another command holds run {run}: nothing was started";
const EMPTY: &str = "an answer needs words, and nothing is left of these once cleaned";
const LONG: &str = "an answer is at most 600 characters, and this one is longer";
const CHANGING: &str =
    "an answer is kept as cleaning leaves it, and cleaning was not done with these words after 8 times";
const NOT_CORES: &str = "the record of run {run} is not one Jellymoon wrote for this wish";
const NO_COPY: &str = "the copy of run {run} is not kept as a run that asks leaves it, so it cannot be answered: \
                       `run toss` lets the wish go";

/// One run of the made-up agent's script, whatever its end.
fn played(script: &str) -> Made {
    let w = world();
    w.play(script, 0);
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let run = printed(&w.start(&wish, &repo))["run"].as_str().unwrap().to_string();
    Made { w, repo, wish, run }
}

#[test]
fn an_answer_for_a_wish_that_is_not_there_or_has_had_no_run_is_refused() {
    let w = world();
    let (repo, wish) = (w.repo("alpha"), w.wish("alpha"));
    let made = Made { w, repo, wish, run: String::new() };
    made.unanswered(ANSWER, "wish m1 has had no run, so there is nothing to answer");
    let other = Made { wish: "m2".to_string(), w: world(), repo: made.repo.clone(), run: String::new() };
    let none = format!("the page of {DAY} has no made wish of that id; a made wish's id is m1 to m8");
    assert_eq!(refused(&other.answer(ANSWER)), said(&none));
    assert_eq!((made.w.starts(), other.w.starts()), (0, 0));
}

#[test]
fn an_answer_for_a_run_that_does_not_ask_is_refused_whatever_its_end() {
    for script in ["edits", "exit-3", "nothing"] {
        let made = played(script);
        made.unanswered(ANSWER, NOT_ASKING);
    }
    // A run that asked and was tossed asks no more.
    let made = asking();
    assert_eq!(printed(&made.toss())["tossed"], true);
    made.unanswered(ANSWER, NOT_ASKING);
    // The control: the run that asks takes the same answer.
    let made = asking();
    made.w.play("nothing", 0);
    assert_eq!(printed(&made.answer(ANSWER))["outcome"], "completed");
}

#[test]
fn an_answer_with_no_words_too_many_or_words_that_cannot_be_cleaned_is_refused() {
    let made = asking();
    for empty in ["", "\n", " \t\r\n ", "\u{1b}\u{7}\u{200b}\n"] {
        made.unanswered(empty, EMPTY);
    }
    made.unanswered(&"w".repeat(601), LONG);
    made.unanswered(&format!("{}\n", "word ".repeat(1000)), LONG);
    made.unanswered(&map_key("kLmNo").repeat(9), CHANGING);
    // The control: an answer of 600 characters is taken.
    made.w.play("nothing", 0);
    let most = "w".repeat(600);
    assert_eq!(printed(&made.answer(&most))["outcome"], "completed");
    assert_eq!(made.w.rec("stdin").unwrap(), format!("{most}\n"));
}

#[test]
fn an_answer_for_a_run_that_another_answer_is_going_for_is_refused_and_the_first_goes_on() {
    let made = asking();
    made.w.play("slow", 0);
    let going = made.answering(&[], ANSWER);
    // The made-up agent has read its input and waits: the record names it, and nothing more is written.
    let live = || made.w.at("rec").join("started").is_file() && made.record()["pid"].is_u64();
    until("the answered run never started its agent", live);
    assert_eq!((&made.w.kept(&made.wish)["state"], made.record()["outcome"].is_null()), (&json!("working"), true));
    made.unanswered("a second answer, never taken", IS_GOING);
    made.w.let_end("go");
    assert_eq!(printed(&ended(going))["outcome"], "completed");
    assert_eq!((made.w.rec("stdin").unwrap(), made.w.starts()), (format!("{ANSWER}\n"), 2));
}

#[test]
fn an_answer_is_refused_while_another_command_holds_the_run_or_three_runs_go() {
    let made = asking();
    let lock = made.w.jm("runs").join(&made.run).join("run.lock");
    let held = fs::File::create(&lock).unwrap();
    held.lock().unwrap();
    made.unanswered(ANSWER, HELD);
    drop(held);
    // Something that is no plain file at the lock's name is no lock, and is not opened.
    fs::remove_file(&lock).unwrap();
    fs::create_dir(&lock).unwrap();
    made.unanswered(ANSWER, "the lock of run {run} could not be made or taken: nothing was started");
    fs::remove_dir(&lock).unwrap();
    fs::write(&lock, "").unwrap();
    // Three runs go: their locks are held, here by the test. An answered run would be a fourth.
    let others: Vec<fs::File> = (1..=3)
        .map(|n| {
            let run = made.w.jm("runs").join(format!("r{n:016x}"));
            fs::create_dir(&run).unwrap();
            let lock = fs::File::create(run.join("run.lock")).unwrap();
            lock.lock().unwrap();
            lock
        })
        .collect();
    let full = "nothing was started: 3 runs are going, and no more than 3 go at once; answer again when one has ended";
    made.unanswered(ANSWER, full);
    drop(others);
    assert!(!super::held(&made), "an answer that was refused left the run's lock held");
    made.w.play("nothing", 0);
    assert_eq!(printed(&made.answer(ANSWER))["outcome"], "completed");
}

#[test]
fn an_answer_for_a_record_a_wish_or_a_copy_that_is_not_as_a_run_that_asks_leaves_it_is_refused() {
    type Change = fn(&mut Value);
    let odd: [(&str, Change, &str); 6] = [
        ("a question the door would change", |record| record["question"] = json!(" Which?"), NOT_CORES),
        ("a session id that is no id of core's", |record| record["session"] = json!("../../etc"), NOT_CORES),
        ("a keep's words", |record| record["kept"] = json!("0123456789abcdef0123456789abcdef01234567"), NOT_CORES),
        ("an end", |record| record["end"] = json!("0123456789abcdef0123456789abcdef01234567"), NOT_CORES),
        ("no start", |record| drop(record.as_object_mut().unwrap().remove("start")), NOT_CORES),
        ("no end written down", |record| record["outcome"] = Value::Null, IS_GOING),
    ];
    for (what, change, why) in odd {
        let made = asking();
        let mut record = made.record();
        change(&mut record);
        made.write(&record);
        made.unanswered(ANSWER, why);
        assert_eq!(made.w.copies().len(), 1, "{what}");
    }
    // A second wish made, by hand, to name the first one's run has no say over it.
    let made = asking();
    made.w.wish("alpha");
    let file = made.w.jm("pages").join(format!("{DAY}.wishes.json"));
    let mut wishes: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    let (run, why) = (wishes["wishes"][0]["run"].clone(), wishes["wishes"][0]["why"].clone());
    (wishes["wishes"][1]["run"], wishes["wishes"][1]["why"]) = (run, why);
    wishes["wishes"][1]["state"] = json!("needs_you");
    fs::write(&file, format!("{wishes}\n")).unwrap();
    let second = Made { wish: "m2".to_string(), ..made };
    second.unanswered(ANSWER, "wish m2 names run {run}, which is another wish's run: nothing was started");
    // The copy's folder is gone, or the copy is marked as a failed run's is.
    let made = asking();
    fs::remove_dir_all(made.copy()).unwrap();
    made.unanswered(ANSWER, NO_COPY);
    let made = asking();
    let file = made.w.jm("runs").join(&made.run).join("copy.json");
    let mut copy: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    copy["failed"] = json!(7);
    fs::write(&file, format!("{copy}\n")).unwrap();
    made.unanswered(ANSWER, NO_COPY);
}

#[test]
fn an_answer_on_a_machine_with_no_harness_is_refused_and_the_run_still_asks() {
    let made = asking();
    let harness = made.w.at("bin").join(format!("claude{}", std::env::consts::EXE_SUFFIX));
    fs::remove_file(harness).unwrap();
    let before = made.home();
    let ran = made.answer(ANSWER);
    assert!(refused(&ran).starts_with("jellymoon: "), "{}", ran.stderr);
    assert!(before == made.home() && !ran.stderr.contains("quince"), "{}", ran.stderr);
    assert_eq!(made.w.on_page(&made.wish)["question"], super::QUESTION);
}

// A file is anyone's to write: an answer in it that the door would not have left, or one on a wish that names no
// run, makes the day's made wishes unused, as a question of that kind does.
#[test]
fn a_wish_whose_answer_is_not_one_the_door_left_is_not_used() {
    let rewrite = |made: &Made, answer: Value, run: bool| {
        let file = made.w.jm("pages").join(format!("{DAY}.wishes.json"));
        let mut wishes: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        let last = wishes["wishes"].as_array().unwrap().len() - 1;
        let wish = &mut wishes["wishes"][last];
        assert_eq!(wish["run"].is_string(), run);
        wish["answer"] = answer;
        fs::write(&file, format!("{wishes}\n")).unwrap();
    };
    let unused = format!("the made wishes of {DAY} are not used: they are not the made wishes of that day");
    for answer in [json!("the old one\u{1b}[31m"), json!(" the old one"), json!(""), json!("w".repeat(601))] {
        let made = asking();
        rewrite(&made, answer.clone(), true);
        assert_eq!(refused(&made.answer(ANSWER)), said(&unused), "{answer}");
        assert_eq!(made.w.starts(), 1, "{answer}");
    }
    // A second wish, with no run, cannot have been answered.
    let made = asking();
    made.w.wish("alpha");
    rewrite(&made, json!("the old one"), false);
    assert_eq!(refused(&made.answer(ANSWER)), said(&unused));
    // The control: an answer as the door leaves one, on the wish that names a run, is used.
    let made = asking();
    rewrite(&made, json!("the old one"), true);
    made.w.play("nothing", 0);
    assert_eq!(printed(&made.answer(ANSWER))["outcome"], "completed");
}

// What changed while an answer waited for its locks counts: with the run's lock held the record is read again,
// and the wish is asked once more behind the day's lock.
#[test]
fn an_answer_reads_the_record_and_the_wish_again_once_it_holds_the_run_s_lock() {
    let waits = |made: &Made| {
        let (go, at) = (made.w.at("rec").join("go.step"), made.w.at("rec").join("go.at"));
        let hold = [("JELLYMOON_TEST_ANSWER_LOCK_WAIT_MS", "60000".into()), ("JELLYMOON_TEST_GO", go.clone().into())];
        let going = made.answering(&hold, ANSWER);
        until("the answer never came to its locks", || at.exists());
        (going, go)
    };
    let made = asking();
    let (going, go) = waits(&made);
    let mut record = made.record();
    record["question"] = json!(" Which?");
    made.write(&record);
    fs::write(&go, "").unwrap();
    assert_eq!(refused(&ended(going)), said(&NOT_CORES.replace("{run}", &made.run)));
    assert_eq!((made.w.starts(), made.w.kept(&made.wish)["state"].clone()), (1, json!("needs_you")));
    // The wish lost its question meanwhile: it asks for the run no more.
    let made = asking();
    let (going, go) = waits(&made);
    let file = made.w.jm("pages").join(format!("{DAY}.wishes.json"));
    let mut wishes: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    wishes["wishes"][0].as_object_mut().unwrap().remove("question");
    fs::write(&file, format!("{wishes}\n")).unwrap();
    let before = (made.home(), made.w.starts());
    fs::write(&go, "").unwrap();
    let why = format!("the wish no longer asks for run {}: nothing was started", made.run);
    assert_eq!(refused(&ended(going)), said(&why));
    assert!(before == (made.home(), made.w.starts()) && !super::held(&made));
}

// The input is read one byte past what the door looks at and no further: an answer that never ends is refused
// as too long while its writer still holds the input open.
#[test]
fn an_answer_that_never_ends_is_refused_as_too_long_without_being_read_to_its_end() {
    let made = asking();
    let mut going = made.fed(&[], Stdio::piped());
    let mut input = going.stdin.take().unwrap();
    input.write_all("word ".repeat(1000).as_bytes()).unwrap();
    input.flush().unwrap();
    until("the answer went on reading its input", || going.try_wait().unwrap().is_some());
    let mut said_why = String::new();
    going.stderr.take().unwrap().read_to_string(&mut said_why).unwrap();
    assert_eq!(said_why, said(LONG));
    drop(input);
    assert_eq!(made.w.starts(), 1);
}

/// A link to a folder, as each system makes one. False where the account may not make links (Windows).
fn linked(to: &Path, link: &Path) -> bool {
    #[cfg(unix)]
    let made = std::os::unix::fs::symlink(to, link);
    #[cfg(windows)]
    let made = std::os::windows::fs::symlink_dir(to, link);
    made.is_ok()
}

// The copy is asked for as itself, never through a link: a copy's name that is a link to a folder, and a
// `copies` that is one, lead wherever whoever made the link chose, and no agent is started there.
#[test]
fn an_answer_for_a_copy_that_is_a_link_or_stands_in_a_linked_copies_is_refused_and_no_agent_is_started() {
    for whole in [false, true] {
        let made = asking();
        let at = if whole { made.w.jm("copies") } else { made.copy() };
        let moved = made.w.at("elsewhere");
        fs::rename(&at, &moved).unwrap();
        if !linked(&moved, &at) {
            return;
        }
        assert!(made.copy().is_dir(), "the link leads to the copy");
        made.unanswered(ANSWER, NO_COPY);
    }
}
