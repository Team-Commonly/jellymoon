//! The pencil in the view: when each picture first came onto the page, how much of it that has drawn, and its
//! strokes as shapes. A picture draws itself once, from the frame its note is first laid out in; after its time
//! nothing of it moves but the tool at work, which rocks by the theme's motion. With motion reduced or a theme
//! that is still, every stroke is whole and nothing rocks. A page told a time ([`Page::sketched_at`]) draws every
//! picture as it is that long after it came, whatever the clock says: that is for a picture file of a half-drawn
//! page.

use std::collections::HashMap;

use super::Page;
use super::paint::Shape;
use super::stage::Pose;
use crate::color::Color;
use crate::layout::Placed;
use crate::motion::rock;
use crate::pencil::{Ink, SOFT, Sketch};
use crate::theme::{Motion, Palette};

/// How far the tool at work rocks to each side, in design units.
const ROCK: f32 = 3.0;

#[derive(Default)]
pub(super) struct Sketched {
    /// The page's time at which each picture was first laid out, by its note's id and what kind of picture it is.
    seen: HashMap<String, f32>,
    /// The time every picture is drawn at, when the page was told one.
    at: Option<f32>,
}

fn key(piece: &Placed, sketch: &Sketch) -> String {
    format!("{:?} {} {}", piece.piece, piece.id, sketch.what.split(' ').next().unwrap_or_default())
}

impl Sketched {
    /// Takes a new layout at the page's time `now`: a picture not seen before begins to draw itself, and one
    /// that is gone is forgotten.
    pub fn meet(&mut self, placed: &[Placed], now: f32) {
        let keys = placed.iter().filter_map(|piece| Some(key(piece, piece.sketch.as_ref()?)));
        let was = std::mem::take(&mut self.seen);
        self.seen = keys.map(|key| (key.clone(), was.get(&key).copied().unwrap_or(now))).collect();
    }

    /// How long `piece`'s picture has been drawing itself at the page's time `t`. None when every stroke is whole
    /// whatever the time: with nothing moving.
    fn since(&self, piece: &Placed, sketch: &Sketch, t: f32, still: bool) -> Option<f32> {
        if self.at.is_some() {
            return self.at;
        }
        let seen = self.seen.get(&key(piece, sketch)).copied().unwrap_or(t);
        (!still).then_some((t - seen).max(0.0))
    }

    /// Whether a picture of `placed` needs another frame: one still drawing itself, or one whose tool rocks.
    pub fn moving(&self, placed: &[Placed], t: f32, still: bool, motion: &Motion) -> bool {
        let moves = |piece: &Placed| {
            let Some(sketch) = &piece.sketch else { return false };
            let rocks = !motion.is_idle_still() && sketch.strokes.iter().any(|stroke| stroke.rocks);
            self.since(piece, sketch, t, still).is_some_and(|since| since < sketch.seconds() || rocks)
        };
        self.at.is_none() && placed.iter().any(moves)
    }

    /// Appends the strokes of `piece`, as far as they are drawn at the page's time `t`, to `out`.
    pub fn add(
        &self,
        piece: &Placed,
        (t, still, k): (f32, bool, f32),
        pose: Pose,
        page: (&Palette, &Motion),
        out: &mut Vec<Shape>,
    ) {
        let Some(sketch) = &piece.sketch else { return };
        let (palette, motion) = page;
        let since = self.since(piece, sketch, t, still);
        let drawn = since.is_some_and(|since| since >= sketch.seconds());
        let sway = if drawn { since.map_or(0.0, |since| rock(motion, since)) * ROCK * k } else { 0.0 };
        for stroke in &sketch.strokes {
            let Some([soft, line]) = stroke.passes(since) else { continue };
            let dx = pose.dx + if stroke.rocks { sway } else { 0.0 };
            let moved = |points: Vec<(f32, f32)>| points.into_iter().map(|(x, y)| (x + dx, y + pose.dy)).collect();
            let ink = match stroke.ink {
                Ink::Pencil => palette.pencil,
                Ink::Added => palette.added,
                Ink::Taken => palette.taken,
                Ink::Quiet => palette.quiet,
            };
            let pass = |share: f32| Color { a: (f32::from(ink.a) * share) as u8, ..ink };
            out.push(Shape::Stroke { points: moved(soft), width: stroke.width * SOFT, color: pass(0.2) });
            out.push(Shape::Stroke { points: moved(line), width: stroke.width, color: pass(0.86) });
        }
    }
}

impl Page {
    /// Draws every picture as it is `seconds` after its note came, in every frame; with None, by the clock.
    pub fn sketched_at(mut self, seconds: Option<f32>) -> Self {
        self.sketched.at = seconds;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Rect;
    use crate::layout::{PieceKind, piece};
    use crate::pencil::Pencil;
    use crate::theme::Theme;

    fn picture(id: &str, rocks: bool) -> Placed {
        let mut pencil = Pencil::new(id, 1.0);
        pencil.line((0.0, 0.0), (40.0, 0.0));
        pencil.rocking(rocks);
        pencil.line((0.0, 9.0), (40.0, 9.0));
        let mut placed = piece(PieceKind::Picture, id, 0, Rect::new(0.0, 0.0, 40.0, 10.0));
        placed.sketch = Some(Sketch { what: "track steps=2".into(), parts: Vec::new(), strokes: pencil.finish() });
        placed
    }

    fn strokes(sketched: &Sketched, piece: &Placed, t: f32, still: bool, theme: &Theme) -> Vec<Shape> {
        let mut out = Vec::new();
        sketched.add(piece, (t, still, 1.0), crate::draw::stage::REST, (&theme.palette, &theme.motion), &mut out);
        out
    }

    #[test]
    fn a_picture_draws_itself_from_when_it_came_and_is_whole_at_once_with_nothing_moving() {
        let (theme, plain) = (Theme::sticker(), Theme::plain());
        let (old, new) = (picture("old", true), picture("new", false));
        let mut sketched = Sketched::default();
        sketched.meet(std::slice::from_ref(&old), 2.0);
        sketched.meet(&[old.clone(), new.clone()], 10.0);
        // Each pass of each stroke is a shape: two strokes, four shapes.
        assert_eq!(strokes(&sketched, &new, 10.0, false, &theme).len(), 0, "nothing of it at the frame it came in");
        assert_eq!(strokes(&sketched, &new, 10.2, false, &theme).len(), 2, "its first stroke, and not yet its second");
        assert_eq!(strokes(&sketched, &new, 12.0, false, &theme).len(), 4);
        assert_eq!(strokes(&sketched, &old, 10.0, false, &theme).len(), 4, "a picture seen before is not drawn again");
        assert_eq!(strokes(&sketched, &new, 10.0, true, &theme).len(), 4, "with motion reduced it is whole at once");
        assert!(sketched.moving(std::slice::from_ref(&new), 10.2, false, &theme.motion));
        assert!(!sketched.moving(std::slice::from_ref(&new), 12.0, false, &theme.motion), "drawn, it is still");
        assert!(!sketched.moving(&[old.clone(), new.clone()], 10.2, true, &theme.motion));
        // The tool at work rocks on, by the theme's motion and never with it reduced or in a still theme.
        let xs = |shapes: Vec<Shape>| -> Vec<f32> {
            let first = |shape: &Shape| match shape {
                Shape::Stroke { points, .. } => points[0].0,
                _ => f32::NAN,
            };
            shapes.iter().map(first).collect()
        };
        let (rest, rocked) =
            (xs(strokes(&sketched, &old, 10.0, true, &theme)), xs(strokes(&sketched, &old, 10.3, false, &theme)));
        assert!(rest[..2] == rocked[..2] && rest[3] != rocked[3], "only the tool moves: {rest:?} {rocked:?}");
        assert!((rest[3] - rocked[3]).abs() <= ROCK);
        assert!(sketched.moving(std::slice::from_ref(&old), 10.0, false, &theme.motion));
        assert_eq!(
            xs(strokes(&sketched, &old, 10.3, false, &plain)),
            rest,
            "a theme that is still when idle rocks nothing"
        );
        assert!(!sketched.moving(std::slice::from_ref(&old), 10.0, false, &plain.motion));
        // A page told a time draws that time, whatever the clock and the motion say.
        sketched.at = Some(0.2);
        assert_eq!(strokes(&sketched, &old, 99.0, true, &theme).len(), 2);
        assert!(!sketched.moving(std::slice::from_ref(&old), 99.0, false, &theme.motion));
    }
}
