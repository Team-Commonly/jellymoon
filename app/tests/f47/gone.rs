//! A choice left on show for work that waits no more (a page that changed under the person), followed in this
//! process: the wish has no run, so core answers from the home alone and no repo is asked anything.

use jellymoon_app::stopping::Chosen;
use jellymoon_app::taps::OnShow;
use jellymoon_app::today::Today;
use sticker_kit::Note;

use super::{CHOICES, WORK, texts};
use crate::support::day::{self, DAY, shown};
use crate::support::wished::SAID;
use crate::support::worked::tree;

fn sends<T: Send>(_: &T) {}

#[test]
fn a_choice_of_work_that_waits_no_more_changes_nothing_and_its_answer_goes_on_the_scene_now_on_show() {
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
    // it is taken away.
    let meanwhile = Note::Plain { title: "meanwhile".into(), body: String::new() };
    on.scene.notes.push(meanwhile.clone());
    let scene = on.worked(work());
    let why = "no work of that wish waits any more, so nothing was changed.";
    assert_eq!(texts(&scene), [("said-m1", "That work was not tossed.", vec![why], vec![])]);
    assert_eq!((scene.notes.last(), &scene), (Some(&meanwhile), &on.scene));
    assert_eq!(tree(&world.home), before, "something was written");
}
