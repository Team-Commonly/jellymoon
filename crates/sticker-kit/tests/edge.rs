//! The edge scene: where a corner window and the one beside it stand, given a screen and the room left on it as
//! numbers, and what a bubble is drawn with. No display, no clock and no word of any app.

use sticker_kit::{
    Corner, EDGE_GAP, EDGE_MARGIN, Edge, EdgeKind, EdgeMood, EdgePlaced, FixedAdvance, MAX_BUBBLE_LINES,
    MAX_BUBBLE_TEXTS, Rect, Role, Said, TextRun, Theme, Tone, Viewport, edge_windows, layout_edge, place_beside,
    place_corner,
};

const SCREEN: Viewport = Viewport { w: 1440.0, h: 900.0 };
const WHOLE: Rect = Rect { x: 0.0, y: 0.0, w: 1440.0, h: 900.0 };
const SMALL: Viewport = Viewport { w: 96.0, h: 96.0 };
const WIDE: Viewport = Viewport { w: 300.0, h: 200.0 };
const CORNERS: [Corner; 4] = [Corner::TopLeft, Corner::TopRight, Corner::BottomLeft, Corner::BottomRight];

fn at(rect: Rect) -> [f32; 4] {
    [rect.x, rect.y, rect.w, rect.h]
}

fn inside(inner: Rect, outer: Rect) -> bool {
    inner.x >= outer.x && inner.y >= outer.y && inner.right() <= outer.right() && inner.bottom() <= outer.bottom()
}

fn said(text: &str, tone: Tone) -> Said {
    Said { text: text.to_string(), tone }
}

fn placed(edge: &Edge) -> EdgePlaced {
    layout_edge(edge, &Theme::sticker(), Corner::BottomRight, &FixedAdvance(0.5))
}

fn bubble(texts: &[Said]) -> Vec<TextRun> {
    let edge = Edge { bubble: texts.to_vec(), ..Edge::default() };
    placed(&edge).bubble.map(|window| window.pieces[0].text.clone()).unwrap_or_default()
}

#[test]
fn a_corner_window_stands_a_margin_from_its_corner_s_two_edges() {
    let stands = |corner| at(place_corner(SCREEN, WHOLE, SMALL, corner, EDGE_MARGIN));
    assert_eq!(stands(Corner::TopLeft), [16.0, 16.0, 96.0, 96.0]);
    assert_eq!(stands(Corner::TopRight), [1328.0, 16.0, 96.0, 96.0]);
    assert_eq!(stands(Corner::BottomLeft), [16.0, 788.0, 96.0, 96.0]);
    assert_eq!(stands(Corner::BottomRight), [1328.0, 788.0, 96.0, 96.0]);
    assert_eq!(Corner::default(), Corner::BottomRight);
    assert_eq!(at(place_corner(SCREEN, WHOLE, SMALL, Corner::BottomRight, 0.0)), [1344.0, 804.0, 96.0, 96.0]);
}

#[test]
fn a_window_stands_inside_the_room_a_menu_bar_and_a_dock_leave() {
    // A menu bar of 25 at the top and a dock of 70 at the bottom; then a dock of 80 on the right.
    let under_bar = Rect::new(0.0, 25.0, 1440.0, 805.0);
    let stands = |room, corner| at(place_corner(SCREEN, room, SMALL, corner, EDGE_MARGIN));
    assert_eq!(stands(under_bar, Corner::BottomRight), [1328.0, 718.0, 96.0, 96.0]);
    assert_eq!(stands(under_bar, Corner::TopLeft), [16.0, 41.0, 96.0, 96.0]);
    assert_eq!(stands(Rect::new(0.0, 25.0, 1360.0, 875.0), Corner::BottomRight), [1248.0, 788.0, 96.0, 96.0]);
    // Only the part of the room that is on the screen counts.
    assert_eq!(stands(Rect::new(-50.0, -50.0, 3000.0, 3000.0), Corner::BottomRight), [1328.0, 788.0, 96.0, 96.0]);
    // A room that is nothing, is off the screen, or is not numbers is the whole screen.
    let none = [
        Rect::new(0.0, 0.0, 0.0, 0.0),
        Rect::new(2000.0, 0.0, 100.0, 100.0),
        Rect::new(0.0, 0.0, -5.0, 900.0),
        Rect::new(f32::NAN, 0.0, 1440.0, 900.0),
        Rect::new(0.0, 0.0, f32::INFINITY, 900.0),
    ];
    for room in none {
        assert_eq!(stands(room, Corner::BottomRight), [1328.0, 788.0, 96.0, 96.0], "{room:?}");
    }
}

#[test]
fn a_room_smaller_than_the_window_keeps_the_window_whole_from_the_room_s_top_left() {
    for corner in CORNERS {
        // The margin gives way before the window leaves the room.
        let tight =
            place_corner(Viewport { w: 100.0, h: 100.0 }, Rect::new(0.0, 0.0, 100.0, 100.0), SMALL, corner, 16.0);
        assert!(inside(tight, Rect::new(0.0, 0.0, 100.0, 100.0)) && (tight.w, tight.h) == (96.0, 96.0), "{tight:?}");
        let small = Viewport { w: 64.0, h: 48.0 };
        assert_eq!(
            at(place_corner(small, Rect::new(0.0, 0.0, 64.0, 48.0), SMALL, corner, 16.0)),
            [0.0, 0.0, 96.0, 96.0]
        );
        let room = Rect::new(10.0, 30.0, 50.0, 40.0);
        assert_eq!(at(place_corner(SCREEN, room, SMALL, corner, 16.0)), [10.0, 30.0, 96.0, 96.0], "{corner:?}");
        assert_eq!(
            at(place_beside(SCREEN, room, Rect::new(10.0, 30.0, 96.0, 96.0), WIDE, corner, 8.0))[..2],
            [10.0, 30.0]
        );
    }
    // Numbers that are none place a window at the screen's start, and never at a place that is no number.
    let odd = place_corner(
        Viewport { w: f32::NAN, h: -4.0 },
        WHOLE,
        Viewport { w: f32::NAN, h: 96.0 },
        Corner::BottomRight,
        f32::NAN,
    );
    assert_eq!(at(odd), [0.0, 0.0, 0.0, 96.0]);
}

#[test]
fn the_second_window_stands_beside_the_first_away_from_the_corner_and_over_it_when_there_is_no_room() {
    let beside = |corner, screen: Viewport| {
        let room = Rect::new(0.0, 0.0, screen.w, screen.h);
        let anchor = place_corner(screen, room, SMALL, corner, EDGE_MARGIN);
        (anchor, place_beside(screen, room, anchor, WIDE, corner, EDGE_GAP))
    };
    for corner in CORNERS {
        let (anchor, second) = beside(corner, SCREEN);
        let (right, bottom) = (matches!(corner, Corner::TopRight | Corner::BottomRight), anchor.y > 450.0);
        assert!(inside(second, WHOLE) && (second.w, second.h) == (300.0, 200.0), "{corner:?}: {second:?}");
        assert_eq!(if right { anchor.x - second.right() } else { second.x - anchor.right() }, 8.0, "{corner:?}");
        assert_eq!(if bottom { second.bottom() - anchor.bottom() } else { second.y - anchor.y }, 0.0, "{corner:?}");
        // Too narrow for both side by side: the second stands over a lower anchor and under an upper one.
        let (anchor, second) = beside(corner, Viewport { w: 320.0, h: 900.0 });
        assert!(inside(second, Rect::new(0.0, 0.0, 320.0, 900.0)), "{corner:?}: {second:?}");
        assert_eq!(if bottom { anchor.y - second.bottom() } else { second.y - anchor.bottom() }, 8.0, "{corner:?}");
        assert_eq!(if right { second.right() - anchor.right() } else { second.x - anchor.x }, 0.0, "{corner:?}");
    }
    // Exactly room enough is room: 16 + 96 + 8 + 300 from the right edge.
    let (anchor, second) = beside(Corner::BottomRight, Viewport { w: 420.0, h: 900.0 });
    assert_eq!((second.x, second.bottom()), (0.0, anchor.bottom()));
    let (anchor, second) = beside(Corner::BottomRight, Viewport { w: 419.0, h: 900.0 });
    assert_eq!(second.bottom() + 8.0, anchor.y);
}

#[test]
fn a_mood_a_count_and_a_ring_change_no_window_and_a_bubble_with_nothing_to_see_is_no_bubble() {
    let (character, window) = edge_windows(&Theme::sticker());
    assert_eq!((character.w, character.h, window.w), (96.0, 96.0, 300.0));
    let moods = [EdgeMood::Floats, EdgeMood::LeansIn, EdgeMood::HoldsUp, EdgeMood::Swims, EdgeMood::Waves];
    for mood in moods.into_iter().chain([EdgeMood::Glows, EdgeMood::Droops]) {
        let shown = placed(&Edge { mood, ..Edge::default() });
        assert_eq!(shown, placed(&Edge::default()), "{mood:?}");
        assert_eq!((shown.character.size, shown.character.pieces.len(), shown.bubble), (character, 1, None));
    }
    // A count is drawn on the character, inside its window; none for 0, and `99+` past 99.
    let count = |count| {
        let shown = placed(&Edge { count, ..Edge::default() });
        let pieces = shown.character.pieces;
        assert!(pieces.iter().all(|piece| inside(piece.rect, Rect::new(0.0, 0.0, 96.0, 96.0))), "{pieces:?}");
        pieces.iter().find(|piece| piece.kind == EdgeKind::Count).map(|piece| piece.text[0].text.clone())
    };
    assert_eq!(
        [count(0), count(1), count(99), count(100), count(usize::MAX)].map(Option::unwrap_or_default),
        ["", "1", "99", "99+", "99+"]
    );
    // Nothing a person sees is no bubble, no second window and no ring.
    let unseen = [said("", Tone::Plain), said("  \u{200b}\n", Tone::Strong), said("\u{1b}", Tone::Quoted)];
    let empty = placed(&Edge { bubble: unseen.to_vec(), ring: Some(0.5), ..Edge::default() });
    assert_eq!((empty.bubble, empty.ring), (None, None));
    // A theme the kit refuses gets no piece.
    let mut broken = Theme::sticker();
    broken.metrics.width = f32::NAN;
    let refused = layout_edge(&Edge { count: 3, ..Edge::default() }, &broken, Corner::BottomRight, &FixedAdvance(0.5));
    assert!(refused.character.pieces.is_empty() && refused.bubble.is_none());
}

#[test]
fn a_bubble_s_window_is_one_size_and_its_bubble_stands_on_the_corner_s_side() {
    let window = edge_windows(&Theme::sticker()).1;
    let long = "words and more words ".repeat(40);
    let cases = [
        vec![said("hi", Tone::Plain)],
        vec![said(&long, Tone::Strong), said(&long, Tone::Small), said(&long, Tone::Plain), said(&long, Tone::Quoted)],
    ];
    for texts in cases {
        for corner in CORNERS {
            let edge = Edge { bubble: texts.clone(), ring: Some(1.0), ..Edge::default() };
            let shown = layout_edge(&edge, &Theme::sticker(), corner, &FixedAdvance(0.5)).bubble.unwrap();
            let (bubble, ring) = (&shown.pieces[0], &shown.pieces[1]);
            assert_eq!(shown.size, window, "what is said never resizes the window");
            assert!(inside(bubble.rect, Rect::new(0.0, 0.0, window.w, window.h)), "{bubble:?}");
            let lower = matches!(corner, Corner::BottomLeft | Corner::BottomRight);
            assert_eq!(if lower { bubble.rect.bottom() } else { bubble.rect.y }, if lower { window.h } else { 0.0 });
            assert!(inside(ring.rect, bubble.rect) && ring.rect.w == ring.rect.h && ring.text.is_empty());
            for run in &bubble.text {
                assert!(inside(run.rect, bubble.rect), "{run:?}");
                let apart = run.rect.right() <= ring.rect.x || run.rect.y >= ring.rect.bottom();
                assert!(apart, "{run:?} runs under the ring");
            }
            assert!(bubble.text.len() <= MAX_BUBBLE_LINES);
        }
    }
}

#[test]
fn each_tone_has_its_role_and_its_most_lines_and_every_text_keeps_a_line() {
    let long = "words and more words ".repeat(40);
    let lines = |tone, role| bubble(&[said(&long, tone)]).iter().filter(|run| run.role == role).count();
    assert_eq!(lines(Tone::Plain, Role::Body), 4);
    assert_eq!(lines(Tone::Strong, Role::RowMain), 3);
    assert_eq!(lines(Tone::Small, Role::RowSub), 1);
    assert_eq!(lines(Tone::Quoted, Role::Quoted), 3);
    // Four long texts: eight lines over all, and the last still has one.
    let all =
        [said(&long, Tone::Plain), said(&long, Tone::Strong), said(&long, Tone::Quoted), said(&long, Tone::Plain)];
    let runs = bubble(&all);
    assert_eq!(runs.len(), MAX_BUBBLE_LINES);
    assert_eq!(runs.iter().filter(|run| run.role == Role::Quoted).count(), 1);
    assert!(runs.last().is_some_and(|run| run.role == Role::Body && run.text.ends_with('…')));
    // A fifth text is not drawn; one with nothing to see takes no place among the four.
    let mut more = vec![said("\u{200b}", Tone::Plain)];
    more.extend((0..=MAX_BUBBLE_TEXTS).map(|at| said(&format!("text {at}"), Tone::Plain)));
    let drawn: Vec<String> = bubble(&more).into_iter().map(|run| run.text).collect();
    assert_eq!(drawn, ["text 0", "text 1", "text 2", "text 3"]);
}

#[test]
fn a_quoted_text_stands_apart_from_the_text_before_it_and_the_one_after() {
    let texts =
        [said("one", Tone::Plain), said("two", Tone::Plain), said("asked", Tone::Quoted), said("four", Tone::Plain)];
    let runs = bubble(&texts);
    let gap = |before: usize| runs[before + 1].rect.y - runs[before].rect.bottom();
    assert!(gap(1) > gap(0) && gap(2) > gap(0) && gap(1) >= 8.0, "{} {} {}", gap(0), gap(1), gap(2));
    assert_eq!(runs.iter().map(|run| run.role).collect::<Vec<_>>(), [Role::Body, Role::Body, Role::Quoted, Role::Body]);
}

#[test]
fn a_text_is_cut_with_an_ellipsis_between_two_characters_and_is_never_more_than_text() {
    let nasty = "<a href=\"x\">go</a> [t](http://x.test) \u{1b}[1m\u{7}bold\nnext\ttab \u{202e}flip";
    for tone in [Tone::Plain, Tone::Strong, Tone::Small, Tone::Quoted] {
        let runs = bubble(&[said(nasty, tone)]);
        let all: String = runs.iter().map(|run| run.text.as_str()).collect::<Vec<_>>().join(" ");
        assert!(!all.contains(|c: char| c.is_control() || c == '\u{202e}'), "{all:?}");
        assert!(all.starts_with("<a href=\"x\">go</a>"), "markup is drawn as the letters it is: {all}");
        assert_eq!(placed(&Edge { bubble: vec![said(nasty, tone)], ..Edge::default() }).character.pieces.len(), 1);
    }
    for one in ["e\u{301}", "👩\u{200d}👩\u{200d}👧\u{200d}👦", "🇯🇵"] {
        let many = one.repeat(300 / one.chars().count());
        let runs = bubble(&[said(&many, Tone::Strong)]);
        assert_eq!(runs.len(), 3);
        assert!(runs[2].text.ends_with('…') && !runs[0].text.contains('…'), "{runs:?}");
        for run in &runs {
            let text = run.text.trim_end_matches('…');
            assert_eq!(text, one.repeat(text.len() / one.len()), "a line holds whole characters only");
        }
    }
}

#[test]
fn a_ring_is_from_full_to_empty_and_a_number_that_is_none_is_empty() {
    let ring = |ring| placed(&Edge { bubble: vec![said("hi", Tone::Plain)], ring, ..Edge::default() }).ring;
    assert_eq!(
        [ring(Some(1.0)), ring(Some(0.25)), ring(Some(0.0)), ring(None)],
        [Some(1.0), Some(0.25), Some(0.0), None]
    );
    assert_eq!(
        [ring(Some(7.0)), ring(Some(-1.0)), ring(Some(f32::NAN)), ring(Some(f32::INFINITY))],
        [Some(1.0), Some(0.0), Some(0.0), Some(0.0)]
    );
    // How full it is moves nothing.
    let at = |ring| placed(&Edge { bubble: vec![said("hi", Tone::Plain)], ring: Some(ring), ..Edge::default() }).bubble;
    assert_eq!(at(1.0), at(0.1));
}
