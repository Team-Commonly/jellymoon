//! The two themes compiled into the kit.

use super::{Fonts, Hue, Labels, Metrics, Motion, Palette, Paper, Stripe, Theme, Tilt};
use crate::color::Color;

const fn hue(base: u32, dark: u32, light: u32) -> Hue {
    Hue { base: Color::rgb(base), dark: Color::rgb(dark), light: Color::rgb(light) }
}

const fn stripe(a: u32, b: u32) -> Stripe {
    Stripe { a: Color::rgb(a), b: Color::rgb(b) }
}

impl Default for Hue {
    fn default() -> Self {
        hue(0xB9A9F6, 0x9684E6, 0xDCD3FE)
    }
}

impl Default for Stripe {
    fn default() -> Self {
        stripe(0xFFC4D6, 0xFFD9E5)
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            desk: Color::rgb(0xEFE8DA),
            paper: Color::rgb(0xFFFDF7),
            paper_edge: Color::rgba(0x3C325A10),
            grid: Color::rgba(0x6E82BE21),
            hole: Color::rgb(0xEFE8DA),
            ink: Color::rgb(0x2B2350),
            soft: Color::rgb(0x5E5878),
            quiet: Color::rgb(0x9390A6),
            rim: Color::rgb(0xFFFFFF),
            shadow: Color::rgba(0x3C325A38),
            star: Color::rgb(0xFFD24D),
            star_shine: Color::rgb(0xFFF3C2),
            accent: Color::rgb(0xFF7A59),
            accent_dark: Color::rgb(0xE4573A),
            cheek: Color::rgba(0xFF8FB199),
            bubble: Color::rgb(0xFFFFFF),
            bubble_ring: Color::rgb(0xCFC8EE),
            tag: Color::rgb(0xFFFFFF),
            dimmed: hue(0xDAD7E4, 0xBFBBCE, 0xEDEBF4),
            tape_date: stripe(0xFFC4D6, 0xFFD9E5),
            tape_list: stripe(0xFFE39A, 0xFFF0C4),
            tape_choice: stripe(0xFF9C82, 0xFFB7A3),
            tape_plain: stripe(0xA6E3C4, 0xD3F4E3),
            note_list: Color::rgb(0xFFFFFF),
            note_choice: Color::rgb(0xFFD8CB),
            note_plain: Color::rgb(0xFFF6D6),
            choice_title: Color::rgb(0x7A2A16),
            choice_footer: Color::rgb(0x9A3A22),
            option: Color::rgb(0xFFFFFF),
            option_edge: Color::rgba(0xE4573A59),
            hues: vec![
                hue(0xB9A9F6, 0x9684E6, 0xDCD3FE),
                hue(0xFFB3CB, 0xF288AB, 0xFFD9E5),
                hue(0xFFDF8A, 0xF0BE4C, 0xFFF0C4),
                hue(0xA6E3C4, 0x74C89F, 0xD3F4E3),
                hue(0xA9D3FF, 0x7AB4F2, 0xD6EAFF),
            ],
        }
    }
}

impl Default for Fonts {
    fn default() -> Self {
        Self {
            display: "Fredoka".into(),
            body: "Nunito".into(),
            hand: "Caveat".into(),
            display_weight: 500,
            body_weight: 600,
            strong_weight: 700,
            hand_weight: 700,
            hand_soft_weight: 600,
        }
    }
}

impl Default for Paper {
    fn default() -> Self {
        Self { grid: true, grid_step: 24.0, radius: 20.0 }
    }
}

impl Default for Tilt {
    fn default() -> Self {
        Self { enabled: true, date_tape: -2.5, note_tape: -3.5, list_note: 1.2, choice_note: 1.6, plain_note: -1.0 }
    }
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            bob_amplitude: 7.0,
            bob_period: 7.6,
            hop_height: 9.0,
            hop_period: 1.5,
            twinkle: 0.22,
            pulse: 0.18,
            stiffness: 260.0,
            damping: 14.0,
            press: 0.08,
        }
    }
}

impl Default for Labels {
    fn default() -> Self {
        Self { max_chars: 26, max_width: 236.0, font_size: 14.0, min_font_size: 11.0 }
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            width: 1440.0,
            height: 900.0,
            margin_x: 48.0,
            margin_y: 34.0,
            content_left: 126.0,
            hero_size: 200.0,
            hero_x: 610.0,
            hero_top: 196.0,
            character_size: 92.0,
            columns: 4,
            row_gap: 8.0,
            rim: 5.0,
            note_left: 888.0,
            note_top: 96.0,
            note_width: 430.0,
            note_gutter: 84.0,
            title_size: 46.0,
            date_size: 26.0,
            note_title_size: 30.0,
            body_size: 15.0,
            small_size: 13.0,
            max_scale: 1.5,
        }
    }
}

fn plain_palette() -> Palette {
    let tape = stripe(0xD5D9DF, 0xD5D9DF);
    Palette {
        desk: Color::rgb(0xECEDEF),
        paper: Color::rgb(0xFFFFFF),
        paper_edge: Color::rgba(0x1F232814),
        grid: Color::rgba(0x1F232800),
        hole: Color::rgb(0xECEDEF),
        ink: Color::rgb(0x1F2328),
        soft: Color::rgb(0x57606A),
        quiet: Color::rgb(0x8C959F),
        rim: Color::rgb(0xFFFFFF),
        shadow: Color::rgba(0x1F232826),
        star: Color::rgb(0x2F6FEB),
        star_shine: Color::rgb(0x8FB3F7),
        accent: Color::rgb(0x2F6FEB),
        accent_dark: Color::rgb(0x1F55C4),
        cheek: Color::rgba(0x1F232800),
        bubble: Color::rgb(0xFFFFFF),
        bubble_ring: Color::rgb(0xC3C9D1),
        tag: Color::rgb(0xFFFFFF),
        dimmed: hue(0xECEDEF, 0xD9DCE1, 0xF6F7F8),
        tape_date: tape,
        tape_list: tape,
        tape_choice: tape,
        tape_plain: tape,
        note_list: Color::rgb(0xFFFFFF),
        note_choice: Color::rgb(0xF4F5F7),
        note_plain: Color::rgb(0xFFFFFF),
        choice_title: Color::rgb(0x1F2328),
        choice_footer: Color::rgb(0x57606A),
        option: Color::rgb(0xFFFFFF),
        option_edge: Color::rgba(0x1F232833),
        hues: vec![
            hue(0xC4CAD3, 0xA2AAB6, 0xE3E6EA),
            hue(0xD3D7DD, 0xB3BAC4, 0xECEEF1),
            hue(0xB7BEC9, 0x959EAC, 0xDADEE4),
        ],
    }
}

impl Theme {
    /// The approved look: pastel stickers on tilted, taped grid paper.
    pub fn sticker() -> Theme {
        Theme { name: "sticker".into(), ..Theme::default() }
    }

    /// Level and quiet: greys and one accent, no tilt, no idle motion,
    /// system fonts.
    pub fn plain() -> Theme {
        Theme {
            name: "plain".into(),
            palette: plain_palette(),
            fonts: Fonts { display: String::new(), body: String::new(), hand: String::new(), ..Fonts::default() },
            paper: Paper { grid: false, radius: 8.0, ..Paper::default() },
            tilt: Tilt {
                enabled: false,
                date_tape: 0.0,
                note_tape: 0.0,
                list_note: 0.0,
                choice_note: 0.0,
                plain_note: 0.0,
            },
            motion: Motion {
                bob_amplitude: 0.0,
                bob_period: 0.0,
                hop_height: 4.0,
                twinkle: 0.0,
                pulse: 0.0,
                press: 0.04,
                ..Motion::default()
            },
            metrics: Metrics { title_size: 34.0, date_size: 18.0, note_title_size: 21.0, ..Metrics::default() },
            ..Theme::default()
        }
    }

    /// A built-in theme by name.
    pub fn builtin(name: &str) -> Option<Theme> {
        match name {
            "sticker" => Some(Theme::sticker()),
            "plain" => Some(Theme::plain()),
            _ => None,
        }
    }
}
