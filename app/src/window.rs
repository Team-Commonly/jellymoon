//! The window: the kit's view of one scene. `t` switches between the two built-in themes. A tap on a little one or on
//! a choice's option is received and, until rulings exist (F29), only written to stderr as one line.
//!
//! The view measures text with GPUI's shaped glyphs, not with the bundled advances the `layout` command uses: shaping
//! kerns, and the plain theme draws in the system's font (`docs/development/app.md` has the measured gap).

use std::io::Write;
use std::time::Duration;

use gpui::{App, Bounds, TitlebarOptions, WindowBounds, WindowHandle, WindowOptions, prelude::*, px, size};
use gpui_platform::application;
use sticker_kit::draw::{Page, Tap, load_fonts};
use sticker_kit::{Scene, Theme, Viewport};

/// Opens the window and runs until it is closed, or until `quit_after` has passed.
pub fn open(scene: Scene, theme: Theme, viewport: Viewport, quit_after: Option<Duration>) {
    application().run(move |cx: &mut App| {
        if let Err(error) = load_fonts(cx) {
            say(&format!("the bundled fonts did not load: {error}"));
        }
        let window = match page(scene, theme, viewport, cx) {
            Ok(window) => window,
            Err(error) => {
                say(&format!("no window: {error}"));
                std::process::exit(1);
            }
        };
        cx.on_window_closed(|cx, _| cx.quit()).detach();
        cx.observe_keystrokes(move |event, _, cx| {
            if event.keystroke.key == "t" {
                let _ = window.update(cx, |page, _, cx| switch_theme(page, cx));
            }
        })
        .detach();
        if let Some(limit) = quit_after {
            cx.spawn(async move |cx| {
                cx.background_executor().timer(limit).await;
                cx.update(|cx| cx.quit());
            })
            .detach();
        }
        cx.activate(true);
    });
}

fn page(scene: Scene, theme: Theme, viewport: Viewport, cx: &mut App) -> gpui::Result<WindowHandle<Page>> {
    let bounds = Bounds::centered(None, size(px(viewport.w), px(viewport.h)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(px(crate::SMALLEST.w), px(crate::SMALLEST.h))),
        titlebar: Some(TitlebarOptions { title: Some("Jellymoon".into()), ..Default::default() }),
        ..Default::default()
    };
    cx.open_window(options, |_, cx| cx.new(|_| Page::new(scene, theme).on_tap(|tap, _, _| received(tap))))
}

fn switch_theme(page: &mut Page, cx: &mut Context<Page>) {
    let next = if page.theme().name == "plain" { Theme::sticker() } else { Theme::plain() };
    page.set_theme(next, cx);
}

/// What a tap will become is F29's. Today it is one line: the piece's id, and the option when it was one.
fn received(tap: &Tap) {
    match tap {
        Tap::Character { id } => say(&format!("tap id={id}")),
        Tap::Option { note_id, index } => say(&format!("tap id={note_id} option={index}")),
    }
}

fn say(line: &str) {
    let _ = std::io::stderr().write_all(format!("jellymoon-app: {line}\n").as_bytes());
}
