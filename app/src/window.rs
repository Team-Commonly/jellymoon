//! The window: the kit's view of one scene, and of the scene that replaces it when a day has been read. `t` switches
//! between the two built-in themes. A tap is written to stderr as one line: the piece's id, and the option when it
//! was one, never a word of the page. A tap on an option of a day's page then goes to `taps.rs`, and the page it
//! gives is shown. Enter in a day's entry goes to `wishing.rs`: the page it gives is shown at once, the wish's run is
//! asked for off this thread, and once a second the day is looked at again, so a run's end is drawn with no tap.
//!
//! The view measures text with GPUI's shaped glyphs, not with the bundled advances the `layout` command uses: shaping
//! kerns, and the plain theme draws in the system's font (`docs/development/app.md` has the measured gap).

use std::cell::{Cell, RefCell};
use std::io::Write;
use std::rc::Rc;
use std::time::Duration;

use gpui::{App, Bounds, Pixels, TitlebarOptions, WindowBounds, WindowHandle, WindowOptions, prelude::*, px, size};
use gpui_platform::application;
use sticker_kit::draw::{EntryMade, Page, Tap, load_fonts};
use sticker_kit::{Scene, Theme, Viewport};

use crate::taps::OnShow;
use crate::today::Later;

/// The day's page on show, which the tap handler and the entry act on and the reading and a look replace. All run
/// on the window's thread.
type Shown = Rc<RefCell<Option<OnShow>>>;

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
        let shown: Shown = Rc::new(RefCell::new(shown));
        let window = match page(scene, theme, viewport, shown.clone(), cx) {
            Ok(window) => window,
            Err(error) => {
                say(&format!("no window: {error}"));
                std::process::exit(1);
            }
        };
        cx.on_window_closed(|cx, _| cx.quit()).detach();
        cx.observe_keystrokes(move |event, _, cx| {
            if event.keystroke.key == "t" {
                // A `t` typed in the entry is a letter of a wish.
                let _ = window.update(cx, |page, window, cx| {
                    if !page.entry_focused(window) {
                        switch_theme(page, cx);
                    }
                });
            }
        })
        .detach();
        let reading = shown.clone();
        cx.spawn(async move |cx| {
            let mut later = later;
            while let Some(work) = later.take() {
                let Some(next) = cx.background_executor().spawn(async move { work() }).await else { break };
                let entry = reading.borrow_mut().as_mut().and_then(|shown| {
                    shown.show(&next);
                    shown.entry()
                });
                let _ = window.update(cx, |page, _, cx| {
                    page.set_entry(entry, cx);
                    page.set_scene(next.scene, cx);
                });
                later = next.later;
            }
        })
        .detach();
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

/// Looks at the day on show once a second, off the window's thread, and shows what a look gives when it differs
/// from what is shown: a run that ended, a wish made in a terminal. It ends with the window, or at once for a page
/// with no day.
async fn looks(shown: Shown, window: WindowHandle<Page>, cx: &mut gpui::AsyncApp) {
    loop {
        cx.background_executor().timer(LOOK_EVERY).await;
        let Some(look) = shown.borrow().as_ref().map(OnShow::look) else { return };
        let Some(seen) = cx.background_executor().spawn(async move { look() }).await else { continue };
        let Some(scene) = shown.borrow_mut().as_mut().and_then(|shown| shown.seen(seen)) else { continue };
        if window.update(cx, |page, _, cx| page.set_scene(scene, cx)).is_err() {
            return;
        }
    }
}

/// Enter in the entry: the wish goes to `wishing.rs`, the page it gives is shown at once, and the asking for the
/// wish's run is run off the window's thread. What that has to say is shown when it ends.
fn entered(made: &EntryMade, shown: &Shown, window: WindowHandle<Page>, cx: &mut App) {
    say(&format!("entry id={} characters={}", made.id, made.text.chars().count()));
    let Some((scene, asking)) = shown.borrow_mut().as_mut().map(|shown| shown.wished(&made.text, made.option)) else {
        return;
    };
    // The view is in the middle of handling the key: it takes the new scene once that is done.
    cx.defer(move |cx| {
        let _ = window.update(cx, |page, _, cx| page.set_scene(scene, cx));
    });
    let (Some(asking), shown) = (asking, shown.clone()) else { return };
    cx.spawn(async move |cx| {
        let said = cx.background_executor().spawn(async move { asking() }).await;
        let Some(scene) = shown.borrow_mut().as_mut().and_then(|shown| shown.asked(said)) else { return };
        let _ = window.update(cx, |page, _, cx| page.set_scene(scene, cx));
    })
    .detach();
}

fn page(
    scene: Scene,
    theme: Theme,
    viewport: Viewport,
    shown: Shown,
    cx: &mut App,
) -> gpui::Result<WindowHandle<Page>> {
    let bounds = Bounds::centered(None, size(px(viewport.w), px(viewport.h)), cx);
    // The handler is made before the window it will redraw, so the window's handle reaches it through this.
    let opened: Rc<Cell<Option<WindowHandle<Page>>>> = Rc::default();
    let (handle, on_show, entry) = (opened.clone(), shown.clone(), shown.borrow().as_ref().and_then(OnShow::entry));
    let tapped = move |tap: &Tap, _: &mut gpui::Window, cx: &mut App| {
        received(tap);
        let (Tap::Option { note_id, index }, Some(window)) = (tap, handle.get()) else { return };
        let Some(after) = on_show.borrow_mut().as_mut().and_then(|shown| shown.tap(note_id, *index)) else { return };
        // The view is in the middle of handling the tap: it takes the new scene once that is done.
        cx.defer(move |cx| {
            let _ = window.update(cx, |page, _, cx| page.set_scene(after, cx));
        });
    };
    let handle = opened.clone();
    let made = move |made: &EntryMade, _: &mut gpui::Window, cx: &mut App| {
        if let Some(window) = handle.get() {
            entered(made, &shown, window, cx);
        }
    };
    let view = move |_: &mut gpui::Window, cx: &mut App| {
        cx.new(|_| {
            let page = Page::new(scene, theme).on_tap(tapped).on_entry(made);
            match entry {
                Some(entry) => page.with_entry(entry),
                None => page,
            }
        })
    };
    let window = cx.open_window(options(bounds), view)?;
    opened.set(Some(window));
    Ok(window)
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

fn switch_theme(page: &mut Page, cx: &mut Context<Page>) {
    let next = if page.theme().name == "plain" { Theme::sticker() } else { Theme::plain() };
    page.set_theme(next, cx);
}

/// One line for a tap: the piece's id, and the option when it was one.
fn received(tap: &Tap) {
    match tap {
        Tap::Character { id } => say(&format!("tap id={id}")),
        Tap::Option { note_id, index } => say(&format!("tap id={note_id} option={index}")),
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
