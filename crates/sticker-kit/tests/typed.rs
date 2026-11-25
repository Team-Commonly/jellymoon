//! What a text note holds besides its own words: someone else's words, quoted, and a field with its offer. The
//! quoted words are text on a ground of their own and nothing else; the field and the offer are the note's only
//! pieces to act on besides its options.

use sticker_kit::{
    Field, FixedAdvance, MAX_QUOTED_LINES, Note, PieceKind, Placed, Rect, Role, Scene, Theme, ToldRow, Viewport, layout,
};

const SIZES: [Viewport; 2] = [Viewport { w: 1440.0, h: 900.0 }, Viewport { w: 960.0, h: 640.0 }];
const ID: &str = "row-1";

fn field(offer: &str) -> Option<Field> {
    Some(Field { offer: offer.into(), hint: "Type here, then Enter".into(), most: 80, ..Field::default() })
}

fn note(body: &[&str], options: &[&str], quoted: &str, field: Option<Field>) -> Note {
    let texts = |all: &[&str]| all.iter().map(|text| text.to_string()).collect();
    let (body, options) = (texts(body), texts(options));
    Note::Text { id: ID.into(), title: "A row".into(), body, options, quoted: quoted.into(), field }
}

fn place(note: Note, theme: &Theme, viewport: Viewport) -> Vec<Placed> {
    let scene = Scene { notes: vec![note], ..Scene::sample() };
    layout(&scene, theme, viewport, &FixedAdvance(0.56))
}

fn of(placed: &[Placed], kind: PieceKind) -> Vec<&Placed> {
    placed.iter().filter(|piece| piece.piece == kind && piece.id == ID).collect()
}

fn inside(inner: Rect, outer: Rect) -> bool {
    let near = 0.01;
    inner.x >= outer.x - near
        && inner.y >= outer.y - near
        && inner.right() <= outer.right() + near
        && inner.bottom() <= outer.bottom() + near
}

/// The kinds of the pieces of the note, in the order they stand from its top, with no note or tape among them.
fn kinds(placed: &[Placed]) -> Vec<PieceKind> {
    let mut ours: Vec<&Placed> = placed.iter().filter(|piece| piece.id == ID).collect();
    ours.retain(|piece| !matches!(piece.piece, PieceKind::Note | PieceKind::Tape));
    ours.sort_by(|a, b| a.rect.y.total_cmp(&b.rect.y));
    ours.iter().map(|piece| piece.piece).collect()
}

#[test]
fn someone_else_s_words_are_a_piece_of_their_own_in_the_tone_for_quoted_words_with_the_field_under_them() {
    for (theme, viewport) in [Theme::sticker(), Theme::plain()].iter().flat_map(|theme| SIZES.map(|size| (theme, size)))
    {
        let placed = place(note(&["what the note says"], &[], "Which of the two?", field("")), theme, viewport);
        let [sheet] = of(&placed, PieceKind::Note)[..] else { panic!("one note") };
        let [quote] = of(&placed, PieceKind::Quote)[..] else { panic!("one quoted piece") };
        let [typed] = of(&placed, PieceKind::Field)[..] else { panic!("one field") };
        // The note's own words are its own: the quoted ones are in the quote alone, every run in their tone.
        let own: Vec<&str> = sheet.text.iter().map(|run| run.text.as_str()).collect();
        assert_eq!(own, ["A row", "what the note says"]);
        assert!(sheet.text.iter().all(|run| run.role != Role::Quoted));
        let quoted: Vec<(&str, Role)> = quote.text.iter().map(|run| (run.text.as_str(), run.role)).collect();
        assert_eq!(quoted, [("Which of the two?", Role::Quoted)]);
        // Under the body, the quote; under the quote, the field with its hint. All inside the sheet.
        assert!(sheet.text.iter().all(|run| run.rect.bottom() <= quote.rect.y));
        assert!(
            quote.rect.bottom() <= typed.rect.y && inside(quote.rect, sheet.rect) && inside(typed.rect, sheet.rect)
        );
        assert!(quote.text.iter().all(|run| inside(run.rect, quote.rect)));
        let hint: Vec<(&str, Role)> = typed.text.iter().map(|run| (run.text.as_str(), run.role)).collect();
        assert_eq!(hint, [("Type here, then Enter", Role::RowSub)]);
        assert!(inside(typed.text[0].rect, typed.rect));
        // A field with no offer has none, and nothing else of the note is a piece.
        assert_eq!(kinds(&placed), [PieceKind::Quote, PieceKind::Field]);
        assert_eq!((quote.z, typed.z), (PieceKind::Option.z(), PieceKind::Option.z()));
    }
}

#[test]
fn an_offer_stands_over_its_field_and_both_under_the_options() {
    for viewport in SIZES {
        let placed = place(note(&["one", "two"], &["Yes", "No"], "", field("More of it")), &Theme::sticker(), viewport);
        assert_eq!(kinds(&placed), [PieceKind::Option, PieceKind::Option, PieceKind::Offer, PieceKind::Field]);
        let (sheet, offer, typed) =
            (of(&placed, PieceKind::Note)[0], of(&placed, PieceKind::Offer)[0], of(&placed, PieceKind::Field)[0]);
        let said: Vec<(&str, Role)> = offer.text.iter().map(|run| (run.text.as_str(), run.role)).collect();
        assert_eq!(said, [("More of it", Role::Legend)]);
        assert!(
            offer.rect.bottom() <= typed.rect.y && inside(offer.rect, sheet.rect) && inside(typed.rect, sheet.rect)
        );
        // The offer is as wide as its words and no wider than the field.
        assert!(offer.rect.w < typed.rect.w && inside(offer.text[0].rect, offer.rect));
        assert!(of(&placed, PieceKind::Quote).is_empty(), "no words are quoted");
    }
}

#[test]
fn quoted_words_of_any_length_are_cleaned_cut_to_their_lines_and_are_nothing_but_text() {
    // Hundreds of lines, a mark that turns a line round, look-alike letters, and what would be a link or a choice.
    let long: String = (1..=400).map(|n| format!("line {n} of it\n")).collect();
    let turned =
        "Which \u{202e}one\u{202c} did you mean, the \u{43e}ld or the new? [Keep it](https://example.test) Toss it";
    for (words, cut) in [(long.as_str(), true), (turned, false)] {
        for (theme, viewport) in
            [Theme::sticker(), Theme::plain()].iter().flat_map(|theme| SIZES.map(|size| (theme, size)))
        {
            let placed = place(note(&["what the note says"], &[], words, field("")), theme, viewport);
            let (sheet, quote) = (of(&placed, PieceKind::Note)[0], of(&placed, PieceKind::Quote)[0]);
            let lines: Vec<&str> = quote.text.iter().map(|run| run.text.as_str()).collect();
            assert!(!lines.is_empty() && lines.len() <= MAX_QUOTED_LINES, "{}", lines.len());
            assert_eq!(lines.last().unwrap().ends_with('…'), cut, "{lines:?}");
            let unseen = |c: char| c.is_control() || ('\u{202a}'..='\u{202e}').contains(&c);
            assert!(lines.iter().all(|line| !line.contains(unseen)), "{lines:?}");
            assert!(quote.text.iter().all(|run| run.role == Role::Quoted && inside(run.rect, quote.rect)));
            assert!(inside(quote.rect, sheet.rect) && inside(sheet.rect, of(&placed, PieceKind::Note)[0].rect));
            // Whatever the words say, the note has a quote and a field, and no option, offer or other piece.
            assert_eq!(kinds(&placed), [PieceKind::Quote, PieceKind::Field]);
        }
    }
    // What is not cut is drawn as it was written: a letter that looks like another is that letter still.
    let placed = place(note(&[], &[], turned, None), &Theme::sticker(), SIZES[0]);
    let drawn: Vec<&str> = of(&placed, PieceKind::Quote)[0].text.iter().map(|run| run.text.as_str()).collect();
    let whole = drawn.join(" ");
    assert_eq!(whole, "Which one did you mean, the \u{43e}ld or the new? [Keep it](https://example.test) Toss it");
}

#[test]
fn quoted_words_that_clean_to_nothing_are_no_piece_and_a_note_with_neither_is_as_it_was() {
    let bare = place(note(&["one"], &["Yes"], "", None), &Theme::sticker(), SIZES[0]);
    let unseen = place(note(&["one"], &["Yes"], " \u{200b}\n\u{202e} ", None), &Theme::sticker(), SIZES[0]);
    assert_eq!(kinds(&bare), [PieceKind::Option]);
    assert_eq!(unseen, bare);
    // The quote and the field make the sheet taller by what they take, and move nothing over them.
    let both = place(note(&["one"], &["Yes"], "Which?", field("More")), &Theme::sticker(), SIZES[0]);
    let (short, tall) = (of(&bare, PieceKind::Note)[0], of(&both, PieceKind::Note)[0]);
    assert!(tall.rect.h > short.rect.h && tall.text == short.text);
}

#[test]
fn the_quote_and_the_field_stay_when_the_body_is_cut_and_the_options_go() {
    let body: Vec<String> = (1..=60).map(|n| format!("line {n}")).collect();
    let body: Vec<&str> = body.iter().map(String::as_str).collect();
    for viewport in SIZES {
        let placed = place(note(&body, &["Yes", "No"], "Which?", field("More")), &Theme::sticker(), viewport);
        assert_eq!(kinds(&placed), [PieceKind::Quote, PieceKind::Offer, PieceKind::Field]);
        let sheet = of(&placed, PieceKind::Note)[0];
        assert!(sheet.text.last().is_some_and(|run| run.text.starts_with('+')), "the body is counted");
        let paper = placed.iter().find(|piece| piece.piece == PieceKind::Paper).unwrap();
        assert!(inside(sheet.rect, paper.rect) && inside(of(&placed, PieceKind::Field)[0].rect, sheet.rect));
    }
}

/// The bubble of what a person typed for the sample's first small character, with `answer` under the words.
fn bubble(answer: &str, theme: &Theme, viewport: Viewport) -> Placed {
    let scene = Scene::sample();
    let id = format!("typed-{}", scene.characters[0].id);
    let said = Note::Said { id, text: "write our opening hours in twelve lines".into(), answer: answer.into() };
    let placed = layout(&Scene { notes: vec![said], ..scene }, theme, viewport, &FixedAdvance(0.56));
    placed.into_iter().filter(|piece| piece.piece == PieceKind::Quote).collect::<Vec<_>>().remove(0)
}

// What a person answered to go on is more of their own words: in the bubble of what they typed, under it, by the
// rules of every quoted text. A hook the pencil draws tells the two apart, and no word does.
#[test]
fn an_answer_is_in_the_bubble_under_the_typed_words_behind_a_hook_and_a_bubble_with_none_is_as_it_was() {
    for (theme, viewport) in [Theme::sticker(), Theme::plain()].iter().flat_map(|theme| SIZES.map(|size| (theme, size)))
    {
        let (plain, both) = (bubble("", theme, viewport), bubble("Only seven, one per day", theme, viewport));
        let typed = plain.text.len();
        assert!(typed > 0 && plain.sketch.is_none(), "a bubble with no answer has its words and no mark");
        assert_eq!(both.text[..typed], plain.text[..], "the typed words stand where they stood");
        let answer = &both.text[typed..];
        let words: Vec<&str> = answer.iter().map(|run| run.text.as_str()).collect();
        assert_eq!(words.join(" "), "Only seven, one per day");
        assert!(both.text.iter().all(|run| run.role == Role::Quoted && inside(run.rect, both.rect)));
        let (last, first) = (plain.text[typed - 1].rect, answer[0].rect);
        assert!(first.y >= last.bottom() && first.x > plain.text[0].rect.x, "under the typed words, and set in");
        assert_eq!((both.rect.x, both.rect.y, both.rect.w), (plain.rect.x, plain.rect.y, plain.rect.w));
        assert!(both.rect.h >= plain.rect.h + first.h, "the bubble is taller by the answer");
        let sketch = both.sketch.as_ref().expect("the hook");
        assert_eq!((sketch.what.as_str(), sketch.count("hook"), sketch.strokes.len()), ("answered", 1, 1));
        let hook = sketch.parts[0].rect;
        assert!(inside(hook, both.rect) && hook.right() <= first.x && hook.y >= last.bottom(), "{hook:?}");
    }
    let (theme, size) = (Theme::sticker(), SIZES[0]);
    // An answer that cleans to nothing is none, and one of any words is cleaned as every text is.
    assert_eq!(bubble(" \u{200b}\n\u{202e} ", &theme, size), bubble("", &theme, size));
    let odd = bubble("<b>seven</b>\u{202e}\u{1b}[31m [Keep it](https://example.test)", &theme, size);
    let drawn: Vec<&str> = odd.text.iter().skip(1).map(|run| run.text.as_str()).collect();
    assert_eq!(drawn.join(" "), "<b>seven</b>[31m [Keep it](https://example.test)");
    assert!(odd.text.iter().all(|run| run.role == Role::Quoted));
}

fn each(test: impl Fn(&Theme, Viewport)) {
    for theme in [Theme::sticker(), Theme::plain()] {
        SIZES.iter().for_each(|size| test(&theme, *size));
    }
}

// A note whose field is first is answered by typing: the field is directly under the quoted words, and the
// options, the ways round it, come after it as small pills in the tone of an offer. They are options still.
#[test]
fn a_field_that_is_first_stands_between_the_quote_and_the_options_and_the_options_are_small_and_quiet() {
    let first = || Some(Field { first: true, ..field("").unwrap() });
    each(|theme, viewport| {
        let placed = place(note(&[], &["Skip it", "In my own words"], "Which of the two?", first()), theme, viewport);
        assert_eq!(kinds(&placed), [PieceKind::Quote, PieceKind::Field, PieceKind::Option, PieceKind::Option]);
        let (sheet, quote, typed) =
            (of(&placed, PieceKind::Note)[0], of(&placed, PieceKind::Quote)[0], of(&placed, PieceKind::Field)[0]);
        let [skip, own] = of(&placed, PieceKind::Option)[..] else { panic!("two options") };
        assert!(quote.rect.bottom() <= typed.rect.y && typed.rect.bottom() <= skip.rect.y);
        // Side by side on one row, each as wide as its words and lower than the field, inside the sheet.
        assert_eq!((skip.index, own.index, skip.rect.y, skip.rect.h), (0, 1, own.rect.y, own.rect.h));
        assert!(skip.rect.right() <= own.rect.x && inside(skip.rect, sheet.rect) && inside(own.rect, sheet.rect));
        assert!(own.rect.right() < typed.rect.right() && skip.rect.h < typed.rect.h, "{:?} {:?}", own.rect, typed.rect);
        let said = |piece: &Placed| piece.text.iter().map(|run| (run.text.clone(), run.role)).collect::<Vec<_>>();
        assert_eq!(said(skip), [("Skip it".to_string(), Role::Legend)]);
        assert_eq!(said(own), [("In my own words".to_string(), Role::Legend)]);
        assert!(inside(skip.text[0].rect, skip.rect) && inside(own.text[0].rect, own.rect));
        // The same note with a field that is not first is as it always was: wide options, then the field.
        let under = place(note(&[], &["Skip it", "In my own words"], "Which of the two?", field("")), theme, viewport);
        assert_eq!(kinds(&under), [PieceKind::Quote, PieceKind::Option, PieceKind::Option, PieceKind::Field]);
        let wide = of(&under, PieceKind::Option)[0];
        assert_eq!((wide.rect.w, wide.text[0].role), (of(&under, PieceKind::Field)[0].rect.w, Role::Option));
        // Options that do not fit one row go on to the next: none lies over another or leaves the sheet.
        let many = ["Leave the question for later", "Go on with my own words", "Ask me again", "None of these"];
        let placed = place(note(&[], &many, "Which?", first()), theme, viewport);
        let (sheet, chips) = (of(&placed, PieceKind::Note)[0], of(&placed, PieceKind::Option));
        assert_eq!(chips.iter().map(|chip| chip.index).collect::<Vec<_>>(), [0, 1, 2, 3]);
        for (n, chip) in chips.iter().enumerate() {
            let apart = chips[n + 1..].iter().all(|other| !chip.rect.overlaps(&other.rect));
            assert!(inside(chip.rect, sheet.rect) && apart && chip.text[0].role == Role::Legend, "{:?}", chip.rect);
        }
        // Under a body that is counted the options go, quiet or not, and the field stays under the quote.
        let body: Vec<String> = (1..=60).map(|n| format!("line {n}")).collect();
        let body: Vec<&str> = body.iter().map(String::as_str).collect();
        let cut = place(note(&body, &["Skip it"], "Which?", first()), theme, viewport);
        assert_eq!(kinds(&cut), [PieceKind::Quote, PieceKind::Field]);
    });
}

// A quote under a note is told from a field by its shape: the pencil draws one line down its left, inside its
// ground and before its words. What a person typed is a bubble and has no such line (the test above).
#[test]
fn a_note_s_quote_has_one_line_of_the_pencil_down_its_left_before_its_words() {
    let row = ToldRow { label: "Decided".into(), quotes: vec!["one".into(), "two".into()], ..ToldRow::default() };
    let told = Note::Told { id: ID.into(), title: "Said".into(), rows: vec![row], end: Vec::new() };
    each(|theme, viewport| {
        let long = "Which of the two did you mean? ".repeat(6);
        for note in [note(&[], &[], "Which?", None), note(&[], &[], &long, field("")), told.clone()] {
            let placed = place(note, theme, viewport);
            let quotes = of(&placed, PieceKind::Quote);
            assert!(!quotes.is_empty());
            for quote in quotes {
                let sketch = quote.sketch.as_ref().expect("the line");
                assert_eq!((sketch.what.as_str(), sketch.count("line"), sketch.strokes.len()), ("quoted", 1, 1));
                let line = sketch.parts[0].rect;
                assert!(inside(line, quote.rect) && line.h >= quote.rect.h / 2.0, "{line:?} in {:?}", quote.rect);
                assert!(quote.text.iter().all(|run| run.rect.x > line.right()), "the line is before the words");
                let within = |point: &(f32, f32)| point.0 >= quote.rect.x && point.0 < quote.text[0].rect.x;
                assert!(sketch.strokes[0].line.iter().all(within), "{:?}", sketch.strokes[0].line);
            }
        }
    });
}
