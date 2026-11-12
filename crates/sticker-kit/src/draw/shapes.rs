//! Turns placed pieces into the flat shapes of the layer under the stickers.

use super::paint::{Shadow, Shape};
use super::stage::Pose;
use crate::color::Color;
use crate::geom::Rect;
use crate::layout::{PieceKind, Placed};
use crate::scene::{Mark, Note, Scene};
use crate::text::Role;
use crate::theme::{Stripe, Theme};

pub(crate) struct Brush<'a> {
    pub theme: &'a Theme,
    pub scene: &'a Scene,
    /// Page scale: design units to logical pixels.
    pub k: f32,
    /// The entry's chosen option, and whether the entry has the keys.
    pub chosen: Option<usize>,
    pub lit: bool,
}

impl Brush<'_> {
    fn shadow(&self, drop: f32, blur: f32, spread: f32) -> Shadow {
        Shadow { color: self.theme.palette.shadow, drop: drop * self.k, blur: blur * self.k, spread: spread * self.k }
    }

    fn disc(&self, centre: (f32, f32), radius: f32, fill: Color, edge: Option<(Color, f32)>) -> Shape {
        Shape::Slab {
            rect: Rect::new(centre.0 - radius, centre.1 - radius, 2.0 * radius, 2.0 * radius),
            radius,
            fill,
            edge,
            shadow: None,
            grid: None,
        }
    }

    fn note_colors(&self, index: usize) -> (Color, Stripe) {
        let p = &self.theme.palette;
        match self.scene.notes.get(index) {
            Some(Note::Choice { .. }) => (p.note_choice, p.tape_choice),
            Some(Note::Plain { .. }) => (p.note_plain, p.tape_plain),
            _ => (p.note_list, p.tape_list),
        }
    }

    /// Appends the shapes for one piece at its current pose.
    pub fn add(&self, piece: &Placed, pose: Pose, out: &mut Vec<Shape>) {
        let p = &self.theme.palette;
        let rect = piece.rect.moved(pose.dx, pose.dy);
        match piece.piece {
            PieceKind::Paper => out.push(Shape::Slab {
                rect,
                radius: self.theme.paper.radius * self.k,
                fill: p.paper,
                edge: Some((p.paper_edge, 1.0)),
                shadow: Some(self.shadow(24.0, 50.0, -20.0)),
                grid: self.theme.paper.grid.then_some((self.theme.paper.grid_step * self.k, p.grid)),
            }),
            PieceKind::Hole => {
                out.push(self.disc(rect.center(), rect.w / 2.0, p.hole, Some((p.paper_edge, 1.0))));
            }
            PieceKind::Tape => {
                let stripe = if piece.id == "date" && piece.z < PieceKind::Tape.z() {
                    p.tape_date
                } else {
                    self.note_colors(piece.index).1
                };
                out.push(Shape::Tape { rect, tilt: piece.tilt_degrees, a: stripe.a, b: stripe.b });
            }
            PieceKind::Note => out.push(Shape::Sheet {
                rect,
                tilt: piece.tilt_degrees,
                fill: self.note_colors(piece.index).0,
                shadow: self.shadow(9.0, 18.0, -3.0),
            }),
            PieceKind::Label => out.push(Shape::Slab {
                rect,
                radius: rect.h / 2.0,
                fill: p.tag,
                edge: None,
                shadow: Some(self.shadow(3.0, 7.0, -1.0)),
                grid: None,
            }),
            PieceKind::Option => out.push(self.option(piece, rect.scaled(pose.scale))),
            PieceKind::Legend => {
                if let Some(item) = self.scene.legend.get(piece.index) {
                    self.mark(item.mark, rect, out);
                }
            }
            PieceKind::Mark => {
                let note = |(n, _): &(usize, &Note)| format!("note-{n}") == piece.id;
                if let Some((_, Note::Whole { rows, .. })) = self.scene.notes.iter().enumerate().find(note)
                    && let Some((mark, _)) = rows.get(piece.index)
                {
                    self.mark(*mark, rect, out);
                }
            }
            // The three are told apart by more than a shade: the field with the keys has a ring, the chosen chip
            // is filled, and the hint is grey ([`Brush::ink`]).
            PieceKind::Entry if self.lit => out.push(self.pill(rect, p.entry, p.entry_ring)),
            PieceKind::Entry => out.push(self.pill(rect, p.entry, p.entry_edge)),
            PieceKind::Chip if self.chosen == Some(piece.index) => {
                out.push(self.pill(rect, p.chip_chosen, p.chip_chosen));
            }
            PieceKind::Chip => out.push(self.pill(rect, p.chip, p.chip)),
            _ => {}
        }
    }

    /// The colour a piece's text is painted in.
    pub fn ink(&self, piece: &Placed, role: Role) -> Color {
        let p = &self.theme.palette;
        match (piece.piece, role) {
            (PieceKind::Entry, _) => p.entry_hint,
            (PieceKind::Chip, _) if self.chosen == Some(piece.index) => p.chip_chosen_ink,
            (_, Role::LabelStruck | Role::Body | Role::Legend) => p.soft,
            (_, Role::RowSub | Role::OptionStruck | Role::Footer) => p.quiet,
            (_, Role::ChoiceTitle) => p.choice_title,
            (_, Role::ChoiceFooter) => p.choice_footer,
            (_, Role::Bullet) => p.star,
            _ => p.ink,
        }
    }

    fn pill(&self, rect: Rect, fill: Color, edge: Color) -> Shape {
        let edge = Some((edge, 1.5 * self.k));
        Shape::Slab { rect, radius: rect.h / 2.0, fill, edge, shadow: None, grid: None }
    }

    fn option(&self, piece: &Placed, rect: Rect) -> Shape {
        let p = &self.theme.palette;
        let struck = matches!(
            self.scene.notes.iter().find(|n| matches!(n, Note::Choice { id, .. } if *id == piece.id)),
            Some(Note::Choice { struck_first: true, .. })
        ) && piece.index == 0;
        let mut fill = p.option;
        if struck {
            fill.a = (f32::from(fill.a) * 0.55) as u8;
        }
        Shape::Slab {
            rect,
            radius: (16.0 * self.k).min(rect.h / 2.0),
            fill,
            edge: None,
            shadow: (!struck).then_some(Shadow { color: p.option_edge, drop: 2.0 * self.k, blur: 0.0, spread: 0.0 }),
            grid: None,
        }
    }

    /// A mark at the left of `rect`, as the legend draws it. A star is text, laid out with the rest.
    fn mark(&self, mark: Mark, rect: Rect, out: &mut Vec<Shape>) {
        let (p, k) = (&self.theme.palette, self.k);
        let centre = (rect.x + 10.0 * k, rect.y + rect.h / 2.0);
        match mark {
            Mark::Star => {}
            Mark::Busy => {
                out.push(self.disc(centre, 10.0 * k, p.bubble, Some((p.bubble_ring, 2.0 * k))));
                for dx in [-3.6, 0.0, 3.6] {
                    out.push(self.disc((centre.0 + dx * k, centre.1), 1.2 * k, p.soft, None));
                }
            }
            Mark::NeedsYou => out.push(self.disc(centre, 7.0 * k, p.accent, None)),
            Mark::Dimmed => out.push(self.disc(centre, 7.0 * k, p.dimmed.dark, None)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::stage::REST;
    use crate::geom::Viewport;
    use crate::layout::layout_entry;
    use crate::scene::Entry;
    use crate::text::FixedAdvance;

    /// The fill and the edge of each of the entry's shapes, and the colour of its text, for an entry with three
    /// chips of which the second is chosen.
    fn painted(theme: &Theme, lit: bool) -> Vec<(Color, Color, Color)> {
        let entry = Entry { id: "wish".into(), hint: "wish".into(), options: vec!["a".into(); 3], most: 0 };
        let placed = layout_entry(&entry, theme, Viewport { w: 1440.0, h: 900.0 }, &FixedAdvance(0.5));
        let brush = Brush { theme, scene: &Scene::default(), k: 1.0, chosen: Some(1), lit };
        let colours = |piece: &Placed| {
            let mut out = Vec::new();
            brush.add(piece, REST, &mut out);
            let [Shape::Slab { fill, edge: Some((edge, _)), .. }] = out[..] else { return None };
            Some((fill, edge, brush.ink(piece, piece.text.first()?.role)))
        };
        placed.iter().filter_map(colours).collect()
    }

    #[test]
    fn the_hint_the_chosen_chip_and_the_ring_are_painted_in_the_theme_s_three_colours() {
        for theme in [Theme::sticker(), Theme::plain()] {
            let p = &theme.palette;
            let chip = (p.chip, p.chip, p.soft);
            let chosen = (p.chip_chosen, p.chip_chosen, p.chip_chosen_ink);
            assert_eq!(painted(&theme, true), [(p.entry, p.entry_ring, p.entry_hint), chip, chosen, chip]);
            assert_eq!(painted(&theme, false)[0], (p.entry, p.entry_edge, p.entry_hint), "no ring without the keys");
        }
    }
}
