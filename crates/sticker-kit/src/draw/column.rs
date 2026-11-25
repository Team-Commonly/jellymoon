//! The column in the view: how far it is moved up the window, what a turn of the wheel does, and the elements of
//! a piece, each drawn where the layout put it less that move. The entry's pieces are the window's and are never
//! moved; the rules of the move itself are plain data (`crate::scroll`).

use gpui::{AnyElement, Context, MouseButton, ScrollEvent, Window, div, prelude::*, px};

use super::Page;
use super::measure::{family, hsla, weight};
use super::shapes::Brush;
use super::stage::Pose;
use crate::geom::Rect;
use crate::layout::{PieceKind, Placed, TextRun, page_scale};
use crate::scene::{Note, Picture, Scene, speaker};
use crate::scroll::Scroll;
use crate::text::Role;

/// Whether the tally of `scene` is open, when it has one.
fn tally(scene: &Scene) -> Option<bool> {
    scene.notes.iter().find_map(|note| match note {
        Note::Drawn { picture: Picture::Tally { open, .. }, .. } => Some(*open),
        _ => None,
    })
}

/// The scroll of a page that opens on `scene`: at the column's end, or at its top when the scene has a tally,
/// which stands there.
pub(super) fn opening(scene: &Scene) -> Scroll {
    let mut scroll = Scroll::default();
    if tally(scene).is_some() {
        scroll.hold();
    }
    scroll
}

impl Page {
    /// `scene` is about to be shown in place of the one on show: when its tally was opened or closed, or it has
    /// one and the one on show has none, the view stays where it is (`Scroll::hold`).
    pub(super) fn hold_for(&mut self, scene: &Scene) {
        if tally(scene).is_some() && tally(scene) != tally(&self.scene) {
            self.scroll.hold();
        }
    }

    /// How far up the window `piece` is drawn from where the layout put it: the column's scroll, and nothing for a
    /// piece of the entry's band.
    pub(super) fn lift(&self, piece: &Placed) -> f32 {
        if piece.piece.in_band() { 0.0 } else { self.scroll.at() }
    }

    /// How far the column is moved up the window now: a piece of it is drawn that much higher than
    /// [`Page::placed`] has it, and a piece of the entry's band where it has it.
    pub fn scrolled(&self) -> f32 {
        self.scroll.at()
    }

    /// Where `piece` of [`Page::placed`] is in the window now.
    pub fn shown_at(&self, piece: &Placed) -> Rect {
        piece.rect.moved(0.0, -self.lift(piece))
    }

    /// A turn of the wheel anywhere on the page moves the column, between its top and its end.
    pub(super) fn wheeled(&mut self, event: &ScrollEvent, _: &mut Window, cx: &mut Context<Self>) {
        let line = px(20.0 * page_scale(&self.theme, self.viewport));
        let was = self.scroll;
        self.scroll.wheel(f32::from(event.delta.pixel_delta(line).y), self.most);
        if self.scroll != was {
            cx.notify();
        }
    }

    /// Whether a tap on `piece` is given to the page's handler: a character, an option, the entry's action, a
    /// drawn note's picture and its mark, and what the person typed for a character of the scene, with the
    /// small picture under it.
    pub(super) fn taps(&self, piece: &Placed) -> bool {
        match piece.piece {
            PieceKind::Hero | PieceKind::Character | PieceKind::Option | PieceKind::Action => true,
            // The mark of a picture is inside it: the picture's own target takes a tap on the mark, once.
            PieceKind::Picture => true,
            PieceKind::Quote | PieceKind::Small => {
                let said = |note: &Note| matches!(note, Note::Said { id, .. } if *id == piece.id);
                self.scene.notes.iter().any(said)
                    && self.scene.characters.iter().any(|who| who.id == speaker(&piece.id))
            }
            _ => false,
        }
    }

    pub(super) fn tap_target(&self, index: usize, piece: &Placed, pose: Pose, cx: &mut Context<Self>) -> AnyElement {
        let rect = piece.rect.moved(pose.dx, pose.dy);
        div()
            .id(("tap", index))
            .absolute()
            .left(px(rect.x))
            .top(px(rect.y))
            .w(px(rect.w))
            .h(px(rect.h))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |page, _, _, cx| {
                    page.pressed = Some(index);
                    cx.notify();
                }),
            )
            .on_click(cx.listener(move |page, _, window, cx| page.tap(index, window, cx)))
            .into_any_element()
    }
}

pub(super) fn text_element(brush: &Brush, piece: &Placed, n: usize, run: &TextRun, pose: Pose) -> AnyElement {
    let (theme, color) = (brush.theme, brush.ink(piece, run.role));
    let selector = format!("{:?}:{}:{}:{n}", piece.piece, piece.id, piece.index);
    let mut element = div()
        .debug_selector(move || selector)
        .absolute()
        .left(px(run.rect.x + pose.dx))
        .top(px(run.rect.y + pose.dy))
        .h(px(run.rect.h))
        .line_height(px(run.rect.h))
        .whitespace_nowrap()
        .font_family(family(theme, run.role))
        .font_weight(weight(theme, run.role))
        .text_size(px(run.size))
        .text_color(hsla(color));
    if matches!(run.role, Role::LabelStruck | Role::OptionStruck) {
        element = element.line_through();
    }
    element.child(run.text.clone()).into_any_element()
}
