//! How the kit's tests try a theme's art: through a trial, in a child, as every caller does. No test reads a
//! theme's picture for drawing any other way.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use sticker_kit::theme::{Accepted, Trial};
use sticker_kit::{Loaded, Theme, ThemeError};

/// A trial whose child is the tests' helper, started as `which`, with no answer kept.
pub fn trial_as(which: &[&str]) -> Trial {
    // A system may look a program over the first time it is started, for seconds (macOS, a fresh build). The
    // helper is started once with no limit before any trial counts its time. An app tries with its own running
    // program, which has been looked over already.
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        let started = std::time::Instant::now();
        let _ = std::process::Command::new(env!("CARGO_BIN_EXE_trial-child")).arg("fails").output();
        eprintln!("the helper's first start took {:?}", started.elapsed());
    });
    let mut trial = Trial::new(PathBuf::from(env!("CARGO_BIN_EXE_trial-child")), which[0], None);
    trial.arguments = which.iter().map(|argument| argument.to_string()).collect();
    trial
}

/// The kit's own child.
pub fn trial() -> Trial {
    trial_as(&["trial"])
}

/// A theme file read and its art tried: the theme and the pictures to draw, or the first thing refused.
pub fn tried(path: &Path) -> Result<(Loaded, Vec<Accepted>), ThemeError> {
    let loaded = Theme::load(path)?;
    let tried = loaded.theme.try_art(&trial());
    match tried.refused.into_iter().next() {
        Some(refused) => Err(refused),
        None => Ok((loaded, tried.accepted)),
    }
}

/// Which pieces were given a picture, and the text of each.
pub fn given(art: &[Accepted]) -> Vec<(&'static str, String)> {
    art.iter().map(|accepted| (accepted.piece(), accepted.svg().to_string())).collect()
}
