//! Reading a theme file. Unknown keys are ignored and reported, so a theme written for a newer kit still loads.
//! One written for an older kit is drawn in its own colours where those read, and text a file makes unreadable is
//! replaced, and reported for a key the file set (`own.rs`).
//! Refused, because they would make a theme more than data or more than a page can hold: a file over its size
//! (`bounds.rs`), a number that is not finite or is outside its range, and a path that leaves the theme's folder.
//!
//! A theme file names no pictures (`docs/adr/0016-theme-files-name-no-pictures.md`). The `art` table an older
//! theme file has is not read: no file it names is opened, looked for or resolved, here or anywhere. It is said
//! in one warning, and the kit's own art is drawn.

use std::fmt;
use std::fs;
use std::io::Read;
use std::path::{Component, Path};

use toml::{Table, Value};

use super::Theme;
use super::bounds::{MAX_THEME_BYTES, check};

/// A theme, and what the loader ignored on the way.
#[derive(Clone, Debug, PartialEq)]
pub struct Loaded {
    pub theme: Theme,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ThemeError {
    Read(String),
    Parse(String),
    /// `key` names a file outside the theme's own folder.
    PathEscapes {
        key: String,
        path: String,
    },
    /// The number at `key` is not finite or is outside `low` to `high`.
    OutOfRange {
        key: String,
        value: String,
        low: f64,
        high: f64,
    },
}

impl fmt::Display for ThemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ThemeError::Read(why) => write!(f, "cannot read the theme: {why}"),
            ThemeError::Parse(why) => write!(f, "cannot parse the theme: {why}"),
            ThemeError::PathEscapes { key, path } => {
                write!(f, "{key} = \"{path}\" leaves the theme's folder")
            }
            ThemeError::OutOfRange { key, value, low, high } => {
                write!(f, "{key} = {value} is not a number from {low} to {high}")
            }
        }
    }
}

impl std::error::Error for ThemeError {}

impl Theme {
    /// Parses theme text.
    pub fn from_toml(text: &str) -> Result<Loaded, ThemeError> {
        if text.len() as u64 > MAX_THEME_BYTES {
            return Err(ThemeError::Read(over(MAX_THEME_BYTES)));
        }
        let mut table: Table = text.parse().map_err(|e: toml::de::Error| ThemeError::Parse(e.to_string()))?;
        let known = Table::try_from(Theme::default()).map_err(|e| ThemeError::Parse(e.to_string()))?;
        let mut warnings = Vec::new();
        no_pictures(&mut table, &mut warnings)?;
        prune(&mut table, &known, "", &mut warnings);
        check(&table, "")?;
        let palette = table.get("palette").and_then(Value::as_table).cloned();
        let mut theme: Theme = table.try_into().map_err(|e: toml::de::Error| ThemeError::Parse(e.to_string()))?;
        theme.palette.own(palette.as_ref(), &mut warnings);
        Ok(Loaded { theme, warnings })
    }

    /// Reads a theme file, and no other file.
    pub fn load(path: &Path) -> Result<Loaded, ThemeError> {
        let text = read_capped(path, MAX_THEME_BYTES).map_err(ThemeError::Read)?;
        let mut loaded = Theme::from_toml(&text)?;
        if loaded.theme.name.is_empty()
            && let Some(stem) = path.file_stem()
        {
            loaded.theme.name = stem.to_string_lossy().into_owned();
        }
        Ok(loaded)
    }

    /// Holds this theme's numbers to the ranges a theme file is held to (`bounds.rs`): the one check, whether the
    /// theme was read from a file or built in code. `layout` runs it and places nothing for a theme it refuses.
    pub fn check(&self) -> Result<(), ThemeError> {
        let table = Table::try_from(self).map_err(|e| ThemeError::Parse(e.to_string()))?;
        check(&table, "")
    }

    pub fn to_toml(&self) -> Result<String, ThemeError> {
        toml::to_string(self).map_err(|e| ThemeError::Parse(e.to_string()))
    }
}

/// Drops every key the kit does not know and records its dotted name.
fn prune(table: &mut Table, known: &Table, prefix: &str, warnings: &mut Vec<String>) {
    let names: Vec<String> = table.keys().cloned().collect();
    for name in names {
        let path = format!("{prefix}{name}");
        let Some(reference) = known.get(&name) else {
            table.remove(&name);
            warnings.push(format!("{path}: unknown key, ignored"));
            continue;
        };
        match (table.get_mut(&name), reference) {
            (Some(Value::Table(inner)), Value::Table(known_inner)) => {
                prune(inner, known_inner, &format!("{path}."), warnings);
            }
            (Some(Value::Array(items)), Value::Array(known_items)) => {
                let Some(Value::Table(known_item)) = known_items.first() else {
                    continue;
                };
                for (i, item) in items.iter_mut().enumerate() {
                    if let Value::Table(inner) = item {
                        prune(inner, known_item, &format!("{path}[{i}]."), warnings);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Takes the `art` table out of a theme file, which named a picture for a piece of the page. Nothing it names is
/// read: only the text of each name is looked at, and one that leaves the theme's folder is refused as it always
/// was, since a theme that points out of its folder is not one to load.
fn no_pictures(table: &mut Table, warnings: &mut Vec<String>) -> Result<(), ThemeError> {
    let Some(art) = table.remove("art") else { return Ok(()) };
    for (key, name) in art.as_table().into_iter().flatten() {
        let name = name.as_str().unwrap_or_default();
        if Path::new(name).components().any(|part| !matches!(part, Component::Normal(_) | Component::CurDir)) {
            return Err(ThemeError::PathEscapes { key: format!("art.{key}"), path: name.to_string() });
        }
    }
    warnings.push("art: pictures in a theme are not read in this version; the kit's own art is drawn".to_string());
    Ok(())
}

fn over(max: u64) -> String {
    format!("it is over {} KiB", max / 1024)
}

/// The text of a regular file of at most `max` bytes. A folder, a pipe or a device is not read at all.
fn read_capped(path: &Path, max: u64) -> Result<String, String> {
    let meta = fs::metadata(path).map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err("it is not a regular file".into());
    }
    let mut text = String::new();
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    file.take(max + 1).read_to_string(&mut text).map_err(|_| "it is not text".to_string())?;
    if meta.len() > max || text.len() as u64 > max {
        return Err(over(max));
    }
    Ok(text)
}
