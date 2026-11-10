//! Opens the sample page in a window: `cargo run --example page`.
//!
//! Press `t` to switch between the two built-in themes. Environment:
//! `KIT_QUIT_AFTER=<seconds>` closes the window by itself,
//! `KIT_CHARACTERS=<n>` shows a crowd of `n` instead of the sample,
//! `KIT_THEME=plain` starts in the plain theme. Frame rate is logged to
//! standard error once a second.

use std::time::{Duration, Instant};

use gpui::{App, Bounds, WindowBounds, WindowHandle, WindowOptions, prelude::*, px, size};
use gpui_platform::application;
use sticker_kit::draw::{Page, load_fonts};
use sticker_kit::{Scene, Theme};

fn env_number(name: &str) -> Option<f32> {
    std::env::var(name).ok()?.parse().ok()
}

fn open(cx: &mut App) -> gpui::Result<WindowHandle<Page>> {
    let scene = match env_number("KIT_CHARACTERS") {
        Some(count) => Scene::crowd(count as usize),
        None => Scene::sample(),
    };
    let theme = std::env::var("KIT_THEME").ok().and_then(|name| Theme::builtin(&name)).unwrap_or_else(Theme::sticker);
    let bounds = Bounds::centered(None, size(px(1440.0), px(900.0)), cx);
    let options = WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() };
    cx.open_window(options, |_, cx| cx.new(|_| Page::new(scene, theme).on_tap(|tap, _, _| println!("tapped {tap:?}"))))
}

fn switch_theme(page: &mut Page, cx: &mut Context<Page>) {
    let next = if page.theme().name == "plain" { Theme::sticker() } else { Theme::plain() };
    page.set_theme(next, cx);
}

fn main() {
    let quit_after = env_number("KIT_QUIT_AFTER");
    application().run(move |cx: &mut App| {
        if let Err(error) = load_fonts(cx) {
            eprintln!("bundled fonts did not load: {error}");
        }
        let window = match open(cx) {
            Ok(window) => window,
            Err(error) => {
                eprintln!("no window: {error}");
                cx.quit();
                return;
            }
        };
        cx.on_window_closed(|cx, _| cx.quit()).detach();
        cx.observe_keystrokes(move |event, _, cx| {
            if event.keystroke.key == "t" {
                let _ = window.update(cx, |page, _, cx| switch_theme(page, cx));
            }
        })
        .detach();
        cx.spawn(async move |cx| {
            let started = Instant::now();
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let seconds = started.elapsed().as_secs_f32();
                let stats = window.update(cx, |page, _, _| page.take_frame_stats());
                if let Ok(stats) = stats {
                    eprintln!("t={seconds:.0}s fps={} worst_ms={:.1}", stats.frames, stats.worst_ms);
                }
                if quit_after.is_some_and(|limit| seconds >= limit) {
                    cx.update(|cx| cx.quit());
                    return;
                }
            }
        })
        .detach();
        cx.activate(true);
    });
}
