//! The bundled font files, and text width from their advance widths alone. Nothing here asks the operating system,
//! so the same text measures the same on every machine. It is not shaping: no kerning and no ligatures.

use ttf_parser::Face as FontFile;

use crate::text::{Face, Measure, Role};
use crate::theme::Theme;

/// One static cut per weight the `sticker` theme asks for. GPUI cannot pick a weight on a variable font, so a theme
/// that asks for another weight of these families gets the nearest cut here.
struct Cut {
    family: &'static str,
    weight: u16,
    bytes: &'static [u8],
}

const CUTS: [Cut; 5] = [
    Cut { family: "Fredoka", weight: 500, bytes: include_bytes!("../assets/fonts/Fredoka-Medium.ttf") },
    Cut { family: "Nunito", weight: 600, bytes: include_bytes!("../assets/fonts/Nunito-SemiBold.ttf") },
    Cut { family: "Nunito", weight: 700, bytes: include_bytes!("../assets/fonts/Nunito-Bold.ttf") },
    Cut { family: "Caveat", weight: 600, bytes: include_bytes!("../assets/fonts/Caveat-SemiBold.ttf") },
    Cut { family: "Caveat", weight: 700, bytes: include_bytes!("../assets/fonts/Caveat-Bold.ttf") },
];

/// Every bundled font file.
#[cfg(feature = "gpui")]
pub(crate) fn files() -> impl Iterator<Item = &'static [u8]> {
    CUTS.iter().map(|cut| cut.bytes)
}

/// The family a role is set in. Empty means the system's interface font.
pub(crate) fn family(theme: &Theme, role: Role) -> &str {
    match role.face() {
        Face::Display => &theme.fonts.display,
        Face::Body => &theme.fonts.body,
        Face::Hand => &theme.fonts.hand,
    }
}

pub(crate) fn weight(theme: &Theme, role: Role) -> u16 {
    let fonts = &theme.fonts;
    let weight = match role {
        Role::ChoiceFooter => fonts.hand_soft_weight,
        Role::RowMain | Role::Option | Role::OptionStruck => fonts.strong_weight,
        _ => match role.face() {
            Face::Display => fonts.display_weight,
            Face::Body => fonts.body_weight,
            Face::Hand => fonts.hand_weight,
        },
    };
    weight.clamp(100, 900)
}

/// The bundled family a font that is not bundled is measured as: the plain sans, the nearest to a system's own.
const STAND_IN: &str = "Nunito";

/// Text width as the sum of each character's advance in a bundled font file.
///
/// - A role is measured in the bundled cut of the theme's family nearest its weight. A family that is not bundled
///   (the system's font, say) is measured as Nunito.
/// - A character the cut lacks is one em wide when it is East Asian, 1.4 em when it is an emoji or a symbol drawn
///   as one, nothing when it only joins or modifies its neighbour, and 0.6 em otherwise.
pub struct BundledMeasure<'a> {
    theme: &'a Theme,
    cuts: Vec<(&'static Cut, FontFile<'static>)>,
}

impl<'a> BundledMeasure<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        let cuts = CUTS.iter().filter_map(|cut| Some((cut, FontFile::parse(cut.bytes, 0).ok()?))).collect();
        Self { theme, cuts }
    }

    fn file(&self, role: Role) -> Option<&FontFile<'static>> {
        let weight = weight(self.theme, role);
        let nearest = |family: &str| {
            let of_family = self.cuts.iter().filter(|(cut, _)| cut.family.eq_ignore_ascii_case(family));
            of_family.min_by_key(|(cut, _)| cut.weight.abs_diff(weight)).map(|(_, file)| file)
        };
        nearest(family(self.theme, role)).or_else(|| nearest(STAND_IN))
    }
}

impl Measure for BundledMeasure<'_> {
    fn width(&self, text: &str, role: Role, size: f32) -> f32 {
        let file = self.file(role);
        text.chars().map(|c| advance(file, c)).sum::<f32>() * size
    }
}

/// How far one character moves the pen, in ems.
fn advance(file: Option<&FontFile<'_>>, c: char) -> f32 {
    let known = file.and_then(|file| {
        let glyph = file.glyph_index(c)?;
        Some(f32::from(file.glyph_hor_advance(glyph)?) / f32::from(file.units_per_em()))
    });
    known.unwrap_or_else(|| missing(c))
}

fn missing(c: char) -> f32 {
    match c as u32 {
        // Combining marks, zero-width spaces and joiners, variation selectors, skin tones, tag characters.
        0x0300..=0x036F | 0x200B..=0x200D | 0x2060 | 0xFE00..=0xFE0F | 0x1F3FB..=0x1F3FF | 0xE0020..=0xE007F => 0.0,
        // Hangul, CJK and kana, full-width forms, and the CJK extension planes.
        0x1100..=0x115F
        | 0x2E80..=0xA4CF
        | 0xAC00..=0xD7A3
        | 0xF900..=0xFAFF
        | 0xFE30..=0xFE4F
        | 0xFF00..=0xFF60
        | 0xFFE0..=0xFFE6
        | 0x20000..=0x3FFFD => 1.0,
        // Symbols drawn as emoji, and emoji. Colour emoji fonts are wider than an em (Apple's measured 1.36).
        0x2600..=0x27BF | 0x2B00..=0x2BFF | 0x1F000..=0x1FAFF => 1.4,
        _ => 0.6,
    }
}
