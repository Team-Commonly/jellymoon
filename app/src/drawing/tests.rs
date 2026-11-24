//! A wish's work as drawn notes, with no core and no window: the work is typed here, and what a person would see
//! is read from the pieces the kit places and from their print.

use std::path::PathBuf;

use jellymoon_core::{KeptItself, Stands, Waits, WorkDone, WorkTap};
use sticker_kit::{
    BundledMeasure, Character, Mark, PICTURE_PLACE, PieceKind, Placed, Theme, ToldRow, Viewport, layout, print,
};

use super::*;
use crate::answering::{MORE, with_go_on, with_more};
use crate::showing::Showing;
use crate::taps::OnShow;
use crate::today::Today;
use crate::working::{notes_of, waiting_notes};

const CHOICES: [&str; 3] = ["Show me", "Keep it", "Toss it"];
const DID: &str = "It now waits and tries the same file again.";

/// Work of the made wish `m1` that waits: three files made, one changed, one removed, and two of them named.
fn waits() -> Waits {
    Waits {
        wish: "m1".to_string(),
        run: "r1".to_string(),
        said: "tidy the uploader".to_string(),
        read: true,
        made: 3,
        changed: 1,
        removed: 1,
        names: vec!["a.txt".to_string(), "b.txt".to_string()],
        more: 3,
        obeyed: Vec::new(),
        plain: true,
        why: "the work of run r1 waits".to_string(),
        result: None,
        project: "alpha".to_string(),
        unkept: None,
    }
}

fn told(wish: &str, name: &str) -> Note {
    let row = ToldRow { name: name.to_string(), quotes: vec![DID.to_string()], ..ToldRow::default() };
    Note::Told { id: format!("told-{wish}"), title: "What it says it did".to_string(), rows: vec![row], end: vec![] }
}

fn text(id: &str, title: &str) -> Note {
    let (body, options) = (vec!["Nothing was changed.".to_string()], vec!["Keep it, with those files".to_string()]);
    Note::Text { id: id.to_string(), title: title.to_string(), body, options, quoted: String::new(), field: None }
}

/// The page of the made wish `m1` with these notes.
fn page(notes: Vec<Note>) -> Scene {
    let made = Note::Whole { title: crate::scene::MADE.to_string(), rows: vec![(Mark::Star, "tidy".to_string())] };
    let m1 = Character { id: "m1".to_string(), ..Character::default() };
    Scene { characters: vec![m1], notes: notes.into_iter().chain([made]).collect(), ..Scene::default() }
}

fn work() -> Note {
    waiting_notes(&waits(), false).pop().unwrap_or_else(|| text("none", "no note"))
}

fn placed(scene: &Scene) -> Vec<Placed> {
    let theme = Theme::sticker();
    layout(scene, &theme, Viewport { w: 1440.0, h: 900.0 }, &BundledMeasure::new(&theme))
}

/// Whether a piece of the note `id` is placed, and the page's print.
fn seen(scene: &Scene, id: &str) -> (bool, String) {
    let placed = placed(scene);
    (placed.iter().any(|piece| piece.id == id), print(&placed))
}

// Work that waits is its sheets: core's three counts, and never the number of the names; core's choices in
// core's order; core's title and then core's lines as its words. A tap on a choice is the tap it was.
#[test]
fn work_that_waits_is_drawn_with_core_s_counts_and_core_s_choices_in_their_order() {
    let (waits, note) = (waits(), work());
    let Note::Drawn { id, picture, words, open, options, field } = &note else { panic!("a drawn note: {note:?}") };
    assert_eq!((id.as_str(), *open, field), ("work-m1", false, &None));
    assert_eq!(*picture, Picture::Sheets { made: 3, changed: 1, removed: 1, state: Kept::Waiting });
    assert_eq!(options, &CHOICES);
    let counted = Waits { names: Vec::new(), more: 5, ..waits.clone() };
    assert_eq!(words, &[vec![waits.title().to_string()], counted.lines()].concat(), "no file is named yet");
    assert!(!format!("{note:?}").contains("a.txt"));
    // What a tap is made of is the note's lines and its options: each place is the tap it was under a text note.
    let (body, choices, _) = held(&note).expect("a note with choices");
    assert_eq!(body, counted.lines());
    let taps: Vec<Option<WorkTap>> = (0..4).map(|at| WorkTap::of(choices.get(at)?, body, 0)).collect();
    let keep = WorkTap::Keep { yes_to: None };
    assert_eq!(taps, [Some(WorkTap::Show { after: 0 }), Some(keep), Some(WorkTap::Toss), None]);
    // After `Show me` the note is the same picture, and its words are the lines core says then, with the names.
    let Some(Note::Drawn { picture: after, words, .. }) = waiting_notes(&waits, true).pop() else { panic!() };
    assert_eq!((&after, words), (picture, [vec![waits.title().to_string()], waits.lines()].concat()));
}

// What holds a refusal or someone else's words stays text: work that was not read, and core's answer to a keep
// or a toss, done or refused, which core does not tell apart but by its sentences.
#[test]
fn work_that_was_not_read_and_every_answer_to_a_keep_or_a_toss_stay_text_notes() {
    let unread = Waits { read: false, made: 0, changed: 0, removed: 0, ..waits() };
    let Some(Note::Text { id, title, options, .. }) = waiting_notes(&unread, false).pop() else { panic!() };
    assert_eq!((id.as_str(), title.as_str(), options.len()), ("work-m1", unread.title(), 0));
    for (title, body) in [
        ("The change was kept.", "The change is in your folder alpha now."),
        ("The change was thrown away.", "The change was thrown away, and your folder alpha is as it was."),
        ("The change was not kept.", "Open the wish again to see where the change stands."),
    ] {
        let said = notes_of("m1", WorkDone::Said(title, vec![body.to_string()]));
        let [Note::Text { id, title: own, body: texts, options, .. }] = &said[..] else { panic!("{said:?}") };
        assert_eq!((id.as_str(), own.as_str(), texts.len(), options.len()), ("said-m1", title, 1, 0));
    }
}

// The field of a follow-up goes under a drawn note as under a text note, with the same key.
#[test]
fn a_follow_up_s_field_goes_under_the_drawn_note_of_work() {
    let offered = with_more(work(), "m1", true);
    let Note::Drawn { field: Some(field), options, .. } = &offered else { panic!("{offered:?}") };
    assert_eq!((field.offer.as_str(), field.key.as_str(), options.len()), (MORE, "more-m1", 3));
    assert_eq!(with_more(work(), "m1", false), work());
    let alone = with_go_on(work(), "m1", true);
    assert!(matches!(&alone, Note::Drawn { field: Some(field), .. } if field.offer.is_empty()), "{alone:?}");
    let pieces = placed(&column(&page(vec![offered]), &BTreeSet::new()));
    let kinds = [PieceKind::Picture, PieceKind::Option, PieceKind::Offer, PieceKind::Field];
    assert!(kinds.iter().all(|kind| pieces.iter().any(|piece| piece.piece == *kind && piece.id == "work-m1")));
}

// Closed, a finished wish is one picture with its choices: no sentence of its note and nothing of what its run
// says it did is placed. A tap on the picture brings every word and the told note; a second folds them away.
// A look, which shows the scene again, changes neither.
#[test]
fn the_sentences_and_what_the_run_says_come_with_a_tap_on_the_picture_and_go_with_the_next() {
    let scene = page(vec![told("m1", ""), work()]);
    let Note::Drawn { words, .. } = work() else { panic!() };
    let open = Open::default();
    let closed = |open: &Open, when: &str| {
        let (told, print) = seen(&open.column(&scene), "told-m1");
        assert!(!told && !print.contains(DID), "{when}: {print}");
        assert!(words.iter().all(|word| !print.contains(word.as_str())), "{when}: {print}");
        assert!(CHOICES.iter().all(|choice| print.contains(choice)) && print.contains("picture work-m1"), "{when}");
    };
    closed(&open, "at first");
    closed(&open, "after a look");
    assert!(open.tapped(&scene, "work-m1"));
    for when in ["after the tap", "after a look", "after another look"] {
        let (told, print) = seen(&open.column(&scene), "told-m1");
        assert!(told && print.contains(DID), "{when}: {print}");
        // Each word is drawn to its end, on as many lines as that takes: the print holds them line by line.
        let lines: String = print.replace("\" \"", " ");
        assert!(words.iter().all(|word| lines.contains(word.as_str())), "{when}: {print}");
        assert!(CHOICES.iter().all(|choice| print.contains(choice)), "{when}");
    }
    assert!(open.tapped(&scene, "work-m1"));
    closed(&open, "after the second tap");
    // A tap on what is no drawn note of the scene is nothing.
    assert!(!open.tapped(&scene, "told-m1") && !open.tapped(&scene, "work-m2"));
    closed(&open, "after a tap elsewhere");
}

// Where a wish's work is no drawn note, what its run says it did is on the page as it was: beside a keep asked
// once more, and alone.
#[test]
fn a_told_note_beside_a_text_note_of_its_wish_or_alone_is_placed() {
    let open = Open::default();
    for notes in
        [vec![told("m1", ""), text("sure-m1", "This work touches files your tools obey.")], vec![told("m1", "")]]
    {
        let (told, print) = seen(&open.column(&page(notes)), "told-m1");
        assert!(told && print.contains(DID), "{print}");
    }
    // Another wish's closed work folds only its own told note.
    let (told, _) = seen(&open.column(&page(vec![told("m2", ""), work()])), "told-m2");
    assert!(told);
}

// A change kept by itself is its sheets in the folder, while core says the folder has it: core's counts, core's
// words and core's one choice, and what its run says behind the same tap. Under later commits it is drawn with no
// choice. Where the folder stands elsewhere, or the change was not read, it stays sentences, with the told note.
#[test]
fn a_change_kept_by_itself_is_drawn_in_the_folder_only_while_core_says_the_folder_has_it() {
    let here = KeptItself { waits: waits(), stands: Stands::Here };
    let note = crate::landing::kept_note(&here);
    let Note::Drawn { id, picture, words, options, .. } = &note else { panic!("a drawn note: {note:?}") };
    assert_eq!((id.as_str(), options), ("kept-m1", &vec!["Undo it".to_string()]));
    assert_eq!(*picture, Picture::Sheets { made: 3, changed: 1, removed: 1, state: Kept::In });
    assert_eq!(words, &[vec![here.title().to_string()], here.lines()].concat());
    let scene = page(vec![told("m1", ""), note.clone()]);
    let open = Open::default();
    let (told_closed, print) = seen(&open.column(&scene), "told-m1");
    assert!(!told_closed && print.contains("Undo it") && !print.contains(here.title()), "{print}");
    assert!(open.tapped(&scene, "kept-m1"));
    let (told_open, print) = seen(&open.column(&scene), "told-m1");
    assert!(told_open && print.contains(DID), "{print}");

    let past = crate::landing::kept_note(&KeptItself { stands: Stands::Past, ..here.clone() });
    assert!(matches!(&past, Note::Drawn { options, .. } if options.is_empty()), "{past:?}");
    let unread = Waits { read: false, ..waits() };
    for stays in [KeptItself { stands: Stands::Elsewhere, ..here.clone() }, KeptItself { waits: unread, ..here }] {
        let note = crate::landing::kept_note(&stays);
        assert!(matches!(&note, Note::Text { id, .. } if id == "kept-m1"), "{note:?}");
        let (told_beside, _) = seen(&Open::default().column(&page(vec![told("m1", ""), note])), "told-m1");
        assert!(told_beside);
    }
}

// An id that leaves the page leaves the set; and what `Show me` brings opens the note it was asked of.
#[test]
fn a_note_that_left_the_page_comes_back_closed_and_show_me_opens_the_work_it_shows() {
    let (scene, open) = (page(vec![told("m1", ""), work()]), Open::default());
    let is_open = |open: &Open, scene: &Scene| {
        let now = open.column(scene);
        now.notes.iter().any(|note| matches!(note, Note::Drawn { open: true, .. }))
    };
    assert!(open.tapped(&scene, "work-m1") && is_open(&open, &scene));
    assert!(!is_open(&open, &page(vec![text("busy-m1", "That work is being read.")])));
    assert!(!is_open(&open, &scene), "the note is back, and closed");
    // `Show me` brought the rows of the changes, or a file: the note is open, and the rows are on the page.
    let file = text("file-2-m1", "File 2 of the work: b.txt");
    for brought in [vec![told("m1", "a.txt"), work()], vec![work(), file, told("m1", "")]] {
        let (shown, open) = (page(brought), Open::default());
        assert_eq!(open.showed(shown.clone()), shown, "the scene is given back as it is");
        assert!(is_open(&open, &shown) && seen(&open.column(&shown), "told-m1").0);
    }
    // An answer that shows nothing opens nothing: the rows of another wish, or no rows at all.
    for other in [vec![told("m2", "a.txt"), work()], vec![told("m1", ""), work()]] {
        let (shown, open) = (page(other), Open::default());
        open.showed(shown.clone());
        assert!(!is_open(&open, &shown));
    }
}

// The window's own call: a tap at the picture's place opens the note on the scene on show and asks core nothing.
#[test]
fn a_tap_at_the_picture_s_place_opens_and_closes_the_note_on_show() {
    let none = PathBuf::from("no-such-folder");
    let today = Today {
        date: "2026-03-10".to_string(),
        logs: none.clone(),
        skip: Vec::new(),
        model: String::new(),
        home: none.clone(),
        harness: Some(none),
        fresh: false,
        settings: None,
        bin: None,
    };
    let scene = page(vec![told("m1", ""), work()]);
    let shown = Showing::of(Some(OnShow { today, page: None, scene: scene.clone() }));
    let told_on = |shown: &Showing| seen(&shown.column(&scene), "told-m1").0;
    assert!(!told_on(&shown));
    let tapped = shown.chose("work-m1", PICTURE_PLACE, "");
    assert!(matches!(&tapped, Some((now, None)) if *now == scene), "the scene on show, and nothing left to do");
    assert!(told_on(&shown) && told_on(&shown));
    assert!(shown.chose("told-m1", PICTURE_PLACE, "").is_none() && told_on(&shown));
    assert!(shown.chose("work-m1", PICTURE_PLACE, "").is_some() && !told_on(&shown));
    assert_eq!(shown.scene(), Some(scene), "the scene the app keeps is as it was");
    assert!(Showing::default().chose("work-m1", PICTURE_PLACE, "").is_none(), "a page with no day has no such tap");
}
