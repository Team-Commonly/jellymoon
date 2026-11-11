//! Page data into the kit's scene: which pieces a page is made of. A pure function with no GPUI in it; where a piece
//! goes and how it looks are the kit's layout and the theme. Every text is handed over as it is, and the kit shows it
//! as text only.

use jellymoon_core::{Clash, PageRead, Wish, WishState};
use sticker_kit::text::clean;
use sticker_kit::{Character, LegendItem, Mark, Mood, Note, Scene, State};

use crate::read::{LeftOut, Page};

/// The longest id a piece carries. Core's are `w1` to `w8`; a file may hold anything.
const MAX_ID_CHARS: usize = 40;

/// The page as the kit's pieces: a hero, one little one per wish in page order with one star per finished job, a list
/// of what came true, one choice note per clash, and a plain note for each thing the app has no piece for. When the
/// file held more than the app draws, the first note says how much was left out.
pub fn scene(page: &Page) -> Scene {
    let data = &page.data;
    let mut notes: Vec<Note> = left_out(page.left_out).into_iter().collect();
    if data.wishes.is_empty() && page.unknown.is_empty() {
        notes.push(Note::Plain {
            title: "Nothing wished for yet today.".to_string(),
            body: "Ask one of your agents for something and it will show up here.".to_string(),
        });
    }
    let came_true: Vec<(String, String)> =
        data.wishes.iter().flat_map(|wish| wish.stars.iter().map(|star| (star.clone(), wish.label.clone()))).collect();
    if !came_true.is_empty() {
        notes.push(Note::List { title: "came true today".to_string(), rows: came_true });
    }
    notes.extend(data.clashes.iter().enumerate().map(|(index, clash)| choice(index, clash)));
    notes.extend(page.unknown.iter().map(|one| Note::Plain { title: one.title.clone(), body: one.body.clone() }));
    Scene {
        characters: data.wishes.iter().enumerate().map(|(index, wish)| little_one(index, wish)).collect(),
        footer: footer(&data.read),
        ..bare(&data.day, notes)
    }
}

/// A day's page with no wish on it: the date, the title, the hero, the legend, and these notes. It is what the app
/// shows while a day is being read and when it could not be.
pub fn bare(day: &str, notes: Vec<Note>) -> Scene {
    Scene {
        date_label: date_label(day),
        title: "what I wished for today".to_string(),
        hero: Some(Character { id: "me".to_string(), ..Character::default() }),
        characters: Vec::new(),
        notes,
        legend: [
            (Mark::Star, "finished"),
            (Mark::Busy, "working on it"),
            (Mark::NeedsYou, "needs you"),
            (Mark::Dimmed, "let go"),
        ]
        .map(|(mark, text)| LegendItem { mark, text: text.to_string() })
        .to_vec(),
        footer: String::new(),
    }
}

/// The note that counts what the file held beyond what is drawn, when it held any.
fn left_out(left_out: LeftOut) -> Option<Note> {
    let counts = [
        (left_out.wishes, "wish", "wishes"),
        (left_out.clashes, "clash", "clashes"),
        (left_out.stars, "star", "stars"),
        (left_out.unknown, "thing in a part this app does not know", "things in parts this app does not know"),
    ];
    let word = |(n, one, many): &(usize, &str, &str)| format!("{n} {}", if *n == 1 { one } else { many });
    let said: Vec<String> = counts.iter().filter(|count| count.0 > 0).map(word).collect();
    (!said.is_empty()).then(|| Note::Plain {
        title: "not everything is drawn".to_string(),
        body: format!("Left out of this page: {}.", said.join(", ")),
    })
}

/// The character that stands for a wish. Its colour only tells neighbours apart: the hero has the first hue, the
/// little ones take turns with the next four.
fn little_one(index: usize, wish: &Wish) -> Character {
    Character {
        id: clean(&wish.id, MAX_ID_CHARS),
        label: wish.label.clone(),
        hue: u8::try_from(index % 4).unwrap_or(0) + 1,
        stars: u8::try_from(wish.stars.len()).unwrap_or(u8::MAX),
        state: match wish.state {
            WishState::Working => State::Busy,
            WishState::Done => State::Idle,
            WishState::NeedsYou => State::NeedsYou,
            WishState::LetGo => State::Dimmed,
        },
        mood: Mood::Happy,
    }
}

/// A clash as an ask: both statements as options. A settled clash strikes the first, which the second replaced.
fn choice(index: usize, clash: &Clash) -> Note {
    let (title, footer) = if clash.settled {
        ("you changed your mind", "the first one was let go")
    } else {
        ("these two clash", "tap the one that stands")
    };
    Note::Choice {
        id: format!("clash-{}", index + 1),
        title: title.to_string(),
        options: vec![format!("“{}”", clash.quote_a), format!("“{}”", clash.quote_b)],
        footer: footer.to_string(),
        struck_first: clash.settled,
    }
}

/// `Tue · Mar 10` for `2026-03-10`. A day that is not a date is shown as it was written.
fn date_label(day: &str) -> String {
    match day.parse::<jiff::civil::Date>() {
        Ok(date) => date.strftime("%a · %b %-d").to_string(),
        Err(_) => day.to_string(),
    }
}

/// What the page was made from, and whose reading it is.
fn footer(read: &PageRead) -> String {
    let count = |n: usize, one: &str| format!("{n} {one}{}", if n == 1 { "" } else { "s" });
    let projects = match read.projects.len() {
        0 => String::new(),
        n => format!(" in {} ({})", count(n, "project"), read.projects.join(", ")),
    };
    format!("Read from {} you typed{projects}. This is the model's reading, unedited.", count(read.turns, "thing"))
}

#[cfg(test)]
mod tests {
    use jellymoon_core::PageData;

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
        }
    }

    fn page(wishes: Vec<Wish>, clashes: Vec<Clash>) -> Page {
        let read = PageRead { turns: 5, projects: vec!["alpha".to_string(), "beta".to_string()] };
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
                Note::Choice { id, options, struck_first, .. } => {
                    Some((id.as_str(), options.as_slice(), *struck_first))
                }
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
        empty.data.read = PageRead { turns: 0, projects: vec![] };
        let scene = scene(&empty);
        assert!(matches!(&scene.notes[..], [Note::Plain { title, .. }] if title == "Nothing wished for yet today."));
        assert_eq!(scene.characters, []);
        assert_eq!(scene.footer, "Read from 0 things you typed. This is the model's reading, unedited.");
    }

    #[test]
    fn the_footer_counts_in_words_that_agree() {
        let read =
            |turns, projects: &[&str]| PageRead { turns, projects: projects.iter().map(|p| p.to_string()).collect() };
        assert_eq!(
            footer(&read(75, &["alpha", "beta"])),
            "Read from 75 things you typed in 2 projects (alpha, beta). This is the model's reading, unedited."
        );
        assert_eq!(
            footer(&read(1, &["alpha"])),
            "Read from 1 thing you typed in 1 project (alpha). This is the model's reading, unedited."
        );
    }

    #[test]
    fn a_day_that_is_not_a_date_and_an_id_with_odd_characters_stay_text() {
        let mut odd = page(vec![wish("w\u{7}1\n<b>", WishState::Done, &[])], vec![]);
        odd.data.day = "someday".to_string();
        let scene = scene(&odd);
        assert_eq!(scene.date_label, "someday");
        assert_eq!(scene.characters[0].id, "w1 <b>");
    }
}
