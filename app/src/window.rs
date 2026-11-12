//! The window: the kit's view of one scene, and of the scene that replaces it when a day has been read. `t` switches
//! between the two built-in themes. A tap is written to stderr as one line: the piece's id, and the option when it
//! was one, never a word of the page. A tap on an option of a day's page then goes to `taps.rs`, and the page it
//! gives is shown. Enter in a day's entry goes to `wishing.rs`, and a tap on a wish's character or on the choice to
//! stop its run to `stopping.rs`: core is asked off this thread, and the page is shown as soon as it has answered.
//! Once a second the day is looked at again, so a run's end is drawn with no tap.
//!
//! This file is wiring. What each act does is `showing.rs`, which a test calls with no window: here its work is
//! run off the window's thread, and each scene it gives is handed to the kit's view. No test opens this window on
//! a screen: `tests/f43/window.rs` opens [`page`] on GPUI's test platform and presses Enter in it.
//!
//! The view measures text with GPUI's shaped glyphs, not with the bundled advances the `layout` command uses: shaping
//! kerns, and the plain theme draws in the system's font (`docs/development/app.md` has the measured gap).

use std::io::Write;
use std::time::Duration;

use gpui::{App, Bounds, Pixels, TitlebarOptions, WindowBounds, WindowHandle, WindowOptions, prelude::*, px, size};
use gpui_platform::application;
use sticker_kit::draw::{EntryMade, Page, Tap, load_fonts};
use sticker_kit::{Scene, Theme, Viewport};

use crate::showing::{Entered, Showing, entry_line, other_theme, switches_theme, tap_line};
use crate::taps::OnShow;
use crate::today::Later;

/// How often a day on show is looked at again.
const LOOK_EVERY: Duration = Duration::from_secs(1);

/// Opens the window on `scene` and runs until it is closed, or until `quit_after` has passed. A day's `later` is
/// started at once, off the window's thread; each scene it gives replaces the one shown, and the work after it
/// starts. A page-data file has no day: it is drawn, and a tap on it does nothing.
pub fn open(
    scene: Scene,
    theme: Theme,
    viewport: Viewport,
    quit_after: Option<Duration>,
    day: Option<(OnShow, Later)>,
) {
    application().run(move |cx: &mut App| {
        if let Err(error) = load_fonts(cx) {
            say(&format!("the bundled fonts did not load: {error}"));
        }
        let (shown, later) = day.unzip();
        let shown = Showing::of(shown);
        let window = match page(scene, theme.clone(), viewport, shown.clone(), cx) {
            Ok(window) => window,
            Err(error) => {
                say(&format!("no window: {error}"));
                std::process::exit(1);
            }
        };
        cx.on_window_closed(|cx, _| cx.quit()).detach();
        cx.observe_keystrokes(move |event, _, cx| key(&event.keystroke.key, window, cx)).detach();
        cx.spawn(async move |cx| arts(theme, window, cx).await).detach();
        let reading = shown.clone();
        cx.spawn(async move |cx| reads(reading, later, window, cx).await).detach();
        cx.spawn(async move |cx| looks(shown, window, cx).await).detach();
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

/// A key pressed anywhere in the window: the one that switches the theme does.
fn key(key: &str, window: WindowHandle<Page>, cx: &mut App) {
    let _ = window.update(cx, |page, window, cx| {
        if switches_theme(key, page.entry_focused(window)) {
            page.set_theme(other_theme(&page.theme().name), cx);
        }
    });
}

/// The theme's own art, each picture tried in a process of its own, off the window's thread (ADR 0016). The page
/// is drawn with the kit's art from the start, and with each picture that passed once all have been tried: at
/// once when their answers are kept, after the trials the first time a picture is seen. A picture that is refused
/// is said on stderr and the kit's art stays.
async fn arts(theme: Theme, window: WindowHandle<Page>, cx: &mut gpui::AsyncApp) {
    if theme.folder().is_none() {
        return;
    }
    let trying = theme.clone();
    let tried = cx.background_executor().spawn(async move { trying.try_art(&crate::trial()) }).await;
    for refused in &tried.refused {
        say(&crate::refused_art(refused));
    }
    // The art is the theme's it was tried for: `t` may have put another on show meanwhile.
    let _ = window.update(cx, |page, _, cx| {
        if *page.theme() == theme {
            page.set_art(tried.accepted, cx);
        }
    });
}

/// The entry of the first page, then each scene the day's reading gives with the entry of its page. Which
/// projects have a chip is asked of the session logs, so all of it is off the window's thread: the first page has
/// no entry until core has answered.
async fn reads(shown: Showing, later: Option<Later>, window: WindowHandle<Page>, cx: &mut gpui::AsyncApp) {
    let Some(first) = shown.offering() else { return };
    let entry = cx.background_executor().spawn(async move { first() }).await;
    let _ = window.update(cx, |page, _, cx| page.set_entry(entry, cx));
    let mut later = later;
    while let Some(work) = later.take().and_then(|step| shown.reading(step)) {
        let Some((next, entry)) = cx.background_executor().spawn(async move { work() }).await else { break };
        let Some(scene) = shown.read(&next) else { return };
        let _ = window.update(cx, |page, _, cx| {
            page.set_entry(entry, cx);
            page.set_scene(scene, cx);
        });
        later = next.later;
    }
}

/// Looks at the day on show once a second, off the window's thread, and shows what a look gives when it differs
/// from what is shown: a run that ended, a wish made in a terminal. It ends with the window, or at once for a page
/// with no day.
async fn looks(shown: Showing, window: WindowHandle<Page>, cx: &mut gpui::AsyncApp) {
    loop {
        cx.background_executor().timer(LOOK_EVERY).await;
        let Some(look) = shown.look() else { return };
        let Some(seen) = cx.background_executor().spawn(async move { look() }).await else { continue };
        let Some(scene) = shown.seen(seen) else { continue };
        if window.update(cx, |page, _, cx| page.set_scene(scene, cx)).is_err() {
            return;
        }
    }
}

/// Enter in the entry. The entry keeps its words and waits while core is asked off the window's thread; then it is
/// answered and the page shown. Then the asking for the wish's run is run, off the window's thread too, and what
/// it has to say is shown when it ends.
fn entered(made: &EntryMade, shown: &Showing, window: WindowHandle<Page>, cx: &mut App) {
    say(&entry_line(&made.id, &made.text));
    let (wishing, shown) = (shown.wishing(&made.text, made.option.as_deref()), shown.clone());
    cx.spawn(async move |cx| {
        let answer = match wishing {
            Some(wishing) => Some(cx.background_executor().spawn(async move { wishing() }).await),
            None => None,
        };
        let Entered { kept, scene, asking } = shown.answered(answer);
        let _ = window.update(cx, |page, window, cx| {
            page.answer_entry(kept, window, cx);
            if let Some(scene) = scene {
                page.set_scene(scene, cx);
            }
        });
        let Some(asking) = asking else { return };
        let said = cx.background_executor().spawn(async move { asking() }).await;
        let Some(scene) = shown.asked(said) else { return };
        let _ = window.update(cx, |page, _, cx| page.set_scene(scene, cx));
    })
    .detach();
}

/// A tap on a character: core is asked off the window's thread, and what it gives is shown.
fn poked(id: &str, shown: &Showing, window: WindowHandle<Page>, cx: &mut App) {
    let (Some(look), shown) = (shown.poked(id), shown.clone()) else { return };
    cx.spawn(async move |cx| {
        let Some(seen) = cx.background_executor().spawn(async move { look() }).await else { return };
        let Some(scene) = shown.seen(seen) else { return };
        let _ = window.update(cx, |page, _, cx| page.set_scene(scene, cx));
    })
    .detach();
}

/// A tap on an option: its scene is shown at once, and a stop, which waits for the run to end, is run off the
/// window's thread.
fn chose(note: &str, index: usize, shown: &Showing, window: WindowHandle<Page>, cx: &mut App) {
    let Some((after, stop)) = shown.chose(note, index) else { return };
    // The view is in the middle of handling the tap: it takes the new scene once that is done.
    cx.defer(move |cx| {
        let _ = window.update(cx, |page, _, cx| page.set_scene(after, cx));
    });
    let (Some(stop), shown) = (stop, shown.clone()) else { return };
    cx.spawn(async move |cx| {
        let stopped = cx.background_executor().spawn(async move { stop() }).await;
        let Some(scene) = shown.stopped(stopped) else { return };
        let _ = window.update(cx, |page, _, cx| page.set_scene(scene, cx));
    })
    .detach();
}

/// The window of one page, its taps and its Enter wired to `shown`. [`open`] opens it on the screen; a test opens
/// it on GPUI's test platform, where there is no screen.
pub fn page(
    scene: Scene,
    theme: Theme,
    viewport: Viewport,
    shown: Showing,
    cx: &mut App,
) -> gpui::Result<WindowHandle<Page>> {
    let bounds = Bounds::centered(None, size(px(viewport.w), px(viewport.h)), cx);
    let on_show = shown.clone();
    let tapped = move |tap: &Tap, window: &mut gpui::Window, cx: &mut App| {
        let (id, option) = match tap {
            Tap::Character { id } => (id, None),
            Tap::Option { note_id, index } => (note_id, Some(*index)),
        };
        say(&tap_line(id, option));
        match option {
            None => poked(id, &on_show, own(window), cx),
            Some(index) => chose(id, index, &on_show, own(window), cx),
        }
    };
    let made = move |made: &EntryMade, window: &mut gpui::Window, cx: &mut App| {
        entered(made, &shown, own(window), cx);
    };
    // The entry comes with the first answer of `reads`.
    let view =
        move |_: &mut gpui::Window, cx: &mut App| cx.new(|_| Page::new(scene, theme).on_tap(tapped).on_entry(made));
    cx.open_window(options(bounds), view)
}

/// The handle of the window a handler of the page was called in. The page is that window's root view, and a
/// handler is given its window: there is no handle to wait for or to miss, so an Enter is always answered.
fn own(window: &gpui::Window) -> WindowHandle<Page> {
    WindowHandle::new(window.window_handle().window_id())
}

/// What the window is asked for: its first place, a title, and never to be smaller than a page is drawn at.
fn options(bounds: Bounds<Pixels>) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(px(crate::SMALLEST.w), px(crate::SMALLEST.h))),
        titlebar: Some(TitlebarOptions { title: Some("Jellymoon".into()), ..Default::default() }),
        ..Default::default()
    }
}

fn say(line: &str) {
    let _ = std::io::stderr().write_all(format!("jellymoon-app: {line}\n").as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This holds what the window is asked for. That the system honours it was seen by hand on the Mac only.
    #[test]
    fn the_window_is_asked_never_to_be_smaller_than_the_smallest_page() {
        let asked = options(Bounds::default());
        assert_eq!(asked.window_min_size, Some(size(px(960.0), px(640.0))));
        assert_eq!((crate::SMALLEST.w, crate::SMALLEST.h), (960.0, 640.0));
    }
}
