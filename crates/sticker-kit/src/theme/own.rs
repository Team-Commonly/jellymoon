//! The entry's colours in a theme file. A key the file does not set takes the file's own colour, never another
//! theme's beside its own: a file written before the key was is drawn in its own palette. And text that cannot be read on its
//! ground is not drawn so: the file loads, the colour is replaced, and one line says which key and by what.
//!
//! The built-in themes are made in code and are not touched here; `tests/colours.rs` holds them to the same floors.

use toml::Table;

use super::Palette;
use crate::color::Color;

/// The least contrast ([`Color::contrast`]) of the chosen chip's name on its chip: what WCAG 2 asks of text.
pub const CHOSEN_INK_FLOOR: f32 = 4.5;
/// The least contrast of the hint, and of words that wait, on the field. A hint is read once and is meant to be
/// pale, so this is under what text is held to: both built-in themes draw theirs near 3.
pub const HINT_FLOOR: f32 = 2.5;
/// The least contrast of the ring of a field that has the keys on the paper: what WCAG 2 asks of a line.
pub const RING_FLOOR: f32 = 3.0;

/// Two fills are close when no channel of theirs is this far apart: then a line tells the chosen chip.
const CLOSE: u8 = 48;

impl Palette {
    /// Gives each of the entry's keys that `set`, the file's `[palette]`, lacks the file's own colour, when the
    /// file has one: the field is its `option`, the field's edge its `bubble_ring`, the caret its `accent_dark`,
    /// a chip its `dimmed.light`, the chosen chip its `star`, that chip's name its `soft`, the hint its `quiet`,
    /// and the ring its caret's. These are what the sticker theme's own keys are, or were before each had a key.
    /// A file that sets neither a key nor the colour it is taken from is the sticker theme there, pair and all.
    pub(super) fn own(&mut self, set: Option<&Table>) {
        let has = |key: &str| set.is_some_and(|table| table.contains_key(key));
        let give = |key: &str, to: &mut Color, from: &str, own: Color| {
            if !has(key) && has(from) {
                *to = own;
            }
        };
        give("entry", &mut self.entry, "option", self.option);
        give("entry_edge", &mut self.entry_edge, "bubble_ring", self.bubble_ring);
        give("caret", &mut self.caret, "accent_dark", self.accent_dark);
        give("chip", &mut self.chip, "dimmed", self.dimmed.light);
        give("chip_chosen", &mut self.chip_chosen, "star", self.star);
        give("chip_chosen_ink", &mut self.chip_chosen_ink, "soft", self.soft);
        give("entry_hint", &mut self.entry_hint, "quiet", self.quiet);
        // The caret is the file's own when it set it, or set what it is taken from.
        give("entry_ring", &mut self.entry_ring, if has("caret") { "caret" } else { "accent_dark" }, self.caret);
    }

    /// Holds the entry's three texts to their floors. One under its floor is replaced by the first of the file's
    /// own colours that is over it, or by black or white, of which one always is; a line in `warnings` names the
    /// key. A file is never refused for this: one that loaded before the floors still loads.
    pub(super) fn readable(&mut self, warnings: &mut Vec<String>) {
        let paper = self.paper;
        let (chosen, field) = (self.chip_chosen.over(paper), self.entry.over(paper));
        let (inks, pale) = ([self.soft, self.ink], [self.quiet, self.soft, self.ink]);
        let lines = [self.caret, self.accent_dark, self.ink];
        let mut hold = |key: &str, text: &mut Color, ground: (&str, Color), floor: f32, own: &[Color]| {
            if text.contrast(ground.1) >= floor {
                return;
            }
            let ends = [Color::rgb(0x000000), Color::rgb(0xFFFFFF)];
            let Some(read) = own.iter().chain(&ends).find(|colour| colour.contrast(ground.1) >= floor) else {
                return;
            };
            let (was, on) = (written(*text), written(ground.1));
            warnings.push(format!(
                "palette.{key}: {was} on palette.{} {on} is under {floor} to 1; {} is used",
                ground.0,
                written(*read)
            ));
            *text = *read;
        };
        hold("chip_chosen_ink", &mut self.chip_chosen_ink, ("chip_chosen", chosen), CHOSEN_INK_FLOOR, &inks);
        hold("entry_hint", &mut self.entry_hint, ("entry", field), HINT_FLOOR, &pale);
        hold("entry_ring", &mut self.entry_ring, ("paper", paper), RING_FLOOR, &lines);
    }

    /// The edge of the chosen chip. Its own fill, so no line shows, when the fill is told from the other chips';
    /// when the two fills are close, the colour of its name, so that a line tells which chip is chosen.
    pub fn chosen_edge(&self) -> Color {
        let (chosen, other) = (self.chip_chosen.over(self.paper), self.chip.over(self.paper));
        let apart = |a: u8, b: u8| a.abs_diff(b) >= CLOSE;
        if apart(chosen.r, other.r) || apart(chosen.g, other.g) || apart(chosen.b, other.b) {
            self.chip_chosen
        } else {
            self.chip_chosen_ink
        }
    }
}

fn written(colour: Color) -> String {
    if colour.a == 255 { colour.hex_rgb() } else { format!("{}{:02X}", colour.hex_rgb(), colour.a) }
}
