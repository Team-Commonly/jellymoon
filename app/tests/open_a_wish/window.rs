//! A tap on a little one in the page's window on GPUI's test platform: a wish of the reading has its project's chip
//! chosen, as a click on the chip chooses it, and what was typed stays; a made wish, or one whose project has no
//! chip, chooses nothing. The wish then entered is kept by core for that project, and no run is started: the day
//! has no installed command.

use gpui::{Entity, Modifiers, ScrollDelta, ScrollEvent, TestAppContext, VisualTestContext, point, px};
use jellymoon_app::showing::Showing;
use sticker_kit::draw::Page;
use sticker_kit::{Note, PieceKind};

use super::read_day;
use crate::support::windowed::{context, held, opened, types};

/// A click on the little one of the wish `id`.
fn tap(page: &Entity<Page>, cx: &mut VisualTestContext, id: &str) {
    let is = |piece: &&sticker_kit::Placed| piece.piece == PieceKind::Character && piece.id == id;
    // The column is longer than the window: the wheel to its top first, then down until the little one is at the
    // window's middle (or as near as the column's end lets it be).
    let wheel = |cx: &mut VisualTestContext, by: f32| {
        let delta = ScrollDelta::Pixels(point(px(0.0), px(by)));
        cx.simulate_event(ScrollEvent { position: point(px(700.0), px(300.0)), delta, ..Default::default() });
        cx.run_until_parked();
    };
    wheel(cx, 1e7);
    let down = page.read_with(cx, |page, _| page.placed().iter().find(is).map(|piece| piece.rect.center().1 - 300.0));
    wheel(cx, -down.unwrap_or_else(|| panic!("no little one {id}")).max(0.0));
    let piece = page.read_with(cx, |page, _| page.placed().iter().find(is).map(|piece| page.shown_at(piece)));
    let (x, y) = piece.unwrap_or_else(|| panic!("no little one {id}")).center();
    assert!(y > 0.0 && y < 760.0, "{id} is in the window, over the entry's band, to be clicked: {y}");
    cx.simulate_click(point(px(x), px(y)), Modifiers::default());
    cx.run_until_parked();
}

/// A click on the day's tally: the day opens under it. Folded, no wish of the reading is placed to be tapped.
fn unfold(page: &Entity<Page>, cx: &mut VisualTestContext) {
    let is = |piece: &&sticker_kit::Placed| piece.piece == PieceKind::Picture && piece.id == "fold";
    let tally = page.read_with(cx, |page, _| page.placed().iter().find(is).map(|piece| page.shown_at(piece)));
    let (x, y) = tally.expect("the day's tally").center();
    cx.simulate_click(point(px(x), px(y)), Modifiers::default());
    cx.run_until_parked();
}

/// Whether the note of the wish `id` is on show.
fn open(shown: &Showing, id: &str) -> bool {
    let named = |note: &Note| matches!(note, Note::Text { id: of, .. } if *of == format!("wish-{id}"));
    shown.scene().is_some_and(|scene| scene.notes.iter().any(named))
}

#[test]
fn a_tap_on_a_wish_of_the_reading_chooses_its_project_s_chip_and_the_next_wish_starts_there() {
    let w = read_day();
    let shown = Showing::of(Some(w.on_show()));
    let mut cx: TestAppContext = context();
    let (page, cx) = opened(&mut cx, shown.clone());
    let (_, options, chosen, _, _) = held(&page, cx);
    assert_eq!((options, chosen), (vec!["alpha".to_string(), "beta".to_string()], Some(0)));
    types(&page, cx, "and the changelog", false);
    unfold(&page, cx);
    tap(&page, cx, "w2");
    let (_, _, chosen, text, keys) = held(&page, cx);
    assert_eq!((chosen, text.as_str(), keys), (Some(1), "and the changelog", true), "beta, and the words stay");
    assert!(open(&shown, "w2"), "the wish opened as well");
    for no_chip in ["w3", "w5"] {
        tap(&page, cx, no_chip);
        assert_eq!(held(&page, cx).2, Some(1), "{no_chip}: no chip, so the chosen one stays");
    }
    assert_eq!(w.starts(), 0, "a tap starts nothing");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    let kept = w.kept();
    assert_eq!((kept["project"].as_str(), kept["said"].as_str()), (Some("beta"), Some("and the changelog")));
    assert_eq!(w.starts(), 0, "no command is installed: nothing was run");
    // A made wish chooses no chip: `m1` is beta's, and alpha stays chosen.
    tap(&page, cx, "w1");
    assert_eq!(held(&page, cx).2, Some(0));
    tap(&page, cx, "m1");
    assert!(open(&shown, "m1"), "the made wish opened");
    assert_eq!(held(&page, cx).2, Some(0), "a made wish chooses no chip");
}
