//! A theme is data: colours, font names, angles, motion amounts and sizes.
//! Every key is optional in a theme file and falls back to the built-in
//! `sticker` theme, which is what each `Default` here returns.

mod bounds;
mod builtin;
mod load;
mod own;

use serde::{Deserialize, Serialize};

use crate::color::Color;

pub use bounds::{MAX_THEME_BYTES, RANGES};
pub use load::{Loaded, ThemeError};
pub use own::{CARET_FLOOR, CHIP_INK_FLOOR, CHOSEN_FILL_FLOOR, CHOSEN_INK_FLOOR, HINT_FLOOR, RING_FLOOR, TYPED_FLOOR};
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub name: String,
    pub palette: Palette,
    pub fonts: Fonts,
    pub paper: Paper,
    pub tilt: Tilt,
    pub motion: Motion,
    pub labels: Labels,
    pub metrics: Metrics,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Hue {
    pub base: Color,
    pub dark: Color,
    pub light: Color,
}

/// The two colours of a striped tape.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Stripe {
    pub a: Color,
    pub b: Color,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Palette {
    pub desk: Color,
    pub paper: Color,
    pub paper_edge: Color,
    pub grid: Color,
    pub hole: Color,
    pub ink: Color,
    pub soft: Color,
    pub quiet: Color,
    pub rim: Color,
    pub shadow: Color,
    pub star: Color,
    pub star_shine: Color,
    pub accent: Color,
    pub accent_dark: Color,
    pub cheek: Color,
    pub bubble: Color,
    pub bubble_ring: Color,
    pub tag: Color,
    pub dimmed: Hue,
    pub tape_date: Stripe,
    pub tape_list: Stripe,
    pub tape_choice: Stripe,
    pub tape_plain: Stripe,
    pub note_list: Color,
    pub note_choice: Color,
    pub note_plain: Color,
    pub choice_title: Color,
    pub choice_footer: Color,
    pub option: Color,
    pub option_edge: Color,
    /// An entry's field and its edge.
    pub entry: Color,
    pub entry_edge: Color,
    /// The field's edge while the entry has the keys: a line, darker than the chosen chip.
    pub entry_ring: Color,
    /// The hint, and words that wait for their answer: a grey.
    pub entry_hint: Color,
    pub caret: Color,
    pub chip: Color,
    /// The chosen chip, filled, and the colour of the name on it.
    pub chip_chosen: Color,
    pub chip_chosen_ink: Color,
    pub hues: Vec<Hue>,
}

/// Family names and weights (100 to 900). An empty name means the system's
/// interface font.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Fonts {
    pub display: String,
    pub body: String,
    pub hand: String,
    /// Labels under characters.
    pub display_weight: u16,
    /// Quiet body text: sub-lines, legend, footer.
    pub body_weight: u16,
    /// Body text that leads: list rows and options.
    pub strong_weight: u16,
    /// The date, the title and note titles.
    pub hand_weight: u16,
    /// The line under a choice's options.
    pub hand_soft_weight: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Paper {
    pub grid: bool,
    pub grid_step: f32,
    pub radius: f32,
}

/// Angles in degrees, clockwise. Only paper and tape tilt; text stays level.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tilt {
    pub enabled: bool,
    pub date_tape: f32,
    pub note_tape: f32,
    pub list_note: f32,
    pub choice_note: f32,
    pub plain_note: f32,
}

/// Distances in logical pixels, periods in seconds. All zero means still.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Motion {
    pub bob_amplitude: f32,
    pub bob_period: f32,
    pub hop_height: f32,
    pub hop_period: f32,
    /// Extra scale at the peak of a star's twinkle (0.2 is a fifth larger).
    pub twinkle: f32,
    /// Extra scale at the peak of a badge's pulse.
    pub pulse: f32,
    pub stiffness: f32,
    pub damping: f32,
    /// How far a tap target shrinks while pressed (0.06 is six percent).
    pub press: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Labels {
    pub max_chars: usize,
    pub max_width: f32,
    pub font_size: f32,
    /// Text never scales below this, however small the window.
    pub min_font_size: f32,
}

/// Sizes on the design canvas (`width` by `height`); the whole page scales
/// to fit the viewport.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Metrics {
    pub width: f32,
    pub height: f32,
    pub margin_x: f32,
    pub margin_y: f32,
    pub content_left: f32,
    pub hero_size: f32,
    pub hero_x: f32,
    pub hero_top: f32,
    pub character_size: f32,
    pub columns: usize,
    pub row_gap: f32,
    pub rim: f32,
    pub note_left: f32,
    pub note_top: f32,
    pub note_width: f32,
    pub note_gutter: f32,
    pub title_size: f32,
    pub date_size: f32,
    pub note_title_size: f32,
    pub body_size: f32,
    pub small_size: f32,
    pub max_scale: f32,
}

impl Theme {
    /// The colours for a character's hue index, wrapping past the end.
    pub fn hue(&self, index: u8) -> Hue {
        let hues = &self.palette.hues;
        if hues.is_empty() {
            return Hue::default();
        }
        hues[usize::from(index) % hues.len()]
    }

    /// `angle` when the theme tilts, zero when it does not.
    pub fn tilted(&self, angle: f32) -> f32 {
        if self.tilt.enabled { angle } else { 0.0 }
    }
}
