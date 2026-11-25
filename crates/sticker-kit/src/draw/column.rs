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
use crate::text::{Role, has_visible};

/// Whether the tally of `scene` is open, when it has one.
fn tally(scene: &Scene) -> Option<bool> {
    scene.notes.iter().find_map(|note| match note {
        Note::Drawn { picture: Picture::Tally { open, .. }, .. } => Some(*open),
        _ => None,
    })
}

/// How many things a person said `scene` holds.
fn said(scene: &Scene) -> usize {
    scene.notes.iter().filter(|note| matches!(note, Note::Said { .. })).count()
}

/// The scroll of a page that opens on `scene`: at the column's end, or at its top when the scene has a tally,
/// which stands there, and nothing a person said, which stands at the end and may wait for them.
pub(super) fn opening(scene: &Scene) -> Scroll {
    let mut scroll = Scroll::default();
    if tally(scene).is_some() && said(scene) == 0 {
        scroll.hold();
    }
    scroll
}

/// What the view does when `scene` takes the place of `shown`.
#[derive(Debug, PartialEq)]
pub(super) enum Turn {
    /// The rules of the scroll as they are.
    Stay,
    /// The view stays where it is (`Scroll::hold`): the tally was opened or closed under the pointer, or it
    /// came to a page that holds nothing a person said.
    Hold,
    /// A view no wheel moved goes to the column's end (`Scroll::entered`): the scene holds one more thing a
    /// person said, wherever it was said (the entry, a note's field, an option, another program).
    Follow,
}

/// `typing` is whether a field of a note holds words the person is typing: the page never moves away from them
/// by itself, so nothing is followed then.
pub(super) fn turn(shown: &Scene, scene: &Scene, typing: bool) -> Turn {
    let (was, is) = (tally(shown), tally(scene));
    if said(scene) > said(shown) && !typing {
        Turn::Follow
    } else if is.is_some() && (was.is_some() && was != is || was.is_none() && said(scene) == 0) {
        Turn::Hold
    } else {
        Turn::Stay
    }
}

impl Page {
    /// `scene` is about to be shown in place of the one on show: the view is held, follows the end or is left
    /// to the scroll's rules ([`turn`]).
    pub(super) fn hold_for(&mut self, scene: &Scene) {
        let typing = self.fields.iter().any(|field| has_visible(field.state.text()));
        match turn(&self.scene, scene, typing) {
            Turn::Hold => self.scroll.hold(),
            Turn::Follow => self.scroll.entered(),
            Turn::Stay => {}
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
            // The mark of a picture is inside it: the picture's own target takes a tap on the mark, once. A
            // picture that takes none (`Picture::taps`) is no target at all.
            PieceKind::Picture => !self
                .scene
                .notes
                .iter()
                .any(|note| matches!(note, Note::Drawn { id, picture, .. } if *id == piece.id && !picture.taps())),
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

#[cfg(test)]
mod tests {
    use super::{Turn, opening, turn};
    use crate::scene::{Note, Picture, Scene};
    use crate::scroll::Scroll;

    fn tally(open: bool) -> Note {
        let picture =
            Picture::Tally { title: "t".into(), rows: Vec::new(), more: String::new(), under: String::new(), open };
        Note::Drawn { id: "day".into(), picture, words: Vec::new(), open: false, options: Vec::new(), field: None }
    }

    fn said(id: &str) -> Note {
        Note::Said { id: id.into(), text: "a wish".into(), answer: String::new(), small: None }
    }

    fn scene(notes: Vec<Note>) -> Scene {
        Scene { notes, ..Scene::default() }
    }

    fn opens_at(scene: &Scene) -> f32 {
        let mut scroll = opening(scene);
        scroll.fit(300.0);
        scroll.at()
    }

    #[test]
    fn a_page_opens_at_its_top_on_a_tally_alone_and_at_its_end_when_a_person_said_something() {
        assert_eq!(opens_at(&scene(vec![tally(false)])), 0.0, "the tally stands at the top");
        assert_eq!(opens_at(&scene(vec![said("m-1"), tally(false)])), 300.0, "what they said may wait at the end");
        assert_eq!(opens_at(&scene(vec![said("m-1")])), 300.0, "as a page with no tally always did");
    }

    #[test]
    fn one_more_thing_said_is_followed_wherever_it_was_said_and_a_turned_tally_holds_the_view() {
        let day = scene(vec![said("m-1"), tally(false)]);
        let more = scene(vec![said("m-1"), said("m-2"), tally(false)]);
        assert_eq!(turn(&day, &more, false), Turn::Follow);
        assert_eq!(turn(&more, &day, false), Turn::Stay, "one fewer is not followed");
        assert_eq!(
            turn(&day, &scene(vec![said("m-1"), tally(true)]), false),
            Turn::Hold,
            "the tally opened under the pointer"
        );
        assert_eq!(
            turn(&scene(Vec::new()), &scene(vec![tally(false)]), false),
            Turn::Hold,
            "a reading came to an empty page"
        );
        assert_eq!(
            turn(&scene(vec![said("m-1")]), &day, false),
            Turn::Stay,
            "a reading that comes over a made wish moves nothing"
        );
        assert_eq!(turn(&day, &day, false), Turn::Stay);
        assert_eq!(turn(&day, &more, true), Turn::Stay, "the page does not leave words that are being typed");
        let (opened, both) =
            (scene(vec![said("m-1"), tally(true)]), scene(vec![said("m-1"), said("m-2"), tally(true)]));
        assert_eq!(turn(&day, &both, true), Turn::Hold, "and a tally turned meanwhile still holds the view");
        assert_eq!(turn(&day, &opened, true), Turn::Hold);
        // Followed means: a held view no wheel moved is at the end at the next layout.
        let mut scroll = Scroll::default();
        scroll.hold();
        scroll.fit(300.0);
        scroll.entered();
        scroll.fit(420.0);
        assert_eq!(scroll.at(), 420.0);
    }
}
