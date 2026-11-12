//! What is offered for a wish, where core is asked, and what a wish's note says and outlives: the findings of
//! F42's review, and of the gate's on F43.

use std::fs;

use jellymoon_app::today::Next;
use jellymoon_app::wishing::{UNKEPT, UNSTARTED};
use serde_json::json;
use sticker_kit::{Mark, Note, Scene, State};

use super::{SAID, chips, drawn, first, kept_wishes, on_show, read};
use crate::support::day::{DAY, World, answer, names, noon, said, shown, work, write_log};

/// Why core keeps no wish for a project it does not offer.
const GONE: &str =
    "no folder of that project that is switched on was typed in on 2026-03-10, so this wish was not kept.";

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

/// Something typed that day in a folder of each name.
fn typed_in(world: &World, folders: &[&str]) {
    for name in folders {
        write_log(world, &format!("-work-{name}/s.jsonl"), &[said(&noon(30), &work(name), "hello")]);
    }
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
    assert_eq!((on.offering())().unwrap().options, ["alpha"]);
    let (scene, asking) = on.wished(SAID, Some("private-notes"));
    assert!(asking.is_none() && kept_wishes(&world).is_none(), "a wish was made for a folder switched off");
    assert_eq!(first(&scene), Some((UNKEPT, format!("“{SAID}”: {GONE}").as_str())));

    on.today.skip.clear();
    fs::write(world.home.join("skip.txt"), "alpha\n").unwrap();
    assert_eq!((on.offering())().unwrap().options, ["private-notes"], "skip.txt switches a folder off too");
    fs::write(world.home.join("skip.txt"), b"alpha\n\xff").unwrap();
    assert!((on.offering())().is_none(), "a list that cannot be read offers nothing");
    fs::remove_file(world.home.join("skip.txt")).unwrap();

    // At the binary: the day cannot be read again, so the page is the one kept from the earlier reading.
    answer(&world, "not a reply", 1);
    let placed = shown(&world, &["--skip", "private-notes"]);
    assert_eq!(chips(&placed), ["alpha"]);
}

// The rule is the reader's, on the folder's path: a name above the folder switches it off, as it does for a run.
#[test]
fn a_project_switched_off_by_a_name_above_its_folder_has_no_chip_and_no_wish_is_kept_for_it() {
    let world = read();
    let mut on = on_show(&world);
    on.today.skip = vec![format!("work{}alph", std::path::MAIN_SEPARATOR)];
    assert_eq!((on.offering())().unwrap().options, ["private-notes"]);
    let (scene, asking) = on.wished(SAID, Some("alpha"));
    assert!(asking.is_none() && kept_wishes(&world).is_none(), "a wish was made for a folder switched off");
    assert_eq!(first(&scene), Some((UNKEPT, format!("“{SAID}”: {GONE}").as_str())));
    on.today.skip = projects(&["work"]);
    assert!((on.offering())().is_none(), "every folder is under a name switched off");
}

#[test]
fn only_a_project_with_a_folder_typed_in_that_day_is_offered() {
    let world = read();
    let mut on = on_show(&world);
    let long = "p".repeat(150);
    // Names of a kept page: no folder of the day has any of these names but the last.
    on.page.as_mut().unwrap().data.read.projects = projects(&["..", ".", "new\nline", &long, "a/b", "beta", "alpha"]);
    assert_eq!((on.offering())().unwrap().options, ["alpha"]);
    assert_eq!(drawn(&on.wished(SAID, Some("alpha")).0, "m1").map(|drawn| drawn.0), Some(State::Busy));
    assert_eq!(kept_wishes(&world).unwrap()[0]["project"], "alpha");
    on.page.as_mut().unwrap().data.read.projects = projects(&["..", "new\nline", "beta"]);
    assert!((on.offering())().is_none(), "no name is left to offer");
}

// The chips change while the words are typed: the wish is for the name that was on the chosen chip, and core is
// asked for that name again.
#[test]
fn a_wish_is_for_the_name_on_its_chip_and_core_refuses_a_name_it_no_longer_offers() {
    let world = read();
    let mut on = on_show(&world);
    typed_in(&world, &["p1", "p2", "p3", "p4", "p5", "p6", "p7", "p8"]);
    on.page.as_mut().unwrap().data.read.projects = projects(&["p1", "p2", "p3", "p4", "p5", "p6", "p7", "p8"]);
    assert_eq!((on.offering())().unwrap().options, ["p1", "p2", "p3", "p4", "p5", "p6"], "the kit offers six");

    // `p2` was the second chip. The first is switched off before Enter: the wish is still for `p2`.
    fs::write(world.home.join("skip.txt"), "p1\n").unwrap();
    assert_eq!((on.offering())().unwrap().options[0], "p2");
    on.wished(SAID, Some("p2"));
    assert_eq!(kept_wishes(&world).unwrap()[0]["project"], "p2");

    // The chosen chip's own project is switched off before Enter: nothing more is kept.
    fs::write(world.home.join("skip.txt"), "p1\np3\n").unwrap();
    for gone in ["p3", "p1", "p9", ""] {
        let (scene, asking) = on.wished(SAID, Some(gone));
        assert!(asking.is_none(), "{gone:?} was taken for a project");
        let (title, body) = first(&scene).unwrap();
        assert!(title == UNKEPT && !body.is_empty());
    }
    assert_eq!(first(&on.wished(SAID, Some("p3")).0), Some((UNKEPT, format!("“{SAID}”: {GONE}").as_str())));
    let (scene, asking) = on.wished(SAID, None);
    assert!(asking.is_none());
    assert!(first(&scene).unwrap().1.ends_with("No project was chosen, so nothing was sent."));
    assert_eq!(kept_wishes(&world).unwrap().as_array().map(Vec::len), Some(1));
}

fn sends<T: Send>(_: &T) {}

// The window's thread makes the work and takes its answer; core is asked in between, on another thread.
#[test]
fn core_is_asked_in_work_that_can_leave_the_window_s_thread_and_its_answer_opens_no_file() {
    let world = read();
    let mut on = on_show(&world);
    let wishing = on.wishing(SAID, Some("alpha"));
    sends(&wishing);
    sends(&on.look());
    sends(&on.offering());
    assert_eq!(kept_wishes(&world), None, "core was asked before the work was run");
    let answer = wishing();
    let kept = json!([{"id": "m1", "said": SAID, "project": "alpha", "state": "working", "stars": []}]);
    assert_eq!(kept_wishes(&world), Some(kept));
    // What is kept is gone: the answer is drawn all the same, so taking it read nothing.
    fs::remove_file(world.home.join("pages").join(format!("{DAY}.wishes.json"))).unwrap();
    let (scene, asking) = on.answered(answer);
    assert_eq!(drawn(&scene, "m1"), Some((State::Busy, 0, Mark::Busy, SAID.to_string())));
    assert!(asking.is_some_and(|asking| {
        sends(&asking);
        true
    }));
    assert_eq!(&scene, &on.scene);

    // A day that is locked: making the work waits for nothing, and the refusal is core's, when the work is run.
    let lock = world.home.join("pages").join(format!("{DAY}.ruled.lock"));
    fs::write(&lock, "").unwrap();
    let before = names(&world.home.join("pages"));
    let wishing = on.wishing(SAID, Some("alpha"));
    assert_eq!(names(&world.home.join("pages")), before);
    let (scene, asking) = on.answered(wishing());
    let (title, body) = first(&scene).unwrap();
    assert!(title == UNKEPT && body.contains("another change to the page of 2026-03-10"), "{body}");
    assert!(asking.is_none(), "words that were not kept ask for no run, and stay in the entry");
    assert_eq!(names(&world.home.join("pages")), before);

    // An answer that comes when another scene is on show leaves its note on that scene, and reads nothing.
    let stale = on.wishing(&"w".repeat(201), Some("alpha"))();
    fs::remove_file(&lock).unwrap();
    on.wished(SAID, Some("alpha"));
    let (scene, asking) = on.answered(stale);
    assert!(asking.is_none());
    assert_eq!(titles(&scene), vec![UNKEPT]);
    assert!(drawn(&scene, "m1").is_some(), "the scene on show was replaced by an older one");
}

/// A link with a password in it, put together here so that no file holds one.
fn link() -> (String, String) {
    let password = ["q7", "z9"].concat();
    (format!("{}://u{}{password}@h.example", "https", ":"), password)
}

// A note names its wish by its start as core cleans it, never by the words as they were typed.
#[test]
fn a_note_names_a_refused_wish_by_core_s_cleaning_and_a_kept_one_by_what_core_kept() {
    let world = read();
    let mut on = on_show(&world);
    let (link, password) = link();
    let hidden = |scene: &Scene| !scene.notes.iter().any(|note| format!("{note:?}").contains(&password));

    // Refused: 200 characters, which are over 200 once the password is replaced. The link is at the start.
    let typed = format!("{link} {}", "w".repeat(199 - link.chars().count()));
    assert_eq!(typed.chars().count(), 200);
    let (scene, asking) = on.wished(&typed, Some("alpha"));
    assert!(asking.is_none() && kept_wishes(&world).is_none(), "the words were kept");
    let (title, body) = first(&scene).unwrap();
    assert_eq!(title, UNKEPT);
    assert!(body.starts_with("“https://u:"), "{body}");
    assert!(body.ends_with("a wish is at most 200 characters, and this one is longer."), "{body}");
    assert!(hidden(&scene), "the note shows what core hides: {body}");

    // Kept: the note of its run names it by the words core kept. No command is installed, so no run is started.
    let (_, asking) = on.wished(&format!("{link} is where the uploader sends"), Some("alpha"));
    let kept = kept_wishes(&world).unwrap()[0]["said"].as_str().unwrap().to_string();
    assert!(!kept.contains(&password), "{kept}");
    let scene = on.asked(asking.unwrap()()).unwrap();
    let (title, body) = first(&scene).unwrap();
    assert_eq!(title, UNSTARTED);
    let named: String = kept.chars().take(39).collect();
    assert!(body.starts_with(&format!("“{named}")) && hidden(&scene), "{body}");
}

#[test]
fn a_wish_s_note_names_its_words_outlives_a_reading_and_an_older_asking_leaves_a_newer_note() {
    let world = read();
    let mut on = on_show(&world);
    let long = "w".repeat(201);
    let (scene, _) = on.wished(&long, Some("alpha"));
    let unkept = format!("“{}…”: a wish is at most 200 characters, and this one is longer.", "w".repeat(39));
    assert_eq!(first(&scene), Some((UNKEPT, unkept.as_str())), "the note names the start of its wish");

    // A reading ends, and its scene replaces the one on show: the note is still the first.
    let page = on.page.clone().unwrap();
    let read = Next { scene: on.today.draw(&page), page: Some(page), later: None };
    assert!(first(&read.scene).is_none_or(|(title, _)| title != UNKEPT));
    let after = on.show(&read);
    assert_eq!((first(&after), &after), (Some((UNKEPT, unkept.as_str())), &on.scene));
    assert_eq!(after.notes.len(), read.scene.notes.len() + 1);

    // A wish is kept, and its run is asked for. Before that asking ends, the next words are refused.
    let (scene, asking) = on.wished(SAID, Some("alpha"));
    assert_eq!(titles(&scene), [""; 0], "a kept wish takes the note of the wish before away");
    on.wished(&long, Some("alpha"));
    // No command is installed in this world, so core's check refuses and nothing is started.
    let scene = on.asked(asking.unwrap()()).unwrap();
    assert_eq!(titles(&scene), [UNKEPT, UNSTARTED], "the older asking took the newer wish's note away");
    let Note::Plain { body, .. } = &scene.notes[1] else { panic!() };
    assert!(body.starts_with(&format!("“{SAID}”: the installed jellymoon command is not used")), "{body}");
    // A look keeps both, and the next wish that is kept takes both away.
    assert!((on.look())().is_none());
    assert_eq!(titles(&on.wished(SAID, Some("alpha")).0), [""; 0]);
}

// Every chip goes while the words are typed and the window's own reading runs: the reading's end has no entry to
// give, and Enter is refused. The page still has the words for that refusal to leave: the kit keeps an entry
// that is taken away while it holds words (`crates/sticker-kit/tests/entry_draw.rs`).
#[test]
fn every_folder_switched_off_under_typed_words_ends_the_reading_with_no_entry_and_refuses_enter() {
    let world = read();
    let mut on = on_show(&world);
    assert_eq!((on.offering())().unwrap().options, ["alpha", "private-notes"]);
    fs::write(world.home.join("skip.txt"), "work\n").unwrap();
    let page = on.page.clone().unwrap();
    let end = Next { scene: on.today.draw(&page), page: Some(page), later: None };
    let (_, entry) = on.today.clone().offered(Box::new(move || Some(end)))().expect("the reading's last step");
    assert!(entry.is_none(), "the reading's end gave an entry with no folder switched on");
    let (scene, asking) = on.wished(SAID, Some("alpha"));
    assert!(asking.is_none() && kept_wishes(&world).is_none(), "a wish was made for a folder switched off");
    assert_eq!(first(&scene), Some((UNKEPT, format!("“{SAID}”: {GONE}").as_str())));
}
