//! A change that no longer fits (ADR 0030, decision 10; `docs/development/app.md`, "A change that no longer
//! fits"). Two made wishes change the same line of one file. The first is kept; a tap on `Keep it` of the second
//! is answered in the window's own sentences, with one choice, `Make it again`; a tap on that makes the wish
//! again with its own words and only then tosses the work that does not fit. Where a finished change is kept by
//! itself, the second's note says so with no tap and has `Make it again` in the place of `Keep it`: core wrote
//! why with the run's record when the run ended. The wishes are made and run by the
//! `jellymoon` command with the made-up agent of F39's tests and real git in a temporary repo, and the taps are
//! carried through `Showing` by the made-up window of F47 (`support/tapper.rs`), a process of its own. Nothing
//! here starts the real Claude Code or opens a window.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "work_shows/runs.rs"]
#[allow(dead_code)]
mod runs;
mod support;

use std::fs;

use runs::{day, ended, going, run, wish};
use serde_json::Value;
use sticker_kit::{Note, Scene};
use support::day::DAY;
use support::wished::{SAID, World, wait_until};

const UNKEPT: &str = "The change was not kept.";
const CLASH: [&str; 3] = [
    "Your folder changed in the same place after this change was made, so the two do not fit together.",
    "Nothing in your folder was changed.",
    "Make it again throws this one away and makes the wish again on your folder as it is now.",
];
const IN_WAY: [&str; 2] = [
    "Your folder has changes of your own that are not saved to the project yet, where this change would go.",
    "Nothing in your folder was changed. Save or put away your own changes, then tap Keep it again.",
];
const AGAIN: &str = "Make it again";
const TOSSED: [&str; 2] = [
    "The change was thrown away, and your folder alpha is as it was.",
    "Your wish was made again, and it is being worked on.",
];
const NOT_TOSSED: &str = "The change that does not fit was not thrown away.";
const NEXT: &str = "Open the wish again to see where the change stands. Here is why it was not kept:";
/// A window tall enough that `layout` places every choice of two finished wishes: a tap is on a placed piece.
const TALL: &str = "size:1440x3000";

/// The title, the texts and the options of the text note `id` of the scene a line holds.
fn said(line: &str, id: &str) -> Option<(String, Vec<String>, Vec<String>)> {
    let scene: Scene = serde_json::from_str::<Option<Scene>>(line).expect(line).expect("a scene was shown");
    scene.notes.iter().find_map(|note| match note {
        Note::Text { id: named, title, body, options, .. } if named == id => {
            Some((title.clone(), body.clone(), options.clone()))
        }
        _ => None,
    })
}

fn texts(said: &[&str]) -> Vec<String> {
    said.iter().map(|text| text.to_string()).collect()
}

/// The day's made wishes as they are kept.
fn kept(w: &World) -> Vec<Value> {
    let kept = fs::read_to_string(w.at("jm").join("pages").join(format!("{DAY}.wishes.json"))).unwrap();
    serde_json::from_str::<Value>(&kept).unwrap()["wishes"].as_array().unwrap().clone()
}

/// The made wishes whose work waits, as core says: `jellymoon run waiting`.
fn waiting(w: &World) -> Vec<String> {
    let logs = w.at("logs");
    let printed = w.jellymoon(&["run", "waiting", "--date", DAY, "--logs", logs.to_str().unwrap()]);
    printed["waits"].as_array().unwrap().iter().map(|waits| waits["wish"].as_str().unwrap().to_string()).collect()
}

/// Two made wishes whose runs ended with work that changes the first line of `a.txt`, each its own way.
fn two_that_clash() -> World {
    let w = day();
    assert_eq!((ended(&w, "edits", "completed"), ended(&w, "follow", "completed")), ("m1".into(), "m2".into()));
    w
}

/// The acts that keep the first wish's change and then tap `Keep it` on the second's: five lines.
const KEEP_BOTH: [&str; 6] = [TALL, "look", "choose:work-m1:1", "run", "choose:work-m2:1", "run"];

/// The words and the options of the drawn note `id` of the scene a line holds: its title is the first word.
fn drawn(line: &str, id: &str) -> Option<(Vec<String>, Vec<String>)> {
    let scene: Scene = serde_json::from_str::<Option<Scene>>(line).expect(line).expect("a scene was shown");
    scene.notes.iter().find_map(|note| match note {
        Note::Drawn { id: named, words, options, .. } if named == id => Some((words.clone(), options.clone())),
        _ => None,
    })
}

/// Two made wishes in a world where a finished change is kept by itself, as it is outside the tests. Their runs
/// go at once and change the first line of `a.txt`, each its own way: the first ends first and its change is
/// kept by itself, so the second's, made from the folder as it was, does not fit when its run ends.
fn one_landed_and_the_next_does_not_fit() -> World {
    let w = day().lands();
    assert_eq!((wish(&w), wish(&w)), ("m1".to_string(), "m2".to_string()));
    going(&w, &["m2"], |go| {
        w.play("follow");
        assert_eq!(run(&w, "m1"), "completed");
        go();
    });
    w
}

#[test]
fn a_change_that_did_not_fit_when_its_run_ended_says_so_with_no_tap_and_has_make_it_again_under_it() {
    const LANDED: &str = "edited by the run, and again by the follow-up\n";
    let w = one_landed_and_the_next_does_not_fit();
    let repo = w.at("work").join("alpha");
    // The first change is in the folder with no tap, and only the second waits.
    assert_eq!((fs::read_to_string(repo.join("a.txt")).unwrap(), waiting(&w)), (LANDED.to_string(), vec!["m2".into()]));
    let run = kept(&w)[1]["run"].as_str().unwrap().to_string();
    w.play("hello");
    let bin = format!("bin:{}", w.install().parent().unwrap().display());
    let acts = [bin.as_str(), TALL, "look", "column", "choose:work-m2:1:Keep it", "choose:work-m2:1", "run"];
    let lines = w.taps("logs", &acts).ended();
    let [_, column, stale, at_once, answer] = &lines[..] else { panic!("five lines: {lines:#?}") };
    // The note of the work, before any tap on a choice: why it is not in the folder and what the way on does,
    // in the sentences the refused keep says, and no `Keep it`, which core has just refused.
    let (words, options) = drawn(column, "work-m2").expect("the note of the work that waits");
    let said = format!("“{SAID}”");
    let whole =
        ["Your folder is not changed yet.", CLASH[0], CLASH[2], said.as_str(), "1 new file, 1 changed, 1 removed."];
    assert_eq!((words, options), (texts(&whole), texts(&["Show me", AGAIN, "Toss it"])));
    assert!(!column.contains(&run) && !column.contains("Keep it"), "{column}");
    // Its picture says it too, with the note closed: the kit draws that state with its own caption.
    assert!(column.contains(r#""state":"unfit""#), "{column}");
    // A tap that carries the words `Keep it` has no such choice there, and is carried nowhere (decision 9).
    assert_eq!(drawn(stale, "work-m2"), drawn(column, "work-m2"), "the scene on show is given back");
    // The tap on `Make it again` does what it does under the refusal: the wish is made again, then the toss.
    let doing = said_of(at_once, "busy-m2");
    assert_eq!((doing.0.as_str(), doing.2.len()), ("That wish is being made again.", 0));
    assert_eq!(said_of(answer, "said-m2"), ("The change was thrown away.".to_string(), texts(&TOSSED), vec![]));
    let made = kept(&w);
    assert_eq!(made.len(), 3, "{made:?}");
    assert_eq!((&made[2]["said"], &made[2]["project"]), (&made[1]["said"], &made[1]["project"]));
    assert_eq!((made[2]["said"].as_str(), made[2].get("after")), (Some(SAID), None));
    wait_until("the new wish's run", || kept(&w)[2]["run"].is_string());
    let new = kept(&w)[2]["run"].as_str().unwrap().to_string();
    wait_until("the end of the run", || !w.at("jm").join("runs").join(&new).join("run.lock").exists());
    // The old work is tossed, and the folder holds the first wish's change and nothing of the second's.
    assert!(!waiting(&w).contains(&"m2".to_string()), "the old work was tossed: {:?}", waiting(&w));
    assert_eq!(w.told(&repo, &["branch", "--list", &format!("*{run}*")]), "", "the old run's branch is gone");
    assert_eq!(fs::read_to_string(repo.join("a.txt")).unwrap(), LANDED);
    assert!(!repo.join("n.txt").exists() && repo.join("b.txt").exists());
}

/// The reason is the run's end's, and the folder can change after it: the change the work clashed with is undone
/// in the window, and a keep would land the work now. Its note is then the waiting note again, with `Keep it`.
#[test]
fn a_change_that_fits_again_once_the_other_was_undone_has_keep_it_back_and_a_tap_keeps_it() {
    let w = one_landed_and_the_next_does_not_fit();
    let acts =
        [TALL, "look", "poke:m1", "choose:kept-m1:0", "run", "look", "column", "choose:work-m2:1:Keep it", "run"];
    let lines = w.taps("logs", &acts).ended();
    let [.., column, _, answer] = &lines[..] else { panic!("three lines at least: {lines:#?}") };
    let (words, options) = drawn(column, "work-m2").expect("the note of the work that waits");
    assert_eq!(options, texts(&["Show me", "Keep it", "Toss it"]), "{words:?}");
    assert!(!words.contains(&CLASH[0].to_string()) && column.contains(r#""state":"waiting""#), "{column}");
    assert_eq!(said(answer, "said-m2").map(|said| said.0), Some("The change was kept.".to_string()), "{answer}");
    // The change that was undone waits again, as an undone change does, and the second is in the folder.
    assert_eq!(waiting(&w), ["m1"]);
}

fn said_of(line: &str, id: &str) -> (String, Vec<String>, Vec<String>) {
    said(line, id).unwrap_or_else(|| panic!("no note {id}: {line}"))
}

#[test]
fn a_change_that_no_longer_fits_is_said_plainly_and_make_it_again_makes_the_wish_again_and_then_tosses_it() {
    let w = two_that_clash();
    let repo = w.at("work").join("alpha");
    // The new wish's run waits for the test's word, and then changes nothing: the folder has that change.
    w.play("steps");
    let bin = format!("bin:{}", w.install().parent().unwrap().display());
    let held = w.at("tmp").join("ended");
    let hold = format!("hold:{}", held.display());
    let again = ["choose:said-m2:0:Keep it", "choose:said-m2:0", "run", "column", hold.as_str(), "look", "column"];
    let mut window = w.taps("logs", &[&[bin.as_str()][..], &KEEP_BOTH[..], &again[..]].concat());
    // The window is on show while the new wish's run goes, and looks again once it has ended.
    window.printed(9);
    fs::write(w.at("rec").join("go"), "").unwrap();
    wait_until("the new wish's run", || kept(&w).get(2).is_some_and(|made| made["run"].is_string()));
    let new = kept(&w)[2]["run"].as_str().unwrap().to_string();
    wait_until("the end of the run", || !w.at("jm").join("runs").join(&new).join("run.lock").exists());
    fs::write(held, "").unwrap();
    let lines = window.ended();
    let [_, _, first, _, refused, stale, at_once, answer, going, _, since] = &lines[..] else {
        panic!("eleven lines: {lines:#?}")
    };
    assert_eq!(said(first, "said-m1").map(|said| said.0), Some("The change was kept.".to_string()));
    // The plain note: the window's sentences, the one choice, and no run's id nor a word of the command line's.
    let note = said(refused, "said-m2").expect("core's answer to the keep that does not fit");
    assert_eq!(note, (UNKEPT.to_string(), texts(&CLASH), vec![AGAIN.to_string()]));
    let run = kept(&w)[1]["run"].as_str().unwrap().to_string();
    assert!(!refused.contains(&run) && !refused.contains("git does not merge"), "{refused}");
    // A tap that carries other words than the note has at that place is carried nowhere (decision 9): the note
    // is on show as it was, and one new wish is made below, not two.
    assert_eq!(said(stale, "said-m2"), Some(note), "the scene on show is given back");
    // The tap: the page says what is being done, and then what the toss came to and that the wish was made.
    let doing = said(at_once, "busy-m2").expect("what is being done");
    assert_eq!((doing.0.as_str(), doing.2.len()), ("That wish is being made again.", 0));
    let done = said(answer, "said-m2").expect("core's answer");
    assert_eq!(done, ("The change was thrown away.".to_string(), texts(&TOSSED), vec![]));
    // Core kept a new wish of the same project with the old wish's own words, and started its run.
    let made = kept(&w);
    assert_eq!(made.len(), 3, "{made:?}");
    assert_eq!((&made[2]["said"], &made[2]["project"]), (&made[1]["said"], &made[1]["project"]));
    assert_eq!((made[2]["said"].as_str(), made[2].get("after")), (Some(SAID), None));
    // While the new wish's run goes the answer says the wish is being worked on. Once that run has ended it
    // says only that the wish was made again: the scene the window gives the kit's view (`column`).
    assert_eq!(said(going, "said-m2"), Some(done), "{going}");
    let made_again = texts(&[TOSSED[0], "Your wish was made again."]);
    assert_eq!(said(since, "said-m2"), Some(("The change was thrown away.".to_string(), made_again, vec![])));
    // The old work is tossed, core says, and the folder holds the first wish's change and nothing of the second's.
    assert!(!waiting(&w).contains(&"m2".to_string()), "the old work was tossed: {:?}", waiting(&w));
    assert_eq!(w.told(&repo, &["branch", "--list", &format!("*{run}*")]), "", "the old run's branch is gone");
    assert_eq!(fs::read_to_string(repo.join("a.txt")).unwrap(), "edited by the run\n");
    assert!(!repo.join("m.txt").exists());
}

#[test]
fn with_the_day_s_made_wishes_full_make_it_again_tosses_nothing_and_says_core_s_sentence() {
    let w = two_that_clash();
    let held = w.at("tmp").join("full");
    let hold = format!("hold:{}", held.display());
    let bin = format!("bin:{}", w.install().parent().unwrap().display());
    let acts = [&[bin.as_str()][..], &KEEP_BOTH[..], &[hold.as_str(), "choose:said-m2:0", "run"][..]].concat();
    let mut window = w.taps("logs", &acts);
    window.printed(5);
    // The day has its eight made wishes before the tap.
    let more: Vec<String> = (0..6).map(|_| wish(&w)).collect();
    assert_eq!(more.last().map(String::as_str), Some("m8"));
    let before = (w.everything(&w.at("work").join("alpha")).1, waiting(&w));
    fs::write(held, "").unwrap();
    let lines = window.ended();
    let (title, body, options) = said(lines.last().unwrap(), "said-m2").expect("core's answer");
    // A typed wish's title and core's sentence, and then that nothing was thrown away.
    assert_eq!((title.as_str(), options.len()), ("That wish was not kept.", 0));
    let full = format!("the page of {DAY} has its 8 made wishes already, so this one was not kept.");
    assert!(body[0].ends_with(&full), "{body:?}");
    assert_eq!(body[1..], [NOT_TOSSED.to_string()]);
    assert_eq!(kept(&w).len(), 8);
    assert_eq!((w.everything(&w.at("work").join("alpha")).1, waiting(&w)), before, "nothing was tossed");
    assert!(before.1.contains(&"m2".to_string()), "the work that does not fit waits still: {:?}", before.1);
}

#[test]
fn with_no_installed_command_make_it_again_makes_no_wish_and_tosses_nothing() {
    let w = two_that_clash();
    // No `bin:` act: the window knows no folder of an installed command, so no run could be started.
    let lines = w.taps("logs", &[&KEEP_BOTH[..], &["choose:said-m2:0", "run"][..]].concat()).ended();
    let (title, body, options) = said(lines.last().unwrap(), "said-m2").expect("core's answer");
    assert_eq!((title.as_str(), options.len(), body.len()), ("That wish was not kept.", 0, 2), "{body:?}");
    assert_eq!(body[1], NOT_TOSSED);
    assert_eq!(kept(&w).len(), 2, "no wish was made");
    assert!(waiting(&w).contains(&"m2".to_string()), "the work that does not fit waits still");
}

#[test]
fn changes_in_the_way_are_said_plainly_with_no_choice_and_every_other_reason_is_told_as_before() {
    let w = day();
    assert_eq!(ended(&w, "edits", "completed"), "m1");
    let repo = w.at("work").join("alpha");
    let run = kept(&w)[0]["run"].as_str().unwrap().to_string();
    let keep = [TALL, "look", "choose:work-m1:1", "run"];
    // An unsaved change of the person's own in a file of the work.
    fs::write(repo.join("a.txt"), "half an edit of the person's\n").unwrap();
    let lines = w.taps("logs", &keep).ended();
    let answer = lines.last().unwrap();
    assert_eq!(said(answer, "said-m1"), Some((UNKEPT.to_string(), texts(&IN_WAY), vec![])));
    assert!(!answer.contains(&run), "{answer}");
    assert_eq!(fs::read_to_string(repo.join("a.txt")).unwrap(), "half an edit of the person's\n");
    // Another reason: the folder stands on no branch. The command line's sentence, with the run's id, as before.
    fs::write(repo.join("a.txt"), "a.txt as it was\n").unwrap();
    w.told(&repo, &["checkout", "-q", "--detach"]);
    let lines = w.taps("logs", &keep).ended();
    let (title, body, options) = said(lines.last().unwrap(), "said-m1").expect("core's answer");
    assert_eq!((title.as_str(), body[0].as_str(), options.len()), (UNKEPT, NEXT, 0));
    let unchanged = format!("the work of run {run} was not kept, and nothing was changed.");
    assert!(body[1].starts_with("the checkout stands on no branch of the person's") && body[1].ends_with(&unchanged));
    assert_eq!(body.len(), 2, "{body:?}");
    assert_eq!(waiting(&w), ["m1"]);
}
