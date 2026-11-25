//! The fold of a read day, with no core and no window: the pages are typed here, drawn by `scene::ruled`, and
//! put in their column's order as a window does it (`Open::column`), which is the column before the day folds.

use jellymoon_core::{Clash, PageRead, Standing, WishState};
use sticker_kit::Field;

use super::*;
use crate::drawing::Open;
use crate::read::{LeftOut, Page};
use crate::scene::ruled;

fn wish(id: &str, project: &str, state: WishState, stars: &[&str]) -> Wish {
    Wish {
        id: id.to_string(),
        label: format!("Label of {id}"),
        said: format!("please do {id}"),
        project: project.to_string(),
        state,
        stars: stars.iter().map(|star| star.to_string()).collect(),
        question: None,
        answered: None,
    }
}

fn page(projects: &[&str], wishes: Vec<Wish>, clashes: Vec<Clash>) -> Page {
    let read = PageRead { turns: 9, projects: projects.iter().map(|name| name.to_string()).collect(), no_logs: false };
    let data = PageData { day: "2026-03-10".to_string(), wishes, clashes, read };
    Page { data, unknown: Vec::new(), left_out: LeftOut::default() }
}

fn clash(a: &str, b: &str, settled: bool) -> Clash {
    let (quote_a, quote_b) = ("add a retry".to_string(), "never retry".to_string());
    Clash { a: a.to_string(), b: b.to_string(), quote_a, quote_b, settled }
}

/// A day read in three projects, typed in the order beta, alpha, gamma: five wishes of the reading, one of each
/// state and a second one done, six things that came true, an open clash of `w2` and one the reading found
/// settled. Its last wish is a made wish of `alpha`.
fn day() -> Page {
    let wishes = vec![
        wish("w1", "alpha", WishState::Done, &["one", "two"]),
        wish("w2", "alpha", WishState::NeedsYou, &["three"]),
        wish("w3", "beta", WishState::Working, &[]),
        wish("w4", "beta", WishState::LetGo, &["four"]),
        wish("w5", "gamma", WishState::Done, &["five", "six"]),
        wish("m1", "alpha", WishState::Done, &["made it"]),
    ];
    page(&["beta", "alpha", "gamma"], wishes, vec![clash("w2", "w3", false), clash("w4", "w5", true)])
}

/// The column of `page` before the day folds, as a window nobody tapped in has it.
fn column(page: &Page, made: usize) -> Scene {
    Open::default().column(&ruled(page, &[], made))
}

fn tally(scene: &Scene) -> (&str, &[TallyRow], &str, &str, bool) {
    match scene.notes.last() {
        Some(Note::Drawn { id, picture: Picture::Tally { title, rows, more, under, open, .. }, .. }) if id == FOLD => {
            (title, rows, more, under, *open)
        }
        other => panic!("no tally at the scene's end: {other:?}"),
    }
}

fn row(label: &str, marks: &[Mark]) -> TallyRow {
    TallyRow { label: label.to_string(), marks: marks.to_vec() }
}

fn titles(scene: &Scene) -> Vec<String> {
    let title = |note: &Note| match note {
        Note::Plain { title, .. } | Note::List { title, .. } | Note::Choice { title, .. } => title.clone(),
        Note::Text { id, .. } | Note::Said { id, .. } | Note::Told { id, .. } | Note::Drawn { id, .. } => id.clone(),
        Note::Whole { title, .. } => title.clone(),
    };
    scene.notes.iter().map(title).collect()
}

const ASKS: Mark = Mark::NeedsYou;

#[test]
fn the_tally_has_a_row_for_each_project_in_the_order_typed_and_a_mark_for_each_piece_of_the_reading() {
    let (page, made) = (day(), 1);
    let folded = folded(column(&page, made), Some(&page.data), false);
    let (title, rows, more, under, open) = tally(&folded);
    assert_eq!((title, more, under, open), ("today so far", "", READING, false));
    let want = [
        row("beta", &[Mark::Busy, Mark::Star, Mark::Dimmed]),
        // The wish that needs the person and the open clash of it; the made wish's star is not the reading's.
        row("alpha", &[ASKS, ASKS, Mark::Star, Mark::Star, Mark::Star]),
        row("gamma", &[Mark::Star, Mark::Star]),
    ];
    assert_eq!(rows, want);
    // One mark for one piece the open page draws: its stars are the rows of what came true.
    let scene = ruled(&page, &[], made);
    let came = scene.notes.iter().find_map(|note| match note {
        Note::List { title, rows } if title == CAME_TRUE => Some(rows.len()),
        _ => None,
    });
    let count = |mark: Mark| rows.iter().flat_map(|row| &row.marks).filter(|one| **one == mark).count();
    assert_eq!(Some(count(Mark::Star)), came);
    let state = |state: State| scene.characters[..5].iter().filter(|who| who.state == state).count();
    assert_eq!((count(Mark::Busy), count(Mark::Dimmed)), (state(State::Busy), state(State::Dimmed)));
    assert_eq!(count(ASKS), state(State::NeedsYou) + 1, "and the one clash that waits");
}

#[test]
fn a_folded_day_has_none_of_the_reading_s_pieces_but_the_ones_that_wait_and_every_made_wish() {
    let (page, made) = (day(), 1);
    let before = column(&page, made);
    let folded = folded(before.clone(), Some(&page.data), false);
    let ids: Vec<&str> = folded.characters.iter().map(|who| who.id.as_str()).collect();
    assert_eq!(ids, ["m1"], "the made wish's little one, and no other");
    assert_eq!(titles(&folded), [CLASH, "typed-m1", FOLD], "the open clash, the made wish, the tally");
    let kept = |scene: &Scene, title: &str| {
        let at = titles(scene).iter().position(|one| one == title).expect(title);
        scene.notes[at].clone()
    };
    for title in [CLASH, "typed-m1"] {
        assert_eq!(kept(&folded, title), kept(&before, title), "{title} is as it was");
    }
    assert_eq!(folded.characters[0], before.characters[5]);
    assert_eq!(
        (&folded.title, &folded.date_label, &folded.footer),
        (&before.title, &before.date_label, &before.footer)
    );
}

#[test]
fn an_open_day_is_the_column_it_was_piece_for_piece_with_the_tally_as_its_last_note() {
    let (page, made) = (day(), 1);
    let before = column(&page, made);
    let mut open = folded(before.clone(), Some(&page.data), true);
    assert!(tally(&open).4, "the tally says it is open");
    let closed = folded(before.clone(), Some(&page.data), false);
    assert_eq!(tally(&open).1, tally(&closed).1, "and counts what it counted");
    open.notes.pop();
    assert_eq!(open, before);
}

/// A scene with the note a tap on the read wish `w1` brings, and what the fold makes of it.
fn tapped(note: Note) -> (Scene, Scene) {
    let page = day();
    let mut scene = ruled(&page, &[], 1);
    scene.notes.insert(0, note);
    let before = Open::default().column(&scene);
    (before.clone(), folded(before, Some(&page.data), false))
}

fn text(id: &str, options: &[&str], field: Option<Field>) -> Note {
    let options = options.iter().map(|option| option.to_string()).collect();
    let (title, body) = ("This looks done.".to_string(), vec!["You typed this.".to_string()]);
    Note::Text { id: id.to_string(), title, body, options, quoted: String::new(), field }
}

// The fold never hides what waits for a tap. The note a tap on a read wish brought says something and asks
// nothing, so it folds with its wish; with a choice or a field in it, it stays. A clash the person chose on is
// theirs, and stays; one the reading found settled is folded.
#[test]
fn the_fold_never_hides_a_note_that_has_a_choice_or_a_field_or_a_clash_the_person_has_a_hand_in() {
    let (_, plain) = tapped(text("wish-w1", &[], None));
    assert!(!titles(&plain).contains(&"wish-w1".to_string()), "{:?}", titles(&plain));
    let field = Field { hint: "what next".to_string(), most: 200, ..Field::default() };
    for note in [text("wish-w1", &["Go on"], None), text("wish-w1", &[], Some(field)), text("wish-m1", &[], None)] {
        let (before, folded) = tapped(note.clone());
        assert!(folded.notes.contains(&note), "{:?}", titles(&folded));
        assert_eq!(
            folded.notes.len() + 1,
            before.notes.len(),
            "less what came true and the settled clash, and the tally"
        );
    }
    // The person chose on the first clash: it is theirs now, with the tap that changes it.
    let page = day();
    let standing = |clash: &Clash| Standing { a: clash.quote_a.clone(), b: clash.quote_b.clone(), stands: None };
    let mut standing: Vec<Standing> = page.data.clashes.iter().map(standing).collect();
    standing[0].stands = Some(jellymoon_core::Side::B);
    let before = Open::default().column(&ruled(&page, &standing, 1));
    let folded = folded(before, Some(&page.data), false);
    assert_eq!(titles(&folded), ["you chose", "typed-m1", FOLD]);
    assert_eq!(tally(&folded).1[1], row("alpha", &[ASKS, Mark::Star, Mark::Star, Mark::Star]), "and it asks no more");
}

#[test]
fn no_word_of_the_model_s_is_in_the_tally_and_projects_past_the_fifth_are_counted_in_one_line() {
    let names = ["p1", "p2", "p3", "p4", "p5", "p6", "p7"];
    let wishes = names.iter().enumerate().map(|(n, name)| wish(&format!("w{}", n + 1), name, WishState::Done, &["x"]));
    let page = page(&names, wishes.collect(), Vec::new());
    let folded = folded(column(&page, 0), Some(&page.data), false);
    let (_, rows, more, ..) = tally(&folded);
    assert_eq!(rows.len(), 7, "every project has its row, and the kit draws the first five");
    assert_eq!(more, "and 2 more projects");
    assert!(rows.iter().zip(names).all(|(row, name)| row.label == name && row.marks == [Mark::Star]));
    let day = day();
    let written = format!("{:?}", tally(&super::folded(column(&day, 1), Some(&day.data), false)));
    for word in ["Label", "please", "one", "retry", "made it"] {
        assert!(!written.contains(word), "{word} in {written}");
    }
    let six = self::page(&names[..6], day.data.wishes.clone(), Vec::new());
    assert_eq!(tally(&super::folded(column(&six, 1), Some(&six.data), false)).2, "", "three rows, and none past them");
}

#[test]
fn a_day_with_nothing_read_and_a_scene_that_is_not_the_page_s_have_no_tally() {
    let made = page(&["alpha"], vec![wish("m1", "alpha", WishState::Done, &[])], Vec::new());
    let before = column(&made, 1);
    assert_eq!(folded(before.clone(), Some(&made.data), false), before, "a made wish alone is no reading");
    let empty = page(&[], Vec::new(), Vec::new());
    assert_eq!(folded(column(&empty, 0), Some(&empty.data), false), column(&empty, 0));
    let day = day();
    let before = column(&day, 1);
    assert_eq!(folded(before.clone(), None, false), before, "a page-data file has no day");
    assert_eq!(folded(before.clone(), Some(&empty.data), false), before, "nothing is folded that cannot be counted");
}

#[test]
fn a_window_starts_folded_and_a_tap_on_the_tally_and_on_nothing_else_opens_and_folds_the_day() {
    let page = day();
    let scene = ruled(&page, &[], 1);
    let open = Open::opening(&scene);
    let is_open = |open: &Open| tally(&open.day(&scene, Some(&page.data))).4;
    assert!(!is_open(&open), "folded at its opening");
    assert!(!open.fold("work-m1") && !is_open(&open), "a tap on another picture is not the tally's");
    assert!(open.fold(FOLD) && is_open(&open));
    assert!(open.fold(FOLD) && !is_open(&open));
    assert!(!is_open(&Open::opening(&scene)), "and a new window starts folded");
}
