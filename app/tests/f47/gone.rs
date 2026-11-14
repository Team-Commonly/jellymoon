//! A choice left on show for work that waits no more (a page that changed under the person), followed in this
//! process: the wish has no run, so core answers from the home alone and no repo is asked anything. And a carried
//! call that panics, with a made-up call in place of core's.

use jellymoon_app::stopping::Chosen;
use jellymoon_app::taps::OnShow;
use jellymoon_app::today::{Next, Today};
use sticker_kit::{Note, Scene};

use super::{CHOICES, WORK, texts};
use crate::support::day::{self, DAY, World, shown};
use crate::support::wished::SAID;
use crate::support::worked::tree;

fn sends<T: Send>(_: &T) {}

/// A day with one made wish, `m1`, that has no run, and on show a note of work of it with its choices.
fn left_on_show() -> (World, OnShow) {
    let world = day::world();
    shown(&world, &[]);
    jellymoon_core::wish(&world.home, Some(DAY), SAID, "alpha").unwrap();
    let today = Today {
        date: DAY.to_string(),
        logs: world.dir.path().join("logs"),
        skip: Vec::new(),
        model: "sonnet".to_string(),
        home: world.home.clone(),
        harness: None,
        fresh: false,
        settings: None,
        bin: None,
    };
    let (scene, page) = today.first();
    let mut on = OnShow { today, page, scene };
    let options = CHOICES.map(str::to_string).to_vec();
    on.scene.notes.insert(0, Note::Text { id: "work-m1".into(), title: WORK.into(), body: Vec::new(), options });
    (world, on)
}

fn ids(scene: &Scene) -> Vec<&str> {
    texts(scene).iter().map(|note| note.0).collect()
}

#[test]
fn a_choice_of_work_that_waits_no_more_changes_nothing_and_its_answer_goes_on_the_scene_now_on_show() {
    let (world, mut on) = left_on_show();
    let before = tree(&world.home);

    assert!(on.working("work-m1", 3).is_none() && on.working("work-m2", 0).is_none(), "no such choice is drawn");
    let (at_once, work) = on.working("work-m1", 2).expect("the choice is on show");
    sends(&work);
    assert_eq!(texts(&at_once), [("busy-m1", "That work is being tossed.", vec![], vec![])]);
    // While core is at work a tap on a choice of that wish is nothing: it is no tap for the page either.
    for (note, index) in [("work-m1", 2), ("work-m1", 0), ("busy-m1", 0), ("said-m1", 0), ("wish-m1", 0)] {
        assert!(matches!(on.chose(note, index), Chosen::Nothing), "a tap on {note:?} did something");
    }
    assert_eq!(on.scene, at_once);

    // The scene on show changes while core is at work: the answer's note is put on that scene, and nothing of
    // it is taken away, a file's note of another wish's work neither.
    let meanwhile = Note::Plain { title: "meanwhile".into(), body: String::new() };
    let other = Note::Text { id: "file-3-m2".into(), title: "theirs".into(), body: Vec::new(), options: Vec::new() };
    on.scene.notes.insert(1, other);
    on.scene.notes.push(meanwhile.clone());
    let scene = on.worked(work());
    let why = "no work of that wish waits any more, so nothing was changed.";
    let notes = [("said-m1", "That work was not tossed.", vec![why], vec![]), ("file-3-m2", "theirs", vec![], vec![])];
    assert_eq!(texts(&scene), notes);
    assert_eq!((scene.notes.last(), &scene), (Some(&meanwhile), &on.scene));
    assert_eq!(tree(&world.home), before, "something was written");

    // A look, and a reading that gave a new scene, leave the notes of an opened wish on show.
    let looked = on.look()().and_then(|seen| on.seen(seen)).expect("the look takes the plain note at the end");
    assert_eq!(ids(&looked), ["said-m1", "file-3-m2"]);
    let (scene, page) = on.today.first();
    assert_eq!(ids(&on.show(&Next { scene, page, later: None })), ["said-m1", "file-3-m2"]);
}

#[test]
fn a_carried_call_that_panics_is_answered_in_core_s_words_and_the_wish_can_be_opened_again() {
    let (world, mut on) = left_on_show();
    let before = tree(&world.home);
    let (at_once, work) =
        on.carrying("work-m1", 1, |_, _, _| panic!("a made-up failure of the carried call")).expect("the choice");
    assert_eq!(texts(&at_once), [("busy-m1", "That work is being kept.", vec![], vec![])]);
    assert!(on.poked("m1")().is_none(), "the wish is opened while its tap is carried");

    // The panic ends where the call was carried: the work gives its answer, and nothing says "being kept" now.
    let scene = on.worked(work());
    let unknown = "Jellymoon failed before it had an answer, so what was changed is not known here. Open the wish \
                   again to see how its work stands.";
    assert_eq!(texts(&scene), [("said-m1", "That did not finish.", vec![unknown], vec![])]);
    let opened = on.poked("m1")().and_then(|seen| on.seen(seen)).expect("the wish is opened again");
    assert_eq!(ids(&opened), ["wish-m1"]);
    assert_eq!(tree(&world.home), before, "something was written");
}
