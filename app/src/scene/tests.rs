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
    assert_eq!(scene.legend.len(), 4);
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
    assert!(matches!(&scene.notes[..], [Note::Plain { title, .. }] if title == "Nothing wished for yet today."));
    assert_eq!(scene.characters, []);
    assert_eq!((scene.footer.as_str(), scene.legend.len()), ("", 0), "nothing to explain, no reading");
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
fn a_day_of_made_wishes_with_nothing_read_has_the_legend_and_no_footer() {
    let mut own = page(vec![wish("m1", WishState::Working, &[])], vec![]);
    own.data.read = PageRead { turns: 0, projects: Vec::new(), no_logs: false };
    let shown = ruled(&own, &[], 1);
    assert_eq!((shown.footer.as_str(), shown.legend.len()), ("", 4));
    own.data.read.no_logs = true;
    let found_none = "No Claude Code session logs were found, so nothing was read. 1 made wish is in your own words.";
    assert_eq!(ruled(&own, &[], 1).footer, found_none);
}

/// The page's notes that say a made wish is being worked on.
fn going(shown: &Scene) -> Vec<(&str, &str)> {
    fn said(note: &Note) -> Option<(&str, &str)> {
        match note {
            Note::Plain { title, body } if title.contains("being worked on") => Some((title.as_str(), body.as_str())),
            _ => None,
        }
    }
    shown.notes.iter().filter_map(said).collect()
}

#[test]
fn the_first_note_says_the_made_wishes_being_worked_on_once_for_all_and_goes_with_their_runs() {
    let made = |state| Wish { said: format!("tidy the {state:?} uploader"), ..wish("m", state, &[]) };
    let mut day = page(vec![wish("w1", WishState::Working, &[]), made(WishState::Working)], vec![]);
    let one =
        ("Your wish is being worked on.", "“tidy the Working uploader”. What it did will show here when it is done.");
    let shown = ruled(&day, &[], 1);
    assert_eq!(going(&shown), [one]);
    assert!(matches!(shown.notes.first(), Some(Note::Plain { title, .. }) if title == one.0), "the first note");
    // A reading's wish that is working is the model's, and no run of Jellymoon's goes for it.
    assert_eq!(going(&ruled(&day, &[], 0)), []);
    day.data.wishes.extend([made(WishState::Done), made(WishState::Working), made(WishState::Working)]);
    day.data.wishes[4].said = "the newest wish".to_string();
    let many = "“the newest wish”, the newest, and 2 more. What each did will show here when it is done.";
    assert_eq!(going(&ruled(&day, &[], 4)), [("Your wishes are being worked on.", many)]);
    // A page with a clash keeps its room for what a person rules on (F37).
    day.data.clashes.push(clash(false));
    assert_eq!(going(&ruled(&day, &[], 4)), []);
    day.data.clashes.clear();
    // A long wish is named by its start, as every note of a wish names it.
    day.data.wishes[4].said = "word ".repeat(30);
    let long = ruled(&day, &[], 4);
    let [(_, body)] = going(&long)[..] else { panic!("one note") };
    assert!(body.starts_with("“word word") && body.len() < 160, "{body}");
    // With no run going there is nothing to say.
    for wish in &mut day.data.wishes {
        wish.state = WishState::Done;
    }
    assert_eq!(going(&ruled(&day, &[], 4)), []);
}

#[test]
fn a_day_that_is_not_a_date_and_an_id_with_odd_characters_stay_text() {
    let mut odd = page(vec![wish("w\u{7}1\n<b>", WishState::Done, &[])], vec![]);
    odd.data.day = "someday".to_string();
    let scene = scene(&odd);
    assert_eq!(scene.date_label, "someday");
    assert_eq!(scene.characters[0].id, "w1 <b>");
}
