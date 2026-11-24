//! Every wish keeps its place (ADR 0030; `docs/development/app.md`, "Every wish keeps its place"): each made wish
//! of the day has its own track while its run goes and its own notes when it has ended, under its own bubble, with
//! no tap; a tap goes to the wish of the note it was made on; and the offer of a follow-up is under one wish. The
//! wishes are made and run by the `jellymoon` command with the made-up agent of F39's tests, and the looks and
//! taps are carried through `Showing` by the made-up window of F47 (`support/tapper.rs`), a process of its own,
//! whose `column` act prints the scene as `Showing::column` gives it to the view. Nothing here starts the real
//! Claude Code or opens a window.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "work_shows/runs.rs"]
#[allow(dead_code)]
mod runs;
mod support;

use std::path::Path;
use std::{fs, thread};

use runs::{day, ended, run, state, wish};
use serde_json::{Value, json};
use sticker_kit::{Note, Scene};
use support::day::DAY;
use support::wished::{World, wait_until};
use support::worked::{finished, landed};

const CHOICES: [&str; 3] = ["Show me", "Keep it", "Toss it"];
const GOING: &str = "Your wish is being worked on.";
const MORE: &str = "More like this";
/// A window tall enough that `layout` places every choice of two finished wishes: a tap is on a placed piece.
const TALL: &str = "size:1440x3000";

fn scene(line: &str) -> Option<Scene> {
    serde_json::from_str(line).expect(line)
}

fn id(note: &Note) -> &str {
    match note {
        Note::Text { id, .. }
        | Note::Choice { id, .. }
        | Note::Told { id, .. }
        | Note::Said { id, .. }
        | Note::Drawn { id, .. } => id,
        Note::Plain { title, .. } | Note::List { title, .. } | Note::Whole { title, .. } => title,
    }
}

/// The notes of the made wish `wish` on a scene, by their ids, in the scene's order.
fn of<'a>(scene: &'a Scene, wish: &str) -> Vec<&'a Note> {
    let named = |note: &&Note| id(note).strip_suffix(wish).is_some_and(|start| start.ends_with('-'));
    scene.notes.iter().filter(named).collect()
}

fn ids(notes: &[&Note]) -> Vec<String> {
    notes.iter().map(|note| id(note).to_string()).collect()
}

/// What stands under the bubble of `wish` in a column: every note from its typed words to the next wish's.
fn under(column: &Scene, wish: &str) -> Vec<String> {
    let all: Vec<&str> = column.notes.iter().map(id).collect();
    let at = all.iter().position(|one| *one == format!("typed-{wish}")).expect("the wish's bubble");
    all[at + 1..].iter().take_while(|one| !one.starts_with("typed-")).map(|one| one.to_string()).collect()
}

/// The options of the note `id`, and whether it is a drawn note that is open.
fn options(scene: &Scene, named: &str) -> Option<(Vec<String>, bool)> {
    scene.notes.iter().find(|note| id(note) == named).map(|note| match note {
        Note::Text { options, .. } => (options.clone(), false),
        Note::Drawn { options, open, .. } => (options.clone(), *open),
        _ => (Vec::new(), false),
    })
}

fn choices(scene: &Scene, named: &str) -> Vec<String> {
    options(scene, named).map(|(options, _)| options).unwrap_or_default()
}

/// The notes with the field of a follow-up under them.
fn offers(scene: &Scene) -> Vec<&str> {
    let offered = |note: &&Note| match note {
        Note::Text { field: Some(field), .. } | Note::Drawn { field: Some(field), .. } => field.offer == MORE,
        _ => false,
    };
    scene.notes.iter().filter(offered).map(id).collect()
}

fn title(scene: &Scene, named: &str) -> String {
    scene.notes.iter().find(|note| id(note) == named).map_or(String::new(), |note| match note {
        Note::Text { title, .. } => title.clone(),
        Note::Drawn { words, .. } => words.first().cloned().unwrap_or_default(),
        _ => String::new(),
    })
}

fn hold(w: &World, name: &str) -> String {
    format!("hold:{}", w.at("tmp").join(name).display())
}

/// A second wish for `alpha`, said as `said`, and its run with the made-up agent playing `script`: its id.
fn another(w: &World, repo: &Path, script: &str, said: &str) -> String {
    w.play(script);
    let made = w.jellymoon(&["wish", "--date", DAY, "--say", said, "--project", "alpha"]);
    let id = made["wish"]["id"].as_str().unwrap().to_string();
    let (logs, named) = (w.at("logs"), repo.to_str().unwrap());
    let ran = w.jellymoon(&["run", "--wish", &id, "--date", DAY, "--repo", named, "--logs", logs.to_str().unwrap()]);
    assert_eq!(ran["outcome"], "completed", "{ran}");
    id
}

#[test]
fn two_wishes_at_work_have_each_their_track_and_one_that_ends_has_its_notes_while_the_other_goes_on() {
    let w = day();
    assert_eq!((wish(&w), wish(&w)), ("m1".to_string(), "m2".to_string()));
    w.play("slow");
    thread::scope(|scope| {
        let runs = ["m1", "m2"].map(|id| scope.spawn(|| run(&w, id)));
        wait_until("the agents", || w.starts() == 2);
        wait_until("the runs", || ["m1", "m2"].iter().all(|id| state(&w, id) == "working"));
        let acts = ["look", "column", "poke:m1", "choose:stop-m1:0", "run", "look", "column"];
        let lines = w.taps("logs", &acts).ended();
        fs::write(w.at("rec").join("go"), "").unwrap();
        let [_, other] = runs.map(|run| run.join().unwrap());
        assert_eq!(other, "completed", "the other run went on to its end");
        let [_, both, .., one] = &lines[..] else { panic!("seven lines: {lines:#?}") };
        // Two tracks, each under the words typed for its wish, each saying that one wish is worked on.
        let both = scene(both).expect("a column");
        assert_eq!((under(&both, "m1"), under(&both, "m2")), (vec!["steps-m1".to_string()], vec!["steps-m2".into()]));
        assert_eq!((title(&both, "steps-m1"), title(&both, "steps-m2")), (GOING.to_string(), GOING.to_string()));
        // The stopped wish has the notes of a wish that has ended, under its own bubble, and no track.
        let one = scene(one).expect("a column");
        let ended = under(&one, "m1");
        assert!(ended.contains(&"wish-m1".to_string()) || ended.contains(&"work-m1".to_string()), "{ended:?}");
        assert!(!ended.contains(&"steps-m1".to_string()), "{ended:?}");
        assert_eq!(under(&one, "m2"), ["steps-m2"], "the other wish's track is where it was");
    });
}

#[test]
fn two_changes_kept_by_themselves_have_each_their_note_and_the_offer_is_under_one_and_goes_with_a_tap() {
    let (w, repo) = landed("edits");
    let first = ["look", "column", &hold(&w, "landed")];
    let taps = ["look", "column", "poke:m1", "column", "poke:m1", "column"];
    let mut window = w.taps("logs", &[&[TALL][..], &first[..], &taps[..]].concat());
    window.printed(2);
    // The second change lands while the page is on show.
    assert_eq!(another(&w, &repo, "hello", "say hello"), "m2");
    fs::write(w.at("tmp").join("landed"), "").unwrap();
    let lines = window.ended();
    let [alone, _, _, both, _, opened, _, closed] = &lines[..] else { panic!("eight lines: {lines:#?}") };
    let alone = scene(alone).expect("the first look shows the kept change");
    assert_eq!(choices(&alone, "kept-m1"), ["Undo it"]);
    // Both notes, with no tap: the older one is made again when the newer lands, and can be undone no more.
    let both = scene(both).expect("a column");
    assert!(under(&both, "m1").contains(&"kept-m1".to_string()) && under(&both, "m2").contains(&"kept-m2".into()));
    assert_eq!(choices(&both, "kept-m1"), [""; 0], "the older change has a commit over it");
    assert_eq!(choices(&both, "kept-m2"), ["Undo it"]);
    assert_eq!(offers(&both), ["kept-m2"], "the offer is under the newest wish, and under that one alone");
    // A tap on the older wish's little one: the offer is under it, and its drawn note is open.
    let opened = scene(opened).expect("a column");
    assert_eq!(offers(&opened), ["kept-m1"]);
    assert_eq!(options(&opened, "kept-m1").map(|(_, open)| open), Some(true));
    assert_eq!(options(&opened, "kept-m2"), Some((vec!["Undo it".to_string()], false)), "nothing else moves");
    // A second tap closes it, and the offer stays.
    let closed = scene(closed).expect("a column");
    assert_eq!((offers(&closed), options(&closed, "kept-m1").map(|(_, open)| open)), (vec!["kept-m1"], Some(false)));
    assert!(options(&closed, "kept-m2").is_some());
}

#[test]
fn a_keep_and_a_toss_go_each_to_the_wish_of_the_note_tapped_and_take_nothing_of_the_other_away() {
    let w = day();
    assert_eq!((ended(&w, "edits", "completed"), ended(&w, "edits", "completed")), ("m1".into(), "m2".into()));
    let repo = w.at("work").join("alpha");
    let acts = [TALL, "look", "choose:work-m1:1", "run", "poke:m2", "choose:work-m2:2", "run"];
    let lines = w.taps("logs", &acts).ended();
    let [looked, at_once, kept, asked, _, tossed] = &lines[..] else { panic!("six lines: {lines:#?}") };
    let looked = scene(looked).expect("the first look");
    assert_eq!(
        (choices(&looked, "work-m1"), choices(&looked, "work-m2")),
        (CHOICES.map(String::from).to_vec(), CHOICES.map(String::from).to_vec())
    );
    let at_once = scene(at_once).expect("a tap on Keep it");
    assert!(ids(&of(&at_once, "m1")).contains(&"busy-m1".to_string()));
    assert_eq!(choices(&at_once, "work-m2"), CHOICES, "the other wish's choices stay");
    // The older wish's change is kept, and the answer is under it. The newer wish's work waits as it did.
    let kept = scene(kept).expect("core's answer");
    assert_eq!(title(&kept, "said-m1"), "The change was kept.");
    assert!(options(&kept, "work-m1").is_none() && choices(&kept, "work-m2") == CHOICES);
    assert_eq!(fs::read_to_string(repo.join("n.txt")).ok(), Some("new from the run\n".to_string()));
    // Core, asked of the newer wish by a tap on its little one, says its work waits still.
    let asked = scene(asked).expect("a tap on the other wish");
    assert_eq!(
        (choices(&asked, "work-m2"), title(&asked, "said-m1")),
        (CHOICES.map(String::from).to_vec(), "The change was kept.".into())
    );
    // The toss is the newer wish's alone: the older wish's answer stays where it is.
    let tossed = scene(tossed).expect("core's answer");
    assert_eq!(title(&tossed, "said-m2"), "The change was thrown away.");
    assert_eq!(title(&tossed, "said-m1"), "The change was kept.");
    assert!(options(&tossed, "work-m2").is_none());
    assert_eq!(fs::read_to_string(repo.join("n.txt")).ok(), Some("new from the run\n".to_string()));
}

#[test]
fn show_me_shows_the_work_of_the_wish_it_was_tapped_on_and_the_other_wish_is_as_it_was() {
    let w = day();
    assert_eq!((ended(&w, "edits", "completed"), ended(&w, "edits", "completed")), ("m1".into(), "m2".into()));
    let lines = w.taps("logs", &[TALL, "look", "column", "choose:work-m1:0", "run", "column"]).ended();
    let [looked, before, _, shown, after] = &lines[..] else { panic!("five lines: {lines:#?}") };
    let (looked, shown) = (scene(looked).expect("the first look"), scene(shown).expect("core's answer"));
    assert_eq!(of(&shown, "m2"), of(&looked, "m2"), "the other wish is as it was");
    // What `Show me` brings is the tapped wish's: a file of its work, or the rows of its changes.
    let brought: Vec<String> =
        ids(&of(&shown, "m1")).into_iter().filter(|id| !ids(&of(&looked, "m1")).contains(id)).collect();
    let rows = |scene: &Scene| format!("{:?}", of(scene, "m1"));
    assert!(!brought.is_empty() || rows(&shown) != rows(&looked), "nothing was shown: {shown:?}");
    assert!(brought.iter().all(|id| id.starts_with("file-")), "{brought:?}");
    assert!(ids(&of(&shown, "m2")).iter().all(|id| !id.starts_with("file-")));
    // Its drawing is open, and the other's is closed, as both were before the tap.
    let (before, after) = (scene(before).expect("a column"), scene(after).expect("a column"));
    let open = |column: &Scene, id: &str| options(column, id).map(|(_, open)| open);
    assert_eq!((open(&before, "work-m1"), open(&before, "work-m2")), (Some(false), Some(false)));
    assert_eq!((open(&after, "work-m1"), open(&after, "work-m2")), (Some(true), Some(false)));
}

// The proof is by what stays on the page: core is told to keep the older wish's change behind the window's back,
// so a look that asked core of that wish would take its choices away. They stay, look after look, until a tap on
// the wish's little one, which does ask.
#[test]
fn a_look_with_nothing_changed_asks_core_nothing_of_a_wish_that_has_ended() {
    let w = day();
    assert_eq!((ended(&w, "edits", "completed"), ended(&w, "edits", "completed")), ("m1".into(), "m2".into()));
    let acts = ["look", "core:m1:1", "look", "look", "look", "look", "look", "poke:m1"];
    let lines = w.taps("logs", &acts).ended();
    let [first, core, looks @ .., tapped] = &lines[..] else { panic!("eight lines: {lines:#?}") };
    assert_eq!(serde_json::from_str::<Value>(core).unwrap()[0], json!("The change was kept."), "{core}");
    let mut on_show = scene(first).expect("the first look");
    assert_eq!(choices(&on_show, "work-m1"), CHOICES);
    for look in looks {
        on_show = scene(look).unwrap_or(on_show);
        assert_eq!(choices(&on_show, "work-m1"), CHOICES, "a look asked core of a wish that had ended: {look}");
    }
    let tapped = scene(tapped).expect("a tap on the wish");
    assert!(options(&tapped, "work-m1").is_none(), "a tap asks core of its wish: {tapped:?}");
    assert_eq!(choices(&tapped, "work-m2"), CHOICES);
}

#[test]
fn a_wish_of_the_reading_opens_and_goes_as_before_and_the_made_wishes_notes_stay() {
    let (w, _) = finished("edits");
    let file = w.at("jm").join("pages").join(format!("{DAY}.json"));
    let mut page: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
    let read = |id: &str| {
        json!({"id": id, "label": "The uploader", "said": "look", "project": "alpha", "state": "done",
            "stars": []})
    };
    page["wishes"].as_array_mut().unwrap().extend([read("a"), read("b")]);
    fs::write(&file, page.to_string()).unwrap();
    let lines = w.taps("logs", &["look", "poke:a", "poke:a", "poke:b", "poke:m1"]).ended();
    let [looked, opened, again, other, made] = &lines[..] else { panic!("five lines: {lines:#?}") };
    let looked = scene(looked).expect("the first look");
    assert_eq!(choices(&looked, "work-m1"), CHOICES);
    // The reading's wish opens over the made wish's notes, which stay.
    let opened = scene(opened).expect("a tap on a wish of the reading");
    assert!(options(&opened, "wish-a").is_some() && of(&opened, "m1") == of(&looked, "m1"), "{opened:?}");
    assert_eq!(again, "null", "its note is on show");
    // One wish of the reading is open at a time, and it goes with a tap on a made wish.
    let other = scene(other).expect("a tap on another wish of the reading");
    assert!(options(&other, "wish-b").is_some() && options(&other, "wish-a").is_none());
    assert_eq!(of(&other, "m1"), of(&looked, "m1"));
    let made = scene(made).expect("a tap on the made wish");
    assert!(options(&made, "wish-b").is_none() && choices(&made, "work-m1") == CHOICES, "{made:?}");
}
