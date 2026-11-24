//! A told note (`Note::Told`): what someone else says, each text a quote and nothing else, under the note's own
//! labels. A row is drawn whole or left out and counted, never cut; the note's own sentences end it; it has
//! nothing to tap; and the notes around it are the notes they are without it, under it.

use sticker_kit::{FixedAdvance, Note, PieceKind, Placed, Role, Scene, Theme, ToldRow, Viewport, layout};

const SIZES: [Viewport; 2] = [Viewport { w: 1440.0, h: 900.0 }, Viewport { w: 960.0, h: 640.0 }];
const ID: &str = "told-1";
const TITLE: &str = "What it says it did";
const END: [&str; 2] = ["Nothing was run to try this.", "Something said could not be checked."];

fn row(label: &str, quotes: &[&str]) -> ToldRow {
    ToldRow {
        label: label.into(),
        quotes: quotes.iter().map(|quote| quote.to_string()).collect(),
        ..ToldRow::default()
    }
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
        // The first row has no label: the title is its label. The second has the note's own word beside two of
        // its three quotes, which are drawn after the word, close together.
        let mut change = row("Changed in a.txt", &["one = 0", "one = 1", "two = 2"]);
        change.marks = ["before", "after", ""].map(str::to_string).to_vec();
        let placed = place(vec![told(vec![row("", &["It made the page."]), change])], theme, viewport);
        let [sheet] = ours(&placed).into_iter().filter(|p| p.piece == PieceKind::Note).collect::<Vec<_>>()[..] else {
            panic!("one note: {placed:?}")
        };
        let own: Vec<&str> = sheet.text.iter().map(|run| run.text.as_str()).collect();
        assert_eq!(own, [TITLE, "Changed in a.txt", "before", "after", END[0], END[1]]);
        let grounds: Vec<&Placed> = ours(&placed).into_iter().filter(|p| p.piece == PieceKind::Quote).collect();
        let marked = sheet.text.iter().find(|run| run.text == "before").unwrap();
        assert!(grounds[1..].iter().all(|ground| ground.rect.x >= marked.rect.x + marked.rect.w), "beside its word");
        assert!(grounds[1].rect.x > grounds[0].rect.x && grounds[2].rect.x == grounds[1].rect.x);
        assert!(sheet.text.iter().all(|run| run.role != Role::Quoted));
        let quotes: Vec<&Placed> = ours(&placed).into_iter().filter(|p| p.piece == PieceKind::Quote).collect();
        let texts: Vec<String> = quotes.iter().map(|quote| said(quote)).collect();
        assert_eq!(texts, ["It made the page.", "one = 0", "one = 1", "two = 2"]);
        assert!(quotes.iter().flat_map(|quote| &quote.text).all(|run| run.role == Role::Quoted));
        // Nothing of it is to act on: a tap finds no option, field, offer or mark of it.
        let kinds = [PieceKind::Note, PieceKind::Quote, PieceKind::Tape];
        assert!(ours(&placed).iter().all(|piece| kinds.contains(&piece.piece)), "{placed:?}");
    });
}

#[test]
fn ten_long_rows_are_all_drawn_whole_and_none_is_counted_for_room() {
    each(|theme, viewport| {
        let long = words("long", 60);
        let rows: Vec<ToldRow> = (0..10).map(|n| row(&format!("Row {n}"), &[&long, &words("more", 20)])).collect();
        let placed = place(vec![told(rows)], theme, viewport);
        let sheet = ours(&placed).into_iter().find(|p| p.piece == PieceKind::Note).expect("the note is drawn");
        let quotes: Vec<String> =
            ours(&placed).iter().filter(|p| p.piece == PieceKind::Quote).map(|quote| said(quote)).collect();
        assert_eq!(quotes.len(), 20, "every row");
        for pair in quotes.chunks(2) {
            assert_eq!(pair, [long.clone(), words("more", 20)], "a row is drawn whole");
        }
        let own: Vec<&str> = sheet.text.iter().map(|run| run.text.as_str()).collect();
        assert_eq!(own[own.len() - 3..], ["Row 9", END[0], END[1]], "{own:?}");
        assert!(!own.iter().any(|run| run.contains('…') || run.contains("more")), "{own:?}");
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
    });
}

#[test]
fn a_long_result_is_drawn_whole_and_costs_the_notes_after_it_nothing() {
    // A long result for wish A, A's work, then B's work with its three choices, on the small page: the result was
    // once left out so that the choices had room, and now every one of them is there.
    let viewport = Viewport { w: 960.0, h: 640.0 };
    let Note::Text { title, body, options, quoted, field, .. } = work() else { unreachable!() };
    let second = Note::Text { id: "work-2".into(), title, body, options, quoted, field };
    for theme in [Theme::sticker(), Theme::plain()] {
        let big = told((0..6).map(|n| row(&format!("Row {n}"), &[&words("w", 40)])).collect());
        let placed = place(vec![big, work(), second.clone()], &theme, viewport);
        let options = |id: &str| placed.iter().filter(|p| p.id == id && p.piece == PieceKind::Option).count();
        assert_eq!((options("work-1"), options("work-2")), (3, 3));
        assert_eq!(ours(&placed).iter().filter(|p| p.piece == PieceKind::Quote).count(), 6, "the result, whole");
        let cards: Vec<&Placed> = placed.iter().filter(|p| p.piece == PieceKind::Note).collect();
        assert_eq!(cards.iter().map(|card| card.id.as_str()).collect::<Vec<_>>(), [ID, "work-1", "work-2"]);
        assert!(cards.windows(2).all(|pair| pair[0].rect.y + pair[0].rect.h <= pair[1].rect.y), "each under the last");
    }
}

#[test]
fn rows_are_drawn_in_order_up_to_the_first_that_would_be_cut_and_none_after_it() {
    each(|theme, viewport| {
        let quotes = |of: &[Placed]| of.iter().filter(|p| p.id == ID && p.piece == PieceKind::Quote).count();
        let over = "x".repeat(sticker_kit::layout::MAX_TEXT_CHARS + 1);
        // The first row would be cut, so the short row after it is not drawn in its place.
        let placed = place(vec![told(vec![row("", &[&over]), row("Short", &["short"])])], theme, viewport);
        let sheet = ours(&placed).into_iter().find(|p| p.piece == PieceKind::Note).expect("the note is drawn");
        let own: Vec<&str> = sheet.text.iter().map(|run| run.text.as_str()).collect();
        assert_eq!(own, [TITLE, "+2 more", END[0], END[1]]);
        assert_eq!(quotes(&placed), 0, "a later row was drawn");
    });
}

#[test]
fn no_word_of_a_name_is_a_line_of_the_note_s_own_unless_the_label_and_it_are_that_one_line() {
    let long = "y".repeat(198);
    let sentence = "xxxx Jellymoon ran every test and all of them passed.md";
    let names = ["docs/notes Decided", sentence, long.as_str(), "src/shipping.rs", "a\u{200B}b.txt"];
    each(|theme, viewport| {
        for name in names {
            let mut file = row("New file", &["one line"]);
            file.name = name.into();
            let placed = place(vec![told(vec![file, row("Decided", &["kept"])])], theme, viewport);
            let sheet = ours(&placed).into_iter().find(|p| p.piece == PieceKind::Note).expect("the note");
            let labels: Vec<&str> =
                sheet.text.iter().filter(|run| run.role == Role::RowMain).map(|run| run.text.as_str()).collect();
            let one = format!("New file {name}");
            let plain = !name.contains([' ', '\u{200B}']) && name.len() < 40;
            for label in &labels {
                assert!(["New file", "Decided"].contains(label) || plain && *label == one, "{name}: {labels:?}");
            }
            let quotes: Vec<String> =
                ours(&placed).iter().filter(|p| p.piece == PieceKind::Quote).map(|quote| said(quote)).collect();
            let bare = |text: &str| text.replace([' ', '\u{200B}'], "");
            if labels.first() == Some(&"New file") {
                assert_eq!(bare(&quotes[0]), bare(name), "the name is the row's first quote, whole");
            }
        }
        // A short plain name shares the label's line.
        let mut file = row("Changed in", &["x = 1"]);
        file.name = "src/shipping.rs".into();
        let placed = place(vec![told(vec![file])], theme, viewport);
        let sheet = ours(&placed).into_iter().find(|p| p.piece == PieceKind::Note).expect("the note");
        assert!(sheet.text.iter().any(|run| run.text == "Changed in src/shipping.rs" && run.role == Role::RowMain));
    });
}
