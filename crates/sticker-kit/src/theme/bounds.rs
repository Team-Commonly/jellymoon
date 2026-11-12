//! The door a theme file comes through: how large it may be, and the range every number in it must be in. A theme
//! outside these is refused, never clamped, so no file makes layout or paint loop without end.

use toml::{Table, Value};

use super::ThemeError;

/// The largest theme file read, in bytes.
pub const MAX_THEME_BYTES: u64 = 256 * 1024;

/// The range of each number, by the start of its dotted key. The first line that matches a key is its range; the
/// last matches every key. A size that art is rasterised from has a range near what a theme needs (the sticker theme
/// has a hero of 200, a character of 92 and a rim of 5), and damping one under which the spring stays stable.
pub const RANGES: [(&str, f64, f64); 11] = [
    ("metrics.width", 200.0, 10_000.0),
    ("metrics.height", 200.0, 10_000.0),
    ("metrics.columns", 1.0, 12.0),
    ("metrics.max_scale", 0.1, 10.0),
    ("metrics.hero_size", 0.0, 800.0),
    ("metrics.character_size", 0.0, 400.0),
    ("metrics.rim", 0.0, 40.0),
    ("motion.damping", 0.0, 200.0),
    ("labels.max_chars", 1.0, 200.0),
    ("tilt.", -45.0, 45.0),
    ("", 0.0, 10_000.0),
];

/// Refuses the first number that is not finite or is outside its range.
pub(super) fn check(table: &Table, prefix: &str) -> Result<(), ThemeError> {
    for (name, value) in table {
        let key = format!("{prefix}{name}");
        match value {
            Value::Table(inner) => check(inner, &format!("{key}."))?,
            Value::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    match item {
                        Value::Table(inner) => check(inner, &format!("{key}[{i}]."))?,
                        other => number(&format!("{key}[{i}]"), other)?,
                    }
                }
            }
            other => number(&key, other)?,
        }
    }
    Ok(())
}

fn number(key: &str, value: &Value) -> Result<(), ThemeError> {
    let n = match value {
        Value::Float(n) => *n,
        Value::Integer(n) => *n as f64,
        _ => return Ok(()),
    };
    let (low, high) = RANGES.iter().find(|range| key.starts_with(range.0)).map_or((0.0, 0.0), |r| (r.1, r.2));
    if n.is_finite() && (low..=high).contains(&n) {
        return Ok(());
    }
    Err(ThemeError::OutOfRange { key: key.to_string(), value: n.to_string(), low, high })
}
