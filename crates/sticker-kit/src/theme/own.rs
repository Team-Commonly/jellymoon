//! The entry's colours in a theme file. A file never draws a pair of them under its floor for a key it did
//! not set: such a key takes the file's own colour only when the pair that makes meets its floor, and is the
//! sticker theme's otherwise. Over its floor a pair may still be drawn worse than before: the file's own colours
//! are the file's. And no text of the entry is drawn where it cannot be read: the file loads, a colour under its
//! floor is replaced, and for a key the file set one line says which and by what.
//!
//! The built-in themes are made in code and are not touched here; `tests/colours.rs` holds them to the same floors.

use toml::{Table, Value};

use super::Palette;
use crate::color::Color;

/// The least contrast ([`Color::contrast`]) of the chosen chip's name on its chip: what WCAG 2 asks of text.
pub const CHOSEN_INK_FLOOR: f32 = 4.5;
/// The least contrast of the hint, and of words that wait, on the field. A hint is read once and is meant to be
/// pale, so this is under what text is held to: both built-in themes draw theirs near 3.
pub const HINT_FLOOR: f32 = 2.5;
/// The least contrast of the ring of a field that has the keys on the paper: what WCAG 2 asks of a line.
pub const RING_FLOOR: f32 = 3.0;
/// The least contrast of a chip's name, the palette's `soft`, on a chip that is not the chosen one: what WCAG 2
/// asks of text. Both built-in themes draw it over 5.
pub const CHIP_INK_FLOOR: f32 = 4.5;
/// The least contrast of what is typed, the palette's `ink`, on the field: what WCAG 2 asks of text. Both built-in
/// themes draw it over 14.
pub const TYPED_FLOOR: f32 = 4.5;
/// The least contrast of the caret on the field: what WCAG 2 asks of a line. The sticker theme draws it at 3.67.
pub const CARET_FLOOR: f32 = 3.0;
/// The least contrast of the chosen chip's fill with the other chips' and with what is under it for the fill
/// alone to tell which chip is chosen; under it a line does. The sticker theme's yellow on its lilac chips is 1.22
/// and is told by its fill, so the floor is just under that.
pub const CHOSEN_FILL_FLOOR: f32 = 1.2;

/// One colour of a pair of the entry, to be settled against the pair's other colour, which is settled.
struct Side<'a> {
    key: &'a str,
    /// Whether the file set `key`.
    set: bool,
    /// The other colour's key, and the colour.
    other: (&'a str, Color),
    /// Whether this colour is the ground the other is drawn on, or is what is drawn on the other.
    ground: bool,
    floor: f32,
    /// What is under the pair, opaque.
    under: Color,
}

impl Side<'_> {
    fn reads(&self, colour: Color) -> bool {
        let (text, ground) = if self.ground { (self.other.1, colour) } else { (colour, self.other.1) };
        text.contrast(ground.over(self.under)) >= self.floor
    }

    /// The colour for the key. `own`, the file's own colour for a key it did not set, when the pair reads with
    /// it; else `now`, what the palette holds (the file's, or the sticker theme's), when the pair reads with that;
    /// else the first of `spare`, black and white the pair reads with, and a line in `warnings` when the file set
    /// the key. `now` when nothing reads, which only a ground under text one sees through can do: that text is
    /// then settled itself ([`Side::text`]).
    fn settle(&self, now: Color, own: Option<Color>, spare: &[Color], warnings: &mut Vec<String>) -> Color {
        if let Some(own) = own.filter(|own| !self.set && self.reads(*own)) {
            return own;
        }
        if self.reads(now) {
            return now;
        }
        let ends = [Color::rgb(0x000000), Color::rgb(0xFFFFFF)];
        let Some(read) = spare.iter().chain(&ends).copied().find(|colour| self.reads(*colour)) else {
            return now;
        };
        if self.set {
            let (key, floor, other, used) = (self.key, self.floor, self.other.0, written(read));
            let was = written(now);
            warnings.push(if self.ground {
                let text = written(self.other.1);
                format!("palette.{key}: palette.{other} {text} on it, {was}, is under {floor} to 1; {used} is used")
            } else {
                let on = written(self.other.1.over(self.under));
                format!("palette.{key}: {was} on palette.{other} {on} is under {floor} to 1; {used} is used")
            });
        }
        read
    }

    /// The text of a pair whose ground is settled, for the `ink` and the `soft`, which a ground gives way to. A
    /// text one sees through is measured as it is drawn, laid on its ground ([`Color::contrast`]), and may be
    /// under its floor on every ground: then it is replaced as any key is, by itself with nothing seen through
    /// where that reads, and said when the file set it.
    fn text(&self, now: Color, warnings: &mut Vec<String>) -> Color {
        self.settle(now, None, &[Color { a: 255, ..now }], warnings)
    }
}

impl Palette {
    /// What the page's pieces are drawn on, opaque. A paper one sees through is laid on the desk, which is what
    /// the kit draws under it; a desk one sees through is laid on white, since the kit does not know what a
    /// window has under its page and has to name one ground.
    pub fn ground(&self) -> Color {
        self.paper.over(self.desk.over(Color::rgb(0xFFFFFF)))
    }

    /// Settles the entry's colours for a theme file whose `[palette]` is `set`. A key the file lacks takes the
    /// file's own colour when the file sets that and the pair reads with it: the field is its `option` under its
    /// `ink`, the caret its `accent_dark` and the hint its `quiet` on the field, the ring its caret's on the
    /// paper, a chip its `dimmed.light` under its `soft`, and the chosen chip and its name its `star` and its
    /// `soft`, both or neither. The field's edge, which no text is on, is its `bubble_ring`. Otherwise the key is
    /// the sticker theme's, as before the file's own colours were taken, and nothing is said.
    ///
    /// Then each text of the entry is held to its floor on its ground: one under it is replaced by the first of
    /// the palette's colours that is over it, or by black or white. The field and a chip give way to the `ink`
    /// and the `soft` on them, which are the whole page's; an `ink` or a `soft` one sees through, which no ground
    /// is read under, is replaced for the whole page. A file is never refused for this, and only a key the file
    /// set is named in `warnings`.
    pub(super) fn own(&mut self, set: Option<&Table>, warnings: &mut Vec<String>) {
        let has = |key: &str| set.is_some_and(|table| table.contains_key(key));
        let (under, desk) = (self.ground(), self.desk.over(Color::rgb(0xFFFFFF)));
        let side = |key, other, ground, floor| Side { key, set: has(key), other, ground, floor, under };
        let own = |from: &str, colour: Color| has(from).then_some(colour);
        if !has("entry_edge") && has("bubble_ring") {
            self.entry_edge = self.bubble_ring;
        }
        let grounds = [self.option, self.paper, self.ink];
        self.entry = side("entry", ("ink", self.ink), true, TYPED_FLOOR).settle(
            self.entry,
            own("option", self.option),
            &grounds,
            warnings,
        );
        let (field, before) = (("entry", self.entry), self.caret);
        self.ink = side("ink", field, false, TYPED_FLOOR).text(self.ink, warnings);
        let lines = [self.accent_dark, self.ink];
        self.caret = side("caret", field, false, CARET_FLOOR).settle(
            self.caret,
            own("accent_dark", self.accent_dark),
            &lines,
            warnings,
        );
        let pale = [self.quiet, self.soft, self.ink];
        self.entry_hint = side("entry_hint", field, false, HINT_FLOOR).settle(
            self.entry_hint,
            own("quiet", self.quiet),
            &pale,
            warnings,
        );
        // The ring is the caret's colour when the caret is the file's, or stands in for one the file set.
        let caret = (has("caret") || self.caret != before).then_some(self.caret);
        let ring = Side { under: desk, ..side("entry_ring", ("paper", self.paper), false, RING_FLOOR) };
        self.entry_ring = ring.settle(self.entry_ring, caret, &[self.caret, self.accent_dark, self.ink], warnings);
        self.chips(set, under, warnings);
    }

    /// The chips' colours, for [`Palette::own`].
    fn chips(&mut self, set: Option<&Table>, under: Color, warnings: &mut Vec<String>) {
        let has = |key: &str| set.is_some_and(|table| table.contains_key(key));
        // A file that sets another shade of `dimmed` and not its light one has no colour of its own for a chip.
        let dimmed = set.and_then(|table| table.get("dimmed")).and_then(Value::as_table);
        let light = dimmed.is_some_and(|hue| hue.contains_key("light")).then_some(self.dimmed.light);
        let (soft, grounds) = (("soft", self.soft), [self.dimmed.light, self.option, self.paper, self.ink]);
        let chip = Side { key: "chip", set: has("chip"), other: soft, ground: true, floor: CHIP_INK_FLOOR, under };
        self.chip = chip.settle(self.chip, light, &grounds, warnings);
        let name = Side { key: "soft", set: has("soft"), other: ("chip", self.chip), ground: false, ..chip };
        self.soft = name.text(self.soft, warnings);

        // The chosen chip and its name are one pair: the file's `star` and `soft` are taken where the file sets
        // neither key's own colour, when the name reads on the chip so, and neither of them when it does not.
        let fill = if !has("chip_chosen") && has("star") { self.star } else { self.chip_chosen };
        let name = if !has("chip_chosen_ink") && has("soft") { self.soft } else { self.chip_chosen_ink };
        if name.contrast(fill.over(under)) >= CHOSEN_INK_FLOOR {
            (self.chip_chosen, self.chip_chosen_ink) = (fill, name);
        }
        let name = Side {
            key: "chip_chosen_ink",
            set: has("chip_chosen_ink"),
            other: ("chip_chosen", self.chip_chosen),
            ground: false,
            floor: CHOSEN_INK_FLOOR,
            under,
        };
        self.chip_chosen_ink = name.settle(self.chip_chosen_ink, None, &[self.soft, self.ink], warnings);
    }

    /// The edge of the chosen chip. Its own fill, so no line shows, when the fill stands out from the other
    /// chips' and from what is under it ([`CHOSEN_FILL_FLOOR`]); else the colour of its name, so that a line
    /// tells which chip is chosen.
    pub fn chosen_edge(&self) -> Color {
        let under = self.ground();
        let chosen = self.chip_chosen.over(under);
        let told = |from: Color| chosen.contrast(from) >= CHOSEN_FILL_FLOOR;
        if told(self.chip.over(under)) && told(under) { self.chip_chosen } else { self.chip_chosen_ink }
    }
}

fn written(colour: Color) -> String {
    if colour.a == 255 { colour.hex_rgb() } else { format!("{}{:02X}", colour.hex_rgb(), colour.a) }
}
