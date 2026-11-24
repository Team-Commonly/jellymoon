//! A change that no longer fits (ADR 0030, decision 10; `docs/development/app.md`, "A change that no longer
//! fits"). Two made wishes change the same line of one file. The first is kept; a tap on `Keep it` of the second
//! is answered in the window's own sentences, with one choice, `Make it again`; a tap on that makes the wish
//! again with its own words and only then tosses the work that does not fit. The wishes are made and run by the
//! `jellymoon` command with the made-up agent of F39's tests and real git in a temporary repo, and the taps are
//! carried through `Showing` by the made-up window of F47 (`support/tapper.rs`), a process of its own. Nothing
//! here starts the real Claude Code or opens a window.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "work_shows/runs.rs"]
#[allow(dead_code)]
mod runs;
mod support;

use std::fs;

use runs::{day, ended, wish};
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

#[test]
fn a_change_that_no_longer_fits_is_said_plainly_and_make_it_again_makes_the_wish_again_and_then_tosses_it() {
    let w = two_that_clash();
    let repo = w.at("work").join("alpha");
    // The new wish's run plays another script, so its work is told from the two before it.
    w.play("hello");
    let bin = format!("bin:{}", w.install().parent().unwrap().display());
    let again = ["choose:said-m2:0:Keep it", "choose:said-m2:0", "run"];
    let lines = w.taps("logs", &[&[bin.as_str()][..], &KEEP_BOTH[..], &again[..]].concat()).ended();
    let [_, _, first, _, refused, stale, at_once, answer] = &lines[..] else { panic!("eight lines: {lines:#?}") };
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
    wait_until("the new wish's run", || kept(&w)[2]["run"].is_string());
    let new = kept(&w)[2]["run"].as_str().unwrap().to_string();
    wait_until("the end of the run", || !w.at("jm").join("runs").join(&new).join("run.lock").exists());
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
