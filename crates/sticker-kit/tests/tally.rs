//! A tally (`Picture::Tally`): rows of a label and counted marks under a title, with a chevron, and where its
//! note stands in the column: at the top, over the small characters' rows, and everything else where a scene
//! without it has it, further down. No GPUI.

use sticker_kit::{
    FixedAdvance, MAX_TALLY_MARKS, MAX_TALLY_ROWS, Mark, Note, Picture, PieceKind, Placed, Rect, Role, Scene, TallyRow,
    TextRun, Theme, Viewport, layout,
};

const SIZES: [Viewport; 2] = [Viewport { w: 1440.0, h: 900.0 }, Viewport { w: 960.0, h: 640.0 }];
/// The kit's smallest tap target, in design units.
const SMALLEST: f32 = 24.0;
const UNDER: &str = "Counted by someone else.";

fn row(label: &str, marks: &[Mark]) -> TallyRow {
    TallyRow { label: label.into(), marks: marks.to_vec() }
}

fn tally(rows: Vec<TallyRow>, more: &str, open: bool) -> Note {
    let picture = Picture::Tally { title: "so far".into(), rows, more: more.into(), under: UNDER.into(), open };
    Note::Drawn { id: "fold".into(), picture, words: Vec::new(), open: false, options: Vec::new(), field: None }
}

/// The sample's page with two cards of its own, and `last` after them.
fn day(last: Option<Note>) -> Scene {
    let card = |title: &str| Note::Plain { title: title.into(), body: "One line.".into() };
    Scene { notes: [card("first"), card("second")].into_iter().chain(last).collect(), ..Scene::sample() }
}

fn each(test: impl Fn(&Theme, Viewport, f32)) {
    for theme in [Theme::sticker(), Theme::plain()] {
        SIZES.iter().for_each(|size| test(&theme, *size, sticker_kit::layout::page_scale(&theme, *size)));
    }
}

fn place(scene: &Scene, theme: &Theme, viewport: Viewport) -> Vec<Placed> {
    layout(scene, theme, viewport, &FixedAdvance(0.56))
}

fn picture(placed: &[Placed]) -> &Placed {
    let own = |piece: &&Placed| piece.piece == PieceKind::Picture && piece.id == "fold";
    placed.iter().find(own).unwrap_or_else(|| panic!("the tally's picture"))
}

/// The named parts of the picture's sketch, in the order they were drawn, with the middle of each.
fn parts(picture: &Placed) -> Vec<(&str, (f32, f32))> {
    let sketch = picture.sketch.as_ref().unwrap_or_else(|| panic!("a sketch"));
    sketch.parts.iter().map(|part| (part.name.as_str(), part.rect.center())).collect()
}

fn words(picture: &Placed, role: Role) -> Vec<&str> {
    picture.text.iter().filter(|run| run.role == role).map(|run| run.text.as_str()).collect()
}

const ALL: [Mark; 4] = [Mark::NeedsYou, Mark::Busy, Mark::Star, Mark::Dimmed];

#[test]
fn a_tally_places_one_row_for_each_row_given_with_its_label_and_its_marks_in_the_order_given() {
    each(|theme, viewport, _| {
        let rows = vec![row("alpha", &ALL), row("beta", &[Mark::Star, Mark::Star]), row("gamma", &[])];
        let placed = place(&day(Some(tally(rows, "", false))), theme, viewport);
        let picture = picture(&placed);
        let names: Vec<&str> = parts(picture).into_iter().map(|(name, _)| name).collect();
        let want = ["opens", "asks", "busy", "star", "gone", "row", "star", "star", "row", "row"];
        assert_eq!(names, want, "the chevron, then each row's marks and the row");
        assert_eq!(words(picture, Role::Caption), ["so far", "alpha", "?", "beta", "gamma"]);
        assert_eq!(words(picture, Role::Footer), [UNDER], "one quiet line at its foot, and none for rows past it");
        let sketch = picture.sketch.as_ref().unwrap();
        assert!(sketch.what.starts_with("tally "), "{}", sketch.what);
        assert!(!sketch.strokes.is_empty());
    });
}

#[test]
fn a_row_of_more_marks_than_fit_is_one_line_that_ends_in_the_count_of_the_rest() {
    each(|theme, viewport, _| {
        let many = vec![Mark::Star; MAX_TALLY_MARKS + 9];
        let placed = place(&day(Some(tally(vec![row("alpha", &many), row("beta", &ALL)], "", false))), theme, viewport);
        let picture = picture(&placed);
        let drawn = parts(picture);
        let stars: Vec<(f32, f32)> = drawn.iter().filter(|(name, _)| *name == "star").map(|(_, at)| *at).collect();
        assert_eq!(stars.len(), MAX_TALLY_MARKS + 1, "twelve of the first row, and the second row's one");
        let first = &stars[..MAX_TALLY_MARKS];
        assert!(first.iter().all(|at| at.1 == first[0].1), "a row never wraps: {first:?}");
        assert!(first.windows(2).all(|pair| pair[0].0 < pair[1].0), "left to right");
        assert_eq!(words(picture, Role::Caption), ["so far", "alpha", "+9", "beta", "?"]);
        let count = picture.text.iter().find(|run| run.text == "+9").unwrap();
        assert!(count.rect.x > first[MAX_TALLY_MARKS - 1].0, "the count is after the last mark");
        assert!(count.rect.right() <= picture.rect.right() + 0.01, "and inside the picture");
        assert!((count.rect.center().1 - first[0].1).abs() < 0.01, "on the row's line");
    });
}

#[test]
fn rows_past_the_fifth_are_not_drawn_and_one_quiet_line_stands_for_them() {
    each(|theme, viewport, _| {
        let rows: Vec<TallyRow> = (0..MAX_TALLY_ROWS + 3).map(|n| row(&format!("p{n}"), &[Mark::Busy])).collect();
        for (more, said) in [("and 3 others", "and 3 others"), ("", "+3 more")] {
            let placed = place(&day(Some(tally(rows.clone(), more, false))), theme, viewport);
            let picture = picture(&placed);
            let count = |name: &str| parts(picture).iter().filter(|(part, _)| *part == name).count();
            assert_eq!((count("row"), count("busy")), (MAX_TALLY_ROWS, MAX_TALLY_ROWS));
            assert_eq!(words(picture, Role::Caption), ["so far", "p0", "p1", "p2", "p3", "p4"]);
            assert_eq!(words(picture, Role::Footer), [said, UNDER], "the quiet line, then the foot");
        }
    });
}

#[test]
fn the_picture_is_a_tap_target_with_a_chevron_that_turns_when_it_is_open() {
    each(|theme, viewport, k| {
        for (open, name) in [(false, "opens"), (true, "closes")] {
            let placed = place(&day(Some(tally(vec![row("alpha", &ALL)], "", open))), theme, viewport);
            let picture = picture(&placed);
            assert!(picture.rect.h >= SMALLEST * k - 0.01 && picture.rect.w >= SMALLEST * k, "{:?}", picture.rect);
            let chevrons: Vec<_> =
                parts(picture).into_iter().filter(|(part, _)| part.starts_with(['o', 'c'])).collect();
            assert_eq!(chevrons.len(), 1, "{chevrons:?}");
            assert_eq!(chevrons[0].0, name);
            let title = picture.text.iter().find(|run| run.text == "so far").unwrap();
            assert!(chevrons[0].1.0 > title.rect.right(), "at the end of the title's line");
            assert!((chevrons[0].1.1 - title.rect.center().1).abs() < 0.01, "and on it");
            // The note has no word of its own, so no `?` stands on it, and nothing else of it is tapped.
            assert!(!placed.iter().any(|piece| piece.piece == PieceKind::Ask && piece.id == "fold"));
        }
    });
}

#[test]
fn a_label_is_cleaned_and_cut_in_its_middle_and_never_reaches_the_marks() {
    each(|theme, viewport, _| {
        let long = "a-project-with-a-very-long-name-that-goes-on-and-on-and-on\u{7}-to-its-end";
        let placed = place(&day(Some(tally(vec![row(long, &[Mark::Star])], "", false))), theme, viewport);
        let picture = picture(&placed);
        let label = &picture.text[1];
        assert!(label.text.starts_with("a-pro") && label.text.ends_with("-end"), "{}", label.text);
        assert!(label.text.contains('…') && !label.text.contains('\u{7}'), "{}", label.text);
        let star = parts(picture).into_iter().find(|(name, _)| *name == "star").unwrap();
        assert!(label.rect.right() < star.1.0, "{:?} {star:?}", label.rect);
    });
}

/// Whether two rectangles are the same but for `down`, how much lower the first is.
fn near(now: Rect, was: Rect, down: f32) -> bool {
    let close = |a: f32, b: f32| (a - b).abs() < 0.01;
    close(now.x, was.x) && close(now.y - down, was.y) && close(now.w, was.w) && close(now.h, was.h)
}

/// Whether `now` is the piece `was`, `down` lower: its kind, its id, its place among its own, its tilt, every
/// run of its text, and what its sketch is of. A sketch's strokes are places too, and are not compared.
fn same(now: &Placed, was: &Placed, down: f32) -> bool {
    let run = |(a, b): (&TextRun, &TextRun)| {
        a.text == b.text && a.role == b.role && a.size == b.size && near(a.rect, b.rect, down)
    };
    let of = |piece: &Placed| piece.sketch.as_ref().map(|sketch| (sketch.what.clone(), sketch.parts.len()));
    (now.piece, &now.id, now.index, now.z) == (was.piece, &was.id, was.index, was.z)
        && now.tilt_degrees == was.tilt_degrees
        && near(now.rect, was.rect, down)
        && now.text.len() == was.text.len()
        && now.text.iter().zip(&was.text).all(run)
        && of(now) == of(was)
}

// The note stands at the column's top though it is the scene's last note: so no other note's place among the
// notes changes, and every other piece is the piece a scene without it has, the same distance further down.
#[test]
fn a_tally_stands_over_the_small_characters_and_every_other_piece_is_as_without_it_further_down() {
    each(|theme, viewport, _| {
        // The paper is as long as its column and its holes go down it: a longer column has more of them.
        let place = |scene: &Scene| {
            let mut placed = place(scene, theme, viewport);
            placed.retain(|piece| piece.piece != PieceKind::Hole);
            placed
        };
        let without = place(&day(None));
        let with = place(&day(Some(tally(vec![row("alpha", &ALL)], "", true))));
        let own = |piece: &&Placed| piece.id == "fold";
        let card = with.iter().find(|piece| own(piece) && piece.piece == PieceKind::Note).expect("its card");
        let first = without.iter().find(|piece| piece.piece == PieceKind::Character).expect("a small character");
        assert!(card.rect.y <= first.rect.y, "where the first row was: {:?} {:?}", card.rect, first.rect);
        let rest: Vec<&Placed> = with.iter().filter(|piece| !own(piece)).collect();
        assert_eq!(rest.len(), without.len());
        let down = rest.iter().find(|piece| piece.piece == PieceKind::Character).unwrap().rect.y - first.rect.y;
        assert!(down >= card.rect.h, "under the card: {down} {:?}", card.rect);
        for (now, was) in rest.iter().zip(&without) {
            if was.piece == PieceKind::Paper {
                continue;
            }
            let by = if was.rect.bottom() < card.rect.y { 0.0 } else { down };
            assert!(same(now, was, by), "{now:?}\n{was:?}");
        }
    });
}

// Closed, the tally is the page's first card and the big character stands beside it; open, the big character is
// where a scene without the tally has it, beside the first card under the rows.
#[test]
fn the_big_character_stands_beside_a_closed_tally_and_where_it_was_beside_an_open_one() {
    each(|theme, viewport, _| {
        let hero =
            |placed: &[Placed]| placed.iter().find(|piece| piece.piece == PieceKind::Hero).map(|hero| hero.rect.y);
        let top = |placed: &[Placed], id: &str| {
            placed.iter().find(|piece| piece.piece == PieceKind::Note && piece.id == id).map(|note| note.rect.y)
        };
        let closed = place(&day(Some(tally(vec![row("alpha", &ALL)], "", false))), theme, viewport);
        let open = place(&day(Some(tally(vec![row("alpha", &ALL)], "", true))), theme, viewport);
        if hero(&closed).is_none() {
            return; // no room left of the column at this size
        }
        assert_eq!(hero(&closed), top(&closed, "fold"));
        assert_eq!(hero(&open), top(&open, "note-0"));
        let heroes = |placed: &[Placed]| placed.iter().filter(|piece| piece.piece == PieceKind::Hero).count();
        assert_eq!((heroes(&closed), heroes(&open)), (1, 1));
    });
}

#[test]
fn a_scene_with_no_tally_has_no_piece_of_one() {
    let placed = place(&day(None), &Theme::sticker(), SIZES[0]);
    assert!(!placed.iter().any(|piece| piece.piece == PieceKind::Picture || piece.id == "fold"));
}
