//! A picture drawn small under what a person typed (`Note::Said`'s `small`): one line under the bubble, at the
//! bubble's end, a sign and the picture's own captions and a chevron, as high as the smallest tap target. A
//! bubble with none is placed as it was before the word was there. No GPUI.

use sticker_kit::{
    FixedAdvance, Kept, Note, Picture, PieceKind, Placed, Rect, Role, Scene, Stage, Theme, Viewport, layout,
};

const SIZES: [Viewport; 2] = [Viewport { w: 1440.0, h: 900.0 }, Viewport { w: 960.0, h: 640.0 }];
/// The kit's smallest tap target, in design units: a quiet option's height, and the side of a picture's `?`.
const SMALLEST: f32 = 24.0;

fn sheets(made: usize, changed: usize, state: Kept) -> Option<Picture> {
    Some(Picture::Sheets { made, changed, removed: 0, state })
}

/// The sample's page with the words typed for its first small character, then a card: the bubble's note has
/// `small`, and `answer` under the typed words.
fn day(text: &str, answer: &str, small: Option<Picture>) -> Scene {
    let scene = Scene::sample();
    let said =
        Note::Said { id: format!("typed-{}", scene.characters[0].id), text: text.into(), answer: answer.into(), small };
    let card = Note::Plain { title: "What came of it".into(), body: "One line.".into() };
    Scene { notes: vec![said, card], ..scene }
}

fn place(scene: &Scene, theme: &Theme, viewport: Viewport) -> Vec<Placed> {
    layout(scene, theme, viewport, &FixedAdvance(0.56))
}

fn one(placed: &[Placed], kind: PieceKind) -> Option<&Placed> {
    placed.iter().find(|piece| piece.piece == kind && piece.id.starts_with("typed-"))
}

fn words(mark: &Placed) -> Vec<&str> {
    mark.text.iter().map(|run| run.text.as_str()).collect()
}

fn each(test: impl Fn(&Theme, Viewport, f32)) {
    for theme in [Theme::sticker(), Theme::plain()] {
        SIZES.iter().for_each(|size| test(&theme, *size, sticker_kit::layout::page_scale(&theme, *size)));
    }
}

const LONG: &str = "write our opening hours in twelve lines, one for each day and one for each of the holidays we keep";

#[test]
fn a_small_picture_is_one_line_under_the_bubble_at_its_end_as_high_as_the_smallest_tap_target() {
    each(|theme, viewport, k| {
        for (text, answer) in [("say hello", ""), (LONG, "Only seven, one per day")] {
            let placed = place(&day(text, answer, sheets(1, 0, Kept::In)), theme, viewport);
            let (bubble, mark) = (one(&placed, PieceKind::Quote).unwrap(), one(&placed, PieceKind::Small).unwrap());
            assert_eq!(mark.id, bubble.id, "it is the bubble's note's");
            assert!(mark.rect.y >= bubble.rect.bottom(), "under the bubble: {:?} {:?}", mark.rect, bubble.rect);
            assert!(mark.rect.y - bubble.rect.bottom() <= 6.0 * k, "and right under it");
            assert!((mark.rect.right() - bubble.rect.right()).abs() < 0.01, "it ends where the bubble ends");
            assert!(mark.rect.h >= SMALLEST * k - 0.01 && mark.rect.w >= SMALLEST * k, "a tap target: {:?}", mark.rect);
            assert_eq!(words(mark), ["1 file", "in your folder"], "the card's own captions, word for word");
            let line = mark.text[0].rect;
            assert!(mark.text.iter().all(|run| run.rect.y == line.y && run.rect.h == line.h), "one line");
            assert!(mark.text.iter().all(|run| run.role == Role::Caption && inside(run.rect, mark.rect)));
            assert!(mark.text[0].rect.right() < mark.text[1].rect.x, "the count, then where it is");
            // The sign stands before the words and the chevron after them, all inside the piece.
            let sketch = mark.sketch.as_ref().expect("its strokes");
            assert_eq!(sketch.what, "small sheets files=1");
            let names: Vec<&str> = sketch.parts.iter().map(|part| part.name.as_str()).collect();
            assert_eq!(names, ["folder solid", "tick", "opens"]);
            assert!(sketch.parts.iter().all(|part| inside(part.rect, mark.rect)), "{:?}", sketch.parts);
            assert!(sketch.parts[0].rect.right() <= line.x && sketch.parts[2].rect.x >= mark.text[1].rect.right());
            // It is over no other piece: the answer's hook is the bubble's, the character is at the bubble's end,
            // and the card comes under it.
            let hook = bubble.sketch.as_ref().map(|sketch| sketch.parts[0].rect);
            assert_eq!(hook.is_some(), !answer.is_empty());
            assert!(hook.is_none_or(|hook| inside(hook, bubble.rect) && !hook.overlaps(&mark.rect)));
            let others = placed.iter().filter(|piece| !matches!(piece.piece, PieceKind::Small | PieceKind::Paper));
            for other in others {
                assert!(!other.rect.overlaps(&mark.rect), "{:?} {} is over the mark", other.piece, other.id);
            }
            let card = placed.iter().find(|piece| piece.piece == PieceKind::Note).unwrap();
            assert!(card.rect.y >= mark.rect.bottom(), "the card stands under the mark");
        }
    });
}

fn inside(inner: Rect, outer: Rect) -> bool {
    let near = 0.01;
    inner.x >= outer.x - near
        && inner.y >= outer.y - near
        && inner.right() <= outer.right() + near
        && inner.bottom() <= outer.bottom() + near
}

#[test]
fn each_state_of_finished_work_has_its_sign_and_the_card_s_words_and_work_thrown_away_is_quiet() {
    let theme = Theme::sticker();
    let drawn = |small: Option<Picture>| {
        let placed = place(&day("say hello", "", small), &theme, SIZES[0]);
        let mark = one(&placed, PieceKind::Small).expect("a small picture").clone();
        let sketch = mark.sketch.clone().expect("its strokes");
        let parts: Vec<String> = sketch.parts.iter().map(|part| part.name.clone()).collect();
        let quiet = sketch.strokes.iter().all(|stroke| stroke.ink == sticker_kit::Ink::Quiet);
        (words(&mark).join(" | "), parts.join(" "), mark.text.iter().all(|run| run.role == Role::Footer) && quiet)
    };
    let said = |words: &str, parts: &str, quiet: bool| (words.to_string(), parts.to_string(), quiet);
    assert_eq!(drawn(sheets(2, 1, Kept::In)), said("3 files | in your folder", "folder solid tick opens", false));
    assert_eq!(
        drawn(sheets(1, 0, Kept::Waiting)),
        said("1 file | not in your folder yet", "folder dashed opens", false)
    );
    assert_eq!(
        drawn(sheets(1, 0, Kept::Unfit)),
        said("1 file | does not fit your folder", "folder dashed opens", false)
    );
    assert_eq!(drawn(sheets(0, 0, Kept::In)), said("nothing changed", "sheet empty opens", false));
    // Thrown away: no folder and no tick, whatever the count, and nothing of it in the pencil's own ink.
    assert_eq!(drawn(sheets(0, 2, Kept::Tossed)), said("2 files | thrown away", "sheet empty cross opens", true));
    assert_eq!(drawn(sheets(0, 0, Kept::Tossed)), said("thrown away", "sheet empty cross opens", true));
}

// The word is optional: a note with none is the note it was, and its page is placed piece for piece as before.
#[test]
fn a_bubble_with_no_small_picture_is_placed_as_it_was_and_one_with_it_moves_only_what_is_under_it() {
    let before = "type = \"said\"\nid = \"typed-x\"\ntext = \"my words\"\n";
    let read: Note = toml::from_str(before).expect("a note written before the word was there");
    assert_eq!(read, Note::Said { id: "typed-x".into(), text: "my words".into(), answer: String::new(), small: None });
    assert_eq!(toml::to_string(&read).expect("it is written"), before, "and it is written as it was");
    each(|theme, viewport, _| {
        for (text, answer) in [("say hello", ""), (LONG, "Only seven, one per day")] {
            let plain = place(&day(text, answer, None), theme, viewport);
            assert!(one(&plain, PieceKind::Small).is_none(), "no picture, no piece");
            // A picture with no small form is no picture here.
            let track = Picture::Track { steps: 3, stage: Stage::Making, file: String::new() };
            assert_eq!(place(&day(text, answer, Some(track)), theme, viewport), plain);
            // With one, every piece down to the bubble is where it was, and nothing but what is under it moved.
            let with = place(&day(text, answer, sheets(1, 0, Kept::In)), theme, viewport);
            let bubble = one(&plain, PieceKind::Quote).unwrap().rect;
            assert_eq!(with.iter().filter(|piece| piece.piece == PieceKind::Small).count(), 1);
            for was in plain.iter().filter(|was| was.rect.bottom() <= bubble.bottom()) {
                assert!(with.contains(was), "{:?} {} moved or changed: {:?}", was.piece, was.id, was.rect);
            }
        }
    });
}
