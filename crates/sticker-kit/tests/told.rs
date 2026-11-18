//! A told note (`Note::Told`): what someone else says, each text a quote and nothing else, under the note's own
//! labels. A row is drawn whole or left out and counted, never cut; the note's own sentences end it; it has
//! nothing to tap; and the notes around it are planned as if it were not there.

use sticker_kit::{
    FixedAdvance, MORE_NOTES_ID, Note, PieceKind, Placed, Role, Scene, Theme, ToldRow, Viewport, layout,
};

const SIZES: [Viewport; 2] = [Viewport { w: 1440.0, h: 900.0 }, Viewport { w: 960.0, h: 640.0 }];
const ID: &str = "told-1";
const TITLE: &str = "What it says it did";
const END: [&str; 2] = ["Nothing was run to try this.", "Something said could not be checked."];

fn row(label: &str, quotes: &[&str]) -> ToldRow {
    ToldRow { label: label.into(), quotes: quotes.iter().map(|quote| quote.to_string()).collect(), with_last: false }
}

fn told(rows: Vec<ToldRow>) -> Note {
    Note::Told { id: ID.into(), title: TITLE.into(), rows, end: END.map(str::to_string).to_vec() }
}

/// A note of work with its choices, as the app puts it after a told note.
fn work() -> Note {
    let texts = |all: &[&str]| all.iter().map(|text| text.to_string()).collect();
    Note::Text {
        id: "work-1".into(),
        title: "Work waits for this wish.".into(),
        body: texts(&["“make it so”", "files made 1, changed 1, removed 0.", "a.txt, n.txt"]),
        options: texts(&["Show me", "Keep it", "Toss it"]),
        quoted: String::new(),
        field: None,
    }
}

fn place(notes: Vec<Note>, theme: &Theme, viewport: Viewport) -> Vec<Placed> {
    let scene = Scene { notes, ..Scene::sample() };
    layout(&scene, theme, viewport, &FixedAdvance(0.56))
}

fn each(test: impl Fn(&Theme, Viewport)) {
    for theme in [Theme::sticker(), Theme::plain()] {
        SIZES.iter().for_each(|size| test(&theme, *size));
    }
}

fn said(piece: &Placed) -> String {
    piece.text.iter().map(|run| run.text.as_str()).collect::<Vec<_>>().join(" ")
}

fn ours(placed: &[Placed]) -> Vec<&Placed> {
    placed.iter().filter(|piece| piece.id == ID).collect()
}

/// A text of `words` words, the first `start`.
fn words(start: &str, count: usize) -> String {
    let rest: Vec<String> = (1..count).map(|n| format!("w{n}")).collect();
    format!("{start} {}", rest.join(" "))
}

#[test]
fn every_text_of_the_other_is_a_quote_piece_and_the_note_s_own_runs_are_its_title_labels_and_sentences() {
    each(|theme, viewport| {
        // The first row has no label: the title is its label.
        let rows = vec![row("", &["It made the page."]), row("Changed in a.txt, after", &["one = 1", "two = 2"])];
        let placed = place(vec![told(rows)], theme, viewport);
        let [sheet] = ours(&placed).into_iter().filter(|p| p.piece == PieceKind::Note).collect::<Vec<_>>()[..] else {
            panic!("one note: {placed:?}")
        };
        let own: Vec<&str> = sheet.text.iter().map(|run| run.text.as_str()).collect();
        assert_eq!(own, [TITLE, "Changed in a.txt, after", END[0], END[1]]);
        assert!(sheet.text.iter().all(|run| run.role != Role::Quoted));
        let quotes: Vec<&Placed> = ours(&placed).into_iter().filter(|p| p.piece == PieceKind::Quote).collect();
        let texts: Vec<String> = quotes.iter().map(|quote| said(quote)).collect();
        assert_eq!(texts, ["It made the page.", "one = 1", "two = 2"]);
        assert!(quotes.iter().flat_map(|quote| &quote.text).all(|run| run.role == Role::Quoted));
        // Nothing of it is to act on: a tap finds no option, field, offer or mark of it.
        let kinds = [PieceKind::Note, PieceKind::Quote, PieceKind::Tape];
        assert!(ours(&placed).iter().all(|piece| kinds.contains(&piece.piece)), "{placed:?}");
    });
}

#[test]
fn a_row_that_does_not_fit_is_left_out_and_counted_and_no_row_is_cut() {
    each(|theme, viewport| {
        let long = words("long", 60);
        let rows: Vec<ToldRow> = (0..10).map(|n| row(&format!("Row {n}"), &[&long, &words("more", 20)])).collect();
        let placed = place(vec![told(rows)], theme, viewport);
        let sheet = ours(&placed).into_iter().find(|p| p.piece == PieceKind::Note).expect("the note is drawn");
        let quotes: Vec<String> =
            ours(&placed).iter().filter(|p| p.piece == PieceKind::Quote).map(|quote| said(quote)).collect();
        assert!(quotes.len() < 20 && quotes.len().is_multiple_of(2), "{} quotes", quotes.len());
        for pair in quotes.chunks(2) {
            assert_eq!(pair, [long.clone(), words("more", 20)], "a row is drawn whole");
        }
        let own: Vec<&str> = sheet.text.iter().map(|run| run.text.as_str()).collect();
        let count = format!("+{} more", 10 - quotes.len() / 2);
        assert_eq!(own[own.len() - 3..], [count.as_str(), END[0], END[1]], "{own:?}");
        assert!(!own.iter().any(|run| run.contains('…')), "{own:?}");
    });
}

#[test]
fn a_row_cleaning_would_cut_is_left_out_and_counted_as_any_other() {
    each(|theme, viewport| {
        let over = "x".repeat(sticker_kit::layout::MAX_TEXT_CHARS + 1);
        let placed = place(vec![told(vec![row("", &["kept"]), row("Too long", &[&over])])], theme, viewport);
        let sheet = ours(&placed).into_iter().find(|p| p.piece == PieceKind::Note).unwrap();
        let own: Vec<&str> = sheet.text.iter().map(|run| run.text.as_str()).collect();
        assert_eq!(own, [TITLE, "+1 more", END[0], END[1]]);
        let quotes: Vec<String> =
            ours(&placed).iter().filter(|p| p.piece == PieceKind::Quote).map(|quote| said(quote)).collect();
        assert_eq!(quotes, ["kept"]);
    });
}

#[test]
fn the_note_after_it_is_the_note_it_is_without_it_moved_down_and_the_told_note_comes_first() {
    each(|theme, viewport| {
        let rows = vec![row("", &[&words("did", 30)]), row("Decided", &["An order of exactly 50 ships free."])];
        let alone = place(vec![work()], theme, viewport);
        let placed = place(vec![told(rows), work()], theme, viewport);
        let first = |id: &str| placed.iter().position(|piece| piece.id == id).unwrap();
        assert!(first(ID) < first("work-1"), "the told note is printed first");
        let work_of = |all: &[Placed]| all.iter().filter(|p| p.id == "work-1").cloned().collect::<Vec<_>>();
        let (was, now) = (work_of(&alone), work_of(&placed));
        assert_eq!(was.iter().filter(|p| p.piece == PieceKind::Option).count(), 3);
        assert_eq!(was.len(), now.len());
        let sheet = ours(&placed).into_iter().find(|p| p.piece == PieceKind::Note).unwrap();
        let moved = now[0].rect.y - was[0].rect.y;
        assert!(moved >= sheet.rect.h, "the work's note is under the told note");
        for (was, now) in was.iter().zip(&now) {
            // A sheet's and a tape's index is the note's place in the scene: one on, with the told note before it.
            let index = if matches!(was.piece, PieceKind::Note | PieceKind::Tape) { was.index + 1 } else { was.index };
            assert_eq!(
                (was.piece, &was.id, index, was.tilt_degrees),
                (now.piece, &now.id, now.index, now.tilt_degrees)
            );
            assert_eq!((was.rect.x, was.rect.w), (now.rect.x, now.rect.w));
            assert!((was.rect.h - now.rect.h).abs() < 0.01, "{was:?} {now:?}");
            assert!((now.rect.y - was.rect.y - moved).abs() < 0.01, "{was:?} {now:?}");
            let runs =
                |p: &Placed| p.text.iter().map(|r| (r.text.clone(), r.role, r.rect.x, r.size)).collect::<Vec<_>>();
            assert_eq!(runs(was), runs(now));
        }
        assert!(placed.iter().all(|piece| piece.id != MORE_NOTES_ID), "no note was left out");
    });
}

#[test]
fn a_told_note_takes_room_from_the_notes_after_the_one_it_is_over_and_never_from_that_one() {
    each(|theme, viewport| {
        // Notes of work that fill the page: the first is placed whole with its choices, the told note over it is
        // drawn, with the rows that fit, and the notes after them give way and are counted.
        let many: Vec<Note> = (0..4).map(|_| work()).collect();
        let big = told((0..3).map(|n| row(&format!("Row {n}"), &[&words("w", 40)])).collect());
        let placed = place([vec![big], many].concat(), theme, viewport);
        let first = |id: &str| placed.iter().position(|piece| piece.id == id).unwrap();
        assert!(first(ID) < first("work-1"));
        let options = placed.iter().filter(|p| p.id == "work-1" && p.piece == PieceKind::Option).count();
        assert!(options >= 3 && options.is_multiple_of(3), "{options}");
        let bottom = placed.iter().map(|piece| piece.rect.y + piece.rect.h).fold(0.0, f32::max);
        assert!(bottom <= viewport.h, "{bottom}");
    });
}

#[test]
fn rows_are_drawn_in_order_up_to_the_first_that_does_not_fit_and_a_unit_is_drawn_whole_or_not_at_all() {
    each(|theme, viewport| {
        let quotes = |of: &[Placed]| of.iter().filter(|p| p.id == ID && p.piece == PieceKind::Quote).count();
        let long: Vec<String> = (0..30).map(|n| words(&format!("t{n}"), 12)).collect();
        let long: Vec<&str> = long.iter().map(String::as_str).collect();
        // The first row has no room, so the short row after it is not drawn in its place.
        let placed = place(vec![told(vec![row("", &long), row("Short", &["short"])])], theme, viewport);
        let sheet = ours(&placed).into_iter().find(|p| p.piece == PieceKind::Note).expect("the note is drawn");
        let own: Vec<&str> = sheet.text.iter().map(|run| run.text.as_str()).collect();
        assert_eq!(own, [TITLE, "+2 more", END[0], END[1]]);
        assert_eq!(quotes(&placed), 0, "a later row was drawn");
        // A row that goes with the one before it: the first would fit alone, the two do not, and neither is drawn.
        let mut after = row("After", &long);
        after.with_last = true;
        let placed = place(vec![told(vec![row("Before", &["one line"]), after])], theme, viewport);
        assert_eq!(quotes(&placed), 0, "half of a unit was drawn");
    });
}
