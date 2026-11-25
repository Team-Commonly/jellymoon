//! The column, from its top: the header; a row for each small character nobody typed for; then the scene's notes
//! in the scene's order, each a card of the column's width at its full height. What the person typed
//! (`Note::Said`) is a bubble at the column's right with its character at the bubble's end, and the big character
//! stands left of the first card of the column and of the first card after each of those bubbles. No piece is
//! moved, made smaller or left out for room: the column is as long as what it holds.

use super::notes::{fill, stick};
use super::page::{self, BODY_BOTTOM, BODY_TOP};
use super::typed::block;
use super::{Ctx, MAX_PIECES, PieceKind, Placed, little, piece};
use crate::geom::Rect;
use crate::pencil::{Part, Pencil, Sketch};
use crate::scene::{Character, Note, Scene, State, speaker};

/// What is left between two cards, in design units: the strip of tape over a card stands in it.
const GAP: f32 = 36.0;
/// How far an answer is set in from the left of the words it is under, in design units: the room of its hook.
const SET_IN: f32 = 18.0;

pub(super) fn lay(cx: &mut Ctx, scene: &Scene) {
    let under = page::header(cx, scene);
    let mut top = rows(cx, scene, under);
    let (gap, mut heroes, mut owed) = (cx.px(GAP), 0, true);
    for (index, note) in scene.notes.iter().enumerate() {
        if cx.out.len() >= MAX_PIECES {
            break;
        }
        if let Note::Said { id, text, answer } = note {
            top = said(cx, scene, (id, text, answer), top) + gap;
            owed = true;
            continue;
        }
        let Some((sheet, id, tilt)) = fill(cx, note, index, top) else { continue };
        let mut bottom = stick(cx, sheet, (&id, index), tilt);
        if std::mem::take(&mut owed) {
            bottom = bottom.max(hero(cx, scene, heroes, top));
            heroes += 1;
        }
        top = bottom + gap;
    }
    page::paper(cx, top - gap);
}

/// Whether a `Said` of the scene is of the character `id`.
fn typed(scene: &Scene, id: &str) -> bool {
    scene.notes.iter().any(|note| matches!(note, Note::Said { id: said, .. } if speaker(said) == id))
}

/// A row for each character no `Said` of the scene is of: the small character, then its label, then what it says.
/// The ones let go come after the others.
/// Returns where the column goes on.
fn rows(cx: &mut Ctx, scene: &Scene, under: f32) -> f32 {
    let ((left, wide), (across, down), gap) = (cx.column(), little::room(cx), cx.px(8.0));
    let mut top = under;
    let rows = |gone: bool| {
        let here = move |who: &Character| (who.state == State::Dimmed) == gone && !typed(scene, &who.id);
        scene.characters.iter().enumerate().filter(move |(_, who)| here(who))
    };
    for (index, who) in rows(false).chain(rows(true)) {
        if cx.out.len() >= MAX_PIECES {
            break;
        }
        little::figure(cx, (left, top), index, who);
        let mut at = left + across + gap;
        for tag in [little::label(cx, index, who, left + wide - at), little::says(cx, index, who, wide / 2.0)] {
            let Some(tag) = tag.filter(|tag| at + tag.rect.w <= left + wide) else { continue };
            let middle = top + (down - tag.rect.h) / 2.0;
            at = little::put(cx, tag, (at, middle)).right() + gap;
        }
        top += down + gap;
    }
    if top > under { top + cx.px(GAP) - gap } else { top }
}

/// What the person typed, at `top`: a bubble that ends short of the column's right edge, where its character
/// stands with its stars and its state, and under them what the character says. Returns the bottom of it all.
/// With an `answer` the bubble holds it too, under the typed words ([`answered`], [`hook`]).
fn said(cx: &mut Ctx, scene: &Scene, (id, text, answer): (&str, &str, &str), top: f32) -> f32 {
    let ((left, wide), (across, down), gap) = (cx.column(), little::room(cx), cx.px(8.0));
    let end = left + wide - across - gap;
    let mut bottom = top;
    if let Some(mut bubble) = block(cx, (id, 0), text, Rect::new(end - wide * 0.72, top, wide * 0.72, 0.0), usize::MAX)
    {
        let under = answered(cx, &mut bubble, answer);
        // The bubble is as wide as its longest line, and ends where its character begins.
        let longest = bubble.text.iter().map(|run| run.rect.right()).fold(bubble.rect.x, f32::max);
        let spare = (bubble.rect.right() - longest - cx.px(10.0)).max(0.0);
        bubble.rect = Rect::new(bubble.rect.x + spare, bubble.rect.y, bubble.rect.w - spare, bubble.rect.h);
        bubble.text.iter_mut().for_each(|run| run.rect = run.rect.moved(spare, 0.0));
        hook(cx, &mut bubble, under);
        bottom = bubble.rect.bottom();
        cx.out.push(bubble);
    }
    let Some((index, who)) = scene.characters.iter().enumerate().find(|(_, who)| who.id == speaker(id)) else {
        return bottom;
    };
    little::figure(cx, (end + gap, top), index, who);
    bottom = bottom.max(top + down);
    if let Some(tag) = little::says(cx, index, who, wide / 2.0) {
        let at = (left + wide - tag.rect.w, bottom + gap);
        bottom = little::put(cx, tag, at).bottom();
    }
    bottom
}

/// The answer a person went on with, in the bubble of what they typed: its lines under the typed ones, set in,
/// and the bubble as much taller. They are the person's own words as the typed ones are: the same block, so
/// the same cleaning and the same role, and never anything but text. Returns the place of its first run among
/// the bubble's, or None for an answer that cleans to nothing: the bubble is then as it was.
fn answered(cx: &Ctx, bubble: &mut Placed, answer: &str) -> Option<usize> {
    let (set_in, first) = (cx.px(SET_IN), bubble.text.len());
    let at = Rect::new(bubble.rect.x + set_in, bubble.rect.bottom() - cx.px(7.0), bubble.rect.w - set_in, 0.0);
    let more = block(cx, (&bubble.id, 0), answer, at, usize::MAX)?;
    bubble.rect.h = more.rect.bottom() - bubble.rect.y;
    bubble.text.extend(more.text);
    Some(first)
}

/// What tells an answer from the typed words over it: a small hook the pencil draws, down from under those
/// words and across to the answer's first line, whose run is the bubble's `first`. No word says it.
fn hook(cx: &Ctx, bubble: &mut Placed, first: Option<usize>) {
    let Some(line) = first.and_then(|first| bubble.text.get(first)).map(|run| run.rect) else { return };
    let (left, right, mid) = (bubble.rect.x + cx.px(13.0), line.x - cx.px(5.0), line.y + line.h / 2.0);
    let mut pencil = Pencil::new(&format!("{}>", bubble.id), cx.k);
    pencil.path(&[(left, line.y), (left, mid), (right, mid)]);
    let part = Part { name: "hook".to_string(), rect: Rect::new(left, line.y, right - left, line.h / 2.0) };
    bubble.sketch = Some(Sketch { what: "answered".to_string(), parts: vec![part], strokes: pencil.finish() });
}

/// The big character left of the card at `top`, the `turn`th down the column, as large as the theme has it or as
/// the room left of the column lets it be. Returns its bottom, or `top` with no big character or no room.
fn hero(cx: &mut Ctx, scene: &Scene, turn: usize, top: f32) -> f32 {
    let m = cx.theme.metrics;
    let Some(hero) = &scene.hero else { return top };
    let right = cx.x(m.content_left) - cx.px(24.0);
    let wide = cx.px(m.hero_size).min(right - cx.x(m.margin_x + 60.0));
    if wide < 1.0 {
        return top;
    }
    let rect = Rect::new(right - wide, top, wide, (BODY_TOP + BODY_BOTTOM) * wide / 200.0);
    cx.out.push(piece(PieceKind::Hero, &hero.id, turn, rect));
    rect.bottom()
}
