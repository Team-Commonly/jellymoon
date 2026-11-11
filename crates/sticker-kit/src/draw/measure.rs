//! Fonts as GPUI sees them: the bundled families registered, and text width from shaped glyphs.

use std::borrow::Cow;

use gpui::{App, Font, FontWeight, Hsla, SharedString, TextRun, WindowTextSystem, font, px};

use crate::color::Color;
use crate::fonts;
use crate::text::{Measure, Role};
use crate::theme::Theme;

/// Registers the fonts the `sticker` theme names. Call once at start-up.
pub fn load_fonts(cx: &App) -> gpui::Result<()> {
    cx.text_system().add_fonts(fonts::files().map(Cow::Borrowed).collect())
}

pub(crate) fn hsla(color: Color) -> Hsla {
    gpui::rgba(color.to_rgba()).into()
}

pub(crate) fn family(theme: &Theme, role: Role) -> SharedString {
    let name = fonts::family(theme, role);
    if name.is_empty() { ".SystemUIFont".into() } else { name.to_string().into() }
}

pub(crate) fn weight(theme: &Theme, role: Role) -> FontWeight {
    FontWeight(f32::from(fonts::weight(theme, role)))
}

pub(crate) fn font_for(theme: &Theme, role: Role) -> Font {
    let mut font = font(family(theme, role));
    font.weight = weight(theme, role);
    font
}

/// Measures with the window's text system, so layout and paint agree.
pub struct GlyphMeasure<'a> {
    pub text: &'a WindowTextSystem,
    pub theme: &'a Theme,
}

impl Measure for GlyphMeasure<'_> {
    fn width(&self, text: &str, role: Role, size: f32) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        let run = TextRun {
            len: text.len(),
            font: font_for(self.theme, role),
            color: gpui::black(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let line = self.text.shape_line(text.to_string().into(), px(size), &[run], None);
        line.width().into()
    }
}
