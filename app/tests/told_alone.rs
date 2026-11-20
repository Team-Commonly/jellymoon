//! Told alone: a made wish whose run ended with no work to wait still says what happened. A run that finished and
//! changed nothing has what it says it did shown, with Jellymoon's sentence that the folder was not changed, when
//! a look finds it ended and on a tap on its character; a run that did not finish says so in Jellymoon's fixed
//! words. The wishes are made and run by the `jellymoon` command with the made-up agent of F39's tests, whose
//! script `told` ends with the last word a test typed; the looks and taps are carried by the made-up window of F47
//! (`support/tapper.rs`). Nothing here starts the real Claude Code or opens a window.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::fs;

use serde_json::json;
use sticker_kit::{Note, Scene, ToldRow};
use support::day::DAY;
use support::wished::{SAID, World, quiet, wait_until};

const TITLE: &str = "What it says it did";
const UNCHANGED: &str = "Nothing in your folder was changed.";
const NOTHING_RUN: &str = "Nothing was run to try this. A run reads and edits files, and no more.";
const UNCHECKED: &str = "Something the run said could not be checked, so it is not here.";
const DONE: &str = "This wish is done.";
const NEEDS: &str = "This wish needs you.";
const UNFINISHED: [&str; 2] = [
    "It did not finish, and nothing in your folder was changed.",
    "You can type the wish again, or type it with what should be different.",
];
/// What the run says it did: the answer to a question, which is the whole of what such a wish asked for.
const DID: &str = "The shop opens at nine. $(touch ran) <b>bold</b>";
const DECIDED: &str = "It read the hours from the shop's page.";

/// The looks that show no work after a choice, a core's answer, or a note in hand (`showing.rs`).
const HOLD: usize = 3;
const CHOICES: [&str; 3] = ["Show me", "Keep it", "Toss it"];

/// A day nothing was typed on, with `alpha` listed.
fn day() -> World {
    let w = quiet();
    let repo = w.at("work").join("alpha");
    let added = w.jellymoon(&["project", "add", "--repo", repo.to_str().unwrap(), "--date", DAY]);
    assert_eq!(added["name"], "alpha", "{added}");
    fs::create_dir(w.at("harness")).unwrap();
    w
}

/// A wish made for `alpha`: its id.
fn wish(w: &World) -> String {
    let made = w.jellymoon(&["wish", "--date", DAY, "--say", SAID, "--project", "alpha"]);
    made["wish"]["id"].as_str().unwrap().to_string()
}

/// The run of the wish `id`, from its start to its end: what it ended as.
fn run(w: &World, id: &str) -> String {
    let logs = w.at("logs");
    let ran = w.jellymoon(&["run", "--wish", id, "--date", DAY, "--logs", logs.to_str().unwrap()]);
    ran["outcome"].as_str().unwrap().to_string()
}

/// A wish whose run the made-up agent ended with `word` and the exit `exit`, as `outcome`: its id.
fn told(w: &World, word: &str, exit: usize, outcome: &str) -> String {
    fs::write(w.at("rec").join("word"), word).unwrap();
    fs::write(w.at("agent").join("play"), format!("told\n{}\n{exit}\n", w.at("rec").display())).unwrap();
    let id = wish(w);
    assert_eq!(run(w, &id), outcome);
    id
}

/// A wish whose run left work that waits: its id.
fn edits(w: &World) -> String {
    w.play("edits");
    let id = wish(w);
    assert_eq!(run(w, &id), "completed");
    id
}

/// A day with one wish, `m1`, whose run the made-up agent ended with `word` and the exit `exit`, as `outcome`.
fn ended(word: &str, exit: usize, outcome: &str) -> World {
    let w = day();
    assert_eq!(told(&w, word, exit, outcome), "m1");
    w
}

/// The last word of a run that is done and says it did [`DID`], made a change to a file of the repo as it is,
/// and decided [`DECIDED`].
fn done() -> String {
    let change = json!({"type": "change", "file": "a.txt", "before": ["a.txt as it was"], "after": ["it is"]});
    let pieces = json!([{"type": "did", "text": DID}, change, {"type": "decided", "text": DECIDED}]);
    json!({"state": "done", "result": {"pieces": pieces}}).to_string()
}

fn id_of(note: &Note) -> Option<&str> {
    match note {
        Note::Text { id, .. } | Note::Choice { id, .. } | Note::Told { id, .. } => Some(id),
        _ => None,
    }
}

fn ids(scene: &Scene) -> Vec<&str> {
    scene.notes.iter().filter_map(id_of).collect()
}

fn note<'a>(scene: &'a Scene, id: &str) -> &'a Note {
    scene.notes.iter().find(|note| id_of(note) == Some(id)).unwrap_or_else(|| panic!("no {id}: {scene:?}"))
}

fn row(label: &str, quote: &str) -> ToldRow {
    ToldRow { label: label.to_string(), quotes: vec![quote.to_string()], ..ToldRow::default() }
}

/// The wish's own note: its title and texts, with no option, and whether it has a field (go on from an answer).
fn plain(scene: &Scene) -> (String, Vec<String>, bool) {
    let Note::Text { title, body, options, field, quoted, .. } = note(scene, "wish-m1") else { panic!("{scene:?}") };
    assert!(options.is_empty() && quoted.is_empty(), "{scene:?}");
    (title.clone(), body.clone(), field.is_some())
}

fn shown(line: &str) -> Option<Scene> {
    serde_json::from_str(line).expect(line)
}

#[test]
fn a_run_that_changed_nothing_shows_what_it_says_it_did_over_the_wish_s_note_and_no_choice() {
    let w = ended(&done(), 0, "completed");
    let opened = w.taps("logs", &["poke:m1"]).showed().remove(0).expect("a tap opens the wish");
    assert_eq!(ids(&opened), ["told-m1", "wish-m1"], "the told note over the wish's own, and no work's note");
    // Every text of the run is a quote, and the change, which no file of a work can hold, is dropped and said.
    let Note::Told { title, rows, end, .. } = note(&opened, "told-m1") else { panic!("{opened:?}") };
    assert_eq!(title, TITLE);
    assert_eq!(rows, &[row("", DID), row("Decided", DECIDED)]);
    assert_eq!(end, &[UNCHANGED, NOTHING_RUN, UNCHECKED]);
    assert_eq!(plain(&opened), (DONE.to_string(), vec![format!("“{SAID}”")], true));
    // A run's text is never a title, a label, a sentence of Jellymoon's, an option or a field.
    let text = serde_json::to_string(&opened).unwrap();
    assert_eq!(text.matches("The shop opens").count(), 1, "{text}");
    assert_eq!(text.matches("It read the hours").count(), 1, "{text}");
    assert!(!w.at("work").join("alpha").join("ran").exists() && !w.at("ran").exists(), "a text of the run was run");
}

#[test]
fn a_look_that_finds_it_ended_shows_it_once_as_a_tap_does() {
    let w = ended(&done(), 0, "completed");
    let lines = w.taps("logs", &["look", "look", "look"]).ended();
    let [first, again, third] = &lines[..] else { panic!("three lines: {lines:#?}") };
    let first = shown(first).expect("the first look shows the newest wish's end");
    let tapped = w.taps("logs", &["poke:m1"]).showed().remove(0).expect("a tap opens the wish");
    assert_eq!(first.notes, tapped.notes);
    assert_eq!((again.as_str(), third.as_str()), ("null", "null"), "shown once");
}

#[test]
fn a_run_that_changed_nothing_and_said_nothing_is_done_as_before() {
    let w = ended(r#"{"state":"done"}"#, 0, "completed");
    let opened = w.taps("logs", &["poke:m1"]).showed().remove(0).expect("a tap opens the wish");
    assert_eq!(ids(&opened), ["wish-m1"]);
    assert_eq!(plain(&opened), (DONE.to_string(), vec![format!("“{SAID}”")], true));
    assert_eq!(w.taps("logs", &["look"]).ended(), ["null"], "a look shows nothing of it");
}

#[test]
fn a_run_that_did_not_finish_says_so_in_jellymoon_s_words_and_nothing_of_core_s_reason() {
    let w = ended(r#"{"state":"thinking"}"#, 0, "failed");
    let opened = w.taps("logs", &["poke:m1"]).showed().remove(0).expect("a tap opens the wish");
    assert_eq!(ids(&opened), ["wish-m1"]);
    let said = [format!("“{SAID}”"), UNFINISHED[0].to_string(), UNFINISHED[1].to_string()];
    assert_eq!(plain(&opened), (NEEDS.to_string(), said.to_vec(), false));
    let text = serde_json::to_string(&opened).unwrap();
    // Core's reason reads "run <id> did not finish (its last word was not `done` alone, ...)".
    for word in ["`", "did not finish (", "last word", "its copy", "r0", "run r"] {
        assert!(!text.contains(word), "{word:?}: {text}");
    }
    assert_eq!(w.taps("logs", &["look"]).ended(), ["null"], "a look does not open it");
}

// A run that said it was done and what it did, and whose harness then exited with a failure, did not finish:
// core keeps nothing of what it said, and a result written into its folder by hand is not read either.
#[test]
fn a_run_that_did_not_finish_shows_no_result_though_one_is_in_its_folder() {
    let w = ended(&done(), 3, "failed");
    let runs = fs::read_dir(w.at("jm").join("runs")).unwrap().flatten();
    let run = runs.map(|entry| entry.path()).find(|path| path.join("run.json").is_file()).expect("the run's folder");
    let file = run.join("result.json");
    assert!(!file.exists());
    fs::write(&file, json!({"pieces": [{"type": "did", "text": DID}]}).to_string()).unwrap();
    #[cfg(unix)]
    fs::set_permissions(&file, std::os::unix::fs::PermissionsExt::from_mode(0o600)).unwrap();
    let opened = w.taps("logs", &["poke:m1"]).showed().remove(0).expect("a tap opens the wish");
    assert_eq!(ids(&opened), ["wish-m1"]);
    assert_eq!(plain(&opened).1[1..], UNFINISHED);
}

fn options(scene: &Scene, id: &str) -> Vec<String> {
    match note(scene, id) {
        Note::Text { options, .. } => options.clone(),
        _ => Vec::new(),
    }
}

/// The ids of a look's scene, or none when it showed nothing.
fn looked(line: &str) -> Vec<String> {
    shown(line).map_or(Vec::new(), |scene| ids(&scene).into_iter().map(str::to_string).collect())
}

// Told alone never takes the first look from work that waits: the older wish's work is shown, with its choices.
#[test]
fn the_first_look_shows_an_older_wish_s_work_that_waits_before_a_newer_wish_told_alone() {
    let w = day();
    assert_eq!((edits(&w), told(&w, &done(), 0, "completed")), ("m1".to_string(), "m2".to_string()));
    let lines = w.taps("logs", &["look"]).ended();
    let first = shown(&lines[0]).expect("the first look shows the work that waits");
    assert_eq!(ids(&first), ["work-m1"]);
    assert_eq!(options(&first, "work-m1"), CHOICES);
}

// Two wishes end while the window looks, the one told alone first in core's order: the other's work is shown, and
// the one passed over still has its turn once the person has chosen and the looks after it are held.
#[test]
fn a_wish_told_alone_passed_over_for_work_that_waits_is_shown_after_the_person_chose() {
    let w = day();
    let made = w.at("tmp").join("made");
    let acts = ["look", &format!("hold:{}", made.display()), "look", "look", "choose:work-m2:2", "run"];
    let mut window = w.taps("logs", &[&acts[..], &["look"; HOLD + 1][..]].concat());
    window.printed(1);
    assert_eq!((told(&w, &done(), 0, "completed"), edits(&w)), ("m1".to_string(), "m2".to_string()));
    fs::write(&made, "").unwrap();
    let lines = window.ended();
    let [_, drawn, work, busy, tossed, held @ .., last] = &lines[..] else { panic!("{lines:#?}") };
    assert!(looked(drawn).iter().all(|id| !id.starts_with("work-") && !id.starts_with("told-")), "{drawn}");
    assert_eq!(looked(work), ["work-m2"], "the work that waits, over the wish told alone");
    assert_eq!((looked(busy), looked(tossed)), (vec!["busy-m2".to_string()], vec!["said-m2".to_string()]));
    assert_eq!(held, ["null"; HOLD]);
    assert_eq!(looked(last)[..2], ["told-m1", "wish-m1"], "the wish passed over has its turn");
}

/// How core keeps the made wish `id`.
fn state(w: &World, id: &str) -> String {
    let kept = fs::read_to_string(w.at("jm").join("pages").join(format!("{DAY}.wishes.json"))).unwrap_or_default();
    let kept: serde_json::Value = serde_json::from_str(&kept).unwrap_or_default();
    let wishes = kept["wishes"].as_array().cloned().unwrap_or_default();
    wishes.iter().find(|wish| wish["id"] == id).map_or(String::new(), |wish| wish["state"].as_str().unwrap().into())
}

// A note told alone, shown by a look, stays: another wish's work that ends meanwhile is not put over it. Its wish's
// note has the field of a follow-up (go on from an answer), so it is in the person's hand as any field is, and
// holds every look while it is on show, past the looks an answer to a choice holds.
#[test]
fn a_note_told_alone_shown_by_a_look_holds_the_looks_after_it() {
    let w = day();
    assert_eq!(told(&w, &done(), 0, "completed"), "m1");
    w.play("slow");
    assert_eq!(wish(&w), "m2");
    let lines = std::thread::scope(|scope| {
        let going = scope.spawn(|| run(&w, "m2"));
        wait_until("the run", || state(&w, "m2") == "working");
        let ended = w.at("tmp").join("ended");
        let acts = ["look", &format!("hold:{}", ended.display())];
        let mut window = w.taps("logs", &[&acts[..], &["look"; HOLD + 2][..]].concat());
        window.printed(1);
        fs::write(w.at("rec").join("go"), "").unwrap();
        assert_eq!(going.join().unwrap(), "completed");
        fs::write(&ended, "").unwrap();
        window.ended()
    });
    let [first, held @ ..] = &lines[..] else { panic!("{lines:#?}") };
    assert_eq!((looked(first)[..2].to_vec(), held.len()), (vec!["told-m1".to_string(), "wish-m1".into()], HOLD + 2));
    for line in held {
        assert!(!looked(line).iter().any(|id| id == "work-m2"), "the told note was taken away: {line}");
    }
}
