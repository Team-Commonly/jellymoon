//! A text note: every text of its body is drawn as text, in order, or the first of them and a count of the rest.

use sticker_kit::{FixedAdvance, MAX_TEXT_LINES, Note, PieceKind, Placed, Scene, Theme, Viewport, layout};

fn place(notes: Vec<Note>, theme: &Theme, viewport: Viewport) -> Vec<Placed> {
    let scene = Scene { notes, ..Scene::sample() };
    layout(&scene, theme, viewport, &FixedAdvance(0.56))
}

fn note(body: Vec<String>, options: &[&str]) -> Note {
    let options = options.iter().map(|option| option.to_string()).collect();
    Note::Text { id: "file-1".into(), title: "A file".into(), body, options }
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
fn a_text_is_drawn_as_the_characters_it_holds_and_what_cannot_be_seen_is_left_out() {
    let odd = "<b>bold</b> \u{1b}[31mred\u{202e} [tap](https://example.test) $(touch ran)".to_string();
    let (texts, _) = said(&place(vec![note(vec![odd], &[])], &Theme::sticker(), Viewport { w: 1440.0, h: 900.0 }));
    let drawn = texts[1..].join(" ");
    assert!(drawn.starts_with("<b>bold</b>") && drawn.ends_with("[tap](https://example.test) $(touch ran)"), "{drawn}");
    assert!(!drawn.contains(['\u{1b}', '\u{202e}']), "{drawn:?}");
}
