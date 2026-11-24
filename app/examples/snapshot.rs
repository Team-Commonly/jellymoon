//! Renders a page-data file under both built-in themes, with no window, for looking at:
//! `cargo run -p jellymoon-app --example snapshot --features headless -- <page file> <folder> <name> [width height
//! [words [option [theme file]]]]` writes `<name>-sticker.png` and `<name>-plain.png` into the folder, and with a
//! theme file a third picture named after that theme. The page has the entry
//! the window gives a day's page, with a chip for each of the page's first six projects: a picture reads no logs,
//! so it does not ask which of them a wish can be made for. With `words` the
//! entry holds them, with the caret after them, and the chip of place `option` (from 0) is the chosen one.
//!
//! `--work <waiting file>` puts the notes of the work that waits, as `jellymoon run waiting` prints it, over the
//! page's notes, and `--told <shown file>` gives the first work the `result` that `jellymoon run show` printed:
//! the notes as they first come onto the page, before `Show me`.

use std::path::PathBuf;
use std::process::ExitCode;

use jellymoon_app::{column::column, file_part, one_line, read, scene, starting, wishing, working};
use jellymoon_core::{Told, Waiting};
use serde::Deserialize;
use sticker_kit::draw::headless::{Typing, snapshot_typing};
use sticker_kit::{Note, Theme};

/// The value after `flag`, taken out of the arguments with it.
fn taken(args: &mut Vec<String>, flag: &str) -> Option<String> {
    let at = args.iter().position(|arg| arg == flag)?;
    let value = args.get(at + 1).cloned();
    args.drain(at..(at + 2).min(args.len()));
    value
}

/// The notes of the work that waits in a `run waiting` file, the first given the result in a `run show` file.
fn work(waiting: &str, told: Option<&str>) -> Result<Vec<Note>, String> {
    let read = |file: &str| std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"));
    let mut waiting: Waiting = serde_json::from_str(&read(waiting)?).map_err(|e| format!("{waiting}: {e}"))?;
    if let (Some(file), Some(first)) = (told, waiting.waits.first_mut()) {
        let shown: serde_json::Value = serde_json::from_str(&read(file)?).map_err(|e| format!("{file}: {e}"))?;
        first.result = Some(Told::deserialize(&shown["result"]).map_err(|e| format!("{file}: {e}"))?);
    }
    Ok(waiting.waits.iter().flat_map(|waits| working::waiting_notes(waits, false)).collect())
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let (waiting, told) = (taken(&mut args, "--work"), taken(&mut args, "--told"));
    let notes = match waiting.map(|file| work(&file, told.as_deref())).transpose() {
        Ok(notes) => notes.unwrap_or_default(),
        Err(why) => {
            eprintln!("snapshot: {}", one_line(&why));
            return ExitCode::FAILURE;
        }
    };
    let [file, folder, name, more @ ..] = args.as_slice() else {
        eprintln!(
            "usage: snapshot [--work <waiting file> [--told <shown file>]] <page file> <folder> <name> [width height \
             [words [option [theme file]]]]"
        );
        return ExitCode::from(2);
    };
    let side = |at: usize, fallback: f32| more.get(at).and_then(|side| side.parse().ok()).unwrap_or(fallback);
    let page = match std::fs::read_to_string(file).map_err(|e| e.to_string()).and_then(|text| read::page(&text)) {
        Ok(page) => page,
        Err(why) => {
            eprintln!("snapshot: {file}: {why}");
            return ExitCode::FAILURE;
        }
    };
    let typed = more.get(2).map(|words| (words.clone(), more.get(3).and_then(|at| at.parse().ok()).unwrap_or(0)));
    // A picture reads no logs: every project of the page has a chip, the first six. A page with none has the
    // entry a day with no project on offer has.
    let entry = Some(wishing::entry_of(page.data.read.projects.iter().take(6).cloned().collect()));
    let mut themes = vec![Theme::sticker(), Theme::plain()];
    // A theme file, to look at what its own colours make of the page: its picture is named after the theme.
    if let Some(file) = more.get(4) {
        match Theme::load(std::path::Path::new(file)) {
            Ok(loaded) => {
                // Said as the app says them: a key of the file is text, and may hold what a terminal obeys.
                loaded.warnings.iter().for_each(|warning| eprintln!("snapshot: {file}: {}", one_line(warning)));
                themes.push(loaded.theme);
            }
            Err(why) => {
                eprintln!("snapshot: {file}: {}", one_line(&why.to_string()));
                return ExitCode::FAILURE;
            }
        }
    }
    let mut shown = scene::scene(&page);
    // With no project the window's empty page says to choose a folder first: the same function says it here.
    if page.data.read.projects.is_empty() {
        starting::no_project(&mut shown);
    }
    shown.notes.splice(0..0, notes);
    for theme in themes {
        // A theme file's `name` is the file's text: cleaned, and only a part of the picture's name, never a path.
        let out = PathBuf::from(folder).join(format!("{name}-{}.png", file_part(&theme.name)));
        let (typing, scene) = (entry.clone().map(|entry| Typing { entry, typed: typed.clone() }), column(&shown));
        if let Err(error) = snapshot_typing(scene, theme, typing, side(0, 1440.0), side(1, 900.0), &out) {
            eprintln!("snapshot: could not render {}: {error}", out.display());
            return ExitCode::FAILURE;
        }
        println!("wrote {}", out.display());
    }
    ExitCode::SUCCESS
}
