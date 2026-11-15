//! The edge scene painted: one view for each of its two windows, the character's and the bubble's. A view lays
//! its scene out with the window's own text system and paints the pieces of its window, each where the layout
//! put it. Text is text: a run is a line of letters in its role's ink, clipped to its bubble. Only the character
//! takes a tap; the view never asks for the keys.

use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, TAU};
use std::sync::Arc;

use gpui::{AnyElement, App, Context, Div, Image, ImageFormat, Window, canvas, div, img, prelude::*, px};

use super::measure::{GlyphMeasure, family, hsla, weight};
use super::paint::paint_polygon;
use crate::art;
use crate::color::Color;
use crate::edge::{Corner, Edge, EdgeMood};
use crate::geom::Rect;
use crate::layout::{EdgeKind, EdgePiece, TextRun, layout_edge};
use crate::text::Role;
use crate::theme::{Palette, Theme};

/// Which of an edge scene's two windows a view paints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgePart {
    Character,
    Bubble,
}

/// The character's art is made at twice the size it is shown at, with this body and rim.
const ART_BODY: f32 = 100.0;
const ART_RIM: f32 = 6.0;
/// How thick the ring is, and how many pieces a full one is painted in.
const RING_WIDTH: f32 = 3.0;
const RING_STEPS: usize = 48;

type TapHandler = Box<dyn Fn(&mut Window, &mut App)>;

pub struct EdgeView {
    part: EdgePart,
    edge: Edge,
    theme: Theme,
    corner: Corner,
    on_tap: Option<TapHandler>,
    placed: Vec<EdgePiece>,
    art: HashMap<EdgeMood, Arc<Image>>,
}

/// The ink a bubble's text of `role` is painted in: the scene's own three, and `quoted` for someone else's words.
pub fn edge_ink(palette: &Palette, role: Role) -> Color {
    match role {
        Role::RowMain => palette.ink,
        Role::RowSub => palette.quiet,
        Role::Quoted => palette.quoted,
        _ => palette.soft,
    }
}

impl EdgeView {
    pub fn new(part: EdgePart, edge: Edge, theme: Theme, corner: Corner) -> Self {
        Self { part, edge, theme, corner, on_tap: None, placed: Vec::new(), art: HashMap::new() }
    }

    /// Called when the character is tapped. The bubble, the count and the part of the window one sees through
    /// take no tap.
    pub fn on_tap(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_tap = Some(Box::new(handler));
        self
    }

    pub fn set_edge(&mut self, edge: Edge, cx: &mut Context<Self>) {
        if edge != self.edge {
            self.edge = edge;
            cx.notify();
        }
    }

    pub fn set_theme(&mut self, theme: Theme, cx: &mut Context<Self>) {
        for image in self.art.drain().map(|entry| entry.1) {
            image.remove_asset(cx);
        }
        self.theme = theme;
        cx.notify();
    }

    pub fn edge(&self) -> &Edge {
        &self.edge
    }

    /// The pieces of this view's window that the last frame painted.
    pub fn placed(&self) -> &[EdgePiece] {
        &self.placed
    }

    fn image(&mut self) -> Arc<Image> {
        let (mood, theme) = (self.edge.mood, &self.theme);
        let draw = || art::edge_character(&theme.palette, theme.hue(0), mood, ART_BODY, ART_RIM);
        self.art
            .entry(mood)
            .or_insert_with(|| Arc::new(Image::from_bytes(ImageFormat::Svg, draw().into_bytes())))
            .clone()
    }

    fn character(&mut self, rect: Rect, cx: &mut Context<Self>) -> AnyElement {
        let (w, h) = art::character_size(ART_BODY, ART_RIM);
        let shown = Rect::new((rect.w - w / 2.0) / 2.0, (rect.h - h / 2.0) / 2.0, w / 2.0, h / 2.0);
        at(rect)
            .id("edge-character")
            .debug_selector(|| "edge:character".to_string())
            .cursor_pointer()
            .on_click(cx.listener(|view, _, window, cx| {
                if let Some(tap) = &view.on_tap {
                    tap(window, cx);
                }
            }))
            .child(img(self.image()).absolute().left(px(shown.x)).top(px(shown.y)).w(px(shown.w)).h(px(shown.h)))
            .into_any_element()
    }

    /// The elements of one piece, in paint order.
    fn piece(&mut self, piece: &EdgePiece, share: f32, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let own = self.theme.clone();
        let (theme, rect) = (&own, piece.rect);
        let p = &theme.palette;
        let runs = |ink: Option<Color>| -> Vec<AnyElement> {
            let line = |(n, run): (usize, &TextRun)| text(theme, run, n, rect, ink.unwrap_or(edge_ink(p, run.role)));
            piece.text.iter().enumerate().map(line).collect()
        };
        match piece.kind {
            EdgeKind::Character => vec![self.character(rect, cx)],
            EdgeKind::Count => {
                let pill =
                    at(rect).debug_selector(|| "edge:count".to_string()).rounded(px(rect.h / 2.0)).bg(hsla(p.ink));
                vec![pill.overflow_hidden().children(runs(Some(p.bubble))).into_any_element()]
            }
            EdgeKind::Bubble => {
                let ground = at(rect).rounded(px(14.0)).bg(hsla(p.bubble)).border_1().border_color(hsla(p.bubble_ring));
                let texts =
                    at(rect).debug_selector(|| "edge:bubble".to_string()).overflow_hidden().children(runs(None));
                vec![ground.into_any_element(), texts.into_any_element()]
            }
            EdgeKind::Ring => {
                let (track, ink) = (p.bubble_ring, p.accent_dark);
                let paint = move |bounds: gpui::Bounds<gpui::Pixels>, _: (), window: &mut Window, _: &mut App| {
                    for (share, colour) in [(1.0, track), (share, ink)] {
                        for part in arc(rect.w, share) {
                            paint_polygon(&part, colour, bounds.origin, window);
                        }
                    }
                };
                let ring = canvas(|_, _, _| (), paint).absolute().size_full();
                vec![at(rect).debug_selector(|| "edge:ring".to_string()).child(ring).into_any_element()]
            }
        }
    }
}

/// A box at `rect`, from its window's top left.
fn at(rect: Rect) -> Div {
    div().absolute().left(px(rect.x)).top(px(rect.y)).w(px(rect.w)).h(px(rect.h))
}

/// One line of text inside the piece at `within`, clipped to it: nothing is painted past a bubble's edge.
fn text(theme: &Theme, run: &TextRun, n: usize, within: Rect, ink: Color) -> AnyElement {
    let seen = Rect::new(run.rect.x, run.rect.y, run.rect.w + 1.0, run.rect.h).clipped(&within);
    div()
        .debug_selector(move || format!("edge:text:{n}"))
        .absolute()
        .left(px(run.rect.x - within.x))
        .top(px(run.rect.y - within.y))
        .w(px(seen.w))
        .h(px(seen.h))
        .overflow_hidden()
        .line_height(px(run.rect.h))
        .whitespace_nowrap()
        .font_family(family(theme, run.role))
        .font_weight(weight(theme, run.role))
        .text_size(px(run.size))
        .text_color(hsla(ink))
        .child(run.text.clone())
        .into_any_element()
}

/// A ring `side` across, `share` of it from the top round to the right, as four-cornered parts. A share that is
/// no number is none, and one out of range is held to its end.
fn arc(side: f32, share: f32) -> Vec<[(f32, f32); 4]> {
    let share = if share.is_finite() { share.clamp(0.0, 1.0) } else { 0.0 };
    let (outer, whole) = (side / 2.0, share * RING_STEPS as f32);
    let point = |step: f32, radius: f32| {
        let angle = step / RING_STEPS as f32 * TAU - FRAC_PI_2;
        (outer + radius * angle.cos(), outer + radius * angle.sin())
    };
    let part = |from: f32| {
        let (to, inner) = ((from + 1.0).min(whole), outer - RING_WIDTH);
        [point(from, outer), point(to, outer), point(to, inner), point(from, inner)]
    };
    (0..RING_STEPS).map(|step| step as f32).take_while(|from| *from < whole).map(part).collect()
}

impl Render for EdgeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let measure = GlyphMeasure { text: window.text_system(), theme: &self.theme };
        let placed = layout_edge(&self.edge, &self.theme, self.corner, &measure);
        let share = placed.ring.unwrap_or(0.0);
        let pieces = match self.part {
            EdgePart::Character => placed.character.pieces,
            EdgePart::Bubble => placed.bubble.map(|window| window.pieces).unwrap_or_default(),
        };
        let painted: Vec<AnyElement> = pieces.iter().flat_map(|piece| self.piece(piece, share, cx)).collect();
        self.placed = pieces;
        div().size_full().relative().children(painted)
    }
}

#[cfg(test)]
mod tests {
    use super::{RING_STEPS, arc};

    #[test]
    fn a_ring_is_painted_from_the_top_as_far_as_its_share_and_no_further() {
        assert_eq!((arc(26.0, 1.0).len(), arc(26.0, 0.5).len(), arc(26.0, 0.0).len()), (RING_STEPS, 24, 0));
        assert_eq!((arc(26.0, 7.0).len(), arc(26.0, -1.0).len(), arc(26.0, f32::NAN).len()), (RING_STEPS, 0, 0));
        // A part of a step is painted as that part: a ring nearly empty is still seen.
        assert_eq!(arc(26.0, 0.001).len(), 1);
        let first = arc(26.0, 1.0)[0];
        assert!((first[0].0 - 13.0).abs() < 1e-4 && first[0].1.abs() < 1e-4, "it starts at the top: {first:?}");
        assert!(first[1].0 > 13.0, "and goes round to the right: {first:?}");
        let every = arc(26.0, 1.0);
        let inside = |(x, y): (f32, f32)| (-0.001..=26.001).contains(&x) && (-0.001..=26.001).contains(&y);
        assert!(every.iter().flatten().all(|point| inside(*point)), "no part leaves the ring's box");
        let half = arc(26.0, 0.5);
        assert!(half.iter().flatten().all(|point| point.0 >= 12.999), "half of it is its right half");
    }
}
