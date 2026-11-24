//! A click means what was seen. A page changes while a pointer is on its way: a card comes above a choice and
//! pushes it down the column, or a card comes right where another choice stood. The view remembers, for each piece
//! of a note that takes a tap (an option, a drawn note's picture with its `?`), where it stands in the COLUMN and
//! since when, and a click on one that has just moved there, or has just come where something else took a tap,
//! is no tap for [`SETTLE`]. The place is the column's and never the window's: a turn of the wheel and a column
//! held at its end are the person's own moves. A click that was no tap is not kept for later and nothing is
//! drawn for it. Characters, bubbles, chips and the entry are not held back.
//!
//! The words a tap on an option carries are kept here too: the scene's own string as it was when the pieces now
//! on the page were laid out, so they are the words that were drawn.

use std::collections::HashMap;
use std::time::Duration;

use gpui::{Context, Window};

use super::{Page, Tap};
use crate::geom::Rect;
use crate::layout::{PieceKind, Placed};
use crate::scene::{PICTURE_PLACE, Scene, speaker};

/// How long a choice that has just moved, or has just come where another piece took a tap, takes none.
pub const SETTLE: Duration = Duration::from_millis(400);

/// How far a piece may move in the column, in design units, and still take a tap at once.
pub(super) const SLACK: f32 = 4.0;

/// A piece across frames: its kind, its note's id and which one of them it is.
type Key = (PieceKind, String, usize);

struct Stood {
    /// Where it stands in the column.
    at: (f32, f32),
    /// The page's time at which it last moved there, and at which it came onto the page.
    moved: f32,
    came: f32,
    /// Where in the window pieces that took a tap were, in the frame before it came, as far as they are under it.
    over: Vec<Rect>,
    /// An option's words, as the scene laid out held them.
    words: String,
}

#[derive(Default)]
pub(super) struct Settled {
    stood: HashMap<Key, Stood>,
    /// Where in the window each piece that takes a tap was drawn in the last frame.
    pub targets: Vec<Rect>,
    /// The page's time, when it was told one.
    told: Option<f32>,
}

fn guarded(piece: &Placed) -> bool {
    matches!(piece.piece, PieceKind::Option | PieceKind::Picture)
}

fn key(piece: &Placed) -> Key {
    (piece.piece, piece.id.clone(), piece.index)
}

impl Settled {
    /// Takes a new layout of `scene` at the page's time `now`, drawn `lift` up the window; `slack` is [`SLACK`] in
    /// logical pixels. With `counts` a piece that is somewhere else in the column has just moved; without it (the
    /// window changed size, which is the person's own move) it is only found again.
    pub fn meet(&mut self, placed: &[Placed], scene: &Scene, (lift, slack): (f32, f32), now: f32, counts: bool) {
        let mut was = std::mem::take(&mut self.stood);
        for piece in placed.iter().filter(|piece| guarded(piece)) {
            let at = (piece.rect.x, piece.rect.y);
            let words = match piece.piece {
                PieceKind::Option => scene.option(&piece.id, piece.index).unwrap_or_default().to_string(),
                _ => String::new(),
            };
            let stood = match was.remove(&key(piece)) {
                Some(old) => {
                    let far = (old.at.0 - at.0).abs().max((old.at.1 - at.1).abs()) > slack;
                    Stood { at, moved: if far && counts { now } else { old.moved }, words, ..old }
                }
                None => {
                    let shown = piece.rect.moved(0.0, -lift);
                    let over = self.targets.iter().filter(|target| target.overlaps(&shown)).copied().collect();
                    Stood { at, moved: f32::NEG_INFINITY, came: now, over, words }
                }
            };
            self.stood.insert(key(piece), stood);
        }
    }

    /// Whether a click on `piece` at the window's point `at` is no tap at the page's time `now`.
    fn swallows(&self, piece: &Placed, at: (f32, f32), now: f32) -> bool {
        let Some(stood) = self.stood.get(&key(piece)).filter(|_| guarded(piece)) else {
            return false;
        };
        let inside = |rect: &Rect| rect.x <= at.0 && at.0 < rect.right() && rect.y <= at.1 && at.1 < rect.bottom();
        let settle = SETTLE.as_secs_f32();
        now - stood.moved < settle || (now - stood.came < settle && stood.over.iter().any(inside))
    }

    fn words(&self, piece: &Placed) -> String {
        self.stood.get(&key(piece)).map(|stood| stood.words.clone()).unwrap_or_default()
    }
}

impl Page {
    /// The page's time for what settles is `seconds`, in every frame, and not the clock's; with None, the clock's
    /// again. It is for a test, which cannot wait for a clock.
    pub fn set_settle_time(&mut self, seconds: Option<f32>) {
        self.settled.told = seconds;
    }

    /// The page's time: the seconds since it was made, by the clock its drawing goes by, or the time it was told.
    pub(super) fn settle_time(&self) -> f32 {
        self.settled.told.unwrap_or_else(|| self.started.elapsed().as_secs_f32())
    }

    /// A click on the piece `index` of the layout: a tap for the page's handler, unless the piece has not settled.
    pub(super) fn tap(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(piece) = self.placed.get(index) else {
            return;
        };
        let pointer = window.mouse_position();
        if self.settled.swallows(piece, (f32::from(pointer.x), f32::from(pointer.y)), self.settle_time()) {
            return;
        }
        let note_id = piece.id.clone();
        let tap = match piece.piece {
            PieceKind::Option => Tap::Option { note_id, index: piece.index, words: self.settled.words(piece) },
            PieceKind::Action => Tap::Action { id: note_id },
            PieceKind::Picture => Tap::Option { note_id, index: PICTURE_PLACE, words: String::new() },
            // A tap on what the person typed is a tap on the character it is of.
            PieceKind::Quote => Tap::Character { id: speaker(&piece.id).to_string() },
            _ => Tap::Character { id: note_id },
        };
        if let Some(handler) = &self.on_tap {
            handler(&tap, window, cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::piece;

    fn option(id: &str, y: f32) -> Placed {
        piece(PieceKind::Option, id, 0, Rect::new(100.0, y, 200.0, 40.0))
    }

    #[test]
    fn a_piece_that_moved_in_the_column_or_came_over_a_target_settles_and_a_resize_moves_nothing() {
        let (scene, mut settled) = (Scene::default(), Settled::default());
        let inside = (150.0, 320.0);
        settled.meet(&[option("a", 300.0)], &scene, (0.0, 4.0), 1.0, true);
        assert!(!settled.swallows(&option("a", 300.0), inside, 1.0), "new where nothing took a tap");
        settled.meet(&[option("a", 304.0)], &scene, (0.0, 4.0), 2.0, true);
        assert!(!settled.swallows(&option("a", 304.0), inside, 2.0), "four units are no move");
        settled.meet(&[option("a", 390.0)], &scene, (0.0, 4.0), 3.0, false);
        assert!(!settled.swallows(&option("a", 390.0), inside, 3.0), "the window changed size");
        settled.meet(&[option("a", 300.0)], &scene, (0.0, 4.0), 4.0, true);
        assert!(settled.swallows(&option("a", 300.0), inside, 4.3), "it has just moved");
        assert!(!settled.swallows(&option("a", 300.0), inside, 4.5), "the settle time is over");
        // A new piece over where another took a tap, and only at the points the other was.
        settled.targets = vec![Rect::new(100.0, 310.0, 50.0, 20.0)];
        settled.meet(&[option("b", 300.0)], &scene, (0.0, 4.0), 5.0, true);
        assert!(settled.swallows(&option("b", 300.0), (120.0, 315.0), 5.1));
        assert!(!settled.swallows(&option("b", 300.0), (220.0, 315.0), 5.1), "nothing stood at this point");
        assert!(!settled.swallows(&option("b", 300.0), (120.0, 315.0), 5.5));
        assert!(!settled.swallows(&option("a", 300.0), inside, 5.0), "a piece that left the page is forgotten");
    }
}
