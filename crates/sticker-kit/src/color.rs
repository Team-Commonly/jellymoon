//! A colour that reads and writes as `#RRGGBB` or `#RRGGBBAA`.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// From `0xRRGGBB`, fully opaque.
    pub const fn rgb(hex: u32) -> Self {
        Self { r: (hex >> 16) as u8, g: (hex >> 8) as u8, b: hex as u8, a: 255 }
    }

    /// From `0xRRGGBBAA`.
    pub const fn rgba(hex: u32) -> Self {
        Self { r: (hex >> 24) as u8, g: (hex >> 16) as u8, b: (hex >> 8) as u8, a: hex as u8 }
    }

    pub fn to_rgba(self) -> u32 {
        u32::from_be_bytes([self.r, self.g, self.b, self.a])
    }

    pub fn opacity(self) -> f32 {
        f32::from(self.a) / 255.0
    }

    /// `#RRGGBB`, without the alpha.
    pub fn hex_rgb(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// This colour laid on `under`, which is taken as opaque: what is seen where the two are drawn.
    pub fn over(self, under: Color) -> Color {
        let mix = |top: u8, low: u8| {
            let (top, low, a) = (f32::from(top), f32::from(low), self.opacity());
            (top * a + low * (1.0 - a)).round() as u8
        };
        Color { r: mix(self.r, under.r), g: mix(self.g, under.g), b: mix(self.b, under.b), a: 255 }
    }

    /// How light the colour is, from 0 for black to 1 for white: the relative luminance of WCAG 2. The alpha is
    /// not counted.
    pub fn luminance(self) -> f32 {
        let part = |c: u8| {
            let c = f32::from(c) / 255.0;
            if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * part(self.r) + 0.7152 * part(self.g) + 0.0722 * part(self.b)
    }

    /// How far this colour, laid on `ground`, stands out from it: the contrast ratio of WCAG 2, from 1 for two
    /// colours alike to 21 for black on white. A colour that is all transparent stands out from nothing.
    pub fn contrast(self, ground: Color) -> f32 {
        let (a, b) = (self.over(ground).luminance(), ground.luminance());
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    pub fn parse(text: &str) -> Option<Self> {
        let digits = text.strip_prefix('#')?;
        if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let value = u32::from_str_radix(digits, 16).ok()?;
        match digits.len() {
            6 => Some(Self::rgb(value)),
            8 => Some(Self::rgba(value)),
            _ => None,
        }
    }
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.a == 255 {
            serializer.serialize_str(&self.hex_rgb())
        } else {
            serializer.serialize_str(&format!("{}{:02X}", self.hex_rgb(), self.a))
        }
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Color::parse(&text).ok_or_else(|| D::Error::custom(format!("`{text}` is not #RRGGBB or #RRGGBBAA")))
    }
}
