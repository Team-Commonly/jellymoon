//! Renders the sample page and the page of every pencil picture under both built-in themes, with no window:
//! `cargo run --example snapshot --features headless -- <folder> [width height] [--at <seconds>]`
//! writes `kit-sticker.png`, `kit-plain.png`, `pencil-sticker.png` and `pencil-plain.png` into the folder. With
//! `--at`, every picture is drawn as it is that many seconds after its note came: a half-drawn page.

use std::path::PathBuf;

use sticker_kit::draw::headless::{snapshot, snapshot_at};
use sticker_kit::{Scene, Theme};

fn main() {
    let mut plain: Vec<String> = std::env::args().skip(1).collect();
    let flag = plain.iter().position(|arg| arg == "--at");
    let at = flag.and_then(|flag| plain.drain(flag..(flag + 2).min(plain.len())).nth(1)).and_then(|at| at.parse().ok());
    let mut args = plain.into_iter();
    let folder = PathBuf::from(args.next().unwrap_or_else(|| ".".into()));
    let mut number = |fallback: f32| args.next().and_then(|a| a.parse().ok()).unwrap_or(fallback);
    let (width, height) = (number(1440.0), number(900.0));
    for theme in [Theme::sticker(), Theme::plain()] {
        // The pictures' page is drawn in a window tall enough to show its whole column.
        for (name, scene, height) in [("kit", Scene::sample(), height), ("pencil", Scene::pictures(), height * 2.6)] {
            let out = folder.join(format!("{name}-{}.png", theme.name));
            let done = match at {
                Some(at) => snapshot_at(scene, theme.clone(), at, width, height, &out),
                None => snapshot(scene, theme.clone(), width, height, &out),
            };
            match done {
                Ok(()) => println!("wrote {}", out.display()),
                Err(error) => {
                    eprintln!("could not render {}: {error}", out.display());
                    std::process::exit(1);
                }
            }
        }
    }
}
