//! Renders a page-data file under both built-in themes, with no window, for looking at:
//! `cargo run -p jellymoon-app --example snapshot --features headless -- <page file> <folder> <name> [width height
//! [words [option]]]` writes `<name>-sticker.png` and `<name>-plain.png` into the folder. The page has the entry
//! the window gives a day's page, with a chip for each of the page's first six projects: a picture reads no logs,
//! so it does not ask which of them a wish can be made for. With `words` the
//! entry holds them, with the caret after them, and the chip of place `option` (from 0) is the chosen one.

use std::path::PathBuf;
use std::process::ExitCode;

use jellymoon_app::{read, scene, wishing};
use sticker_kit::Theme;
use sticker_kit::draw::headless::{Typing, snapshot_typing};

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
    // A picture reads no logs: every project of the page has a chip, the first six.
    let entry = wishing::entry_of(page.data.read.projects.iter().take(6).cloned().collect());
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
