//! Renders the sample page under both built-in themes, with no window:
//! `cargo run --example snapshot --features headless -- <folder> [width height]`
//! writes `kit-sticker.png` and `kit-plain.png` into the folder.

use std::path::PathBuf;

use sticker_kit::draw::headless::snapshot;
use sticker_kit::{Scene, Theme};

fn main() {
    let mut args = std::env::args().skip(1);
    let folder = PathBuf::from(args.next().unwrap_or_else(|| ".".into()));
    let mut number = |fallback: f32| args.next().and_then(|a| a.parse().ok()).unwrap_or(fallback);
    let (width, height) = (number(1440.0), number(900.0));
    for theme in [Theme::sticker(), Theme::plain()] {
        let out = folder.join(format!("kit-{}.png", theme.name));
        match snapshot(Scene::sample(), theme, width, height, &out) {
            Ok(()) => println!("wrote {}", out.display()),
            Err(error) => {
                eprintln!("could not render {}: {error}", out.display());
                std::process::exit(1);
            }
        }
    }
}
