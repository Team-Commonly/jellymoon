//! Finished wishes tuck away (ADR 0030, decision 11; `docs/development/app.md`, "Every wish keeps its place"): a
//! made wish that is settled has no note on the page once the next wish is made, and a tap on it brings its notes
//! back. The wishes are typed through the made-up window of F47 (`support/tapper.rs`), a process of its own, and
//! run by the `jellymoon` command with the made-up agent of F39's tests, in a world where a finished change is
//! kept by itself, with the real git in a repo of the test's own. The window's `column` act prints the scene as
//! `Showing::column` gives it to the view. Nothing here starts the real Claude Code or opens a window.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use std::path::{Path, PathBuf};
use std::{fs, thread};

use sticker_kit::{Note, Scene};
use support::day::DAY;
use support::wished::{World, wait_until, world};
use support::worked::landed;

/// A window tall enough that every note of the day is placed.
const TALL: &str = "size:1440x3000";
const UNDO: [&str; 1] = ["Undo it"];

fn column(line: &str) -> Scene {
    serde_json::from_str::<Option<Scene>>(line).expect(line).expect("a column")
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

/// What stands under the bubble of `wish` in a column: every note from its typed words to the next wish's. The
/// bubble itself has to be there.
fn under(column: &Scene, wish: &str) -> Vec<String> {
    let all: Vec<&str> = column.notes.iter().map(id).collect();
    let at = all.iter().position(|one| *one == format!("typed-{wish}")).expect("the wish's bubble");
    all[at + 1..].iter().take_while(|one| !one.starts_with("typed-")).map(|one| one.to_string()).collect()
}

fn choices(column: &Scene, named: &str) -> Vec<String> {
    column.notes.iter().find(|note| id(note) == named).map_or(Vec::new(), |note| match note {
        Note::Text { options, .. } | Note::Drawn { options, .. } => options.clone(),
        _ => Vec::new(),
    })
}

/// The run of the made wish `id`, with the made-up agent playing `script`: how it ended.
fn ran(w: &World, repo: &Path, id: &str, script: &str) -> String {
    w.play(script);
    let (logs, named) = (w.at("logs"), repo.to_str().unwrap());
    let ran = w.jellymoon(&["run", "--wish", id, "--date", DAY, "--repo", named, "--logs", logs.to_str().unwrap()]);
    ran["outcome"].as_str().unwrap().to_string()
}

#[test]
fn a_settled_wish_tucks_away_when_the_next_wish_is_made_and_a_tap_or_a_new_window_shows_what_is_owed() {
    let (w, repo) = landed("edits");
    let landing = w.at("tmp").join("landed");
    let hold = format!("hold:{}", landing.display());
    let [second, third] = ["type:wish@alpha:say hello", "type:wish@alpha:and nothing more"];
    let acts =
        [TALL, "look", "column", second, "column", "poke:m1", "column", third, "column", &hold, "look", "column"];
    let mut window = w.taps("logs", &acts);
    window.printed(8);
    // The second wish's change lands after the third wish was made.
    assert_eq!(ran(&w, &repo, "m2", "hello"), "completed");
    fs::write(&landing, "").unwrap();
    let lines = window.ended();
    let [_, alone, kept, made, _, back, _, again, _, landed] = &lines[..] else { panic!("ten lines: {lines:#?}") };
    // A wish lands, and its result is on the page.
    let alone = column(alone);
    assert!(under(&alone, "m1").contains(&"kept-m1".to_string()), "{alone:?}");
    // A second wish is made, by Enter in the entry: the first one's notes are gone, and its bubble is there.
    assert!(kept.contains(r#""kept":true"#), "{kept}");
    let made = column(made);
    assert_eq!(under(&made, "m1"), [""; 0], "{made:?}");
    // A tap on the first wish brings its notes back, as core says the wish stands: its change can be undone.
    let back = column(back);
    assert!(under(&back, "m1").contains(&"kept-m1".to_string()), "{back:?}");
    assert_eq!(choices(&back, "kept-m1"), UNDO);
    // The next made wish tucks it away again.
    let again = column(again);
    assert_eq!(under(&again, "m1"), [""; 0], "{again:?}");
    // The second wish settled after the third was made: it is tucked away as the first is, under the third.
    let landed = column(landed);
    assert_eq!((under(&landed, "m2"), under(&landed, "m1")), (Vec::<String>::new(), Vec::new()), "{landed:?}");

    // A window opened on that day, once the third wish's run failed and a fourth changed nothing: the notes of
    // the newest wish, and of the wish that needs the person, and of no other.
    assert_ne!(ran(&w, &repo, "m3", "is-error"), "completed");
    let fourth = w.jellymoon(&["wish", "--date", DAY, "--say", "look again", "--project", "alpha"]);
    assert_eq!(fourth["wish"]["id"], "m4", "{fourth}");
    assert_eq!(ran(&w, &repo, "m4", "nothing"), "completed");
    let lines = w.taps("logs", &[TALL, "look", "column"]).ended();
    let opened = column(lines.last().expect("two lines"));
    assert_eq!((under(&opened, "m1"), under(&opened, "m2")), (Vec::<String>::new(), Vec::new()), "{opened:?}");
    assert!(under(&opened, "m3").contains(&"wish-m3".to_string()), "{opened:?}");
    assert!(under(&opened, "m4").contains(&"wish-m4".to_string()), "{opened:?}");
}

/// A day with no made wish yet, where a finished change is kept by itself when `lands`, and its repo.
fn empty(lands: bool) -> (World, PathBuf) {
    let w = world();
    fs::create_dir(w.at("harness")).unwrap();
    let repo = w.at("work").join("alpha");
    (if lands { w.lands() } else { w }, repo)
}

/// A column in a line: each made wish with the notes under its bubble, and each note's choices.
fn page(line: &str) -> String {
    let column = column(line);
    let bubbles = column.notes.iter().filter_map(|note| id(note).strip_prefix("typed-"));
    let noted = |wish: &str| {
        let with = |note: String| match &choices(&column, &note)[..] {
            [] => note,
            options => format!("{note}[{}]", options.join("/")),
        };
        format!("{wish}:{}", under(&column, wish).into_iter().map(with).collect::<Vec<_>>().join(","))
    };
    bubbles.map(noted).collect::<Vec<_>>().join(" ")
}

const TWO: [&str; 2] = ["type:wish@alpha:make a file", "type:wish@alpha:say hello"];

/// The window of a day where two wishes are entered back to back: it waits until the test leaves `held`, and
/// then makes two looks with the page after each, and `more`.
fn two_then(w: &World, held: &Path, more: &[&str]) -> support::wished::Window {
    let hold = format!("hold:{}", held.display());
    let acts = [TALL, "look", TWO[0], TWO[1], hold.as_str(), "look", "column", "look", "column"];
    w.taps("logs", &[&acts[..], more].concat())
}

/// Two wishes entered back to back, and then their runs in `order`, each ending after both wishes were made:
/// the page after the first run (and after one more look), after the second, and after a tap on the first wish.
fn back_to_back(order: [(&str, &str); 2]) -> [String; 4] {
    let (w, repo) = empty(true);
    let held = ["one", "two"].map(|name| w.at("tmp").join(name));
    let second = format!("hold:{}", held[1].display());
    let mut window = two_then(&w, &held[0], &[&second, "look", "column", "poke:m1", "column"]);
    for (at, ((wish, script), held)) in order.iter().zip(&held).enumerate() {
        window.printed(3 + 4 * at);
        assert_eq!(ran(&w, &repo, wish, script), "completed");
        fs::write(held, "").unwrap();
    }
    let lines = window.ended();
    let [.., first, _, again, _, second, _, tapped] = &lines[..] else { panic!("eleven lines: {lines:#?}") };
    [first, again, second, tapped].map(|line| page(line))
}

// An earlier wish tucks when it settles: the page is as if each wish had been made after the one before it ended.
#[test]
fn an_earlier_wish_that_ends_after_a_later_wish_was_made_tucks_away_whichever_of_the_two_ends_first() {
    let in_order = back_to_back([("m1", "edits"), ("m2", "hello")]);
    let after = "m1: m2:kept-m2[Undo it]";
    // A tap on the tucked wish brings its notes back; its change has the newer one over it, so core offers no undo.
    assert_eq!(in_order, ["m1: m2:steps-m2", "m1: m2:steps-m2", after, "m1:kept-m1 m2:kept-m2[Undo it]"]);
    let newest_first = back_to_back([("m2", "hello"), ("m1", "edits")]);
    let alone = "m1:steps-m1 m2:kept-m2[Undo it]";
    assert_eq!(newest_first, [alone, alone, "m1: m2:kept-m2", "m1:kept-m1[Undo it] m2:kept-m2"]);
}

#[test]
fn of_three_wishes_entered_back_to_back_only_the_newest_has_its_result_on_the_page() {
    let (w, repo) = empty(true);
    let held = w.at("tmp").join("ended");
    let hold = format!("hold:{}", held.display());
    let acts = [TALL, "look", TWO[0], TWO[1], "type:wish@alpha:and nothing more", &hold, "look", "column"];
    let mut window = w.taps("logs", &acts);
    window.printed(4);
    for (wish, script) in [("m1", "edits"), ("m2", "hello"), ("m3", "nothing")] {
        assert_eq!(ran(&w, &repo, wish, script), "completed");
    }
    fs::write(held, "").unwrap();
    assert_eq!(page(window.ended().last().unwrap()), "m1: m2: m3:wish-m3");
}

// A wish that needs the person when its run ends is not tucked away, though a later wish stands: work that waits
// for a tap to keep (and it stays once kept), a question, a run that failed.
#[test]
fn an_earlier_wish_that_needs_the_person_when_its_run_ends_stays_on_the_page_under_a_later_wish() {
    let (keep, none): (&[&str], &[&str]) = (&["choose:work-m1:1", "run", "column"], &[]);
    for (lands, script, note, more) in
        [(false, "edits", "work-m1", keep), (true, "asks", "wish-m1", none), (true, "is-error", "wish-m1", none)]
    {
        let (w, repo) = empty(lands);
        let held = w.at("tmp").join("ended");
        let mut window = two_then(&w, &held, more);
        window.printed(3);
        ran(&w, &repo, "m1", script);
        fs::write(held, "").unwrap();
        let lines = window.ended();
        for line in [&lines[4], &lines[6]] {
            assert!(under(&column(line), "m1").contains(&note.to_string()), "{script}: {}", page(line));
        }
        let kept = lines.get(9).map(|line| under(&column(line), "m1"));
        assert!(kept.is_none_or(|kept| kept.contains(&"said-m1".to_string())), "{script}: {lines:#?}");
    }
}

// The earlier wish's change does not fit when its run ends, as the later wish's landed in the same place first.
#[test]
fn an_earlier_wish_whose_change_does_not_fit_when_its_run_ends_stays_on_the_page_with_make_it_again() {
    let (w, repo) = empty(true);
    let held = w.at("tmp").join("ended");
    let mut window = two_then(&w, &held, &[]);
    window.printed(3);
    let before = w.starts();
    w.play("slow");
    let (logs, named) = (w.at("logs"), repo.to_str().unwrap());
    let args = ["run", "--wish", "m1", "--date", DAY, "--repo", named, "--logs", logs.to_str().unwrap()];
    thread::scope(|scope| {
        let first = scope.spawn(|| w.jellymoon(&args)["outcome"].as_str().unwrap().to_string());
        wait_until("the first wish's agent", || w.starts() > before);
        assert_eq!(ran(&w, &repo, "m2", "follow"), "completed");
        fs::write(w.at("rec").join("go"), "").unwrap();
        assert_eq!(first.join().unwrap(), "completed");
    });
    fs::write(held, "").unwrap();
    let lines = window.ended();
    let unfit = "m1:work-m1[Show me/Make it again/Toss it] m2:kept-m2[Undo it]";
    assert_eq!([&lines[4], &lines[6]].map(|line| page(line)), [unfit, unfit]);
}
