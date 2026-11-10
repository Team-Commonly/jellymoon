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
