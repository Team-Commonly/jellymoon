//! Page data into the kit's scene: which pieces a page is made of. A pure function with no GPUI in it; where a piece
//! goes and how it looks are the kit's layout and the theme. Every text is handed over as it is, and the kit shows it
//! as text only.

use jellymoon_core::{Clash, PageData, Side, Standing, Wish, WishState};
use sticker_kit::text::clean;
use sticker_kit::{Character, Mark, Mood, Note, Scene, State};

use crate::read::{LeftOut, Page};

/// The longest id a piece carries. Core's are `w1` to `w8`; a file may hold anything.
const MAX_ID_CHARS: usize = 40;

/// The sides of a clash in the order its options are drawn. The one the person chose is drawn last, under the one
/// it replaced, which is struck: the kit strikes the first option of a choice and no other.
pub fn order(stands: Option<Side>) -> [Side; 2] {
    if stands == Some(Side::A) { [Side::B, Side::A] } else { [Side::A, Side::B] }
}

/// The title of the note that holds the day's made wishes, each in the person's exact words.
pub const MADE: &str = "wished here today";

/// The footer of a day with no session logs at all.
const FOUND_NO_LOGS: &str = "No Claude Code session logs were found, so nothing was read.";

/// The titles of the note that says the person's made wishes are being worked on: of one, and of more. Core's
/// words for a wish at work are "is being worked on" too.
pub(crate) const GOING: [&str; 2] = ["Your wish is being worked on.", "Your wishes are being worked on."];

/// The title of the one note of a page with no wish on it, and its body when a project is on offer. With none,
/// the body says to choose a folder first (`Today::draw`).
pub const NOTHING: &str = "Nothing wished for yet today.";
pub const NOTHING_HERE: &str = "Type what you want done in the field at the bottom.";
pub const NO_PROJECT: &str =
    "Choose the folder you want to work in, or type a name for a new project in the field at the bottom.";

/// [`ruled`] for a page nobody ruled on, with no made wish on it.
pub fn scene(page: &Page) -> Scene {
    ruled(page, &[], 0)
}

/// The page as the kit's pieces: a hero, one little one per wish in page order with one star per finished job, a list
/// of what came true (a made wish that was let go has no star and is not listed), one choice note per clash, and a
/// plain note for each thing the app has no piece for. When the file held more than the app draws, the first note
/// says how much was left out. `standing` holds, clash by clash, what core says of it: its two statements as a
/// ruling would carry them, and the side the person chose with a tap.
/// A clash core says is no choice (its statements are the same once cleaned, or one is empty) is a plain note.
/// The page's last `made` wishes are the person's made wishes. A reading's wish has a title the model wrote; a made
/// wish has only the person's words, so one note after the clashes lists them, each whole or counted, never cut.
/// While a run of the made wishes goes, the first note says so ([`going`]), on a page with no clash.
pub fn ruled(page: &Page, standing: &[Standing], made: usize) -> Scene {
    let data = &page.data;
    let first = data.wishes.len().saturating_sub(made);
    let made = &data.wishes[first..];
    // A made wish that was let go had its change thrown away: what its run reported did not come true, so its star
    // is not drawn and not listed. A reading's wish that was let go keeps the stars of the jobs that finished.
    let came = |index: usize, wish: &Wish| index < first || wish.state != WishState::LetGo;
    // A page with a clash has none: a made wish moves and shrinks nothing a person rules on (F37).
    let says = going(made).filter(|_| data.clashes.is_empty());
    let mut notes: Vec<Note> = left_out(page.left_out).into_iter().chain(says).collect();
    if data.wishes.is_empty() && page.unknown.is_empty() {
        notes.push(Note::Plain { title: NOTHING.to_string(), body: NOTHING_HERE.to_string() });
    }
    let true_of = data.wishes.iter().enumerate().filter(|(index, wish)| came(*index, wish));
    let came_true: Vec<(String, String)> =
        true_of.flat_map(|(_, wish)| wish.stars.iter().map(|star| (star.clone(), wish.label.clone()))).collect();
    if !came_true.is_empty() {
        notes.push(Note::List { title: "came true today".to_string(), rows: came_true });
    }
    // A clash the person has not chosen on comes before one they have: when the notes do not all fit, the ones left
    // out and counted are the ones already settled. A clash keeps its id, its place among the page's clashes.
    let chosen = |index: &usize| standing.get(*index).is_some_and(|standing| standing.stands.is_some());
    let (open, settled): (Vec<usize>, Vec<usize>) = (0..data.clashes.len()).partition(|index| !chosen(index));
    let clashes = open.into_iter().chain(settled);
    notes.extend(clashes.map(|index| match standing.get(index) {
        // Nothing to choose, so nothing to tap: the clash is said, and neither statement is drawn.
        Some(standing) if !standing.is_choice() => Note::Plain {
            title: "two things you said clash".to_string(),
            body: "Their words cannot be told apart here, so there is nothing to choose.".to_string(),
        },
        standing => choice(index, &data.clashes[index], standing),
    }));
    if !made.is_empty() {
        let rows = made.iter().map(|wish| (mark(wish.state), wish.said.clone())).collect();
        notes.push(Note::Whole { title: MADE.to_string(), rows });
    }
    notes.extend(page.unknown.iter().map(|one| Note::Plain { title: one.title.clone(), body: one.body.clone() }));
    // Nothing read and no wish, projects on offer or not: there is no reading to speak of.
    let blank = data.read.turns == 0 && data.clashes.is_empty() && data.wishes.is_empty();
    let read = footer(data, made.len());
    // A footer that says nothing of a reading says nothing at all: every wish on the page is the person's own.
    let read = if read.is_empty() { read } else { [read, own(made.len())].join(" ").trim_end().to_string() };
    let drawn = |(index, wish): (usize, &Wish)| {
        let one = little_one(index, wish);
        if came(index, wish) { one } else { Character { stars: 0, ..one } }
    };
    Scene {
        characters: data.wishes.iter().enumerate().map(drawn).collect(),
        footer: if blank { String::new() } else { read },
        ..bare(&data.day, notes)
    }
}

/// A day's page with no wish on it: the date, the title, the hero, and these notes. It is what the app shows while a day is being read and when it could not be.
pub fn bare(day: &str, notes: Vec<Note>) -> Scene {
    Scene {
        date_label: date_label(day),
        title: "what I wished for today".to_string(),
        hero: Some(Character { id: "me".to_string(), ..Character::default() }),
        characters: Vec::new(),
        notes,
        footer: String::new(),
    }
}

/// What the note of a tapped wish of the reading says: what the reading took it for, never that a run goes; the
/// person's words; where they typed them, and that Jellymoon ran nothing; and how to go on, when its project has
/// a chip.
pub(crate) fn read_said(wish: &Wish, chip: bool) -> (&'static str, Vec<String>) {
    let title = match wish.state {
        WishState::Working => "This looks unfinished.",
        WishState::Done => "This looks done.",
        WishState::NeedsYou => "This looks like it needs you.",
        WishState::LetGo => "This looks like you let it go.",
    };
    let (project, said) = (&wish.project, format!("“{}”", wish.said));
    let at = if project.is_empty() { String::new() } else { format!(", in {project}") };
    let from = format!("You typed this in Claude Code{at}. Jellymoon did not run it.");
    let next = format!("To go on with it here, type what you want next. It will start in {project}.");
    (title, [said, from].into_iter().chain(chip.then_some(next)).collect())
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

/// How many of the page's wishes are the person's own made wishes, for the footer. Nothing when there is none.
pub fn own(made: usize) -> String {
    match made {
        0 => String::new(),
        1 => "1 made wish is in your own words.".to_string(),
        n => format!("{n} made wishes are in your own words."),
    }
}

/// The note that says the made wishes whose run goes are being worked on, one for all of them: the newest named
/// by the start of its words, as every note of a wish names it, and the rest counted. It has no option and no
/// field, so it is never in the person's hand (`working::in_hand`), and it goes with the last run. None when no
/// run goes.
fn going(made: &[Wish]) -> Option<Note> {
    let working: Vec<&Wish> = made.iter().filter(|wish| wish.state == WishState::Working).collect();
    let newest = working.last()?;
    let named = format!("“{}”", jellymoon_core::wish_start(&newest.said, crate::wishing::NAMED_MAX));
    let (title, body) = match working.len() {
        1 => (GOING[0], format!("{named}. What it did will show here when it is done.")),
        n => (
            GOING[1],
            format!("{named}, the newest, and {} more. What each did will show here when it is done.", n - 1),
        ),
    };
    Some(Note::Plain { title: title.to_string(), body })
}

/// The mark of a wish's state, on its row of the made wishes.
fn mark(state: WishState) -> Mark {
    match state {
        WishState::Working => Mark::Busy,
        WishState::Done => Mark::Star,
        WishState::NeedsYou => Mark::NeedsYou,
        WishState::LetGo => Mark::Dimmed,
    }
}

/// The id of a wish as its character carries it, which is what a tap on the character gives.
pub(crate) fn drawn_id(id: &str) -> String {
    clean(id, MAX_ID_CHARS)
}

/// The character that stands for a wish. Its colour only tells neighbours apart: the hero has the first hue, the
/// little ones take turns with the next four.
fn little_one(index: usize, wish: &Wish) -> Character {
    Character {
        id: drawn_id(&wish.id),
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
        says: String::new(),
    }
}

/// A statement as an option of a clash: its words between quotation marks, and nothing else.
pub fn quoted(statement: &str) -> String {
    format!("“{statement}”")
}

/// A clash as an ask: both statements as options, in [`order`]. A settled clash strikes the first, which the second
/// replaced: by the person's own tap (`standing`), or as the reading found it. With `standing` (a day's page, which
/// can be tapped) the words are core's, exactly as a ruling would carry them, and the choice is whole: every word is
/// drawn or the note is left out. A page-data file with no day shows the file's own words, cut to fit.
fn choice(index: usize, clash: &Clash, standing: Option<&Standing>) -> Note {
    let stands = standing.and_then(|standing| standing.stands);
    let (a, b) = standing.map_or((&clash.quote_a, &clash.quote_b), |standing| (&standing.a, &standing.b));
    let (title, footer) = match (stands, clash.settled) {
        (Some(_), _) => ("you chose", "tap the other to change it"),
        (None, true) => ("you changed your mind", "the first one was let go"),
        (None, false) => ("these two clash", "tap the one that stands"),
    };
    let quote = |side: Side| quoted(if side == Side::A { a } else { b });
    Note::Choice {
        id: format!("clash-{}", index + 1),
        title: title.to_string(),
        options: order(stands).map(quote).to_vec(),
        footer: footer.to_string(),
        struck_first: stands.is_some() || clash.settled,
        whole: standing.is_some(),
    }
}

/// `Tue · Mar 10` for `2026-03-10`. A day that is not a date is shown as it was written.
fn date_label(day: &str) -> String {
    match day.parse::<jiff::civil::Date>() {
        Ok(date) => date.strftime("%a · %b %-d").to_string(),
        Err(_) => day.to_string(),
    }
}

/// Whether the page holds nothing a reading gives: no turn, no project, no clash, and no wish but its last `made`.
/// Core has no check of a whole page, so `read.no_logs` is asked together with this, here and in `today.rs`.
pub(crate) fn read_nothing(data: &PageData, made: usize) -> bool {
    data.read.turns == 0 && data.read.projects.is_empty() && data.clashes.is_empty() && data.wishes.len() <= made
}

/// What the page was made from, and whose reading it is. A page that holds nothing a reading gives names no
/// model: it says that no session logs were found (`jellymoon_core::page_with_no_logs`), or, when the logs gave
/// no turn of the day, nothing.
fn footer(data: &PageData, made: usize) -> String {
    let read = &data.read;
    if read_nothing(data, made) {
        return if read.no_logs { FOUND_NO_LOGS.to_string() } else { String::new() };
    }
    let count = |n: usize, one: &str| format!("{n} {one}{}", if n == 1 { "" } else { "s" });
    let projects = match read.projects.len() {
        0 => String::new(),
        n => format!(" in {} ({})", count(n, "project"), read.projects.join(", ")),
    };
    format!("Read from {} you typed{projects}. This is the model's reading, unedited.", count(read.turns, "thing"))
}

#[cfg(test)]
mod tests;
