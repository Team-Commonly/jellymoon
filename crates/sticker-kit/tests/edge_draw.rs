//! The edge scene painted, headless: no window is shown. Each of its two windows has a view of its own; a tap
//! reaches the character's alone.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::rc::Rc;

use gpui::{Entity, Modifiers, TestAppContext, TestDispatcher, VisualTestContext, point, px, size};
use sticker_kit::draw::headless::text_system;
use sticker_kit::draw::{EdgePart, EdgeView, edge_ink, load_fonts};
use sticker_kit::{Color, Corner, Edge, EdgeKind, EdgeMood, EdgePiece, EdgeText, Role, Theme, Tone, edge_windows};

const MOODS: [EdgeMood; 7] = [
    EdgeMood::Floats,
    EdgeMood::LeansIn,
    EdgeMood::HoldsUp,
    EdgeMood::Swims,
    EdgeMood::Waves,
    EdgeMood::Glows,
    EdgeMood::Droops,
];

fn context() -> TestAppContext {
    TestAppContext::build_with_text_system(TestDispatcher::new(0), None, text_system())
}

/// One window of an edge scene in a test window of its own size, and how many taps reached its callback.
fn open(
    cx: &mut TestAppContext,
    part: EdgePart,
    edge: Edge,
    theme: Theme,
) -> (Entity<EdgeView>, &mut VisualTestContext, Rc<Cell<usize>>) {
    cx.update(|cx| load_fonts(cx)).expect("bundled fonts load");
    let (character, bubble) = edge_windows(&theme);
    let window = if part == EdgePart::Character { character } else { bubble };
    let taps = Rc::new(Cell::new(0));
    let seen = taps.clone();
    let (view, cx) = cx.add_window_view(move |_, _| {
        EdgeView::new(part, edge, theme, Corner::BottomRight).on_tap(move |_, _| seen.set(seen.get() + 1))
    });
    cx.simulate_resize(size(px(window.w), px(window.h)));
    cx.run_until_parked();
    (view, cx, taps)
}

fn placed(view: &Entity<EdgeView>, cx: &mut VisualTestContext) -> Vec<EdgePiece> {
    view.read_with(cx, |view, _| view.placed().to_vec())
}

fn said(text: &str, tone: Tone) -> EdgeText {
    EdgeText { text: text.to_string(), tone }
}

fn talk() -> Edge {
    let texts = vec![said("make it so", Tone::Strong), said("alpha", Tone::Small), said("Which one?", Tone::Quoted)];
    Edge { bubble: texts, ring: Some(0.5), ..Edge::default() }
}

#[test]
fn the_character_s_window_paints_each_mood_and_a_count_and_never_a_bubble() {
    for theme in [Theme::sticker(), Theme::plain()] {
        let mut cx = context();
        let (view, cx, _) = open(&mut cx, EdgePart::Character, Edge::default(), theme);
        for mood in MOODS {
            view.update(cx, |view, cx| view.set_edge(Edge { mood, count: 3, ..talk() }, cx));
            cx.run_until_parked();
            let kinds: Vec<EdgeKind> = placed(&view, cx).iter().map(|piece| piece.kind).collect();
            assert_eq!(kinds, [EdgeKind::Character, EdgeKind::Count], "{mood:?}");
            assert_eq!(view.read_with(cx, |view, _| view.edge().mood), mood);
            #[cfg(debug_assertions)]
            {
                let painted = cx.debug_bounds("edge:character").expect("the character was painted");
                assert_eq!((f32::from(painted.size.width), f32::from(painted.origin.x)), (80.0, 8.0), "{mood:?}");
                assert!(cx.debug_bounds("edge:count").is_some() && cx.debug_bounds("edge:bubble").is_none());
            }
        }
        // With no count none is painted.
        view.update(cx, |view, cx| view.set_edge(Edge::default(), cx));
        cx.run_until_parked();
        assert_eq!(placed(&view, cx).len(), 1);
    }
}

#[test]
fn a_tap_on_the_character_reaches_the_callback_and_a_tap_beside_it_or_on_the_bubble_reaches_none() {
    let mut cx = context();
    let (_, cx, taps) = open(&mut cx, EdgePart::Character, Edge::default(), Theme::sticker());
    cx.simulate_click(point(px(48.0), px(48.0)), Modifiers::default());
    cx.run_until_parked();
    assert_eq!(taps.get(), 1, "a tap on the character");
    // The part of the window one sees through is no part of the character.
    cx.simulate_click(point(px(3.0), px(3.0)), Modifiers::default());
    cx.run_until_parked();
    assert_eq!(taps.get(), 1, "a tap beside it");

    let mut cx = context();
    let (view, cx, taps) = open(&mut cx, EdgePart::Bubble, talk(), Theme::sticker());
    for piece in placed(&view, cx) {
        let (x, y) = piece.rect.center();
        cx.simulate_click(point(px(x), px(y)), Modifiers::default());
        cx.run_until_parked();
    }
    assert_eq!(taps.get(), 0, "the bubble takes no tap");
}

#[cfg(debug_assertions)]
#[test]
fn the_bubble_s_window_paints_each_line_where_it_was_laid_out_and_its_ring() {
    for theme in [Theme::sticker(), Theme::plain()] {
        let mut cx = context();
        let (view, cx, _) = open(&mut cx, EdgePart::Bubble, talk(), theme);
        let pieces = placed(&view, cx);
        assert_eq!(pieces.iter().map(|piece| piece.kind).collect::<Vec<_>>(), [EdgeKind::Bubble, EdgeKind::Ring]);
        let bubble = &pieces[0];
        assert_eq!(bubble.text.len(), 3);
        for (n, run) in bubble.text.iter().enumerate() {
            let selector: &'static str = Box::leak(format!("edge:text:{n}").into_boxed_str());
            let painted = cx.debug_bounds(selector).expect("the line was painted");
            let got = [f32::from(painted.origin.x), f32::from(painted.origin.y), f32::from(painted.size.height)];
            for (got, want) in got.iter().zip([run.rect.x, run.rect.y, run.rect.h]) {
                assert!((got - want).abs() <= 1.0, "{}: painted {painted:?}, laid out {:?}", run.text, run.rect);
            }
            let right = f32::from(painted.origin.x) + f32::from(painted.size.width);
            assert!(right <= bubble.rect.right() + 0.5, "{} is painted past the bubble", run.text);
        }
        let ring = cx.debug_bounds("edge:ring").expect("the ring was painted");
        assert!(
            (f32::from(ring.origin.x) - pieces[1].rect.x).abs() <= 1.0 && cx.debug_bounds("edge:character").is_none()
        );
        // Words that end leave nothing painted: the window is removed by whoever opened it.
        view.update(cx, |view, cx| view.set_edge(Edge::default(), cx));
        cx.run_until_parked();
        assert!(placed(&view, cx).is_empty() && cx.debug_bounds("edge:text:0").is_none());
    }
}

#[test]
fn each_tone_is_painted_in_an_ink_of_its_own_and_someone_else_s_words_in_the_theme_s_quoted() {
    for theme in [Theme::sticker(), Theme::plain()] {
        let p = &theme.palette;
        let inks = [Role::RowMain, Role::Body, Role::RowSub, Role::Quoted].map(|role| edge_ink(p, role));
        assert_eq!(inks, [p.ink, p.soft, p.quiet, p.quoted], "{}", theme.name);
        // What a frame painted, run by run: the review of F57, finding 4. A text of each tone, in the bubble.
        let mut cx = context();
        let tones = [Tone::Strong, Tone::Small, Tone::Plain, Tone::Quoted];
        let texts = tones.map(|tone| said("make it so", tone)).to_vec();
        let (view, cx, _) = open(&mut cx, EdgePart::Bubble, Edge { bubble: texts, ..Edge::default() }, theme.clone());
        let (pieces, painted) = view.read_with(cx, |view, _| (view.placed().to_vec(), view.painted().to_vec()));
        let roles: Vec<Role> = pieces[0].text.iter().map(|run| run.role).collect();
        assert_eq!(roles, [Role::RowMain, Role::RowSub, Role::Body, Role::Quoted], "{}", theme.name);
        let painted_in: Vec<Color> = painted.iter().map(|run| run.ink).collect();
        assert_eq!(painted_in, [p.ink, p.quiet, p.soft, p.quoted], "{}", theme.name);
        for (run, how) in pieces[0].text.iter().zip(&painted) {
            assert!(how.seen.inside(&pieces[0].rect) && how.seen.w >= run.rect.w, "{run:?} painted as {how:?}");
        }
        // The count is written on its pill in the bubble's colour, and a frame with no text painted none.
        let mut cx = context();
        let (view, cx, _) = open(&mut cx, EdgePart::Character, Edge { count: 3, ..Edge::default() }, theme.clone());
        let count: Vec<Color> = view.read_with(cx, |view, _| view.painted().iter().map(|run| run.ink).collect());
        assert_eq!(count, [p.bubble], "{}", theme.name);
        view.update(cx, |view, cx| view.set_edge(Edge::default(), cx));
        cx.run_until_parked();
        assert!(view.read_with(cx, |view, _| view.painted().is_empty()));
        // Told apart from the scene's own three inks, and read on the bubble as text is.
        assert!(inks[..3].iter().all(|own| *own != p.quoted), "{}", theme.name);
        assert!(p.quoted.contrast(p.bubble) >= 4.5, "{}: {}", theme.name, p.quoted.contrast(p.bubble));
    }
}
