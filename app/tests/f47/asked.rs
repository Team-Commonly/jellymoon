//! The choices that change something: `Keep it` and `Toss it`, each carried to core off the window's thread and
//! one at a time; a keep core refuses; work that touches files the person's tools obey, kept only at the second
//! yes; and work whose repo is not found, where no choice is offered and core's own gate changes nothing.

use std::fs;

use sticker_kit::State;

use super::{CHOICES, WORK, branches, drawn, holds, lines, persons, quoted, texts, waits};
use crate::support::day::DAY;
use crate::support::wished::SAID;
use crate::support::worked::finished;

#[test]
fn keep_it_is_carried_off_the_window_s_thread_one_tap_at_a_time_and_the_wish_stays_done_with_its_star() {
    let (w, repo) = finished("edits");
    let before = w.everything(&repo);
    let go = w.at("go");
    let hold = format!("hold:{}", go.display());
    // While core has not begun: a second tap on the choice, a tap on another choice of it and a tap on the
    // little one are nothing.
    let acts = [
        "poke:m1",
        "choose:work-m1:1",
        "choose:work-m1:1",
        "choose:work-m1:2",
        "poke:m1",
        &hold,
        "run",
        "choose:work-m1:1",
        "poke:m1",
    ];
    let mut window = w.taps("logs", &acts);
    window.printed(5);
    // The window has shown that the work is being kept, and its own thread has done nothing to the repo or the
    // home: the keep is the work it left for another thread, which has not been run.
    assert_eq!(w.everything(&repo), before, "the tap itself changed something");
    fs::write(&go, "").unwrap();
    let showed = window.showed();
    let [_, at_once, second, other, poked, kept, late, stands] = &showed[..] else { panic!("eight lines") };

    let at_once = at_once.as_ref().unwrap();
    assert_eq!(texts(at_once), [("busy-m1", "That work is being kept.", vec![], vec![])]);
    assert_eq!(drawn(at_once, "m1"), Some((State::Idle, 1)));
    assert!(second.is_none() && other.is_none() && poked.is_none(), "one tap of a wish is carried at a time");

    let kept = kept.as_ref().unwrap();
    let run = w.kept()["run"].as_str().unwrap().to_string();
    let [(id, title, body, options)] = &texts(kept)[..] else { panic!("one note: {:?}", texts(kept)) };
    assert_eq!((*id, *title, options.len()), ("said-m1", "That work was kept.", 0));
    assert_eq!(body, &[format!("run {run} was kept: the branch was moved forward to the run's commit.")]);
    assert_eq!(drawn(kept, "m1"), Some((State::Idle, 1)), "the wish stays done with its star");
    assert!(late.is_none(), "the choices went with the note");
    assert_eq!(
        texts(stands.as_ref().unwrap()),
        [("wish-m1", "This wish is done.", vec![quoted(SAID).as_str()], vec![])]
    );

    // The work is in the person's checkout, and it waits no more.
    let landed = (holds(&repo, "a.txt"), holds(&repo, "n.txt"), holds(&repo, "b.txt"));
    assert_eq!(landed, (Some("edited by the run\n".into()), Some("new from the run\n".into()), None));
    assert!(waits(&w, &repo).is_none() && branches(&w, &repo).is_empty());
    assert!(w.placed().iter().all(|piece| !piece.id().starts_with("work-")));
}

#[test]
fn toss_it_removes_the_work_and_the_wish_is_drawn_let_go() {
    let (w, repo) = finished("edits");
    let mine = persons(&w, &repo);
    assert_eq!(branches(&w, &repo).len(), 1);
    let showed = w.taps("logs", &["poke:m1", "choose:work-m1:2", "run", "poke:m1"]).showed();
    let [_, at_once, tossed, stands] = &showed[..] else { panic!("four acts") };
    assert_eq!(texts(at_once.as_ref().unwrap()), [("busy-m1", "That work is being tossed.", vec![], vec![])]);

    let tossed = tossed.as_ref().unwrap();
    let run = w.kept()["run"].as_str().unwrap().to_string();
    let why = format!("the work of run {run} was tossed: its branch was deleted, and the wish is let go.");
    assert_eq!(texts(tossed), [("said-m1", "That work was tossed.", vec![why.as_str()], vec![])]);
    assert_eq!(drawn(tossed, "m1"), Some((State::Dimmed, 1)), "the wish is drawn let go");
    let said = quoted(SAID);
    assert_eq!(texts(stands.as_ref().unwrap()), [("wish-m1", "This wish was let go.", vec![said.as_str()], vec![])]);

    assert_eq!(persons(&w, &repo), mine, "a toss touched what is the person's");
    assert!(branches(&w, &repo).is_empty() && waits(&w, &repo).is_none());
}

#[test]
fn a_keep_core_refuses_leaves_one_plain_note_with_core_s_reason_and_the_work_still_waits() {
    let (w, repo) = finished("edits");
    // A file of the person's own is where a file of the work would go.
    fs::write(repo.join("n.txt"), "the person's own\n").unwrap();
    let before = (persons(&w, &repo), branches(&w, &repo));
    let showed = w.taps("logs", &["poke:m1", "choose:work-m1:1", "run", "poke:m1"]).showed();
    let [_, _, refused, again] = &showed[..] else { panic!("four acts") };

    let refused = refused.as_ref().unwrap();
    let run = w.kept()["run"].as_str().unwrap().to_string();
    let [(id, title, body, options)] = &texts(refused)[..] else { panic!("one note: {:?}", texts(refused)) };
    assert_eq!((*id, *title, options.len(), body.len()), ("said-m1", "That work was not kept.", 0, 1));
    let unchanged = format!("the work of run {run} was not kept, and nothing was changed.");
    assert!(body[0].ends_with(&unchanged) && body[0].len() > unchanged.len(), "{body:?}");
    assert_eq!(drawn(refused, "m1"), Some((State::Idle, 1)));

    assert_eq!((persons(&w, &repo), branches(&w, &repo)), before, "a refused keep changed something");
    let work = lines(&w, &repo);
    let work: Vec<&str> = work.iter().map(String::as_str).collect();
    assert_eq!(texts(again.as_ref().unwrap()), [("work-m1", WORK, work, CHOICES.to_vec())], "the work still waits");
}

#[test]
fn work_that_touches_files_the_person_s_tools_obey_names_them_and_is_kept_only_at_the_second_yes() {
    let (w, repo) = finished("rules");
    let before = w.everything(&repo);
    let go = w.at("go");
    let hold = format!("hold:{}", go.display());
    let mut window = w.taps("logs", &["poke:m1", "choose:work-m1:1", "run", &hold, "choose:sure-m1:1", "run"]);
    window.printed(3);
    // The first tap on the keep was carried to core and answered, and nothing is changed.
    assert_eq!(w.everything(&repo), before, "the first tap kept something");
    assert_eq!(holds(&repo, "CLAUDE.md"), None);
    fs::write(&go, "").unwrap();
    let showed = window.showed();
    let [_, _, asks, at_once, kept] = &showed[..] else { panic!("five lines") };

    let told = "Nothing was changed. Kept, they are in your checkout, where your tools read them.";
    let again = ["Show me", "Keep it, with those files", "Toss it"];
    let note = ("sure-m1", "This work touches files your tools obey.", vec!["CLAUDE.md", told], again.to_vec());
    assert_eq!(texts(asks.as_ref().unwrap()), [note]);
    assert_eq!(texts(at_once.as_ref().unwrap()), [("busy-m1", "That work is being kept.", vec![], vec![])]);
    let kept = kept.as_ref().unwrap();
    assert_eq!(
        texts(kept).iter().map(|note| (note.0, note.1)).collect::<Vec<_>>(),
        [("said-m1", "That work was kept.")]
    );
    assert_eq!(holds(&repo, "CLAUDE.md"), Some("rules from the run\n".into()));
    assert!(waits(&w, &repo).is_none());
}

#[test]
fn work_whose_repo_is_not_found_says_so_in_core_s_words_offers_no_choice_and_core_changes_nothing() {
    let (w, repo) = finished("edits");
    fs::create_dir(w.at("no-logs")).unwrap();
    let before = w.everything(&repo);
    // The window is opened on logs that hold no session: the wish's folder was typed in nowhere.
    let acts =
        ["poke:m1", "choose:work-m1:2", "choose:work-m1:1", "choose:work-m1:0", "core:m1:2", "core:m1:1", "core:m1:0"];
    let mut window = w.taps("no-logs", &acts);
    let lines = window.ended();
    let scenes: Vec<Option<sticker_kit::Scene>> =
        lines[..4].iter().map(|line| serde_json::from_str(line).unwrap()).collect();
    let why = format!("no folder of the name the wish gives was typed in on {DAY}; name the folder with --repo.");
    let said = quoted(SAID);
    let unread =
        ("work-m1", "Work waits for this wish, and it was not read.", vec![said.as_str(), why.as_str()], vec![]);
    assert_eq!(texts(scenes[0].as_ref().unwrap()), [unread]);
    assert!(scenes[1..].iter().all(Option::is_none), "a choice that is not drawn is no tap");
    // Asked all the same, core does nothing and says why: a toss here would remove the copy and leave the branch.
    let core: Vec<Option<(String, String)>> =
        lines[4..].iter().map(|line| serde_json::from_str(line).unwrap()).collect();
    let titles = ["That work was not tossed.", "That work was not kept.", "No file of that work was shown."];
    assert_eq!(core, titles.map(|title| Some((title.to_string(), why.clone()))));
    assert_eq!(w.everything(&repo), before, "something was changed where the repo is not found");
    assert_eq!(branches(&w, &repo).len(), 1);
}
