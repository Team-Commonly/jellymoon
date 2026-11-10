//! Reading a theme file. Unknown keys are ignored and reported, so a theme
//! written for a newer kit still loads. Two things are refused, because they
//! would make a theme more than data: a path that leaves the theme's folder,
//! and an SVG that could run code.

use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

use toml::{Table, Value};

use super::{Theme, svg_refusal};

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
        }
    }
}

impl std::error::Error for ThemeError {}

impl Theme {
    /// Parses theme text. Art names are checked by shape only, since there
    /// is no folder to resolve them in.
    pub fn from_toml(text: &str) -> Result<Loaded, ThemeError> {
        let mut table: Table = text.parse().map_err(|e: toml::de::Error| ThemeError::Parse(e.to_string()))?;
        let known = Table::try_from(Theme::default()).map_err(|e| ThemeError::Parse(e.to_string()))?;
        let mut warnings = Vec::new();
        prune(&mut table, &known, "", &mut warnings);
        let theme: Theme = table.try_into().map_err(|e: toml::de::Error| ThemeError::Parse(e.to_string()))?;
        for (key, name) in theme.art.entries() {
            if escapes_by_shape(name) {
                return Err(escape(key, name));
            }
        }
        Ok(Loaded { theme, warnings })
    }

    /// Reads a theme file and checks every art file it names.
    pub fn load(path: &Path) -> Result<Loaded, ThemeError> {
        let text = fs::read_to_string(path).map_err(|e| ThemeError::Read(e.to_string()))?;
        let mut loaded = Theme::from_toml(&text)?;
        let folder = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        if loaded.theme.name.is_empty()
            && let Some(stem) = path.file_stem()
        {
            loaded.theme.name = stem.to_string_lossy().into_owned();
        }
        for (key, name) in loaded.theme.art.clone().entries() {
            if name.is_empty() {
                continue;
            }
            if read_art(&folder, key, name)?.is_none() {
                loaded.warnings.push(format!("art.{key}: `{name}` was not found; the built-in art is used"));
                loaded.theme.art.clear(key);
            }
        }
        loaded.theme.folder = Some(folder);
        Ok(loaded)
    }

    pub fn to_toml(&self) -> Result<String, ThemeError> {
        toml::to_string(self).map_err(|e| ThemeError::Parse(e.to_string()))
    }

    /// The checked SVG text replacing a piece's art (`hero`, `character`,
    /// `star`, `bubble` or `badge`), when the theme names one.
    pub fn art_svg(&self, key: &str) -> Option<String> {
        let folder = self.folder.as_deref()?;
        let name = self.art.entries().iter().find(|e| e.0 == key)?.1;
        if name.is_empty() {
            return None;
        }
        read_art(folder, key, name).ok().flatten()
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

fn escape(key: &str, name: &str) -> ThemeError {
    ThemeError::PathEscapes { key: format!("art.{key}"), path: name.to_string() }
}

fn escapes_by_shape(name: &str) -> bool {
    Path::new(name).components().any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
}

/// The art file's text, `None` when the file is missing. Symlinks are
/// resolved first, so a link that points out of the folder is an escape.
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
    let text = fs::read_to_string(&real).map_err(|_| refuse("it is not text".into()))?;
    match svg_refusal(&text) {
        Some(reason) => Err(refuse(reason)),
        None => Ok(Some(text)),
    }
}
