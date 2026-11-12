//! Renders a page-data file under both built-in themes, with no window, for looking at:
//! `cargo run -p jellymoon-app --example snapshot --features headless -- <page file> <folder> <name> [width height
//! [words [option]]]` writes `<name>-sticker.png` and `<name>-plain.png` into the folder. The page has the entry
//! the window gives a day's page: a chip for each of the page's projects a wish can be made for. With `words` the
//! entry holds them, with the caret after them, and the chip of place `option` (from 0) is the chosen one.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use jellymoon_app::today::Today;
use jellymoon_app::{read, scene};
use sticker_kit::Theme;
use sticker_kit::draw::headless::{Typing, snapshot_typing};

/// The page's day with nothing of a person's in it: its folders are names under the pictures' folder that nothing
/// makes, so no folder is switched off and nothing is read or kept.
fn day(page: &read::Page, folder: &Path) -> Today {
    Today {
        date: page.data.day.clone(),
        logs: folder.join("no-logs"),
        skip: Vec::new(),
        model: String::new(),
        home: folder.join("no-home"),
        harness: None,
        fresh: false,
        settings: None,
        bin: None,
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [file, folder, name, more @ ..] = args.as_slice() else {
        eprintln!("usage: snapshot <page file> <folder> <name> [width height [words [option]]]");
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
    let entry = day(&page, Path::new(folder)).entry(Some(&page));
    for theme in [Theme::sticker(), Theme::plain()] {
        let out = PathBuf::from(folder).join(format!("{name}-{}.png", theme.name));
        let typing = entry.clone().map(|entry| Typing { entry, typed: typed.clone() });
        if let Err(error) = snapshot_typing(scene::scene(&page), theme, typing, side(0, 1440.0), side(1, 900.0), &out) {
            eprintln!("snapshot: could not render {}: {error}", out.display());
            return ExitCode::FAILURE;
        }
        println!("wrote {}", out.display());
    }
    ExitCode::SUCCESS
}
