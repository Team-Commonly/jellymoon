//! Page data into pieces, with no core and no window: the pages are typed here.

use jellymoon_core::PageRead;

use super::*;
use crate::read::Unknown;

fn wish(id: &str, state: WishState, stars: &[&str]) -> Wish {
    Wish {
        id: id.to_string(),
        label: format!("Label of {id}"),
        said: format!("please do {id}"),
        project: "alpha".to_string(),
        state,
        stars: stars.iter().map(|star| star.to_string()).collect(),
        question: None,
        answered: None,
    }
}

fn page(wishes: Vec<Wish>, clashes: Vec<Clash>) -> Page {
    let read = PageRead { turns: 5, projects: vec!["alpha".to_string(), "beta".to_string()], no_logs: false };
    let data = PageData { day: "2026-03-10".to_string(), wishes, clashes, read };
    Page { data, unknown: Vec::new(), left_out: LeftOut::default() }
}

fn clash(settled: bool) -> Clash {
    Clash {
        a: "w1".to_string(),
        b: "w2".to_string(),
        quote_a: "add a retry".to_string(),
        quote_b: "never retry".to_string(),
        settled,
    }
}

#[test]
fn one_character_per_wish_in_page_order_with_its_state_and_stars() {
    let scene = scene(&page(
        vec![
            wish("w1", WishState::Working, &["a"]),
            wish("w2", WishState::Done, &["b", "c"]),
            wish("w3", WishState::NeedsYou, &[]),
            wish("w4", WishState::LetGo, &["d", "e", "f", "g", "h"]),
        ],
        vec![],
    ));
    let seen: Vec<(&str, &str, State, u8)> =
        scene.characters.iter().map(|c| (c.id.as_str(), c.label.as_str(), c.state, c.stars)).collect();
    assert_eq!(
        seen,
        [
            ("w1", "Label of w1", State::Busy, 1),
            ("w2", "Label of w2", State::Idle, 2),
            ("w3", "Label of w3", State::NeedsYou, 0),
            ("w4", "Label of w4", State::Dimmed, 5),
        ]
    );
    assert_eq!(scene.characters.iter().map(|c| c.hue).collect::<Vec<_>>(), [1, 2, 3, 4]);
    assert_eq!(little_one(4, &wish("w5", WishState::Done, &[])).hue, 1, "the hues take turns; the hero keeps 0");
    assert_eq!(scene.hero.map(|hero| (hero.id, hero.hue)), Some(("me".to_string(), 0)));
    assert_eq!(scene.title, "what I wished for today");
    assert_eq!(scene.date_label, "Tue · Mar 10");
}

#[test]
fn the_first_note_lists_every_star_with_its_wish() {
    let scene =
        scene(&page(vec![wish("w1", WishState::Done, &["a", "b"]), wish("w2", WishState::Done, &["c"])], vec![]));
    let rows = [("a", "Label of w1"), ("b", "Label of w1"), ("c", "Label of w2")];
    let rows = rows.map(|(star, label)| (star.to_string(), label.to_string())).to_vec();
    assert_eq!(scene.notes, [Note::List { title: "came true today".to_string(), rows }]);
    let none = super::scene(&page(vec![wish("w1", WishState::Working, &[])], vec![]));
    assert_eq!(none.notes, [], "no star, no list");
}

#[test]
fn a_made_wish_that_was_let_go_has_no_star_and_a_reading_s_keeps_the_stars_of_its_jobs() {
    let wishes = vec![wish("w1", WishState::LetGo, &["a"]), wish("m1", WishState::LetGo, &["b"])];
    let scene = ruled(&page(wishes, vec![]), &[], 1);
    let rows = vec![("a".to_string(), "Label of w1".to_string())];
    assert_eq!(scene.notes[0], Note::List { title: "came true today".to_string(), rows });
    assert_eq!(scene.characters.iter().map(|one| one.stars).collect::<Vec<_>>(), [1, 0]);
}

#[test]
fn a_clash_is_a_choice_of_both_quotes_and_only_a_settled_one_is_struck() {
    let scene = scene(&page(vec![wish("w1", WishState::LetGo, &[])], vec![clash(true), clash(false)]));
    let choices: Vec<(&str, &[String], bool)> = scene
        .notes
        .iter()
        .filter_map(|note| match note {
            Note::Choice { id, options, struck_first, .. } => Some((id.as_str(), options.as_slice(), *struck_first)),
            _ => None,
        })
        .collect();
    let quotes = ["“add a retry”".to_string(), "“never retry”".to_string()];
    assert_eq!(choices, [("clash-1", &quotes[..], true), ("clash-2", &quotes[..], false)]);
}

#[test]
fn what_the_app_has_no_piece_for_becomes_a_plain_note_after_the_rest() {
    let mut page = page(vec![wish("w1", WishState::Done, &["a"])], vec![clash(false)]);
    page.unknown = vec![Unknown { title: "reminders".to_string(), body: "Water the plants".to_string() }];
    let scene = scene(&page);
    assert_eq!(scene.notes.len(), 3);
    let plain = Note::Plain { title: "reminders".to_string(), body: "Water the plants".to_string() };
    assert_eq!(scene.notes.last(), Some(&plain));
}

#[test]
fn what_was_left_out_is_the_first_note_and_names_only_what_went() {
    let mut page = page(vec![wish("w1", WishState::Done, &["a"])], vec![clash(false)]);
    page.left_out = LeftOut { wishes: 3, stars: 1, ..LeftOut::default() };
    let first = Note::Plain {
        title: "not everything is drawn".to_string(),
        body: "Left out of this page: 3 wishes, 1 star.".to_string(),
    };
    let notes = scene(&page).notes;
    assert_eq!((notes.len(), notes.first()), (3, Some(&first)));
}

#[test]
fn a_page_with_nothing_on_it_says_so() {
    let mut empty = page(vec![], vec![]);
    empty.data.read = PageRead { turns: 0, projects: vec![], no_logs: false };
    let scene = scene(&empty);
    // The strip that shows how a wish goes, alone: three sketches in the order a wish goes, each with its
    // caption, one line under them and nothing to tap.
    let [Note::Drawn { id, picture: Picture::Strip { panels }, words, open: true, options, field: None }] =
        &scene.notes[..]
    else {
        panic!("the strip alone: {:?}", scene.notes)
    };
    let drawn: Vec<(Shows, &str)> = panels.iter().map(|panel| (panel.shows, panel.caption.as_str())).collect();
    let want = [(Shows::Speech, "say a wish"), (Shows::Sheet, "it gets made"), (Shows::Folder, "keep or toss")];
    assert_eq!((id.as_str(), drawn, &words[..]), (HOW, want.to_vec(), &[NOTHING_HERE.to_string()][..]));
    assert!(options.is_empty(), "{options:?}");
    let old = "Nothing wished for yet today.";
    assert!(!format!("{scene:?}").contains(old), "the old title is said nowhere");
    assert_eq!(scene.characters, []);
    assert_eq!(scene.footer, "", "no reading to speak of");
}

#[test]
fn the_footer_counts_in_words_that_agree() {
    let read = |turns, projects: &[&str], no_logs| {
        let mut page = page(vec![], vec![]);
        page.data.read = PageRead { turns, projects: projects.iter().map(|p| p.to_string()).collect(), no_logs };
        page.data
    };
    // No logs to read is said, and nothing of a reading or of things typed.
    let found_none = "No Claude Code session logs were found, so nothing was read.";
    assert_eq!(footer(&read(0, &[], true), 0), found_none);
    assert_eq!(
        footer(&read(75, &["alpha", "beta"], false), 0),
        "Read from 75 things you typed in 2 projects (alpha, beta). This is the model's reading, unedited."
    );
    let one = "Read from 1 thing you typed in 1 project (alpha). This is the model's reading, unedited.";
    assert_eq!(footer(&read(1, &["alpha"], false), 0), one);
    // A page that says no logs were found and holds a turn, or a wish of its own, is a reading's page.
    assert_eq!(footer(&read(1, &["alpha"], true), 0), one);
    let typed = "Read from 1 thing you typed. This is the model's reading, unedited.";
    assert_eq!(footer(&read(1, &[], true), 0), typed);
    let mut own = read(0, &[], true);
    own.wishes = vec![wish("w1", WishState::Working, &[])];
    assert_eq!(footer(&own, 0), "Read from 0 things you typed. This is the model's reading, unedited.");
    assert_eq!(footer(&own, 1), found_none);
}

#[test]
fn a_day_of_made_wishes_with_nothing_read_has_no_footer() {
    let mut own = page(vec![wish("m1", WishState::Working, &[])], vec![]);
    own.data.read = PageRead { turns: 0, projects: Vec::new(), no_logs: false };
    let shown = ruled(&own, &[], 1);
    assert_eq!(shown.footer, "");
    own.data.read.no_logs = true;
    let found_none = "No Claude Code session logs were found, so nothing was read. 1 made wish is in your own words.";
    assert_eq!(ruled(&own, &[], 1).footer, found_none);
}

/// The page's tracks: for each, its id, and the two sentences that say a made wish is being worked on.
fn going(shown: &Scene) -> Vec<(&str, &str, &str)> {
    fn track(note: &Note) -> Option<(&str, &str, &str)> {
        match note {
            Note::Drawn { id, words, options, field: None, .. } if options.is_empty() => {
                Some((id.as_str(), words[0].as_str(), words[1].as_str()))
            }
            _ => None,
        }
    }
    shown.notes.iter().filter_map(track).collect()
}

#[test]
fn every_made_wish_whose_run_goes_has_its_own_track_that_names_that_one_wish_and_goes_with_its_run() {
    let made = |id: &str, state| Wish { said: format!("tidy the {id} uploader"), ..wish(id, state, &[]) };
    let mut day = page(vec![wish("w1", WishState::Working, &[]), made("m1", WishState::Working)], vec![]);
    let title = "Your wish is being worked on.";
    let named = |id: &str| format!("“tidy the {id} uploader”. What it did will show here when it is done.");
    let shown = ruled(&day, &[], 1);
    assert_eq!(going(&shown), [("steps-m1", title, named("m1").as_str())]);
    assert!(matches!(shown.notes.first(), Some(Note::Drawn { id, .. }) if id == "steps-m1"), "the first note");
    // A reading's wish that is working is the model's, and no run of Jellymoon's goes for it.
    assert_eq!(going(&ruled(&day, &[], 0)), []);
    // Three at work and one done: a track for each that works, each of one wish, in the wishes' order. No note
    // says "wishes", and none counts the others.
    day.data.wishes.extend([
        made("m2", WishState::Done),
        made("m3", WishState::Working),
        made("m4", WishState::Working),
    ]);
    let all = ruled(&day, &[], 4);
    let want = ["m1", "m3", "m4"].map(|id| (format!("steps-{id}"), title.to_string(), named(id)));
    let have: Vec<_> =
        going(&all).into_iter().map(|(id, title, body)| (id.into(), title.into(), body.into())).collect();
    assert_eq!(have, want);
    assert!(!format!("{:?}", all.notes).contains("more"), "{:?}", all.notes);
    // A page with a clash keeps its room for what a person rules on (F37): a working wish has its pill only.
    day.data.clashes.push(clash(false));
    let clashing = ruled(&day, &[], 4);
    assert_eq!(going(&clashing), []);
    assert_eq!(clashing.characters[4].state, State::Busy);
    day.data.clashes.clear();
    // A long wish is named by its start, as every note of a wish names it.
    day.data.wishes[4].said = "word ".repeat(30);
    let long = ruled(&day, &[], 4);
    let body = going(&long)[2].2;
    assert!(body.starts_with("“word word") && body.len() < 160, "{body}");
    // With no run going there is no track.
    for wish in &mut day.data.wishes {
        wish.state = WishState::Done;
    }
    assert_eq!(going(&ruled(&day, &[], 4)), []);
}

// A made wish's stars are on its little one and in its own notes, under its typed words: the list of what came
// true holds the reading's rows only, and a list with no row is not drawn.
#[test]
fn what_came_true_lists_the_reading_s_stars_and_no_row_of_a_made_wish() {
    let wishes = vec![wish("w1", WishState::Done, &["a"]), wish("m1", WishState::Done, &["b", "c"])];
    let both = ruled(&page(wishes.clone(), vec![]), &[], 1);
    let rows = vec![("a".to_string(), "Label of w1".to_string())];
    assert_eq!(both.notes[0], Note::List { title: "came true today".to_string(), rows });
    assert_eq!(both.characters.iter().map(|one| one.stars).collect::<Vec<_>>(), [1, 2], "the stars stay drawn");
    // The same two wishes, both read: every row is the reading's.
    let Note::List { rows, .. } = &ruled(&page(wishes.clone(), vec![]), &[], 0).notes[0] else { panic!("a list") };
    assert_eq!(rows.len(), 3);
    // Only made wishes have stars: no list.
    let made = ruled(&page(wishes, vec![]), &[], 2);
    assert!(!made.notes.iter().any(|note| matches!(note, Note::List { .. })), "{:?}", made.notes);
}

#[test]
fn a_day_that_is_not_a_date_and_an_id_with_odd_characters_stay_text() {
    let mut odd = page(vec![wish("w\u{7}1\n<b>", WishState::Done, &[])], vec![]);
    odd.data.day = "someday".to_string();
    let scene = scene(&odd);
    assert_eq!(scene.date_label, "someday");
    assert_eq!(scene.characters[0].id, "w1 <b>");
}

// The typed words alone do not say what a wish's run was asked: the answer it went on with is on its row, under
// them, and on no other wish's. A wish of the reading has no row, and so shows none.
#[test]
fn a_made_wish_s_row_holds_the_answer_it_went_on_with_and_a_row_with_none_is_its_typed_words() {
    let made = |id: &str, answer: Option<&str>| Wish {
        answered: answer.map(str::to_string),
        ..wish(id, WishState::Done, &[])
    };
    let rows = |scene: Scene| {
        let made = |note: Note| match note {
            Note::Whole { title, rows } if title == MADE => Some(rows.into_iter().map(|row| row.1).collect()),
            _ => None,
        };
        scene.notes.into_iter().find_map(made).unwrap_or_else(Vec::new)
    };
    let day = page(vec![made("m1", None), made("m2", Some("only seven")), made("m3", None)], vec![]);
    assert_eq!(rows(ruled(&day, &[], 3)), ["please do m1", "please do m2\nonly seven", "please do m3"]);
    let read = ruled(&page(vec![made("w1", Some("only seven"))], vec![]), &[], 0);
    assert!(!format!("{read:?}").contains("only seven"), "{read:?}");
    // A row is two lines at most: a file's line break in either text is a space.
    let odd = Wish { said: "one\ntwo".to_string(), ..made("m1", Some("three\nfour")) };
    assert_eq!(rows(ruled(&page(vec![odd], vec![]), &[], 1)), ["one two\nthree four"]);
}
