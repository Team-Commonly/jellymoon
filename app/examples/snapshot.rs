//! Renders a page-data file under both built-in themes, with no window, for looking at:
//! `cargo run -p jellymoon-app --example snapshot --features headless -- <page file> <folder> <name> [width height]`
//! writes `<name>-sticker.png` and `<name>-plain.png` into the folder.

use std::path::PathBuf;
use std::process::ExitCode;

use jellymoon_app::{read, scene};
use sticker_kit::Theme;
use sticker_kit::draw::headless::snapshot;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [file, folder, name, size @ ..] = args.as_slice() else {
        eprintln!("usage: snapshot <page file> <folder> <name> [width height]");
        return ExitCode::from(2);
    };
    let side = |at: usize, fallback: f32| size.get(at).and_then(|side| side.parse().ok()).unwrap_or(fallback);
    let page = match std::fs::read_to_string(file).map_err(|e| e.to_string()).and_then(|text| read::page(&text)) {
        Ok(page) => page,
        Err(why) => {
            eprintln!("snapshot: {file}: {why}");
            return ExitCode::FAILURE;
        }
    };
    for theme in [Theme::sticker(), Theme::plain()] {
        let out = PathBuf::from(folder).join(format!("{name}-{}.png", theme.name));
        if let Err(error) = snapshot(scene::scene(&page), theme, side(0, 1440.0), side(1, 900.0), &out) {
            eprintln!("snapshot: could not render {}: {error}", out.display());
            return ExitCode::FAILURE;
        }
        println!("wrote {}", out.display());
    }
    ExitCode::SUCCESS
}
