//! Reading a theme file. Unknown keys are ignored and reported, so a theme written for a newer kit still loads.
//! One written for an older kit is drawn in its own colours, and text it makes unreadable is replaced and
//! reported (`own.rs`).
//! Refused, because they would make a theme more than data or more than a page can hold: a file over its size
//! (`bounds.rs`), a number that is not finite or is outside its range, a path that leaves the theme's folder, and an
//! SVG that is not a regular file of text within its size.
//!
//! No picture is parsed here. What a picture holds is judged by a trial (`trial.rs`), in another process, when
//! [`Theme::try_art`] is asked: only what that gives back is ever drawn.

use std::fmt;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use toml::{Table, Value};

use super::bounds::{MAX_ART_BYTES, MAX_THEME_BYTES, check};
use super::{ArtCost, Theme, Trial};

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
    /// The SVG named by `key` is more than a picture.
    RefusedSvg {
        key: String,
        reason: String,
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
            ThemeError::RefusedSvg { key, reason } => {
                write!(f, "the SVG for {key} is refused: {reason}")
            }
            ThemeError::OutOfRange { key, value, low, high } => {
                write!(f, "{key} = {value} is not a number from {low} to {high}")
            }
        }
    }
}

impl std::error::Error for ThemeError {}

impl Theme {
    /// Parses theme text. Art names are checked by shape only, since there
    /// is no folder to resolve them in.
    pub fn from_toml(text: &str) -> Result<Loaded, ThemeError> {
        if text.len() as u64 > MAX_THEME_BYTES {
            return Err(ThemeError::Read(over(MAX_THEME_BYTES)));
        }
        let mut table: Table = text.parse().map_err(|e: toml::de::Error| ThemeError::Parse(e.to_string()))?;
        let known = Table::try_from(Theme::default()).map_err(|e| ThemeError::Parse(e.to_string()))?;
        let mut warnings = Vec::new();
        prune(&mut table, &known, "", &mut warnings);
        check(&table, "")?;
        let palette = table.get("palette").and_then(Value::as_table).cloned();
        let mut theme: Theme = table.try_into().map_err(|e: toml::de::Error| ThemeError::Parse(e.to_string()))?;
        for (key, name) in theme.art.entries() {
            if escapes_by_shape(name) {
                return Err(escape(key, name));
            }
        }
        theme.palette.own(palette.as_ref());
        theme.palette.readable(&mut warnings);
        Ok(Loaded { theme, warnings })
    }

    /// Reads a theme file, and each art file it names as bytes: that it is there, inside the folder, a regular
    /// file of text and within its size. What the pictures hold is [`Theme::try_art`]'s to say.
    pub fn load(path: &Path) -> Result<Loaded, ThemeError> {
        let text = read_capped(path, MAX_THEME_BYTES).map_err(ThemeError::Read)?;
        let mut loaded = Theme::from_toml(&text)?;
        // A bare file name has the empty path for a parent: that is the folder the program runs in.
        let folder = path.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let folder = folder.to_path_buf();
        if loaded.theme.name.is_empty()
            && let Some(stem) = path.file_stem()
        {
            loaded.theme.name = stem.to_string_lossy().into_owned();
        }
        for (key, name) in loaded.theme.art.clone().entries() {
            if !name.is_empty() && read_art(&folder, key, name)?.is_none() {
                loaded.warnings.push(format!("art.{key}: `{name}` was not found; the built-in art is used"));
                loaded.theme.art.clear(key);
            }
        }
        loaded.theme.folder = Some(folder);
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

    /// Tries the art replacing each piece (`hero`, `character`, `star`, `bubble`, `badge`), for the pieces the
    /// theme names a file for, one after another. Each file is read once, here, and the text a trial accepted is
    /// the text given back, whatever is written to the folder meanwhile. A file a trial refuses, or one that
    /// takes the files before it past a count ([`ArtCost`]), is named in `refused` with its one line, and the
    /// kit's own art is drawn for its piece.
    ///
    /// This waits for every trial that has no kept answer: call it off the window's thread.
    pub fn try_art(&self, trial: &Trial) -> Tried {
        let (mut all, mut tried) = (ArtCost::default(), Tried::default());
        let Some(folder) = &self.folder else { return tried };
        for (piece, name) in self.art.entries().into_iter().filter(|(_, name)| !name.is_empty()) {
            let refuse = |reason: String| ThemeError::RefusedSvg { key: format!("art.{piece}"), reason };
            let judged = match read_art(folder, piece, name) {
                Ok(Some(svg)) => trial.judge(&svg).map(|one| (svg, one)).map_err(refuse),
                Ok(None) => continue,
                Err(refused) => Err(refused),
            };
            let together = judged.and_then(|(svg, one)| match all.with(one).refusal() {
                Some(reason) => Err(refuse(format!("with the art before it, {}", reason.trim_start_matches("its ")))),
                None => Ok((svg, one)),
            });
            match together {
                Ok((svg, one)) => {
                    all = all.with(one);
                    tried.accepted.push(Accepted { piece, svg });
                }
                Err(refused) => tried.refused.push(refused),
            }
        }
        tried
    }
}

/// A picture a trial accepted, for one piece: the only text of a theme's that the drawing half takes. There is
/// no other way to make one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Accepted {
    piece: &'static str,
    svg: String,
}

impl Accepted {
    pub fn piece(&self) -> &'static str {
        self.piece
    }

    pub fn svg(&self) -> &str {
        &self.svg
    }
}

/// What [`Theme::try_art`] found: the pictures to draw, and why each of the others is not drawn.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tried {
    pub accepted: Vec<Accepted>,
    pub refused: Vec<ThemeError>,
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

fn escape(key: &str, name: &str) -> ThemeError {
    ThemeError::PathEscapes { key: format!("art.{key}"), path: name.to_string() }
}

fn escapes_by_shape(name: &str) -> bool {
    Path::new(name).components().any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
}

/// The art file's text, `None` when the file is missing. Symlinks are resolved first, so a link that points out
/// of the folder is an escape.
fn read_art(folder: &Path, key: &str, name: &str) -> Result<Option<String>, ThemeError> {
    if escapes_by_shape(name) {
        return Err(escape(key, name));
    }
    let Ok(real) = fs::canonicalize(folder.join(name)) else {
        return Ok(None);
    };
    let root: PathBuf = fs::canonicalize(folder).map_err(|e| ThemeError::Read(e.to_string()))?;
    if !real.starts_with(&root) {
        return Err(escape(key, name));
    }
    let refuse = |reason: String| ThemeError::RefusedSvg { key: format!("art.{key}"), reason };
    read_capped(&real, MAX_ART_BYTES).map(Some).map_err(refuse)
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
