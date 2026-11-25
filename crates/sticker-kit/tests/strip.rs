//! A strip (`Picture::Strip`): up to three small sketches in one row, an arrow from each to the next and each
//! caption under its sketch. It is a picture and nothing else: its note has no `?`, and it draws itself by the
//! rule every picture does. That a click on it is no tap is the view's, in `tests/column_draw.rs`. No GPUI.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use sticker_kit::pencil::{DRAWN_IN, REACH};
use sticker_kit::{
    FixedAdvance, Ink, Kept, MAX_STRIP_PANELS, Note, Panel, Picture, PieceKind, Placed, Rect, Role, Scene, Shows,
    Theme, Viewport, layout,
};

/// The design size, the smallest window the kit is laid out for, and one far under it.
const SIZES: [Viewport; 3] =
    [Viewport { w: 1440.0, h: 900.0 }, Viewport { w: 960.0, h: 640.0 }, Viewport { w: 520.0, h: 400.0 }];
const SAYS: [&str; 3] = ["ask for it", "it is made", "keep or drop"];
const WORDS: &str = "Made up: begin at the bottom.";

fn panel(shows: Shows, caption: &str) -> Panel {
    Panel { shows, caption: caption.into() }
}

fn three() -> Vec<Panel> {
    [Shows::Speech, Shows::Sheet, Shows::Folder].into_iter().zip(SAYS).map(|(shows, says)| panel(shows, says)).collect()
}

fn strip(panels: Vec<Panel>, open: bool) -> Note {
    let picture = Picture::Strip { panels };
    Note::Drawn { id: "how".into(), picture, words: vec![WORDS.into()], open, options: Vec::new(), field: None }
}

/// The sample's page with two cards of its own, and `last` after them.
fn day(last: Option<Note>) -> Scene {
    let card = |title: &str| Note::Plain { title: title.into(), body: "One line.".into() };
    let notes = [card("first"), card("second")].into_iter().chain(last).collect();
    Scene { notes, footer: String::new(), ..Scene::sample() }
}

fn each(test: impl Fn(&Theme, Viewport)) {
    for theme in [Theme::sticker(), Theme::plain()] {
        SIZES.iter().for_each(|size| test(&theme, *size));
    }
}

fn place(scene: &Scene, theme: &Theme, viewport: Viewport) -> Vec<Placed> {
    layout(scene, theme, viewport, &FixedAdvance(0.56))
}

fn picture(placed: &[Placed]) -> &Placed {
    let own = |piece: &&Placed| piece.piece == PieceKind::Picture && piece.id == "how";
    placed.iter().find(own).unwrap_or_else(|| panic!("the strip's picture"))
}

fn parts(picture: &Placed) -> Vec<(&str, Rect)> {
    let sketch = picture.sketch.as_ref().unwrap_or_else(|| panic!("a sketch"));
    sketch.parts.iter().map(|part| (part.name.as_str(), part.rect)).collect()
}

/// The room of the sketch whose parts are named `names`: from its leftmost side to its rightmost, top to bottom.
fn sketch(picture: &Placed, names: &[&str]) -> Rect {
    let own: Vec<Rect> =
        parts(picture).into_iter().filter(|(name, _)| names.contains(name)).map(|part| part.1).collect();
    assert!(!own.is_empty(), "no part of {names:?}");
    let low = |pick: fn(&Rect) -> f32| own.iter().map(pick).fold(f32::INFINITY, f32::min);
    let high = |pick: fn(&Rect) -> f32| own.iter().map(pick).fold(f32::NEG_INFINITY, f32::max);
    let (x, y) = (low(|rect| rect.x), low(|rect| rect.y));
    Rect::new(x, y, high(|rect| rect.right()) - x, high(|rect| rect.bottom()) - y)
}

const SKETCHES: [&[&str]; 3] = [&["speech"], &["sheet", "pencil"], &["folder solid", "tick", "cross"]];

#[test]
fn a_strip_places_its_sketches_left_to_right_with_an_arrow_between_neighbours_and_each_caption_under_its_sketch() {
    each(|theme, viewport| {
        let placed = place(&day(Some(strip(three(), true))), theme, viewport);
        let picture = picture(&placed);
        let names: Vec<&str> = parts(picture).into_iter().map(|(name, _)| name).collect();
        let want = ["speech", "arrow", "sheet", "pencil", "arrow", "folder solid", "tick", "cross"];
        assert_eq!(names, want, "each sketch after the arrow that points at it");
        let rooms = SKETCHES.map(|names| sketch(picture, names));
        let arrows: Vec<Rect> =
            parts(picture).into_iter().filter(|(name, _)| *name == "arrow").map(|part| part.1).collect();
        for (n, arrow) in arrows.iter().enumerate() {
            assert!(rooms[n].right() < arrow.x && arrow.right() < rooms[n + 1].x, "{n}: {arrow:?} {rooms:?}");
            assert!(rooms[n].y < arrow.center().1 && arrow.center().1 < rooms[n].bottom(), "level with the sketches");
        }
        // One row: every sketch shares a band of the picture's height with every other.
        let (top, bottom) = (rooms.map(|room| room.y), rooms.map(|room| room.bottom()));
        assert!(top.into_iter().fold(0.0, f32::max) < bottom.into_iter().fold(f32::INFINITY, f32::min), "{rooms:?}");
        let captions: Vec<_> = picture.text.iter().filter(|run| run.role == Role::Caption).collect();
        assert_eq!(captions.iter().map(|run| run.text.as_str()).collect::<Vec<_>>(), SAYS, "{viewport:?}");
        for (run, room) in captions.iter().zip(rooms) {
            let mid = run.rect.center().0;
            assert!(room.x < mid && mid < room.right(), "{}: under its own sketch: {:?} {room:?}", run.text, run.rect);
            assert!(run.rect.y >= room.bottom(), "{}: {:?} {room:?}", run.text, run.rect);
        }
        assert_eq!(picture.sketch.as_ref().unwrap().what, "strip panels=3");
    });
}

#[test]
fn a_strip_draws_three_panels_at_most_and_one_panel_has_no_arrow() {
    each(|theme, viewport| {
        let count = |panels: Vec<Panel>| {
            let placed = place(&day(Some(strip(panels, false))), theme, viewport);
            let picture = picture(&placed);
            let of = |name: &str| parts(picture).iter().filter(|(part, _)| *part == name).count();
            (of("speech"), of("sheet"), of("folder solid"), of("arrow"), picture.text.len())
        };
        let more = three().into_iter().chain([panel(Shows::Speech, "a fourth"), panel(Shows::Speech, "a fifth")]);
        assert_eq!(MAX_STRIP_PANELS, 3);
        assert_eq!(count(more.collect()), (1, 1, 1, 2, 3), "the panels past the third are not drawn");
        assert_eq!(count(three()), (1, 1, 1, 2, 3));
        assert_eq!(count(vec![panel(Shows::Folder, "alone")]), (0, 0, 1, 0, 1));
        assert_eq!(count(Vec::new()), (0, 0, 0, 0, 0), "a strip of nothing is an empty picture");
    });
}

// Every length of a strip is the column's design unit: a narrow window draws the same row smaller. A caption
// that does not fit its panel wraps, in two lines at most, and what is left over ends in the kit's ellipsis.
#[test]
fn a_strip_stays_on_one_row_in_the_narrowest_column_and_a_caption_too_long_wraps_and_ends_in_an_ellipsis() {
    each(|theme, viewport| {
        let long = "a caption that is much too long to stand under one small sketch of a strip\u{7} in any window";
        let panels = vec![panel(Shows::Speech, SAYS[0]), panel(Shows::Sheet, long), panel(Shows::Folder, SAYS[2])];
        let placed = place(&day(Some(strip(panels, true))), theme, viewport);
        let picture = picture(&placed);
        let card = placed.iter().find(|piece| piece.piece == PieceKind::Note && piece.id == "how").expect("its card");
        assert!(picture.rect.inside(&card.rect), "{:?} {:?}", picture.rect, card.rect);
        let rooms = SKETCHES.map(|names| sketch(picture, names));
        assert!(rooms.iter().all(|room| room.inside(&picture.rect)), "{viewport:?}: {rooms:?} {:?}", picture.rect);
        assert!(rooms.windows(2).all(|pair| pair[0].right() < pair[1].x), "no sketch over its neighbour: {rooms:?}");
        let third = picture.rect.w / 3.0;
        let within = |n: usize| {
            let (left, right) = (picture.rect.x + third * n as f32, picture.rect.x + third * (n + 1) as f32);
            let own = move |run: &&sticker_kit::TextRun| left <= run.rect.x && run.rect.right() <= right + 0.01;
            picture.text.iter().filter(own).map(|run| run.text.as_str()).collect::<Vec<&str>>()
        };
        assert_eq!((within(0), within(2)), (vec![SAYS[0]], vec![SAYS[2]]), "{viewport:?}: each in its own third");
        let wrapped = within(1);
        assert_eq!(wrapped.len(), 2, "two lines at most: {wrapped:?}");
        assert!(wrapped[0].starts_with("a caption") && wrapped[1].ends_with('…'), "{wrapped:?}");
        assert!(!wrapped.concat().contains('\u{7}'), "cleaned as every caption is");
        assert_eq!(picture.text.len(), 4, "and no run of it anywhere else");
        // Nothing of it is out of its picture by more than the pencil's reach, as of every picture.
        let room = picture.rect.grown(REACH);
        let strokes = &picture.sketch.as_ref().unwrap().strokes;
        let points = strokes.iter().flat_map(|stroke| stroke.line.iter().chain(&stroke.soft));
        assert!(points.clone().all(|at| Rect::new(at.0, at.1, 0.0, 0.0).inside(&room)), "{viewport:?}");
        assert!(picture.text.iter().all(|run| run.rect.inside(&room)), "{viewport:?}: a caption is out of it");
    });
}

// A picture that takes no tap has no mark that would take one, open or closed, and no option. Its note's words
// are drawn while the scene says the note is open, as under any picture.
#[test]
fn a_strip_s_note_has_no_mark_and_its_words_are_drawn_under_it_while_it_is_open() {
    assert!(!Picture::Strip { panels: three() }.taps());
    assert!(Picture::Sheets { made: 1, changed: 0, removed: 0, state: Kept::In }.taps(), "every other picture does");
    each(|theme, viewport| {
        for open in [true, false] {
            let placed = place(&day(Some(strip(three(), open))), theme, viewport);
            let own: Vec<&Placed> = placed.iter().filter(|piece| piece.id == "how").collect();
            let kinds: Vec<PieceKind> = own.iter().map(|piece| piece.piece).collect();
            assert!(kinds.contains(&PieceKind::Picture) && kinds.contains(&PieceKind::Note), "{kinds:?}");
            assert!(!kinds.contains(&PieceKind::Ask) && !kinds.contains(&PieceKind::Option), "{open}: {kinds:?}");
            let card = own.iter().find(|piece| piece.piece == PieceKind::Note).unwrap();
            let said = card.text.iter().map(|run| run.text.as_str()).collect::<Vec<&str>>().join(" ");
            assert_eq!(said, if open { WORDS } else { "" }, "{viewport:?}");
            let under = card.text.iter().all(|run| run.rect.y >= picture(&placed).rect.bottom());
            assert!(under, "the words stand under the picture");
        }
    });
}

#[test]
fn a_scene_without_a_strip_is_placed_as_it_was_and_one_with_it_last_changes_no_piece_before_it() {
    each(|theme, viewport| {
        // The paper is as long as its column and its holes go down it: a longer column has more of them.
        let place = |scene: &Scene| {
            let mut placed = place(scene, theme, viewport);
            placed.retain(|piece| !matches!(piece.piece, PieceKind::Paper | PieceKind::Hole));
            placed
        };
        let (without, with) = (place(&day(None)), place(&day(Some(strip(three(), true)))));
        assert!(!without.iter().any(|piece| piece.piece == PieceKind::Picture || piece.id == "how"));
        let rest: Vec<Placed> = with.into_iter().filter(|piece| piece.id != "how").collect();
        assert_eq!(rest, without, "{viewport:?}");
    });
}

// The rule every picture draws itself by: stroke after stroke, in the pencil's time at most. Nothing of a
// strip rocks once it is drawn: no run is at work in it. The tick has the added ink and the cross the taken one.
#[test]
fn a_strip_draws_itself_stroke_after_stroke_in_the_pencil_s_time_and_nothing_of_it_rocks() {
    each(|theme, viewport| {
        let placed = place(&day(Some(strip(three(), true))), theme, viewport);
        let sketch = picture(&placed).sketch.clone().unwrap();
        let seconds = sketch.seconds();
        assert!(seconds > 0.0 && seconds <= DRAWN_IN + 0.001, "{seconds}");
        let turns = sketch.strokes.windows(2).all(|pair| (pair[0].start + pair[0].length - pair[1].start).abs() < 1e-3);
        assert!(turns, "each stroke begins where the one before it ended");
        assert!(sketch.strokes.iter().all(|stroke| stroke.part(Some(0.0)) == 0.0), "no ink at no time");
        assert!(sketch.strokes.iter().all(|stroke| stroke.part(None) == 1.0), "whole with nothing moving");
        assert!(sketch.strokes.iter().all(|stroke| !stroke.rocks), "the pencil of a strip is at rest");
        let inked = |ink: Ink| sketch.strokes.iter().filter(|stroke| stroke.ink == ink).count();
        assert_eq!((inked(Ink::Added), inked(Ink::Taken)), (2, 2), "the tick and the cross");
        // The first panel is whole before the last has a stroke: the strip is drawn in the order it is read.
        let last = sketch.strokes.last().unwrap();
        assert!(sketch.strokes[0].part(Some(last.start)) == 1.0 && last.part(Some(last.start)) == 0.0);
    });
}
