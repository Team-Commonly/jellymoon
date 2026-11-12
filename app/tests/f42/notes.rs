//! What is offered for a wish, where core is asked, and what a wish's note says and outlives: the findings of
//! F42's review.

use std::fs;

use jellymoon_app::today::Next;
use jellymoon_app::wishing::{UNKEPT, UNSTARTED};
use serde_json::json;
use sticker_kit::{Mark, Note, Scene, State};

use super::{SAID, chips, drawn, first, kept_wishes, on_show, read};
use crate::support::day::{DAY, answer, names, shown};

fn projects(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
}

fn titles(scene: &Scene) -> Vec<&str> {
    fn title(note: &Note) -> Option<&str> {
        match note {
            Note::Plain { title, .. } => Some(title.as_str()),
            _ => None,
        }
    }
    scene.notes.iter().map_while(title).collect()
}

#[test]
fn a_project_switched_off_is_not_offered_from_the_first_scene_on_and_no_wish_is_made_for_it() {
    let world = read();
    let mut on = on_show(&world);
    on.today.skip = projects(&["private-notes"]);
    // The first scene is the kept page, which was read before the folder was switched off.
    let (_, kept) = on.today.first();
    assert_eq!(kept.as_ref().map(|page| page.data.read.projects.clone()), Some(projects(&["alpha", "private-notes"])));
    assert_eq!(on.today.entry(kept.as_ref()).unwrap().options, ["alpha"]);
    assert_eq!(on.entry().unwrap().options, ["alpha"]);
    let (scene, asking) = on.wished(SAID, Some(1));
    assert!(asking.is_none() && kept_wishes(&world).is_none(), "a wish was made for a folder switched off");
    assert_eq!(first(&scene).map(|(title, _)| title), Some(UNKEPT));

    on.today.skip.clear();
    fs::write(world.home.join("skip.txt"), "alpha\n").unwrap();
    assert_eq!(on.entry().unwrap().options, ["private-notes"], "skip.txt switches a folder off too");
    fs::write(world.home.join("skip.txt"), b"alpha\n\xff").unwrap();
    assert!(on.entry().is_none(), "a list that cannot be read offers nothing");
    fs::remove_file(world.home.join("skip.txt")).unwrap();

    // At the binary: the day cannot be read again, so the page is the one kept from the earlier reading.
    answer(&world, "not a reply", 1);
    let placed = shown(&world, &["--skip", "private-notes"]);
    assert_eq!(chips(&placed), ["alpha"]);
}

#[test]
fn only_a_name_core_takes_for_a_project_is_offered() {
    let world = read();
    let mut on = on_show(&world);
    let long = "p".repeat(150);
    on.page.as_mut().unwrap().data.read.projects = projects(&["..", ".", "new\nline", &long, "a/b", "alpha"]);
    assert_eq!(on.entry().unwrap().options, ["alpha"]);
    // The place of a chip is its place among the names offered.
    assert_eq!(drawn(&on.wished(SAID, Some(0)).0, "m1").map(|drawn| drawn.0), Some(State::Busy));
    assert_eq!(kept_wishes(&world).unwrap()[0]["project"], "alpha");
    on.page.as_mut().unwrap().data.read.projects = projects(&["..", "new\nline"]);
    assert!(on.entry().is_none(), "no name is left to offer");
}

#[test]
fn a_place_past_the_six_chips_offered_is_no_project() {
    let world = read();
    let mut on = on_show(&world);
    on.page.as_mut().unwrap().data.read.projects = projects(&["p1", "p2", "p3", "p4", "p5", "p6", "p7", "p8"]);
    assert_eq!(on.entry().unwrap().options.len(), 6);
    for past in [6, 7, 8, usize::MAX] {
        let (scene, asking) = on.wished(SAID, Some(past));
        assert!(asking.is_none(), "place {past} was taken for a project");
        let (title, body) = first(&scene).unwrap();
        assert_eq!(title, UNKEPT);
        assert!(body.ends_with("No project of this page was chosen, so nothing was sent."), "{body}");
    }
    assert_eq!(kept_wishes(&world), None);
    on.wished(SAID, Some(5));
    assert_eq!(kept_wishes(&world).unwrap()[0]["project"], "p6");
}

fn sends<T: Send>(_: &T) {}

// The window's thread makes the work and takes its answer; core is asked in between, on another thread.
#[test]
fn core_is_asked_in_work_that_can_leave_the_window_s_thread_and_its_answer_opens_no_file() {
    let world = read();
    let mut on = on_show(&world);
    let wishing = on.wishing(SAID, Some(0));
    sends(&wishing);
    sends(&on.look());
    assert_eq!(kept_wishes(&world), None, "core was asked before the work was run");
    let answer = wishing();
    let kept = json!([{"id": "m1", "said": SAID, "project": "alpha", "state": "working", "stars": []}]);
    assert_eq!(kept_wishes(&world), Some(kept));
    // What is kept is gone: the answer is drawn all the same, so taking it read nothing.
    fs::remove_file(world.home.join("pages").join(format!("{DAY}.wishes.json"))).unwrap();
    let (scene, asking, back) = on.answered(answer);
    assert_eq!(drawn(&scene, "m1"), Some((State::Busy, 0, Mark::Busy, SAID.to_string())));
    assert!(asking.is_some_and(|asking| {
        sends(&asking);
        true
    }));
    assert_eq!((back, &scene), (None, &on.scene));

    // A day that is locked: making the work waits for nothing, and the refusal is core's, when the work is run.
    let lock = world.home.join("pages").join(format!("{DAY}.ruled.lock"));
    fs::write(&lock, "").unwrap();
    let before = names(&world.home.join("pages"));
    let wishing = on.wishing(SAID, Some(0));
    assert_eq!(names(&world.home.join("pages")), before);
    let (scene, asking, back) = on.answered(wishing());
    let (title, body) = first(&scene).unwrap();
    assert!(title == UNKEPT && body.contains("another change to the page of 2026-03-10"), "{body}");
    assert!(asking.is_none() && back.as_deref() == Some(SAID));
    assert_eq!(names(&world.home.join("pages")), before);

    // An answer that comes when another scene is on show leaves its note on that scene, and reads nothing.
    let stale = on.wishing(&"w".repeat(201), Some(0))();
    fs::remove_file(&lock).unwrap();
    on.wished(SAID, Some(0));
    let (scene, _, back) = on.answered(stale);
    assert_eq!((titles(&scene), back.map(|back| back.len())), (vec![UNKEPT], Some(201)));
    assert!(drawn(&scene, "m1").is_some(), "the scene on show was replaced by an older one");
}

// Words core refuses are still the person's: the answer gives them back for the entry. These are 195 characters,
// and are over 200 once the password in the link is replaced.
#[test]
fn words_core_refuses_are_given_back_for_the_entry_and_kept_words_are_not() {
    let world = read();
    let mut on = on_show(&world);
    let link = format!("{}://u{}p@h.example", "https", ":");
    let typed = format!("{} {link}", "w".repeat(194 - link.chars().count()));
    assert_eq!(typed.chars().count(), 195);
    let answer = on.wishing(&typed, Some(0))();
    let (scene, asking, back) = on.answered(answer);
    assert!(asking.is_none() && kept_wishes(&world).is_none(), "the words were kept");
    assert_eq!(back.as_deref(), Some(typed.as_str()));
    let (title, body) = first(&scene).unwrap();
    assert_eq!(title, UNKEPT);
    assert!(body.ends_with("a wish is at most 200 characters, and this one is longer."), "{body}");
    assert!(!scene.notes.iter().any(|note| format!("{note:?}").contains("u:p@")), "the note shows what core hides");

    let answer = on.wishing(SAID, Some(0))();
    assert_eq!(on.answered(answer).2, None, "words that were kept are not typed again");
}

#[test]
fn a_wish_s_note_names_its_words_outlives_a_reading_and_an_older_asking_leaves_a_newer_note() {
    let world = read();
    let mut on = on_show(&world);
    let long = "w".repeat(201);
    let (scene, _) = on.wished(&long, Some(0));
    let unkept = format!("“{}…”: a wish is at most 200 characters, and this one is longer.", "w".repeat(40));
    assert_eq!(first(&scene), Some((UNKEPT, unkept.as_str())), "the note names the start of its wish");

    // A reading ends, and its scene replaces the one on show: the note is still the first.
    let page = on.page.clone().unwrap();
    let read = Next { scene: on.today.draw(&page), page: Some(page), later: None };
    assert!(first(&read.scene).is_none_or(|(title, _)| title != UNKEPT));
    let after = on.show(&read);
    assert_eq!((first(&after), &after), (Some((UNKEPT, unkept.as_str())), &on.scene));
    assert_eq!(after.notes.len(), read.scene.notes.len() + 1);

    // A wish is kept, and its run is asked for. Before that asking ends, the next words are refused.
    let (scene, asking) = on.wished(SAID, Some(0));
    assert_eq!(titles(&scene), [""; 0], "a kept wish takes the note of the wish before away");
    on.wished(&long, Some(0));
    // No command is installed in this world, so core's check refuses and nothing is started.
    let scene = on.asked(asking.unwrap()()).unwrap();
    assert_eq!(titles(&scene), [UNKEPT, UNSTARTED], "the older asking took the newer wish's note away");
    let Note::Plain { body, .. } = &scene.notes[1] else { panic!() };
    assert!(body.starts_with(&format!("“{SAID}”: the installed jellymoon command is not used")), "{body}");
    // A look keeps both, and the next wish that is kept takes both away.
    assert!((on.look())().is_none());
    assert_eq!(titles(&on.wished(SAID, Some(0)).0), [""; 0]);
}
