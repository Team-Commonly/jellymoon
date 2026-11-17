//! A text note: every text of its body is drawn as text, in order, or the first of them and a count of the rest.
//! Its options are drawn only under a body drawn whole.

use sticker_kit::layout::MAX_TEXT_CHARS;
use sticker_kit::{
    FixedAdvance, MAX_TEXT_LINES, MORE_NOTES_ID, Note, PieceKind, Placed, Scene, Theme, Viewport, layout,
};

const SIZES: [Viewport; 2] = [Viewport { w: 1440.0, h: 900.0 }, Viewport { w: 960.0, h: 640.0 }];

fn place(notes: Vec<Note>, theme: &Theme, viewport: Viewport) -> Vec<Placed> {
    let scene = Scene { notes, ..Scene::sample() };
    layout(&scene, theme, viewport, &FixedAdvance(0.56))
}

fn note(body: Vec<String>, options: &[&str]) -> Note {
    let options = options.iter().map(|option| option.to_string()).collect();
    Note::Text { id: "file-1".into(), title: "A file".into(), body, options, quoted: String::new(), field: None }
}

/// The note's own texts, a run each, and its options' words.
fn said(placed: &[Placed]) -> (Vec<String>, Vec<String>) {
    let texts = |piece: &Placed| piece.text.iter().map(|run| run.text.clone()).collect::<Vec<_>>();
    let of = |kind: PieceKind| placed.iter().filter(move |piece| piece.piece == kind && piece.id == "file-1");
    (
        of(PieceKind::Note).flat_map(texts).collect(),
        of(PieceKind::Option).map(|option| texts(option).join(" ")).collect(),
    )
}

#[test]
fn a_text_note_draws_its_title_each_text_of_its_body_in_order_and_its_options() {
    let body = vec!["first line".to_string(), "second line".to_string(), "third".to_string()];
    for theme in [Theme::sticker(), Theme::plain()] {
        let placed = place(vec![note(body.clone(), &["Show me", "Keep it"])], &theme, Viewport { w: 1440.0, h: 900.0 });
        let (texts, options) = said(&placed);
        assert_eq!(texts, ["A file", "first line", "second line", "third"]);
        assert_eq!(options, ["Show me", "Keep it"]);
    }
}

#[test]
fn a_body_longer_than_its_room_is_drawn_from_its_start_and_the_rest_is_counted() {
    let body: Vec<String> = (1..=60).map(|n| format!("line {n}")).collect();
    for viewport in [Viewport { w: 1440.0, h: 900.0 }, Viewport { w: 960.0, h: 640.0 }] {
        let (texts, _) = said(&place(vec![note(body.clone(), &[])], &Theme::sticker(), viewport));
        let [title, drawn @ .., count] = &texts[..] else { panic!("{texts:?}") };
        assert_eq!(title, "A file");
        assert!(!drawn.is_empty() && drawn.len() < MAX_TEXT_LINES, "{}", drawn.len());
        assert_eq!(drawn, &body[..drawn.len()], "the first texts, whole and in order");
        assert_eq!(count, &format!("+{} more", body.len() - drawn.len()), "no text is hidden without the count");
    }
}

#[test]
fn a_body_that_does_not_fit_is_drawn_with_its_count_and_no_option() {
    let options = ["Show me", "Yes to those", "No"];
    for viewport in SIZES {
        // The most texts of a line each that are drawn whole with the options, and one more.
        let lines = |n: usize| (1..=n).map(|n| format!("name {n}")).collect::<Vec<_>>();
        let at = |n: usize| said(&place(vec![note(lines(n), &options)], &Theme::sticker(), viewport));
        let most = (1..=MAX_TEXT_LINES).rev().find(|n| !at(*n).1.is_empty()).expect("a short body has its options");
        assert!(most >= 3, "{most}");
        let (texts, placed) = at(most);
        assert_eq!((&texts[1..], placed.join("|")), (&lines(most)[..], options.join("|")), "whole, with every option");
        // One text more has no room with the options, and they are not drawn over a part of it.
        assert!(at(most + 1).1.is_empty(), "an option under a body of {} texts that had no room", most + 1);
        let (texts, placed) = at(60);
        assert!(placed.is_empty(), "an option under a body of 60 texts");
        let count = texts.last().unwrap();
        assert!(texts.len() < 60 && count == &format!("+{} more", 62 - texts.len()), "the rest is counted: {texts:?}");
    }
}

#[test]
fn a_text_that_is_cut_is_drawn_with_no_option_and_one_drawn_to_its_last_character_with_them() {
    let long = |chars: usize| "a".repeat(chars - 1) + "z";
    for viewport in SIZES {
        let at =
            |text: String| said(&place(vec![note(vec!["first".into(), text], &["Yes"])], &Theme::sticker(), viewport));
        let (texts, options) = at(long(MAX_TEXT_CHARS));
        assert_eq!(options, ["Yes"]);
        assert!(texts.last().unwrap().ends_with('z'), "drawn to its last character: {texts:?}");
        let (texts, options) = at(long(MAX_TEXT_CHARS + 1));
        assert!(options.is_empty(), "an option under a text that was cut");
        assert!(texts.last().unwrap().ends_with('…') && !texts.concat().contains('z'), "{texts:?}");
    }
}

#[test]
fn a_text_note_with_options_is_given_the_room_of_its_whole_body_before_the_notes_after_it() {
    let body: Vec<String> = (1..=6).map(|n| format!("name {n}")).collect();
    let plain = |n: usize| Note::Plain { title: format!("Plain {n}"), body: "words ".repeat(40) };
    for viewport in SIZES {
        let notes = vec![note(body.clone(), &["Yes", "No"]), plain(1), plain(2), plain(3), plain(4)];
        let placed = place(notes, &Theme::sticker(), viewport);
        let (texts, options) = said(&placed);
        assert_eq!((&texts[1..], &options[..]), (&body[..], &["Yes".to_string(), "No".to_string()][..]));
        // What it left no room for is counted, as notes are.
        assert!(placed.iter().any(|piece| piece.id == MORE_NOTES_ID), "the notes after it are counted");
    }
}

#[test]
fn a_title_that_is_cut_is_drawn_with_no_option_and_one_drawn_to_its_last_character_with_them() {
    // The first is where the gate's fuzz drew a cut title over placed options.
    let cases = [
        (Theme::plain(), Viewport { w: 2315.0, h: 610.0 }, 0.9),
        (Theme::sticker(), SIZES[0], 0.56),
        (Theme::sticker(), SIZES[1], 0.56),
    ];
    // A title over one text and one option: whether it was drawn whole, and the options.
    let under = |title: String, theme: &Theme, viewport: Viewport, advance: f32| {
        let (body, options) = (vec!["CLAUDE.md".to_string()], vec!["Yes".to_string()]);
        let note =
            Note::Text { id: "file-1".into(), title: title.clone(), body, options, quoted: String::new(), field: None };
        let scene = Scene { notes: vec![note], ..Scene::sample() };
        let (texts, options) = said(&layout(&scene, theme, viewport, &FixedAdvance(advance)));
        let [drawn @ .., text] = &texts[..] else { panic!("{texts:?}") };
        assert_eq!(text, "CLAUDE.md");
        (drawn.join(" ") == title, options)
    };
    for (theme, viewport, advance) in cases {
        let titled = |words: usize| under(vec!["word"; words].join(" "), &theme, viewport, advance);
        let most = (1..200).take_while(|words| titled(*words).0).count();
        assert!(most > 1, "{most}");
        assert_eq!(titled(most), (true, vec!["Yes".to_string()]), "a title drawn whole has its options");
        for words in [most + 1, most + 2, 150, 2000] {
            assert_eq!(titled(words), (false, Vec::new()), "a title of {words} words is cut, and has no option");
        }
    }
    // Letters so narrow that any title fits a line: one of more characters than a text is drawn with is cut.
    let long = |chars: usize| under("a".repeat(chars), &Theme::sticker(), SIZES[0], 0.001);
    assert_eq!(long(MAX_TEXT_CHARS), (true, vec!["Yes".to_string()]));
    assert_eq!(long(MAX_TEXT_CHARS + 1), (false, Vec::new()));
}

#[test]
fn a_text_is_drawn_as_the_characters_it_holds_and_what_cannot_be_seen_is_left_out() {
    let odd = "<b>bold</b> \u{1b}[31mred\u{202e} [tap](https://example.test) $(touch ran)".to_string();
    let (texts, _) = said(&place(vec![note(vec![odd], &[])], &Theme::sticker(), Viewport { w: 1440.0, h: 900.0 }));
    let drawn = texts[1..].join(" ");
    assert!(drawn.starts_with("<b>bold</b>") && drawn.ends_with("[tap](https://example.test) $(touch ran)"), "{drawn}");
    assert!(!drawn.contains(['\u{1b}', '\u{202e}']), "{drawn:?}");
}
